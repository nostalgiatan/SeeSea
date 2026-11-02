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
