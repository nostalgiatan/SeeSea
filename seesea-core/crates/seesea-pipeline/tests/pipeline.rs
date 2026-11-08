//! End-to-end behaviour of the pipeline with scripted sources and pages (no network).

use async_trait::async_trait;
use seesea_extract::Scope;
use seesea_fetch::FetchError;
use seesea_pipeline::content::{FetchedPage, PageFetcher};
use seesea_pipeline::error::{FailureKind, SourceError};
use seesea_pipeline::store::MemoryCache;
use seesea_pipeline::{
    Attribution, Mode, Pipeline, PipelineConfig, RawHit, SearchRequest, Source, SourceQuery,
};
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

struct FakeSource {
    name: &'static str,
    hits: Vec<RawHit>,
    error: Option<SourceError>,
    delay: Duration,
    calls: AtomicUsize,
}

impl FakeSource {
    fn ok(name: &'static str, hits: Vec<RawHit>) -> Arc<Self> {
        Arc::new(Self {
            name,
            hits,
            error: None,
            delay: Duration::ZERO,
            calls: AtomicUsize::new(0),
        })
    }
    fn failing(name: &'static str, kind: FailureKind) -> Arc<Self> {
        Arc::new(Self {
            name,
            hits: vec![],
            error: Some(SourceError::new(kind, "boom")),
            delay: Duration::ZERO,
            calls: AtomicUsize::new(0),
        })
    }
    fn slow(name: &'static str, delay: Duration) -> Arc<Self> {
        Arc::new(Self {
            name,
            hits: vec![],
            error: None,
            delay,
            calls: AtomicUsize::new(0),
        })
    }
}

#[async_trait]
impl Source for FakeSource {
    fn name(&self) -> &str {
        self.name
    }
    async fn search(&self, _q: &SourceQuery) -> Result<Vec<RawHit>, SourceError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        tokio::time::sleep(self.delay).await;
        match &self.error {
            Some(e) => Err(e.clone()),
            None => Ok(self.hits.clone()),
        }
    }
}

/// A specialised source: only used when a search names it.
struct OptIn(Arc<FakeSource>);

#[async_trait]
impl Source for OptIn {
    fn name(&self) -> &str {
        self.0.name
    }
    fn on_by_default(&self) -> bool {
        false
    }
    async fn search(&self, q: &SourceQuery) -> Result<Vec<RawHit>, SourceError> {
        self.0.search(q).await
    }
}

struct FakePages {
    pages: HashMap<String, String>,
    requested: Mutex<Vec<String>>,
}

impl FakePages {
    fn new(pages: &[(&str, String)]) -> Arc<Self> {
        Arc::new(Self {
            pages: pages
                .iter()
                .map(|(u, h)| (u.to_string(), h.clone()))
                .collect(),
            requested: Mutex::new(vec![]),
        })
    }
}

#[async_trait]
impl PageFetcher for FakePages {
    async fn fetch(&self, url: &str) -> Result<FetchedPage, FetchError> {
        self.requested.lock().unwrap().push(url.to_string());
        self.pages
            .get(url)
            .map(|h| FetchedPage {
                final_url: url.to_string(),
                html: h.clone(),
            })
            .ok_or(FetchError::Status(404))
    }
}

fn hit(url: &str, title: &str, snippet: &str) -> RawHit {
    RawHit {
        url: url.into(),
        title: title.into(),
        snippet: snippet.into(),
        ..Default::default()
    }
}

fn page(title: &str, paragraphs: &[&str]) -> String {
    let body: String = paragraphs.iter().map(|p| format!("<p>{p}</p>")).collect();
    format!(
        "<html><head><title>{title}</title></head><body><nav><a href=\"/\">Home</a> <a href=\"/x\">Menu</a></nav><article><h1>{title}</h1>{body}</article><footer>Copyright</footer></body></html>"
    )
}

const RUNTIME: [&str; 3] = [
    "Tokio is an asynchronous runtime for the Rust programming language. It provides the building blocks for writing network applications without giving up speed.",
    "The runtime schedules async tasks on a work-stealing thread pool, drives timers and sockets, and lets tasks run concurrently on every available core.",
    "Spawning a task is cheap: tasks are small state machines, so a program can run hundreds of thousands of them at the same time with modest memory.",
];
const SPAM: [&str; 2] = [
    "Cheap watches and handbags at discount prices with free shipping on every order placed today, while stocks last, from our online warehouse.",
    "Sign up to our newsletter to receive more exclusive deals, coupons and promotions every week straight to your inbox, no obligation to buy.",
];

fn standard_pipeline(sources: Vec<Arc<dyn Source>>, pages: Arc<FakePages>) -> Pipeline {
    Pipeline::new(sources, pages).config(PipelineConfig {
        source_timeout: Duration::from_millis(300),
        ..Default::default()
    })
}

#[tokio::test]
async fn page_content_beats_a_title_that_only_looks_right() {
    // The bait result is returned first by both sources with a perfect title; the real answer is
    // returned lower with a plainer title. Only the page content tells them apart.
    let bait = "https://bait.example/rust-async-runtime";
    let real = "https://docs.example/tokio";
    let a = FakeSource::ok(
        "a",
        vec![
            hit(bait, "Rust async runtime", "Click here"),
            hit(
                real,
                "Tokio overview",
                "Tokio is an asynchronous runtime for Rust",
            ),
        ],
    );
    let b = FakeSource::ok(
        "b",
        vec![
            hit(bait, "Rust async runtime", "Best deals"),
            hit(real, "Tokio overview", ""),
        ],
    );
    let pages = FakePages::new(&[
        (bait, page("Rust async runtime", &SPAM)),
        (real, page("Tokio overview", &RUNTIME)),
    ]);
    let p = standard_pipeline(vec![a, b], pages);

    let resp = p
        .search(&SearchRequest::new("rust async runtime"))
        .await
        .unwrap();
    assert_eq!(resp.hits.len(), 2);
    assert_eq!(
        resp.hits[0].url,
        real,
        "{:#?}",
        resp.hits
            .iter()
            .map(|h| (&h.url, h.score, &h.score_parts))
            .collect::<Vec<_>>()
    );
    let top = &resp.hits[0];
    let content = top.content.as_ref().expect("content attached");
    assert!(content.confidence > 0.3);
    assert!(
        content
            .sections
            .iter()
            .any(|s| s.markdown.contains("work-stealing"))
    );
    assert!(top.score_parts.title_support > 0.4);
    let bait_hit = &resp.hits[1];
    assert!(
        bait_hit.score_parts.title_support < 0.2,
        "{:?}",
        bait_hit.score_parts
    );
    assert_eq!(top.found_by.len(), 2);
    assert!(resp.stages.iter().any(|s| s.stage == "fetch_extract"));
}

#[tokio::test]
async fn fast_mode_fetches_nothing() {
    let pages = FakePages::new(&[]);
    let p = standard_pipeline(
        vec![FakeSource::ok(
            "a",
            vec![
                hit("https://x.org/1", "One", "first"),
                hit("https://x.org/2", "Two", "second"),
            ],
        )],
        pages.clone(),
    );
    let mut req = SearchRequest::new("one");
    req.mode = Mode::Fast;
    let resp = p.search(&req).await.unwrap();
    assert_eq!(resp.hits.len(), 2);
    assert!(
        resp.hits
            .iter()
            .all(|h| h.content.is_none() && h.content_error.is_none())
    );
    assert!(pages.requested.lock().unwrap().is_empty());
}

#[tokio::test]
async fn failed_sources_and_pages_degrade_instead_of_failing() {
    let ok = "https://docs.example/tokio";
    let missing = "https://gone.example/page";
    let a = FakeSource::ok(
        "a",
        vec![
            hit(ok, "Tokio overview", "asynchronous runtime"),
            hit(missing, "Tokio runtime notes", "async runtime"),
        ],
    );
    let pages = FakePages::new(&[(ok, page("Tokio overview", &RUNTIME))]);
    let p = standard_pipeline(
        vec![
            a,
            FakeSource::failing("b", FailureKind::Http),
            FakeSource::slow("c", Duration::from_secs(5)),
        ],
        pages,
    );

    let resp = p
        .search(&SearchRequest::new("tokio async runtime"))
        .await
        .unwrap();
    assert_eq!(resp.hits.len(), 2);
    let kinds: Vec<(&str, FailureKind)> = resp
        .failures
        .iter()
        .map(|f| (f.source.as_str(), f.kind))
        .collect();
    assert!(kinds.contains(&("b", FailureKind::Http)), "{kinds:?}");
    assert!(kinds.contains(&("c", FailureKind::Timeout)), "{kinds:?}");
    assert!(
        resp.degraded
            .iter()
            .any(|d| d.contains("2 of 3 sources failed")),
        "{:?}",
        resp.degraded
    );
    assert!(
        resp.degraded.iter().any(|d| d.contains("1 of 2 pages")),
        "{:?}",
        resp.degraded
    );
    let gone = resp.hits.iter().find(|h| h.url == missing).unwrap();
    assert!(gone.content.is_none());
    assert!(
        gone.content_error.as_deref().unwrap().contains("404"),
        "{:?}",
        gone.content_error
    );
}

#[tokio::test]
async fn request_and_configuration_errors_are_typed() {
    let pages = FakePages::new(&[]);
    let p = standard_pipeline(
        vec![
            FakeSource::failing("a", FailureKind::Parse),
            FakeSource::failing("b", FailureKind::Blocked),
        ],
        pages.clone(),
    );

    let err = p.search(&SearchRequest::new("q")).await.unwrap_err();
    assert_eq!(err.code(), "all_sources_failed");
    assert_eq!(err.body().failures.len(), 2);
    assert_eq!(err.http_status(), 502);

    assert_eq!(
        p.search(&SearchRequest::new("  "))
            .await
            .unwrap_err()
            .code(),
        "invalid_request"
    );
    let mut req = SearchRequest::new("q");
    req.sources = vec!["nope".into()];
    let err = p.search(&req).await.unwrap_err();
    assert_eq!(err.code(), "invalid_request");
    assert!(err.to_string().contains("available: a, b"), "{err}");

    let empty = standard_pipeline(vec![], pages);
    assert_eq!(
        empty
            .search(&SearchRequest::new("q"))
            .await
            .unwrap_err()
            .code(),
        "no_sources"
    );
}

#[tokio::test]
async fn source_filter_limit_and_empty_results() {
    let a = FakeSource::ok(
        "a",
        (1..=6)
            .map(|i| {
                hit(
                    &format!("https://x.org/{i}"),
                    &format!("Result {i}"),
                    "text",
                )
            })
            .collect(),
    );
    let b = FakeSource::ok("b", vec![]);
    let (a_probe, b_probe) = (a.clone(), b.clone());
    let p = standard_pipeline(vec![a, b], FakePages::new(&[]));

    let mut req = SearchRequest::new("result");
    req.mode = Mode::Fast;
    req.limit = 3;
    req.sources = vec!["B".into()];
    let resp = p.search(&req).await.unwrap();
    assert!(resp.hits.is_empty(), "an empty answer is not an error");
    assert_eq!(a_probe.calls.load(Ordering::SeqCst), 0);
    assert_eq!(b_probe.calls.load(Ordering::SeqCst), 1);

    req.sources.clear();
    assert_eq!(p.search(&req).await.unwrap().hits.len(), 3);
}

#[tokio::test]
async fn specialised_sources_are_used_only_when_named() {
    let general = FakeSource::ok("web", vec![hit("https://x.org/1", "One", "first")]);
    let niche = FakeSource::ok(
        "papers",
        vec![hit("https://arxiv.org/abs/1", "Paper", "abstract")],
    );
    let niche_probe = niche.clone();
    let p = standard_pipeline(vec![general, Arc::new(OptIn(niche))], FakePages::new(&[]));

    let mut req = SearchRequest::new("one");
    req.mode = Mode::Fast;
    let resp = p.search(&req).await.unwrap();
    assert_eq!(resp.hits.len(), 1);
    assert_eq!(niche_probe.calls.load(Ordering::SeqCst), 0);

    req.sources = vec!["papers".into()];
    let resp = p.search(&req).await.unwrap();
    assert_eq!(resp.hits[0].url, "https://arxiv.org/abs/1");
    assert_eq!(niche_probe.calls.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn identical_searches_are_served_from_the_cache() {
    let a = FakeSource::ok("a", vec![hit("https://x.org/1", "One", "first")]);
    let probe = a.clone();
    let p = standard_pipeline(vec![a], FakePages::new(&[])).cache(Arc::new(MemoryCache::new(16)));
    let mut req = SearchRequest::new("one");
    req.mode = Mode::Fast;

    let first = p.search(&req).await.unwrap();
    let second = p.search(&req).await.unwrap();
    assert!(!first.cached && second.cached);
    assert_eq!(second.hits[0].url, first.hits[0].url);
    assert_eq!(probe.calls.load(Ordering::SeqCst), 1);

    req.limit = 5;
    p.search(&req).await.unwrap();
    assert_eq!(
        probe.calls.load(Ordering::SeqCst),
        2,
        "different parameters are a different search"
    );
}

#[tokio::test]
async fn reworded_queries_share_a_cache_entry_and_similar_ones_only_when_asked() {
    let fast = |q: &str| {
        let mut r = SearchRequest::new(q);
        r.mode = Mode::Fast;
        r
    };
    let make = |threshold: Option<f64>| {
        let a = FakeSource::ok("a", vec![hit("https://x.org/1", "One", "first")]);
        let probe = a.clone();
        let p = Pipeline::new(vec![a], FakePages::new(&[]))
            .config(PipelineConfig {
                fuzzy_cache_threshold: threshold,
                ..Default::default()
            })
            .cache(Arc::new(MemoryCache::new(16)));
        (p, probe)
    };

    let (p, probe) = make(None);
    p.search(&fast("how to use rust async runtime"))
        .await
        .unwrap();
    let again = p.search(&fast("Async runtime, rust use?")).await.unwrap();
    assert!(again.cached, "same meaningful words in another order");
    let near = p
        .search(&fast("rust async runtime tutorial"))
        .await
        .unwrap();
    assert!(!near.cached, "similar is not the same unless enabled");
    assert_eq!(probe.calls.load(Ordering::SeqCst), 2);
    let stats = p.cache_stats();
    assert_eq!((stats["search"].hits, stats["search"].misses), (1, 2));
    assert_eq!(stats["search"].puts, 2);

    let (p, probe) = make(Some(0.6));
    p.search(&fast("rust async runtime")).await.unwrap();
    assert!(
        p.search(&fast("rust async runtime tutorial"))
            .await
            .unwrap()
            .cached
    );
    assert!(
        !p.search(&fast("rust 1.70 async runtime"))
            .await
            .unwrap()
            .cached,
        "a number the cached query lacks"
    );
    assert_eq!(probe.calls.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn full_content_and_section_budget() {
    let url = "https://docs.example/tokio";
    let long: Vec<String> = (0..30).map(|i| format!("Paragraph {i} about unrelated gardening topics such as soil, compost and watering schedules for vegetables.")).collect();
    let mut paragraphs: Vec<&str> = long.iter().map(String::as_str).collect();
    paragraphs.insert(15, RUNTIME[1]);
    let pages = FakePages::new(&[(url, page("Notes", &paragraphs))]);
    let p = standard_pipeline(
        vec![FakeSource::ok(
            "a",
            vec![hit(url, "Notes", "work-stealing scheduler")],
        )],
        pages,
    );

    let mut req = SearchRequest::new("work-stealing thread pool scheduler");
    req.max_chars_per_result = 400;
    let resp = p.search(&req).await.unwrap();
    let c = resp.hits[0].content.as_ref().unwrap();
    let text: String = c.sections.iter().map(|s| s.markdown.as_str()).collect();
    assert!(text.chars().count() <= 400, "{}", text.chars().count());
    assert!(c.truncated);
    assert!(c.markdown.is_none());

    req.full_content = true;
    let resp = p.search(&req).await.unwrap();
    let c = resp.hits[0].content.as_ref().unwrap();
    assert!(c.markdown.as_deref().unwrap().contains("work-stealing"));
    assert!(c.sections.is_empty());
}

#[tokio::test]
async fn extract_returns_the_whole_page_or_the_relevant_sections() {
    let url = "https://docs.example/guide";
    let html = "<html><head><title>Guide</title></head><body><article><h1>Guide</h1>\
        <h2>Installing</h2><p>Download the installer from the releases page, run it and restart the terminal so the new PATH entry is picked up by the shell.</p>\
        <h2>Async runtime</h2><p>The async runtime schedules tasks on a thread pool, drives timers and sockets, and wakes tasks when their I/O is ready to proceed.</p>\
        <h2>Licensing</h2><p>The software is released under the MIT license, which permits commercial use, modification and distribution with attribution to the authors.</p>\
        </article></body></html>".to_string();
    let p = standard_pipeline(vec![], FakePages::new(&[(url, html.clone())]));

    let whole = p.extract(url, Scope::Core, None, 3000).await.unwrap();
    assert!(whole.markdown.as_deref().unwrap().contains("MIT license"));
    assert_eq!(whole.meta.title.as_deref(), Some("Guide"));

    let focused = p
        .extract(url, Scope::Core, Some("async runtime timers"), 200)
        .await
        .unwrap();
    assert!(focused.markdown.is_none());
    assert!(focused.truncated);
    assert_eq!(focused.sections.len(), 1);
    assert!(focused.sections[0].markdown.contains("thread pool"));

    let local = p
        .extract_html(html, url, Scope::Core, None, 3000)
        .await
        .unwrap();
    assert!(local.markdown.unwrap().contains("Download the installer"));

    assert_eq!(
        p.extract("", Scope::Core, None, 10)
            .await
            .unwrap_err()
            .code(),
        "invalid_request"
    );
    assert_eq!(
        p.extract("https://nowhere.example/", Scope::Core, None, 10)
            .await
            .unwrap_err()
            .code(),
        "fetch_failed"
    );
}

#[tokio::test]
async fn upstream_agreement_is_visible_in_found_by() {
    let mut h = hit("https://x.org/1", "One", "first");
    h.upstream = vec![
        Attribution {
            source: "searxng/google".into(),
            rank: 2,
        },
        Attribution {
            source: "searxng/brave".into(),
            rank: 1,
        },
    ];
    let p = standard_pipeline(
        vec![FakeSource::ok("searxng", vec![h])],
        FakePages::new(&[]),
    );
    let mut req = SearchRequest::new("one");
    req.mode = Mode::Fast;
    let resp = p.search(&req).await.unwrap();
    let names: Vec<&str> = resp.hits[0]
        .found_by
        .iter()
        .map(|a| a.source.as_str())
        .collect();
    assert_eq!(names, vec!["searxng", "searxng/google", "searxng/brave"]);
}

#[tokio::test]
async fn probe_reports_each_source_and_treats_silence_as_a_problem() {
    let p = standard_pipeline(
        vec![
            FakeSource::ok("good", vec![hit("https://x.org/1", "One", "first")]),
            FakeSource::ok("empty", vec![]),
            FakeSource::failing("broken", FailureKind::Http),
            FakeSource::slow("slow", Duration::from_secs(5)),
        ],
        FakePages::new(&[]),
    );
    let probes = p.probe("anything").await;
    let by = |n: &str| probes.iter().find(|x| x.source == n).unwrap();
    assert!(by("good").ok && by("good").hits == 1 && by("good").error.is_none());
    assert!(!by("empty").ok);
    assert_eq!(by("empty").error.as_ref().unwrap().kind, FailureKind::Parse);
    assert_eq!(by("broken").error.as_ref().unwrap().kind, FailureKind::Http);
    assert_eq!(
        by("slow").error.as_ref().unwrap().kind,
        FailureKind::Timeout
    );
}

struct BrokenEmbedder;

impl seesea_pipeline::semantic::TextEmbedder for BrokenEmbedder {
    fn name(&self) -> &str {
        "broken"
    }
    fn embed_query(&self, _: &str) -> Result<Vec<f32>, String> {
        Err("weights are corrupt".into())
    }
    fn embed_passages(&self, _: &[&str]) -> Result<Vec<Vec<f32>>, String> {
        Err("weights are corrupt".into())
    }
}

#[tokio::test]
async fn a_failing_semantic_reranker_degrades_to_lexical_and_says_so() {
    let url = "https://docs.example/tokio";
    let pages = FakePages::new(&[(url, page("Tokio", &RUNTIME))]);
    let source = FakeSource::ok("a", vec![hit(url, "Tokio", "runtime")]);
    let semantic = Arc::new(seesea_pipeline::semantic::SemanticReranker::new(
        Arc::new(BrokenEmbedder),
        Default::default(),
    ));
    let p = standard_pipeline(vec![source.clone()], pages.clone()).reranker(semantic);
    let resp = p
        .search(&SearchRequest::new("tokio async runtime"))
        .await
        .unwrap();
    assert_eq!(resp.hits.len(), 1);
    assert!(resp.hits[0].content.is_some(), "content still returned");
    assert!(
        resp.degraded
            .iter()
            .any(|d| d.contains("weights are corrupt")),
        "{:?}",
        resp.degraded
    );

    let plain = standard_pipeline(vec![source], pages)
        .search(&SearchRequest::new("tokio async runtime"))
        .await
        .unwrap();
    assert_eq!(
        resp.hits[0].score, plain.hits[0].score,
        "same scores as lexical"
    );
}

#[tokio::test]
async fn stage_one_learns_which_hosts_deliver() {
    use seesea_pipeline::{LearnConfig, Learner};
    let bait = "https://bait.example/rust-async-runtime";
    let real = "https://docs.example/tokio";
    let mk = || {
        let a = FakeSource::ok(
            "a",
            vec![
                hit(bait, "Rust async runtime", "Click here"),
                hit(
                    real,
                    "Tokio overview",
                    "Tokio is an asynchronous runtime for Rust",
                ),
            ],
        );
        let pages = FakePages::new(&[
            (bait, page("Rust async runtime", &SPAM)),
            (real, page("Tokio overview", &RUNTIME)),
        ]);
        (a, pages)
    };
    let learner = Arc::new(Learner::new(LearnConfig::default()));
    let (a, pages) = mk();
    let p = standard_pipeline(vec![a], pages).learner(Arc::clone(&learner));
    // Different wordings so the response cache does not answer.
    let first = p
        .search(&SearchRequest::new("rust async runtime"))
        .await
        .unwrap();
    assert!(
        first.hits.iter().all(|h| h.score_parts.learned == 0.0),
        "nothing known yet"
    );
    for q in [
        "async runtime rust",
        "tokio rust runtime",
        "rust runtime for async code",
    ] {
        p.search(&SearchRequest::new(q)).await.unwrap();
    }
    let later = p
        .search(&SearchRequest::new("what is the rust async runtime"))
        .await
        .unwrap();
    let learned = |url: &str| {
        later
            .hits
            .iter()
            .find(|h| h.url == url)
            .unwrap()
            .score_parts
            .learned
    };
    assert!(
        learned(real) > learned(bait),
        "real {} bait {}",
        learned(real),
        learned(bait)
    );
    assert!(learned(bait) < 0.0);
    assert_eq!(learner.report(5).hosts, 2);
}

struct NewsOnly(Arc<FakeSource>);

#[async_trait]
impl Source for NewsOnly {
    fn name(&self) -> &str {
        "news"
    }
    fn recency_only(&self) -> bool {
        true
    }
    async fn search(
        &self,
        q: &SourceQuery,
    ) -> Result<Vec<RawHit>, seesea_pipeline::error::SourceError> {
        self.0.search(q).await
    }
}

#[tokio::test]
async fn news_sources_are_asked_only_about_recent_things() {
    let general = FakeSource::ok(
        "web",
        vec![hit("https://w.example/1", "Rust ownership", "explained")],
    );
    let news_inner = FakeSource::ok(
        "news",
        vec![hit(
            "https://n.example/1",
            "Rust release",
            "announced today",
        )],
    );
    let p = standard_pipeline(
        vec![general, Arc::new(NewsOnly(Arc::clone(&news_inner)))],
        FakePages::new(&[]),
    );
    let mut timeless = SearchRequest::new("rust ownership");
    timeless.mode = Mode::Fast;
    let r = p.search(&timeless).await.unwrap();
    assert_eq!(news_inner.calls.load(Ordering::SeqCst), 0, "{r:?}");
    let mut recent = SearchRequest::new("rust latest release");
    recent.mode = Mode::Fast;
    p.search(&recent).await.unwrap();
    assert_eq!(news_inner.calls.load(Ordering::SeqCst), 1);
    // Naming it asks for it whatever the query.
    let mut named = SearchRequest::new("rust ownership");
    named.mode = Mode::Fast;
    named.sources = vec!["news".into()];
    p.search(&named).await.unwrap();
    assert_eq!(news_inner.calls.load(Ordering::SeqCst), 2);
}

#[test]
fn breaking_queries_prefer_hours_old_over_days_old() {
    use chrono::{Duration as D, Utc};
    use seesea_pipeline::rank::{QueryInfo, RankConfig, score};
    let cand = |age: D| seesea_pipeline::fusion::Candidate {
        key: "k".into(),
        url: "https://a.example/x".into(),
        title: "t".into(),
        snippet: "s".into(),
        published: Some(Utc::now() - age),
        found_by: vec![],
        fusion: 0.5,
    };
    let cfg = RankConfig::default();
    let gap = |q: &str| {
        let q = QueryInfo::new(q);
        let now = Utc::now();
        score(&cand(D::hours(2)), &q, &cfg, now).stage1
            - score(&cand(D::days(3)), &q, &cfg, now).stage1
    };
    assert!(
        gap("breaking earthquake today") > 2.0 * gap("rust ownership"),
        "{} {}",
        gap("breaking earthquake today"),
        gap("rust ownership")
    );
}

struct Local(Vec<RawHit>);

#[async_trait]
impl Source for Local {
    fn name(&self) -> &str {
        "local"
    }
    fn auxiliary(&self) -> bool {
        true
    }
    async fn search(
        &self,
        _q: &SourceQuery,
    ) -> Result<Vec<RawHit>, seesea_pipeline::error::SourceError> {
        Ok(self.0.clone())
    }
}

#[tokio::test]
async fn an_empty_local_index_does_not_hide_a_web_outage() {
    let down = FakeSource::failing("web", FailureKind::Http);
    let p = standard_pipeline(
        vec![down.clone(), Arc::new(Local(vec![]))],
        FakePages::new(&[]),
    );
    let mut req = SearchRequest::new("anything");
    req.mode = Mode::Fast;
    assert_eq!(
        p.search(&req).await.unwrap_err().code(),
        "all_sources_failed"
    );
    // With something in the index the answer is the index's, marked degraded.
    let p = standard_pipeline(
        vec![
            down,
            Arc::new(Local(vec![hit(
                "https://l.example/1",
                "Anything local",
                "x",
            )])),
        ],
        FakePages::new(&[]),
    );
    let r = p.search(&req).await.unwrap();
    assert_eq!(r.hits.len(), 1);
    assert!(!r.degraded.is_empty());
}

#[tokio::test]
async fn domain_filters_and_news_topic_apply() {
    let general = FakeSource::ok(
        "web",
        vec![
            hit("https://docs.rs/a", "A on docs", "text"),
            hit("https://blog.example.com/b", "B on blog", "text"),
            hit("https://www.other.org/c", "C elsewhere", "text"),
        ],
    );
    let news_inner = FakeSource::ok(
        "news",
        vec![hit("https://n.example/1", "Rust release", "today")],
    );
    let p = standard_pipeline(
        vec![general, Arc::new(NewsOnly(Arc::clone(&news_inner)))],
        FakePages::new(&[]),
    );
    let mut only = SearchRequest::new("rust ownership");
    only.mode = Mode::Fast;
    only.include_domains = vec!["docs.rs".into(), "example.com".into()];
    let urls: Vec<String> = p
        .search(&only)
        .await
        .unwrap()
        .hits
        .into_iter()
        .map(|h| h.url)
        .collect();
    assert_eq!(urls.len(), 2, "{urls:?}");
    assert!(
        urls.iter()
            .all(|u| u.contains("docs.rs") || u.contains("example.com"))
    );

    let mut not = SearchRequest::new("rust ownership");
    not.mode = Mode::Fast;
    not.exclude_domains = vec!["other.org".into()];
    let urls: Vec<String> = p
        .search(&not)
        .await
        .unwrap()
        .hits
        .into_iter()
        .map(|h| h.url)
        .collect();
    assert!(urls.iter().all(|u| !u.contains("other.org")), "{urls:?}");
    // A suffix that is not a domain boundary must not match.
    not.exclude_domains = vec!["ther.org".into()];
    assert_eq!(p.search(&not).await.unwrap().hits.len(), 3);

    // topic=news asks the news source even for a timeless-looking query.
    assert_eq!(news_inner.calls.load(Ordering::SeqCst), 0);
    let mut news = SearchRequest::new("rust ownership");
    news.mode = Mode::Fast;
    news.topic = Some("news".into());
    p.search(&news).await.unwrap();
    assert_eq!(news_inner.calls.load(Ordering::SeqCst), 1);
    news.topic = Some("sports".into());
    assert!(p.search(&news).await.is_err());
}

struct Collect(Mutex<Vec<seesea_pipeline::Remembered>>);
impl seesea_pipeline::Sink for Collect {
    fn remember(&self, pages: Vec<seesea_pipeline::Remembered>) {
        self.0.lock().unwrap().extend(pages);
    }
}

#[tokio::test]
async fn only_pages_that_answered_are_remembered() {
    let good = "https://good.example/tokio";
    let bad = "https://spam.example/tokio";
    let web = FakeSource::ok(
        "web",
        vec![
            hit(
                good,
                "How the Tokio runtime works",
                "tokio runtime schedules async tasks",
            ),
            hit(bad, "Tokio runtime - Best Guide 2025!", "tokio runtime"),
        ],
    );
    let pages = FakePages::new(&[
        (good, page("How the Tokio runtime works", &RUNTIME)),
        (bad, page("Tokio runtime - Best Guide 2025!", &SPAM)),
    ]);
    let sink = Arc::new(Collect(Mutex::new(vec![])));
    let p = standard_pipeline(vec![web], pages).sink(sink.clone());
    let mut req = SearchRequest::new("tokio runtime scheduler");
    req.mode = Mode::Standard;
    p.search(&req).await.unwrap();
    let kept = sink.0.lock().unwrap().clone();
    assert_eq!(kept.len(), 1, "{kept:?}");
    assert_eq!(kept[0].url, good);
    assert!(kept[0].lead.contains("Tokio"));
}

#[tokio::test]
async fn a_page_that_cannot_be_read_is_replaced_by_the_next_candidate() {
    let urls = [
        "https://one.org/tokio",
        "https://two.net/tokio",
        "https://three.io/tokio",
    ];
    for missing in 0..3 {
        let hits: Vec<RawHit> = urls
            .iter()
            .enumerate()
            .map(|(i, u)| {
                hit(
                    u,
                    [
                        "Tokio runtime internals explained",
                        "Getting started with async Rust tasks",
                        "Scheduler design notes from the maintainers",
                    ][i],
                    &format!("asynchronous runtime for Rust, chapter {i}"),
                )
            })
            .collect();
        let pages: Vec<(&str, String)> = urls
            .iter()
            .enumerate()
            .filter(|(i, _)| *i != missing)
            .map(|(i, u)| {
                let lead = format!("Edition {i} of the tokio notes covers the async runtime in a different way, with its own examples number {i}.");
                (*u, page("Tokio runtime overview", &[lead.as_str(), RUNTIME[0], RUNTIME[1], RUNTIME[2]]))
            })
            .collect();
        let p = Pipeline::new(vec![FakeSource::ok("a", hits)], FakePages::new(&pages)).config(
            PipelineConfig {
                fetch_standard: 2,
                ..Default::default()
            },
        );
        let mut req = SearchRequest::new("tokio async runtime");
        req.limit = 2;
        let resp = p.search(&req).await.unwrap();
        assert_eq!(
            resp.hits.len(),
            2,
            "missing={missing}: {:?} {:?}",
            resp.hits.iter().map(|h| &h.url).collect::<Vec<_>>(),
            resp.degraded
        );
        assert!(
            resp.hits.iter().all(|h| h.content.is_some()),
            "missing={missing}: {:?}",
            resp.hits
                .iter()
                .map(|h| (&h.url, h.content.is_some()))
                .collect::<Vec<_>>()
        );
        assert!(resp.stages.iter().any(|s| s.stage == "fetch_slowest_page"));
    }
}

#[tokio::test]
async fn pages_nobody_read_rank_behind_the_ones_that_were_read() {
    let good = "https://good.example/a";
    let weak = "https://weak.example/b";
    let gone = "https://gone.example/c";
    let hits = vec![
        hit(
            gone,
            "Tokio async runtime guide",
            "tokio async runtime guide for rust",
        ),
        hit(good, "Tokio overview", "asynchronous runtime"),
        hit(weak, "Cheap watches", "discount tokio"),
    ];
    let pages = FakePages::new(&[
        (good, page("Tokio overview", &RUNTIME)),
        (weak, page("Cheap watches", &SPAM)),
    ]);
    let p = standard_pipeline(vec![FakeSource::ok("a", hits)], pages);
    let resp = p
        .search(&SearchRequest::new("tokio async runtime"))
        .await
        .unwrap();
    assert_eq!(resp.hits[0].url, good);
    let gone_at = resp.hits.iter().position(|h| h.url == gone).unwrap();
    assert!(gone_at >= 1 && resp.hits[gone_at].content_error.is_some());
}

#[tokio::test]
async fn a_blocked_source_is_left_alone_for_a_while() {
    let ok = FakeSource::ok(
        "a",
        vec![hit(
            "https://docs.example/tokio",
            "Tokio overview",
            "asynchronous runtime",
        )],
    );
    let blocked = FakeSource::failing("b", FailureKind::Blocked);
    let pages = FakePages::new(&[(
        "https://docs.example/tokio",
        page("Tokio overview", &RUNTIME),
    )]);
    let p = standard_pipeline(vec![ok, blocked.clone()], pages);
    p.search(&SearchRequest::new("tokio async runtime"))
        .await
        .unwrap();
    assert_eq!(blocked.calls.load(Ordering::SeqCst), 1);
    let again = p
        .search(&SearchRequest::new("tokio runtime scheduler"))
        .await
        .unwrap();
    assert_eq!(
        blocked.calls.load(Ordering::SeqCst),
        1,
        "asked again while paused"
    );
    assert!(
        again.degraded.iter().any(|d| d.contains("b skipped")),
        "{:?}",
        again.degraded
    );
}

/// Serves pages like `FakePages`, each after its own delay.
struct DelayedPages {
    inner: Arc<FakePages>,
    delays: HashMap<String, Duration>,
}

#[async_trait]
impl PageFetcher for DelayedPages {
    async fn fetch(&self, url: &str) -> Result<FetchedPage, FetchError> {
        if let Some(d) = self.delays.get(url) {
            tokio::time::sleep(*d).await;
        }
        self.inner.fetch(url).await
    }
}

fn two_fast_two_slow(wait_for_all: bool) -> (Pipeline, [&'static str; 4]) {
    let urls = [
        "https://fast.example/a",
        "https://fast.example/b",
        "https://slow.example/c",
        "https://slow.example/d",
    ];
    let titles = [
        "Tokio scheduler internals",
        "Getting started with async Rust",
        "Work stealing explained",
        "Timers and sockets in the runtime",
    ];
    let hits: Vec<RawHit> = urls
        .iter()
        .zip(titles)
        .map(|(u, t)| hit(u, t, &format!("{t}: asynchronous runtime for Rust")))
        .collect();
    let pages: Vec<(&str, String)> = urls
        .iter()
        .zip(titles)
        .map(|(u, t)| (*u, page(t, &RUNTIME)))
        .collect();
    let fetcher = Arc::new(DelayedPages {
        inner: FakePages::new(&pages),
        delays: urls[2..]
            .iter()
            .map(|u| (u.to_string(), Duration::from_secs(5)))
            .collect(),
    });
    let p = Pipeline::new(vec![FakeSource::ok("a", hits)], fetcher).config(PipelineConfig {
        fetch_enough: if wait_for_all { 0 } else { 2 },
        fetch_grace: Duration::from_millis(100),
        ..Default::default()
    });
    (p, urls)
}

#[tokio::test]
async fn slow_pages_are_left_behind_once_enough_others_are_in() {
    let (p, urls) = two_fast_two_slow(false);
    let t = std::time::Instant::now();
    let resp = p
        .search(&SearchRequest::new("tokio async runtime"))
        .await
        .unwrap();
    assert!(
        t.elapsed() < Duration::from_secs(3),
        "waited for the slow pages: {:?}",
        t.elapsed()
    );
    assert_eq!(resp.hits.len(), 4);
    let read = |u: &str| {
        resp.hits
            .iter()
            .find(|h| h.url == u)
            .unwrap()
            .content
            .is_some()
    };
    assert!(read(urls[0]) && read(urls[1]));
    assert!(!read(urls[2]) && !read(urls[3]));
    assert!(
        resp.degraded
            .iter()
            .any(|d| d.contains("2 of 4 pages") && d.contains("not read")),
        "{:?}",
        resp.degraded
    );
    // A page nobody waited for is not a failed page: it carries no error, and ranks behind the read ones.
    assert!(resp.hits[2..].iter().all(|h| h.content_error.is_none()));
    assert!(resp.hits[..2].iter().all(|h| h.content.is_some()));
}

#[tokio::test]
async fn without_an_early_stop_every_page_is_waited_for() {
    // The slow pages take 400 ms here, so the test stays quick and the default timeout covers them.
    let urls_delay = Duration::from_millis(400);
    let urls = [
        "https://fast.example/a",
        "https://fast.example/b",
        "https://slow.example/c",
        "https://slow.example/d",
    ];
    let pages: Vec<(&str, String)> = urls
        .iter()
        .map(|u| (*u, page("Tokio runtime notes", &RUNTIME)))
        .collect();
    let fetcher = Arc::new(DelayedPages {
        inner: FakePages::new(&pages),
        delays: urls[2..]
            .iter()
            .map(|u| (u.to_string(), urls_delay))
            .collect(),
    });
    let hits: Vec<RawHit> = urls
        .iter()
        .map(|u| hit(u, "Tokio runtime notes", "runtime"))
        .collect();
    let p = Pipeline::new(vec![FakeSource::ok("a", hits)], fetcher);
    let resp = p
        .search(&SearchRequest::new("tokio async runtime"))
        .await
        .unwrap();
    assert!(resp.hits.iter().all(|h| h.content.is_some()));
    assert!(!resp.degraded.iter().any(|d| d.contains("not read")));
    let fetch = resp
        .stages
        .iter()
        .find(|s| s.stage == "fetch_extract")
        .unwrap();
    assert!(fetch.ms >= 400, "{}", fetch.ms);
}

fn vetted(sources: Vec<Arc<dyn Source>>, pages: Arc<FakePages>) -> Pipeline {
    use seesea_vet::{RankTable, ThreatList, Vet, VetConfig};
    let mut threats = ThreatList::new();
    threats.add_url("http://kit.example/login.php");
    let vet = Vet::new(VetConfig::default())
        .with_ranks(RankTable::from_csv("20,paypal.com\n900,docs.rs\n"))
        .with_threats(threats);
    standard_pipeline(sources, pages).vet(Arc::new(vet))
}

#[tokio::test]
async fn the_vet_removes_listed_addresses_and_never_opens_risky_ones() {
    let listed = "http://kit.example/login.php";
    let lookalike = "https://paypal-secure-login.example.com/verify/account";
    let honest = "https://docs.rs/tokio";
    let q = "tokio runtime";
    let a = FakeSource::ok(
        "a",
        vec![
            hit(listed, "Tokio runtime login", "tokio runtime"),
            hit(lookalike, "Tokio runtime verify", "tokio runtime"),
            hit(
                honest,
                "Tokio runtime",
                "Tokio is an asynchronous runtime for Rust",
            ),
        ],
    );
    let pages = FakePages::new(&[
        (lookalike, page("Tokio runtime verify", &SPAM)),
        (honest, page("Tokio runtime", &RUNTIME)),
        (listed, page("Tokio runtime login", &SPAM)),
    ]);
    let p = vetted(vec![a], pages.clone());

    let resp = p.search(&SearchRequest::new(q)).await.unwrap();
    let urls: Vec<&str> = resp.hits.iter().map(|h| h.url.as_str()).collect();
    assert!(!urls.contains(&listed), "{urls:?}");
    assert!(
        resp.degraded.iter().any(|d| d.contains("kit.example")),
        "{:?}",
        resp.degraded
    );
    let requested = pages.requested.lock().unwrap().clone();
    assert!(
        !requested.iter().any(|u| u == listed || u == lookalike),
        "{requested:?}"
    );
    assert!(requested.iter().any(|u| u == honest));
    // The look-alike is still shown, flagged, below the page that was read.
    assert_eq!(resp.hits[0].url, honest);
    let flagged = resp.hits.iter().find(|h| h.url == lookalike).unwrap();
    let info = flagged.vet.as_ref().expect("flagged");
    assert!(
        info.flags.iter().any(|f| f == "brand_in_subdomain"),
        "{info:?}"
    );
    assert!(flagged.score_parts.vet < 0.0 && flagged.content.is_none());
}

#[tokio::test]
async fn official_and_popular_sites_gain_a_little_and_say_so() {
    let official = "https://www.paypal.com/en/home";
    let small = "https://blog.example.org/paypal-guide";
    let a = FakeSource::ok(
        "a",
        vec![
            hit(small, "Paypal guide", "A guide to paypal accounts and fees"),
            hit(
                official,
                "Paypal guide",
                "A guide to paypal accounts and fees",
            ),
        ],
    );
    let p = vetted(vec![a], FakePages::new(&[]));
    let mut req = SearchRequest::new("paypal guide");
    req.mode = Mode::Fast;
    let resp = p.search(&req).await.unwrap();
    assert_eq!(resp.hits[0].url, official, "{:#?}", resp.hits);
    let info = resp.hits[0]
        .vet
        .as_ref()
        .expect("official sites are marked");
    assert_eq!(info.official.as_deref(), Some("PayPal"));
    assert!(resp.hits[0].score_parts.vet > 0.0);
    assert!(
        resp.hits[1].vet.is_none(),
        "an unknown site with nothing wrong carries no mark"
    );
}

#[tokio::test]
async fn without_a_vet_nothing_changes() {
    let a = FakeSource::ok(
        "a",
        vec![hit("http://kit.example/login.php", "t", "tokio runtime")],
    );
    let p = standard_pipeline(vec![a], FakePages::new(&[]));
    let mut req = SearchRequest::new("tokio runtime");
    req.mode = Mode::Fast;
    let resp = p.search(&req).await.unwrap();
    assert_eq!(resp.hits.len(), 1);
    assert!(resp.hits[0].vet.is_none());
}

#[tokio::test]
async fn a_page_that_collects_a_brand_login_falls_behind_after_it_is_read() {
    let kit = "https://camp-7.pages.dev/tokio";
    let honest = "https://docs.rs/tokio";
    let a = FakeSource::ok(
        "a",
        vec![
            hit(
                kit,
                "Tokio runtime",
                "Tokio is an asynchronous runtime for Rust",
            ),
            hit(
                honest,
                "Tokio runtime",
                "Tokio is an asynchronous runtime for Rust",
            ),
        ],
    );
    let trap = page("PayPal Tokio runtime", &RUNTIME).replace(
        "</body>",
        r#"<form action="https://collect.example/p.php"><input type="password" name="p"></form></body>"#,
    );
    let pages = FakePages::new(&[(kit, trap), (honest, page("Tokio runtime", &RUNTIME))]);
    let p = vetted(vec![a], pages);
    let resp = p
        .search(&SearchRequest::new("tokio runtime"))
        .await
        .unwrap();
    let k = resp.hits.iter().find(|h| h.url == kit).unwrap();
    let flags = &k.vet.as_ref().expect("flagged").flags;
    assert!(
        flags.iter().any(|f| f == "form_elsewhere")
            && flags.iter().any(|f| f == "brand_login_page"),
        "{flags:?}"
    );
    assert_eq!(resp.hits[0].url, honest, "{:#?}", resp.hits);
}

struct CountingSections(Mutex<Vec<String>>);

impl seesea_pipeline::sections::SectionScorer for CountingSections {
    fn score(&self, _q: &str, texts: &[&str]) -> Result<Vec<f64>, String> {
        self.0
            .lock()
            .unwrap()
            .extend(texts.iter().map(|t| t.to_string()));
        Ok(texts.iter().map(|_| 0.5).collect())
    }
}

#[tokio::test]
async fn the_section_model_only_reads_pages_that_are_shown() {
    let long = |tag: &str| {
        let sec = |n: usize| {
            format!(
                "<h2>Part {n} of {tag}</h2><p>{tag} tokio runtime part {n}. {}</p>",
                RUNTIME[n % 3]
            )
        };
        format!(
            "<html><head><title>Tokio runtime {tag}</title></head><body><article><h1>Tokio runtime {tag}</h1>{}</article></body></html>",
            (0..6).map(sec).collect::<String>()
        )
    };
    let urls: Vec<String> = (0..4)
        .map(|i| format!("https://site{i}.example/tokio"))
        .collect();
    let a = FakeSource::ok(
        "a",
        urls.iter()
            .map(|u| {
                hit(
                    u,
                    "Tokio runtime",
                    "Tokio is an asynchronous runtime for Rust",
                )
            })
            .collect(),
    );
    let pages: Vec<(&str, String)> = urls
        .iter()
        .enumerate()
        .map(|(i, u)| (u.as_str(), long(&format!("zz{i}zz"))))
        .collect();
    let counter = Arc::new(CountingSections(Mutex::new(Vec::new())));
    let p = standard_pipeline(vec![a], FakePages::new(&pages)).section_scorer(counter.clone());
    let mut req = SearchRequest::new("tokio runtime");
    req.limit = 2;
    req.max_chars_per_result = 300;
    let resp = p.search(&req).await.unwrap();
    assert_eq!(resp.hits.len(), 2);
    let shown: Vec<String> = resp.hits.iter().map(|h| h.url.clone()).collect();
    let scored = counter.0.lock().unwrap().clone();
    assert!(!scored.is_empty(), "the model should have been asked");
    for (i, u) in urls.iter().enumerate() {
        let mark = format!("zz{i}zz");
        if !shown.contains(u) {
            assert!(
                !scored.iter().any(|t| t.contains(&mark)),
                "{u} was not shown but was scored"
            );
        }
    }
    assert!(
        resp.hits
            .iter()
            .all(|h| !h.content.as_ref().unwrap().sections.is_empty())
    );
}
