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
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet, VecDeque};
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
    sink: Option<Arc<dyn crate::source::Sink>>,
    extractor: OnceLock<Arc<Extractor>>,
    reranker: Arc<dyn Reranker>,
    section_scorer: Option<Arc<dyn SectionScorer>>,
    cache: Arc<dyn Cache>,
    metered: Option<Arc<Metered>>,
    recent_queries: Mutex<VecDeque<RecentQuery>>,
    /// Sources that answered with a block (captcha, rate limit): left alone until the time shown,
    /// which doubles with every consecutive block, so a blocked engine is not asked on every search.
    paused: Mutex<HashMap<String, (Instant, u32)>>,
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
/// One site must not fill the page: from the third result of a host on, the score falls by an eighth per result.
/// The order is then rebuilt, so a site with many good pages still keeps its best two near the top.
fn diversify(mut hits: Vec<Hit>) -> Vec<Hit> {
    let mut seen: HashMap<String, i32> = HashMap::new();
    for h in &mut hits {
        let n = seen.entry(host_of(&h.url)).or_insert(0);
        *n += 1;
        if *n > 2 {
            h.score *= 0.88f64.powi(*n - 2);
        }
    }
    hits.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    hits
}

pub fn host_of(url: &str) -> String {
    url::Url::parse(url)
        .ok()
        .and_then(|u| {
            u.host_str()
                .map(|h| h.trim_start_matches("www.").to_lowercase())
        })
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
            sink: None,
            extractor: OnceLock::new(),
            reranker: Arc::new(LexicalReranker),
            section_scorer: None,
            cache: Arc::new(NoCache),
            metered: None,
            recent_queries: Mutex::new(VecDeque::new()),
            paused: Mutex::new(HashMap::new()),
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

    /// Where pages that were read and proved useful are kept for later searches.
    pub fn sink(mut self, sink: Arc<dyn crate::source::Sink>) -> Self {
        self.sink = Some(sink);
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
                    // A local store with nothing matching the probe query is not broken.
                    Ok(_) if s.auxiliary() => SourceProbe { source: name, ok: true, hits: 0, ms: ms(t), error: None },
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

    fn select_sources(
        &self,
        wanted: &[String],
        recency: rank::Recency,
    ) -> Result<Vec<Arc<dyn Source>>> {
        if self.sources.is_empty() {
            return Err(Error::NoSources);
        }
        if wanted.is_empty() {
            let general: Vec<_> = self
                .sources
                .iter()
                .filter(|s| {
                    s.on_by_default() && (!s.recency_only() || recency > rank::Recency::Evergreen)
                })
                .cloned()
                .collect();
            return Ok(if general.is_empty() {
                self.sources.clone()
            } else {
                general
            });
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
            "{:?}:{}:{:?}:{:?}:{}:{}:{}:{}:{}:{:?}",
            req.mode,
            req.limit,
            req.time_range,
            req.language,
            names.join(","),
            req.full_content,
            req.max_chars_per_result,
            req.include_domains.join(","),
            req.exclude_domains.join(","),
            req.topic,
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
        let recency = if req.is_news() {
            rank::Recency::of(&req.query).max(rank::Recency::Recent)
        } else {
            rank::Recency::of(&req.query)
        };
        let sources = self.select_sources(&req.sources, recency)?;

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
        if req.mode != Mode::Fast {
            self.reranker.wake();
        }

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
        // A source that was blocked a moment ago is skipped, unless that would leave nothing to ask.
        let now = Instant::now();
        let (active, skipped): (Vec<Arc<dyn Source>>, Vec<Arc<dyn Source>>) = {
            let paused = self.paused.lock().unwrap();
            let (skipped, active): (Vec<_>, Vec<_>) = sources
                .iter()
                .cloned()
                .partition(|s| paused.get(s.name()).is_some_and(|(until, _)| *until > now));
            if active.is_empty() {
                (skipped, Vec::new())
            } else {
                (active, skipped)
            }
        };
        for s in &skipped {
            let wait = self
                .paused
                .lock()
                .unwrap()
                .get(s.name())
                .map(|(u, _)| u.saturating_duration_since(now).as_secs() / 60 + 1);
            degraded.push(format!(
                "{} skipped: it blocked us recently (asked again in about {} min)",
                s.name(),
                wait.unwrap_or(1)
            ));
        }
        let outcomes = futures::future::join_all(active.iter().map(|s| {
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
            {
                let mut paused = self.paused.lock().unwrap();
                match &r {
                    Err(e) if e.kind == FailureKind::Blocked => {
                        let strikes = paused.get(&name).map_or(0, |(_, n)| *n) + 1;
                        let minutes = (5u64 << (strikes - 1).min(4)).min(60);
                        paused.insert(
                            name.clone(),
                            (Instant::now() + Duration::from_secs(minutes * 60), strikes),
                        );
                    }
                    Ok(_) => {
                        paused.remove(&name);
                    }
                    Err(_) => {}
                }
            }
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
        let auxiliary: Vec<&str> = sources
            .iter()
            .filter(|s| s.auxiliary())
            .map(|s| s.name())
            .collect();
        let only_empty_auxiliary = !failures.is_empty()
            && results
                .iter()
                .all(|r| auxiliary.contains(&r.source.as_str()) && r.hits.is_empty());
        if results.is_empty() || only_empty_auxiliary {
            return Err(Error::AllSourcesFailed(failures));
        }
        // A general engine that answers with nothing while the others found something has most likely been blocked.
        let silent: Vec<&str> = results
            .iter()
            .filter(|r| r.hits.is_empty() && !auxiliary.contains(&r.source.as_str()))
            .map(|r| r.source.as_str())
            .filter(|n| sources.iter().any(|s| s.name() == *n && !s.recency_only()))
            .collect();
        if !silent.is_empty() && results.iter().any(|r| !r.hits.is_empty()) {
            degraded.push(format!("returned nothing: {}", silent.join(", ")));
        }
        if !failures.is_empty() {
            degraded.push(format!(
                "{} of {} sources failed",
                failures.len(),
                active.len()
            ));
        }

        // Stage 2: fuse and rank on snippets.
        let t = Instant::now();
        let mut candidates = fusion::fuse(&results, &self.cfg.fusion);
        if !req.include_domains.is_empty() || !req.exclude_domains.is_empty() {
            let on = |host: &str, list: &[String]| {
                list.iter().any(|d| {
                    let d = d.trim().trim_start_matches("www.").to_lowercase();
                    !d.is_empty() && (host == d || host.ends_with(&format!(".{d}")))
                })
            };
            candidates.retain(|c| {
                let host = host_of(&c.url);
                (req.include_domains.is_empty() || on(&host, &req.include_domains))
                    && !on(&host, &req.exclude_domains)
            });
        }
        let mut q = QueryInfo::new(&req.query);
        q.recency = recency;
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

        // Stage 3: fetch and extract the best candidates. A page that cannot be read is replaced by the
        // next candidate (once), so the answer is not padded with results nobody looked at.
        let t = Instant::now();
        let mut loaded: Vec<Option<std::result::Result<Loaded, String>>> =
            (0..scored.len()).map(|_| None).collect();
        let (mut fetched, mut timed_out, mut failed_total) = (0usize, 0usize, 0usize);
        if fetch_k > 0 {
            let titles: Arc<Vec<String>> =
                Arc::new(scored.iter().map(|(c, _)| c.title.clone()).collect());
            let readable = |loaded: &[Option<std::result::Result<Loaded, String>>]| {
                loaded
                    .iter()
                    .filter(|l| matches!(l, Some(Ok(l)) if !l.result.core.trim().is_empty()))
                    .count()
            };
            let (mut from, mut to) = (0usize, fetch_k);
            for round in 0..2 {
                let urls: Vec<String> = scored[from..to]
                    .iter()
                    .map(|(c, _)| c.url.clone())
                    .collect();
                // Pages are handed to the reranker's `prepare` as they arrive, so the encoder works while
                // the slower downloads are still going; it never waits for that work.
                let on_loaded: Option<content::OnLoaded> =
                    tokio::runtime::Handle::try_current().ok().map(|rt| {
                        let reranker = Arc::clone(&self.reranker);
                        let query = req.query.clone();
                        let titles = Arc::clone(&titles);
                        Arc::new(move |i: usize, l: &content::Loaded| {
                            if l.result.core.trim().is_empty() {
                                return;
                            }
                            let (reranker, query, titles) =
                                (Arc::clone(&reranker), query.clone(), Arc::clone(&titles));
                            let (body, confidence) = (l.result.core.clone(), l.result.confidence);
                            let at = from + i;
                            rt.spawn_blocking(move || {
                                reranker.prepare(
                                    &query,
                                    &DocView {
                                        title: &titles[at],
                                        body: &body,
                                        confidence,
                                    },
                                );
                            });
                        }) as content::OnLoaded
                    });
                let outs = content::load_pages_then(
                    &self.fetcher,
                    self.renderer.as_ref(),
                    self.extractor_handle(),
                    &urls,
                    Scope::Core,
                    self.cfg.concurrency,
                    self.cfg.page_timeout,
                    on_loaded,
                )
                .await;
                fetched += outs.len();
                for (k, o) in outs.into_iter().enumerate() {
                    if let Err(e) = &o {
                        failed_total += 1;
                        if matches!(e, Error::Timeout { .. }) {
                            timed_out += 1;
                        }
                    }
                    loaded[from + k] = Some(o.map_err(|e| e.to_string()));
                }
                let have = readable(&loaded);
                if round == 1 || have >= req.limit || to >= scored.len() {
                    break;
                }
                (from, to) = (to, (to + (req.limit - have)).min(scored.len()));
            }
            if failed_total > 0 {
                degraded.push(format!(
                    "content: {failed_total} of {fetched} pages could not be fetched or extracted ({timed_out} timed out)"
                ));
            }
            let (mut fetch_max, mut extract_max, mut extract_sum, mut render_max) = (0, 0, 0, 0);
            for l in loaded.iter().flatten().filter_map(|l| l.as_ref().ok()) {
                fetch_max = fetch_max.max(l.timing.fetch_ms);
                extract_max = extract_max.max(l.timing.extract_ms);
                extract_sum += l.timing.extract_ms;
                render_max = render_max.max(l.timing.render_ms);
            }
            // Pages download side by side, so the slowest one sets the pace; extraction is CPU work.
            for (stage, v) in [
                ("fetch_slowest_page", fetch_max),
                ("extract_slowest_page", extract_max),
                ("extract_all_pages", extract_sum),
                ("render_slowest_page", render_max),
            ] {
                stages.push(StageTiming {
                    stage: stage.into(),
                    ms: v,
                });
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
                    let official = if rs.relevance >= 0.3
                        && rs.body_quality >= 0.5
                        && names_site(&req.query, &cand.url)
                    {
                        0.08
                    } else {
                        0.0
                    };
                    0.2 * s1n + 0.7 * rs.relevance + 0.1 * rs.body_quality + official
                }
                None => {
                    match &loaded[i] {
                        Some(Err(e)) => content_error = Some(e.clone()),
                        Some(Ok(_)) => {
                            content_error = Some("no main content could be extracted".into())
                        }
                        None => {}
                    }
                    // Nobody read this page: its snippet-level score only places it behind the pages that
                    // were read and found relevant, never among them.
                    0.45 * (0.2 * s1n + 0.7 * f64::min(0.6, 0.6 * s1n))
                }
            };
            if let Some(l) = &self.learner {
                let host = host_of(&cand.url);
                let sources: Vec<&str> = cand.found_by.iter().map(|a| a.source.as_str()).collect();
                match (&content, loaded[i].is_some()) {
                    (Some(_), _) => l.observe(
                        &host,
                        &sources,
                        0.7 * parts.relevance + 0.3 * parts.body_quality,
                        1.0,
                    ),
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
        let mut hits = diversify(hits);
        hits.truncate(req.limit);
        if let Some(sink) = &self.sink {
            let pages: Vec<crate::source::Remembered> = hits
                .iter()
                .filter(|h| h.score_parts.relevance >= 0.5 && h.score_parts.body_quality >= 0.4)
                .filter_map(|h| {
                    let lead: String = h
                        .content
                        .as_ref()?
                        .sections
                        .first()?
                        .markdown
                        .chars()
                        .take(400)
                        .collect();
                    Some(crate::source::Remembered {
                        url: h.url.clone(),
                        title: h.title.clone(),
                        lead,
                        published: h.published,
                    })
                })
                .collect();
            if !pages.is_empty() {
                sink.remember(pages);
            }
        }
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
            timing: Default::default(),
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

#[cfg(test)]
mod diversity_tests {
    use super::*;

    #[test]
    fn one_host_cannot_fill_the_page() {
        let mk = |u: &str, score: f64| Hit {
            url: u.into(),
            title: String::new(),
            snippet: String::new(),
            published: None,
            score,
            score_parts: Default::default(),
            found_by: vec![],
            content: None,
            content_error: None,
            related: vec![],
        };
        let hits = vec![
            mk("https://w.org/1", 0.97),
            mk("https://w.org/2", 0.96),
            mk("https://w.org/3", 0.95),
            mk("https://w.org/4", 0.94),
            mk("https://w.org/5", 0.93),
            mk("https://other.example/a", 0.80),
        ];
        let out = diversify(hits);
        let pos = out.iter().position(|h| h.url.contains("other")).unwrap();
        assert!(pos <= 3, "other site at {pos}");
        assert!(out[0].url.ends_with("/1") && out[1].url.ends_with("/2"));
    }
}

/// The query names the site: a word of it is the whole host label (`tokio` for tokio.rs, `postgresql`
/// for postgresql.org) or the first path segment of a docs host (docs.rs/tokio/...). Such a site is the
/// project's own, which a good answer to a question about the project usually leans on.
fn names_site(query: &str, url: &str) -> bool {
    const COMMON: [&str; 12] = [
        "what",
        "latest",
        "stable",
        "version",
        "difference",
        "with",
        "from",
        "that",
        "this",
        "best",
        "model",
        "docs",
    ];
    let Ok(u) = url::Url::parse(url) else {
        return false;
    };
    let Some(host) = u.host_str() else {
        return false;
    };
    let q = query.to_lowercase();
    let words: Vec<&str> = q
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|w| w.len() >= 4 && !COMMON.contains(w))
        .collect();
    let labels: Vec<&str> = host.split('.').collect();
    let registrable = labels.len().saturating_sub(2);
    let on_host = labels[registrable..labels.len().saturating_sub(1).max(registrable)]
        .iter()
        .flat_map(|l| l.split('-'))
        .any(|l| words.contains(&l));
    let first_path = u
        .path_segments()
        .and_then(|mut p| p.next())
        .unwrap_or("")
        .to_lowercase();
    on_host
        || (labels.first().is_some_and(|l| *l == "docs") && words.contains(&first_path.as_str()))
}

#[cfg(test)]
mod site_tests {
    use super::names_site;

    #[test]
    fn a_site_named_by_the_query_is_recognised() {
        assert!(names_site(
            "rust tokio select",
            "https://tokio.rs/tokio/tutorial/select"
        ));
        assert!(names_site(
            "rust tokio select",
            "https://docs.rs/tokio/latest/tokio/macro.select.html"
        ));
        assert!(names_site(
            "latest stable version of PostgreSQL",
            "https://www.postgresql.org/docs/release/"
        ));
        assert!(!names_site(
            "latest stable version of PostgreSQL",
            "https://versionlog.com/postgresql/17/"
        ));
        assert!(!names_site(
            "rust tokio select",
            "https://docs.rs/serde/latest/serde/"
        ));
        assert!(!names_site(
            "rust tokio select",
            "https://blog.csdn.net/x/article/details/1"
        ));
    }
}
