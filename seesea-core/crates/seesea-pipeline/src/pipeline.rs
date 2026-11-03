//! Orchestration: the stages in order, with timeouts, degradation and caching.

use crate::content::{self, CachedFetcher, Loaded, NetFetcher, PageFetcher};
use crate::error::{Error, FailureKind, Result, SourceError, SourceFailure};
use crate::fusion::{self, FusionConfig, SourceResults};
use crate::model::*;
use crate::rank::{self, QueryInfo, RankConfig};
use crate::rerank::{DocView, LexicalReranker, Reranker};
use crate::sections::{self, SectionScorer};
use crate::source::{Source, SourceQuery};
use crate::store::{Cache, CacheCounts, Metered, NoCache};
use crate::text::{jaccard, normalize_query, token_set};
use chrono::Utc;
use seesea_extract::{Extractor, Meta, Scope};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet, HashSet, VecDeque};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

pub struct PipelineConfig {
    pub fusion: FusionConfig,
    pub rank: RankConfig,
    /// Longest a single source may take before it is counted as failed.
    pub source_timeout: Duration,
    pub page_timeout: Duration,
    /// Pages fetched at the same time.
    pub concurrency: usize,
    /// How many of the best candidates are fetched in standard and deep mode (never fewer than `limit`).
    pub fetch_standard: usize,
    pub fetch_deep: usize,
    pub search_cache_ttl: Duration,
    /// Starting lifetime of a cached page; it then adapts per URL between `page_ttl_min` and
    /// `page_ttl_max` depending on whether the page changed when it was last refetched.
    pub page_cache_ttl: Duration,
    pub page_ttl_min: Duration,
    pub page_ttl_max: Duration,
    /// Serve a cached response for a query whose meaningful words overlap this much (Jaccard) with
    /// an earlier one. Off by default: similar is not the same, and `seesea replay` shows what it costs.
    pub fuzzy_cache_threshold: Option<f64>,
}

impl Default for PipelineConfig {
    fn default() -> Self {
        Self {
            fusion: FusionConfig::default(),
            rank: RankConfig::default(),
            source_timeout: Duration::from_secs(8),
            page_timeout: Duration::from_secs(10),
            concurrency: 8,
            fetch_standard: 8,
            fetch_deep: 20,
            search_cache_ttl: Duration::from_secs(600),
            page_cache_ttl: Duration::from_secs(3600),
            page_ttl_min: Duration::from_secs(300),
            page_ttl_max: Duration::from_secs(7 * 24 * 3600),
            fuzzy_cache_threshold: None,
        }
    }
}

pub struct Pipeline {
    sources: Vec<Arc<dyn Source>>,
    fetcher: Arc<dyn PageFetcher>,
    /// Fetches pages through a headless browser; used when the static HTML holds no content.
    renderer: Option<Arc<dyn PageFetcher>>,
    /// Nudges stage-1 scores by what hosts and sources delivered in earlier searches.
    learner: Option<Arc<crate::learn::Learner>>,
    journal: Option<Arc<crate::journal::Journal>>,
    extractor: OnceLock<Arc<Extractor>>,
    reranker: Arc<dyn Reranker>,
    section_scorer: Option<Arc<dyn SectionScorer>>,
    cache: Arc<dyn Cache>,
    metered: Option<Arc<Metered>>,
    recent_queries: Mutex<VecDeque<RecentQuery>>,
    cfg: PipelineConfig,
}

/// An earlier search kept so near-identical queries can reuse its cached response.
struct RecentQuery {
    /// Everything in the cache key except the query.
    scope: String,
    tokens: HashSet<String>,
    key: String,
}

const RECENT_QUERIES: usize = 1000;

/// The result of `extract`: one page, with the sections that matter for an optional query.
#[derive(Debug, Clone, Serialize)]
pub struct ExtractedPage {
    pub url: String,
    pub final_url: String,
    pub meta: Meta,
    pub sections: Vec<SelectedSection>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub markdown: Option<String>,
    /// Comment and related-link blocks that were left out of the main content.
    pub asides: Vec<seesea_extract::Section>,
    pub confidence: f32,
    pub needs_render: bool,
    /// The content was produced by a headless browser running the page's scripts.
    #[serde(default)]
    pub rendered: bool,
    pub truncated: bool,
}

/// What one source did when asked for a test query.
#[derive(Debug, Clone, Serialize)]
pub struct SourceProbe {
    pub source: String,
    pub ok: bool,
    pub hits: usize,
    pub ms: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<SourceFailure>,
}

/// Host of a result URL, without a leading `www.`; empty when it does not parse.
pub fn host_of(url: &str) -> String {
    url::Url::parse(url)
        .ok()
        .and_then(|u| u.host_str().map(|h| h.trim_start_matches("www.").to_lowercase()))
        .unwrap_or_default()
}

fn ms(since: Instant) -> u64 {
    since.elapsed().as_millis() as u64
}

impl Pipeline {
    pub fn new(sources: Vec<Arc<dyn Source>>, fetcher: Arc<dyn PageFetcher>) -> Self {
        Self {
            sources,
            fetcher,
            renderer: None,
            learner: None,
            journal: None,
            extractor: OnceLock::new(),
            reranker: Arc::new(LexicalReranker),
            section_scorer: None,
            cache: Arc::new(NoCache),
            metered: None,
            recent_queries: Mutex::new(VecDeque::new()),
            cfg: PipelineConfig::default(),
        }
    }

    /// A pipeline that fetches pages over the network.
    pub fn with_net_fetcher(
        sources: Vec<Arc<dyn Source>>,
        options: seesea_fetch::FetchOptions,
    ) -> Self {
        Self::new(sources, Arc::new(NetFetcher::new(options)))
    }

    /// Fetch pages whose static HTML holds no content again through `renderer` (a headless
    /// browser). Its pages are cached like any other.
    pub fn renderer(mut self, renderer: Arc<dyn PageFetcher>) -> Self {
        self.renderer = Some(self.cached_renderer(renderer));
        self
    }

    /// Learn stage-1 weights per host and source from what the later stages find.
    pub fn learner(mut self, learner: Arc<crate::learn::Learner>) -> Self {
        self.learner = Some(learner);
        self
    }

    pub fn journal(mut self, journal: Arc<crate::journal::Journal>) -> Self {
        self.journal = Some(journal);
        self
    }

    pub fn journal_handle(&self) -> Option<&Arc<crate::journal::Journal>> {
        self.journal.as_ref()
    }

    pub fn learner_handle(&self) -> Option<&Arc<crate::learn::Learner>> {
        self.learner.as_ref()
    }

    fn cached_renderer(&self, renderer: Arc<dyn PageFetcher>) -> Arc<dyn PageFetcher> {
        if self.metered.is_none() {
            return renderer;
        }
        Arc::new(
            CachedFetcher::with_ttl_range(
                renderer,
                Arc::clone(&self.cache),
                self.cfg.page_cache_ttl,
                self.cfg.page_ttl_min,
                self.cfg.page_ttl_max,
            )
            .with_prefix("render:"),
        )
    }

    pub fn config(mut self, cfg: PipelineConfig) -> Self {
        self.cfg = cfg;
        self
    }

    pub fn reranker(mut self, reranker: Arc<dyn Reranker>) -> Self {
        self.reranker = reranker;
        self
    }

    /// Choose page sections by meaning instead of by shared words alone.
    pub fn section_scorer(mut self, scorer: Arc<dyn SectionScorer>) -> Self {
        self.section_scorer = Some(scorer);
        self
    }

    /// Select sections on a blocking thread (a model may be involved).
    async fn select_sections(
        &self,
        core: String,
        query: String,
        budget: usize,
    ) -> (Vec<SelectedSection>, bool) {
        let scorer = self.section_scorer.clone();
        let lexical = (core.clone(), query.clone());
        tokio::task::spawn_blocking(move || {
            sections::select_with(&core, &query, budget, scorer.as_deref())
        })
        .await
        .unwrap_or_else(|_| sections::select(&lexical.0, &lexical.1, budget))
    }

    pub fn extractor(self, extractor: Arc<Extractor>) -> Self {
        let _ = self.extractor.set(extractor);
        self
    }

    /// Cache search responses and fetched pages.
    pub fn cache(mut self, cache: Arc<dyn Cache>) -> Self {
        let metered = Arc::new(Metered::new(cache));
        let cache: Arc<dyn Cache> = metered.clone();
        self.fetcher = Arc::new(CachedFetcher::with_ttl_range(
            Arc::clone(&self.fetcher),
            Arc::clone(&cache),
            self.cfg.page_cache_ttl,
            self.cfg.page_ttl_min,
            self.cfg.page_ttl_max,
        ));
        self.cache = cache;
        self.metered = Some(metered);
        if let Some(r) = self.renderer.take() {
            self.renderer = Some(self.cached_renderer(r));
        }
        self
    }

    /// Hits, misses and writes per kind of cache entry (`search`, `page`, ...).
    pub fn cache_stats(&self) -> BTreeMap<String, CacheCounts> {
        self.metered.as_ref().map(|m| m.stats()).unwrap_or_default()
    }

    pub fn source_names(&self) -> Vec<String> {
        self.sources.iter().map(|s| s.name().to_string()).collect()
    }

    /// Ask every source the same question and report, per source, whether it answered, with how many
    /// results and how fast. A source returning zero results counts as not ok: it usually means a
    /// blocked request or a page layout the parser no longer understands.
    pub async fn probe(&self, query: &str) -> Vec<SourceProbe> {
        let sq = SourceQuery {
            query: query.to_string(),
            limit: 10,
            page: 1,
            time_range: None,
            language: None,
        };
        futures::future::join_all(self.sources.iter().map(|s| {
            let (sq, s, timeout) = (sq.clone(), Arc::clone(s), self.cfg.source_timeout);
            async move {
                let t = Instant::now();
                let r = tokio::time::timeout(timeout, s.search(&sq))
                    .await
                    .unwrap_or_else(|_| Err(SourceError::new(FailureKind::Timeout, format!("no answer within {} ms", timeout.as_millis()))));
                let name = s.name().to_string();
                match r {
                    Ok(hits) if !hits.is_empty() => SourceProbe { source: name, ok: true, hits: hits.len(), ms: ms(t), error: None },
                    Ok(_) => SourceProbe {
                        source: name.clone(),
                        ok: false,
                        hits: 0,
                        ms: ms(t),
                        error: Some(SourceFailure { source: name, kind: FailureKind::Parse, message: "answered but returned no results (blocked, or the page layout changed)".into() }),
                    },
                    Err(e) => SourceProbe { source: name.clone(), ok: false, hits: 0, ms: ms(t), error: Some(SourceFailure { source: name, kind: e.kind, message: e.message }) },
                }
            }
        }))
        .await
    }

    /// The model is large to load, so it is built on first use: fast searches never pay for it.
    fn extractor_handle(&self) -> &Arc<Extractor> {
        self.extractor.get_or_init(|| Arc::new(Extractor::new()))
    }

    fn select_sources(&self, wanted: &[String], recency: rank::Recency) -> Result<Vec<Arc<dyn Source>>> {
        if self.sources.is_empty() {
            return Err(Error::NoSources);
        }
        if wanted.is_empty() {
            let general: Vec<_> = self.sources
                .iter()
                .filter(|s| s.on_by_default() && (!s.recency_only() || recency > rank::Recency::Evergreen))
                .cloned()
                .collect();
            return Ok(if general.is_empty() { self.sources.clone() } else { general });
        }
        let known = self.source_names();
        if let Some(bad) = wanted
            .iter()
            .find(|w| !known.iter().any(|k| k.eq_ignore_ascii_case(w)))
        {
            return Err(Error::InvalidRequest(format!(
                "unknown source '{bad}', available: {}",
                known.join(", ")
            )));
        }
        Ok(self
            .sources
            .iter()
            .filter(|s| wanted.iter().any(|w| w.eq_ignore_ascii_case(s.name())))
            .cloned()
            .collect())
    }

    /// The part of the cache key that does not depend on the query text.
    fn cache_scope(req: &SearchRequest, sources: &[Arc<dyn Source>]) -> String {
        let mut names: Vec<&str> = sources.iter().map(|s| s.name()).collect();
        names.sort_unstable();
        format!(
            "{:?}:{}:{:?}:{:?}:{}:{}:{}",
            req.mode,
            req.limit,
            req.time_range,
            req.language,
            names.join(","),
            req.full_content,
            req.max_chars_per_result,
        )
    }

    /// A cached key for an earlier query that is close enough to this one, if fuzzy matching is on.
    /// Queries must agree on every number: "rust 1.70" is not "rust 1.71".
    fn similar_key(&self, scope: &str, tokens: &HashSet<String>) -> Option<String> {
        let threshold = self.cfg.fuzzy_cache_threshold?;
        let digits = |t: &HashSet<String>| -> BTreeSet<String> {
            t.iter()
                .filter(|w| w.chars().any(|c| c.is_ascii_digit()))
                .cloned()
                .collect()
        };
        let recent = self.recent_queries.lock().ok()?;
        recent
            .iter()
            .filter(|r| r.scope == scope && digits(&r.tokens) == digits(tokens))
            .map(|r| (jaccard(&r.tokens, tokens), r))
            .filter(|(j, _)| *j >= threshold)
            .max_by(|a, b| a.0.total_cmp(&b.0))
            .map(|(_, r)| r.key.clone())
    }

    fn remember_query(&self, scope: String, tokens: HashSet<String>, key: String) {
        if self.cfg.fuzzy_cache_threshold.is_none() {
            return;
        }
        if let Ok(mut recent) = self.recent_queries.lock() {
            recent.retain(|r| r.key != key);
            recent.push_back(RecentQuery { scope, tokens, key });
            while recent.len() > RECENT_QUERIES {
                recent.pop_front();
            }
        }
    }

    pub async fn search(&self, req: &SearchRequest) -> Result<SearchResponse> {
        let result = self.search_inner(req).await;
        if let Some(j) = &self.journal {
            j.search(req, &result);
        }
        result
    }

    async fn search_inner(&self, req: &SearchRequest) -> Result<SearchResponse> {
        req.validate()?;
        let started = Instant::now();
        let sources = self.select_sources(&req.sources, rank::Recency::of(&req.query))?;

        let scope = Self::cache_scope(req, &sources);
        let tokens = token_set(&req.query);
        let key = format!("search:{scope}:{}", normalize_query(&req.query));
        let mut lookups = vec![key.clone()];
        lookups.extend(self.similar_key(&scope, &tokens).filter(|k| *k != key));
        for k in lookups {
            if let Some(bytes) = self.cache.get(&k).await
                && let Ok(mut cached) = serde_json::from_slice::<SearchResponse>(&bytes)
            {
                cached.cached = true;
                cached.took_ms = ms(started);
                return Ok(cached);
            }
        }

        let mut stages = Vec::new();
        let mut degraded = Vec::new();

        // Stage 1: fan out.
        let t = Instant::now();
        let per_source = (req.limit * 2).max(10);
        let sq = SourceQuery {
            query: req.query.clone(),
            limit: per_source,
            page: 1,
            time_range: req.time_range,
            language: req.language.clone(),
        };
        let outcomes = futures::future::join_all(sources.iter().map(|s| {
            let sq = sq.clone();
            let s = Arc::clone(s);
            let timeout = self.cfg.source_timeout;
            async move {
                let r = tokio::time::timeout(timeout, s.search(&sq))
                    .await
                    .unwrap_or_else(|_| {
                        Err(SourceError::new(
                            FailureKind::Timeout,
                            format!("no answer within {} ms", timeout.as_millis()),
                        ))
                    });
                (s.name().to_string(), s.weight(), r)
            }
        }))
        .await;
        let mut results = Vec::new();
        let mut failures = Vec::new();
        for (name, weight, r) in outcomes {
            match r {
                Ok(hits) => results.push(SourceResults {
                    source: name,
                    weight,
                    hits,
                }),
                Err(e) => failures.push(SourceFailure {
                    source: name,
                    kind: e.kind,
                    message: e.message,
                }),
            }
        }
        stages.push(StageTiming {
            stage: "search".into(),
            ms: ms(t),
        });
        let auxiliary: Vec<&str> = sources.iter().filter(|s| s.auxiliary()).map(|s| s.name()).collect();
        let only_empty_auxiliary = !failures.is_empty()
            && results.iter().all(|r| auxiliary.contains(&r.source.as_str()) && r.hits.is_empty());
        if results.is_empty() || only_empty_auxiliary {
            return Err(Error::AllSourcesFailed(failures));
        }
        if !failures.is_empty() {
            degraded.push(format!(
                "{} of {} sources failed",
                failures.len(),
                sources.len()
            ));
        }

        // Stage 2: fuse and rank on snippets.
        let t = Instant::now();
        let candidates = fusion::fuse(&results, &self.cfg.fusion);
        let q = QueryInfo::new(&req.query);
        let now = Utc::now();
        let mut scored: Vec<(fusion::Candidate, ScoreParts)> = candidates
            .into_iter()
            .map(|c| {
                let mut parts = rank::score(&c, &q, &self.cfg.rank, now);
                if let Some(l) = &self.learner {
                    let sources: Vec<&str> = c.found_by.iter().map(|a| a.source.as_str()).collect();
                    parts.learned = l.adjustment(&host_of(&c.url), &sources);
                    parts.stage1 += parts.learned;
                }
                (c, parts)
            })
            .collect();
        scored.sort_by(|a, b| {
            b.1.stage1
                .partial_cmp(&a.1.stage1)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.0.key.cmp(&b.0.key))
        });
        stages.push(StageTiming {
            stage: "fuse_rank".into(),
            ms: ms(t),
        });

        if let Some(l) = &self.learner {
            let fetch_k = match req.mode {
                Mode::Fast => 0,
                Mode::Standard => self.cfg.fetch_standard.max(req.limit),
                Mode::Deep => self.cfg.fetch_deep.max(req.limit),
            };
            if let Some(j) = l.explore_index(fetch_k, scored.len()) {
                scored.swap(fetch_k - 1, j);
            }
        }
        let mut hits: Vec<Hit> = Vec::new();
        let max_stage1 = scored
            .first()
            .map(|(_, p)| p.stage1)
            .filter(|m| *m > 0.0)
            .unwrap_or(1.0);
        let fetch_k = match req.mode {
            Mode::Fast => 0,
            Mode::Standard => self.cfg.fetch_standard.max(req.limit),
            Mode::Deep => self.cfg.fetch_deep.max(req.limit),
        }
        .min(scored.len());

        // Stage 3: fetch and extract the best candidates.
        let t = Instant::now();
        let mut loaded: Vec<Option<std::result::Result<Loaded, String>>> =
            (0..scored.len()).map(|_| None).collect();
        if fetch_k > 0 {
            let urls: Vec<String> = scored
                .iter()
                .take(fetch_k)
                .map(|(c, _)| c.url.clone())
                .collect();
            let outs = content::load_pages(
                &self.fetcher,
                self.renderer.as_ref(),
                self.extractor_handle(),
                &urls,
                Scope::Core,
                self.cfg.concurrency,
                self.cfg.page_timeout,
            )
            .await;
            let failed = outs.iter().filter(|o| o.is_err()).count();
            if failed > 0 {
                degraded.push(format!(
                    "content: {failed} of {fetch_k} pages could not be fetched or extracted"
                ));
            }
            for (i, o) in outs.into_iter().enumerate() {
                loaded[i] = Some(o.map_err(|e| e.to_string()));
            }
        }
        stages.push(StageTiming {
            stage: "fetch_extract".into(),
            ms: ms(t),
        });

        // Stage 4: rerank on content.
        let t = Instant::now();
        let ok_idx: Vec<usize> = loaded
            .iter()
            .enumerate()
            .filter(|(_, l)| matches!(l, Some(Ok(l)) if !l.result.core.trim().is_empty()))
            .map(|(i, _)| i)
            .collect();
        let rerank_scores = if ok_idx.is_empty() {
            Vec::new()
        } else {
            // Owned copies, so the work can move to a blocking thread: an encoder is CPU-bound.
            let owned: Vec<(String, String, f32)> = ok_idx
                .iter()
                .map(|&i| {
                    let l = loaded[i]
                        .as_ref()
                        .and_then(|l| l.as_ref().ok())
                        .expect("index came from the ok list");
                    (
                        scored[i].0.title.clone(),
                        l.result.core.clone(),
                        l.result.confidence,
                    )
                })
                .collect();
            let reranker = Arc::clone(&self.reranker);
            let query = req.query.clone();
            let job = tokio::task::spawn_blocking(move || {
                let docs: Vec<DocView<'_>> = owned
                    .iter()
                    .map(|(title, body, confidence)| DocView {
                        title,
                        body,
                        confidence: *confidence,
                    })
                    .collect();
                reranker.rerank_noted(&query, &docs)
            })
            .await;
            match job {
                Ok((scores, note)) => {
                    degraded.extend(note);
                    scores
                }
                Err(e) => {
                    degraded.push(format!("rerank failed ({e}); content scores skipped"));
                    Vec::new()
                }
            }
        };
        stages.push(StageTiming {
            stage: "rerank".into(),
            ms: ms(t),
        });

        // Stage 5: assemble.
        let t = Instant::now();
        let mut selections: Vec<Option<(Vec<SelectedSection>, bool)>> = Vec::new();
        if !req.full_content {
            for &j in &ok_idx {
                let core = loaded[j]
                    .as_ref()
                    .and_then(|l| l.as_ref().ok())
                    .map(|l| l.result.core.clone())
                    .unwrap_or_default();
                selections.push(Some(
                    self.select_sections(core, req.query.clone(), req.max_chars_per_result)
                        .await,
                ));
            }
        }
        for (i, (cand, mut parts)) in scored.into_iter().enumerate() {
            let s1n = parts.stage1 / max_stage1;
            let mut content_error = None;
            let mut content = None;
            let final_score = match ok_idx.iter().position(|&j| j == i) {
                Some(pos) => {
                    let rs = rerank_scores.get(pos).copied().unwrap_or_default();
                    parts.relevance = rs.relevance;
                    parts.title_support = rs.title_support;
                    parts.body_quality = rs.body_quality;
                    let l = loaded[i]
                        .as_ref()
                        .and_then(|l| l.as_ref().ok())
                        .expect("index came from the ok list");
                    let (selected, truncated) = selections
                        .get_mut(pos)
                        .and_then(Option::take)
                        .unwrap_or_default();
                    content = Some(Content {
                        sections: selected,
                        markdown: req.full_content.then(|| l.result.core.clone()),
                        confidence: l.result.confidence,
                        needs_render: l.result.needs_render,
                        truncated,
                    });
                    0.2 * s1n + 0.7 * rs.relevance + 0.1 * rs.body_quality
                }
                None => {
                    match &loaded[i] {
                        Some(Err(e)) => content_error = Some(e.clone()),
                        Some(Ok(_)) => {
                            content_error = Some("no main content could be extracted".into())
                        }
                        None => {}
                    }
                    // Without page content the best evidence is the snippet-level score.
                    0.2 * s1n + 0.7 * f64::min(0.6, 0.6 * s1n)
                }
            };
            if let Some(l) = &self.learner {
                let host = host_of(&cand.url);
                let sources: Vec<&str> = cand.found_by.iter().map(|a| a.source.as_str()).collect();
                match (&content, loaded[i].is_some()) {
                    (Some(_), _) => l.observe(&host, &sources, 0.7 * parts.relevance + 0.3 * parts.body_quality, 1.0),
                    (None, true) => l.failed(&host, &sources),
                    (None, false) => {}
                }
            }
            parts.stage1 = s1n;
            hits.push(Hit {
                url: cand.url,
                title: cand.title,
                snippet: cand.snippet,
                published: cand.published,
                score: final_score,
                score_parts: parts,
                found_by: cand.found_by,
                content,
                content_error,
                related: Vec::new(),
            });
        }
        hits.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        let hits = crate::cluster::group_events(hits);
        let mut hits = hits;
        hits.truncate(req.limit);
        stages.push(StageTiming {
            stage: "assemble".into(),
            ms: ms(t),
        });

        if req.mode == Mode::Deep {
            degraded.push(
                "deep: query rewriting is not available (no language model configured)".into(),
            );
        }
        let resp = SearchResponse {
            query: req.query.clone(),
            mode: req.mode,
            took_ms: ms(started),
            hits,
            failures,
            degraded,
            stages,
            cached: false,
        };
        if !resp.hits.is_empty()
            && let Ok(bytes) = serde_json::to_vec(&resp)
        {
            self.cache.put(&key, bytes, self.cfg.search_cache_ttl).await;
            self.remember_query(scope, tokens, key);
        }
        Ok(resp)
    }

    /// Fetch one URL and return its main content. With a `query`, only the sections that answer
    /// it are returned (within `max_chars`); without one, the whole page.
    pub async fn extract(
        &self,
        url: &str,
        scope: Scope,
        query: Option<&str>,
        max_chars: usize,
    ) -> Result<ExtractedPage> {
        if url.trim().is_empty() {
            return Err(Error::InvalidRequest("url must not be empty".into()));
        }
        let loaded = content::load_page(
            self.fetcher.as_ref(),
            self.renderer.as_deref(),
            self.extractor_handle(),
            url,
            scope,
            self.cfg.page_timeout,
        )
        .await?;
        let selection = self.selection_for(&loaded, query, max_chars).await;
        Ok(Self::page_from(url, loaded, selection))
    }

    /// Build the response for HTML that is already in hand (a local file, or a page fetched elsewhere).
    pub async fn extract_html(
        &self,
        html: String,
        url: &str,
        scope: Scope,
        query: Option<&str>,
        max_chars: usize,
    ) -> Result<ExtractedPage> {
        let extractor = Arc::clone(self.extractor_handle());
        let owned_url = url.to_string();
        let result = tokio::task::spawn_blocking(move || {
            extractor.extract(
                &html,
                &owned_url,
                &seesea_extract::Options {
                    scope,
                    ..Default::default()
                },
            )
        })
        .await
        .map_err(|e| Error::Extract(e.to_string()))?;
        if result.diagnostics.not_text {
            return Err(Error::Extract(
                "the input is not text (compressed or binary data)".into(),
            ));
        }
        let loaded = Loaded {
            final_url: url.to_string(),
            result,
            rendered: false,
        };
        let selection = self.selection_for(&loaded, query, max_chars).await;
        Ok(Self::page_from(url, loaded, selection))
    }

    /// Sections for `query`, or `None` when there is no query (the whole page is returned).
    async fn selection_for(
        &self,
        loaded: &Loaded,
        query: Option<&str>,
        max_chars: usize,
    ) -> Option<(Vec<SelectedSection>, bool)> {
        let q = query.filter(|q| !q.trim().is_empty())?;
        Some(
            self.select_sections(loaded.result.core.clone(), q.to_string(), max_chars.max(1))
                .await,
        )
    }

    fn page_from(
        url: &str,
        loaded: Loaded,
        selection: Option<(Vec<SelectedSection>, bool)>,
    ) -> ExtractedPage {
        let r = loaded.result;
        let (selected, truncated, markdown) = match selection {
            Some((s, t)) => (s, t, None),
            None => (Vec::new(), false, Some(r.core.clone())),
        };
        ExtractedPage {
            url: url.to_string(),
            final_url: loaded.final_url,
            meta: r.meta,
            sections: selected,
            markdown,
            asides: r.sections,
            confidence: r.confidence,
            needs_render: r.needs_render,
            rendered: loaded.rendered,
            truncated,
        }
    }
}
