//! Fetching result pages and extracting their main content.

use crate::error::{Error, Result};
use crate::store::Cache;
use async_trait::async_trait;
use futures::StreamExt;
use seesea_extract::{ExtractResult, Extractor, Options, Scope};
use seesea_fetch::{FetchError, FetchOptions};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct FetchedPage {
    pub final_url: String,
    pub html: String,
}

/// Where page HTML comes from. The default goes to the network; tests supply canned pages.
#[async_trait]
pub trait PageFetcher: Send + Sync {
    async fn fetch(&self, url: &str) -> std::result::Result<FetchedPage, FetchError>;
}

pub struct NetFetcher {
    pub options: FetchOptions,
}

impl NetFetcher {
    pub fn new(options: FetchOptions) -> Self {
        Self { options }
    }
}

#[async_trait]
impl PageFetcher for NetFetcher {
    async fn fetch(&self, url: &str) -> std::result::Result<FetchedPage, FetchError> {
        let page = seesea_fetch::fetch(url, &self.options).await?;
        Ok(FetchedPage {
            final_url: page.final_url,
            html: page.html,
        })
    }
}

/// Pages rendered by a headless browser (Lightpanda); see [`seesea_fetch::render`].
pub struct RenderFetcher {
    pub renderer: seesea_fetch::render::Renderer,
    pub options: FetchOptions,
}

#[async_trait]
impl PageFetcher for RenderFetcher {
    async fn fetch(&self, url: &str) -> std::result::Result<FetchedPage, FetchError> {
        let page = self.renderer.render(url, &self.options).await?;
        Ok(FetchedPage {
            final_url: page.final_url,
            html: page.html,
        })
    }
}

/// What is remembered about a URL between fetches, to tune how long its page is kept.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct PageHistory {
    /// FNV-1a of the HTML at the last fetch.
    hash: u64,
    /// Lifetime given to the cached copy at the last fetch, in milliseconds.
    ttl_ms: u64,
}

fn fnv1a(data: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in data {
        h ^= *b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

/// Wraps a fetcher with a cache of page HTML. Pages above `max_cached_bytes` are not cached.
///
/// How long a page is kept adapts per URL: each time a copy has expired and the page is fetched
/// again, the lifetime doubles if the content is unchanged and halves if it changed, within
/// `[min_ttl, max_ttl]`. Documentation sites drift to days, front pages to minutes.
pub struct CachedFetcher {
    inner: Arc<dyn PageFetcher>,
    cache: Arc<dyn Cache>,
    ttl: Duration,
    min_ttl: Duration,
    max_ttl: Duration,
    max_cached_bytes: usize,
    /// Namespaces the keys, so a second fetcher (the renderer) does not share entries with the first.
    prefix: &'static str,
}

/// How long the per-URL history outlives the page itself.
const HISTORY_TTL: Duration = Duration::from_secs(30 * 24 * 3600);

impl CachedFetcher {
    pub fn new(inner: Arc<dyn PageFetcher>, cache: Arc<dyn Cache>, ttl: Duration) -> Self {
        Self::with_ttl_range(inner, cache, ttl, ttl, ttl)
    }

    pub fn with_ttl_range(
        inner: Arc<dyn PageFetcher>,
        cache: Arc<dyn Cache>,
        ttl: Duration,
        min_ttl: Duration,
        max_ttl: Duration,
    ) -> Self {
        Self {
            inner,
            cache,
            ttl,
            min_ttl,
            max_ttl,
            max_cached_bytes: 2 * 1024 * 1024,
            prefix: "",
        }
    }

    /// Keep this fetcher's entries apart from another's (`page:` becomes `<prefix>page:`).
    pub fn with_prefix(mut self, prefix: &'static str) -> Self {
        self.prefix = prefix;
        self
    }

    async fn next_ttl(&self, url: &str, hash: u64) -> Duration {
        let key = format!("{}pagehist:{url}", self.prefix);
        let previous = self
            .cache
            .get(&key)
            .await
            .and_then(|b| serde_json::from_slice::<PageHistory>(&b).ok());
        let ttl = match previous {
            None => self.ttl,
            Some(h) if h.hash == hash => Duration::from_millis(h.ttl_ms.saturating_mul(2)),
            Some(h) => Duration::from_millis(h.ttl_ms / 2),
        }
        .clamp(self.min_ttl, self.max_ttl.max(self.min_ttl));
        if let Ok(bytes) = serde_json::to_vec(&PageHistory {
            hash,
            ttl_ms: ttl.as_millis() as u64,
        }) {
            self.cache.put(&key, bytes, HISTORY_TTL).await;
        }
        ttl
    }
}

#[async_trait]
impl PageFetcher for CachedFetcher {
    async fn fetch(&self, url: &str) -> std::result::Result<FetchedPage, FetchError> {
        let key = format!("{}page:{url}", self.prefix);
        if let Some(bytes) = self.cache.get(&key).await
            && let Ok(text) = String::from_utf8(bytes)
            && let Some((final_url, html)) = text.split_once('\n')
        {
            return Ok(FetchedPage {
                final_url: final_url.to_string(),
                html: html.to_string(),
            });
        }
        let page = self.inner.fetch(url).await?;
        if page.html.len() <= self.max_cached_bytes {
            let ttl = self.next_ttl(url, fnv1a(page.html.as_bytes())).await;
            self.cache
                .put(
                    &key,
                    format!("{}\n{}", page.final_url, page.html).into_bytes(),
                    ttl,
                )
                .await;
        }
        Ok(page)
    }
}

#[derive(Debug, Clone)]
pub struct Loaded {
    pub final_url: String,
    pub result: ExtractResult,
    /// The content comes from a headless browser, because the static HTML held none.
    pub rendered: bool,
}

async fn extract_blocking(
    extractor: &Arc<Extractor>,
    html: String,
    url: String,
    scope: Scope,
) -> Result<ExtractResult> {
    let extractor = Arc::clone(extractor);
    tokio::task::spawn_blocking(move || {
        extractor.extract(
            &html,
            &url,
            &Options {
                scope,
                ..Options::default()
            },
        )
    })
    .await
    .map_err(|e| Error::Extract(e.to_string()))
}

/// Fetch and extract one page. Parsing and scoring are CPU work, so they run on the blocking pool.
///
/// When the static HTML holds no content (`needs_render`) and a `renderer` is given, the page is
/// fetched again through it and the result with more extracted text wins.
pub async fn load_page(
    fetcher: &dyn PageFetcher,
    renderer: Option<&dyn PageFetcher>,
    extractor: &Arc<Extractor>,
    url: &str,
    scope: Scope,
    timeout: Duration,
) -> Result<Loaded> {
    let page = tokio::time::timeout(timeout, fetcher.fetch(url))
        .await
        .map_err(|_| Error::Timeout {
            stage: "fetch",
            ms: timeout.as_millis() as u64,
        })??;
    let result = extract_blocking(extractor, page.html, page.final_url.clone(), scope).await?;
    if result.diagnostics.not_text {
        return Err(Error::Extract(
            "the response is not text (compressed or binary data)".into(),
        ));
    }
    let mut loaded = Loaded {
        final_url: page.final_url,
        result,
        rendered: false,
    };
    if loaded.result.needs_render
        && let Some(renderer) = renderer
    {
        // A failed render leaves the static result in place: it is still what the server sent.
        if let Ok(Ok(page)) = tokio::time::timeout(timeout, renderer.fetch(url)).await
            && let Ok(result) =
                extract_blocking(extractor, page.html, page.final_url.clone(), scope).await
            && result.diagnostics.selected_chars > loaded.result.diagnostics.selected_chars
        {
            loaded = Loaded {
                final_url: page.final_url,
                result,
                rendered: true,
            };
        }
    }
    Ok(loaded)
}

/// Load many pages with bounded concurrency. Results come back in the order of `urls`.
pub async fn load_pages(
    fetcher: &Arc<dyn PageFetcher>,
    renderer: Option<&Arc<dyn PageFetcher>>,
    extractor: &Arc<Extractor>,
    urls: &[String],
    scope: Scope,
    concurrency: usize,
    timeout: Duration,
) -> Vec<Result<Loaded>> {
    load_pages_then(fetcher, renderer, extractor, urls, scope, concurrency, timeout, None).await
}

/// Called with a page's position in `urls` and the page, the moment that page is ready (not when all are).
pub type OnLoaded = Arc<dyn Fn(usize, &Loaded) + Send + Sync>;

#[allow(clippy::too_many_arguments)]
pub async fn load_pages_then(
    fetcher: &Arc<dyn PageFetcher>,
    renderer: Option<&Arc<dyn PageFetcher>>,
    extractor: &Arc<Extractor>,
    urls: &[String],
    scope: Scope,
    concurrency: usize,
    timeout: Duration,
    on_loaded: Option<OnLoaded>,
) -> Vec<Result<Loaded>> {
    futures::stream::iter(urls.iter().cloned().enumerate())
        .map(|(i, url)| {
            let fetcher = Arc::clone(fetcher);
            let renderer = renderer.cloned();
            let extractor = Arc::clone(extractor);
            let on_loaded = on_loaded.clone();
            async move {
                let out = load_page(fetcher.as_ref(), renderer.as_deref(), &extractor, &url, scope, timeout).await;
                if let (Ok(l), Some(cb)) = (&out, &on_loaded) {
                    cb(i, l);
                }
                out
            }
        })
        .buffered(concurrency.max(1))
        .collect()
        .await
}

#[cfg(test)]
pub(crate) mod testing {
    use super::*;
    use std::collections::HashMap;
    use std::sync::Mutex;

    /// Serves canned HTML by URL and counts requests.
    pub struct MockFetcher {
        pub pages: HashMap<String, String>,
        pub delay: Option<Duration>,
        pub calls: Mutex<Vec<String>>,
    }

    impl MockFetcher {
        pub fn new(pages: &[(&str, &str)]) -> Self {
            Self {
                pages: pages
                    .iter()
                    .map(|(u, h)| (u.to_string(), h.to_string()))
                    .collect(),
                delay: None,
                calls: Mutex::new(Vec::new()),
            }
        }
    }

    #[async_trait]
    impl PageFetcher for MockFetcher {
        async fn fetch(&self, url: &str) -> std::result::Result<FetchedPage, FetchError> {
            self.calls.lock().unwrap().push(url.to_string());
            if let Some(d) = self.delay {
                tokio::time::sleep(d).await;
            }
            match self.pages.get(url) {
                Some(html) => Ok(FetchedPage {
                    final_url: url.to_string(),
                    html: html.clone(),
                }),
                None => Err(FetchError::Status(404)),
            }
        }
    }

    pub fn article(title: &str, paragraphs: &[&str]) -> String {
        let body: String = paragraphs.iter().map(|p| format!("<p>{p}</p>")).collect();
        format!(
            "<html><head><title>{title}</title></head><body><nav><a href=\"/\">Home</a> <a href=\"/about\">About</a></nav><article><h1>{title}</h1>{body}</article><footer>Copyright notice and links</footer></body></html>"
        )
    }
}

#[cfg(test)]
mod tests {
    use super::testing::*;
    use super::*;
    use crate::store::MemoryCache;

    const P1: &str = "Tokio is an asynchronous runtime for the Rust programming language, providing the building blocks needed for writing network applications.";
    const P2: &str = "It gives you the flexibility to target a wide range of systems, from large servers with dozens of cores to small embedded devices.";

    #[tokio::test]
    async fn extracts_main_content_and_keeps_order() {
        let fetcher: Arc<dyn PageFetcher> = Arc::new(MockFetcher::new(&[
            ("https://a.org/1", &article("One", &[P1, P2])),
            ("https://a.org/2", &article("Two", &[P2, P1])),
        ]));
        let extractor = Arc::new(Extractor::new());
        let urls = vec![
            "https://a.org/2".to_string(),
            "https://a.org/missing".to_string(),
            "https://a.org/1".to_string(),
        ];
        let out = load_pages(
            &fetcher,
            None,
            &extractor,
            &urls,
            Scope::Core,
            4,
            Duration::from_secs(5),
        )
        .await;
        assert_eq!(out.len(), 3);
        assert!(
            out[0]
                .as_ref()
                .unwrap()
                .result
                .core
                .contains("asynchronous runtime")
        );
        assert_eq!(out[1].as_ref().unwrap_err().code(), "fetch_failed");
        assert!(out[2].as_ref().unwrap().result.core.contains("flexibility"));
    }

    #[tokio::test]
    async fn slow_pages_time_out() {
        let mut mock = MockFetcher::new(&[("https://a.org/1", &article("One", &[P1]))]);
        mock.delay = Some(Duration::from_millis(200));
        let err = load_page(
            &mock,
            None,
            &Arc::new(Extractor::new()),
            "https://a.org/1",
            Scope::Core,
            Duration::from_millis(20),
        )
        .await
        .unwrap_err();
        assert_eq!(err.code(), "timeout");
    }

    #[tokio::test]
    async fn binary_bodies_are_an_extract_error_not_content() {
        let garbage: String = (0..3000u32)
            .map(|i| char::from_u32(if i % 2 == 0 { 0xFFFD } else { 2 }).unwrap())
            .collect();
        let mock = MockFetcher::new(&[("https://a.org/bin", &garbage)]);
        let err = load_page(
            &mock,
            None,
            &Arc::new(Extractor::new()),
            "https://a.org/bin",
            Scope::Core,
            Duration::from_secs(5),
        )
        .await
        .unwrap_err();
        assert_eq!(err.code(), "extract_failed");
    }

    const SHELL: &str = r#"<html><head><title>App</title></head><body><div id="root"></div><script src="/app.js"></script></body></html>"#;

    #[tokio::test]
    async fn an_empty_app_shell_is_fetched_again_through_the_renderer() {
        let url = "https://a.org/app";
        let plain = MockFetcher::new(&[(url, SHELL)]);
        let browser = MockFetcher::new(&[(url, &article("One", &[P1, P2]))]);
        let extractor = Arc::new(Extractor::new());
        let loaded = load_page(&plain, Some(&browser), &extractor, url, Scope::Core, Duration::from_secs(5))
            .await
            .unwrap();
        assert!(loaded.rendered);
        assert!(loaded.result.core.contains("asynchronous runtime"), "{}", loaded.result.core);
        assert!(!loaded.result.needs_render);
    }

    #[tokio::test]
    async fn pages_with_content_never_reach_the_renderer() {
        let url = "https://a.org/1";
        let plain = MockFetcher::new(&[(url, &article("One", &[P1, P2]))]);
        let browser = MockFetcher::new(&[]);
        let extractor = Arc::new(Extractor::new());
        let loaded = load_page(&plain, Some(&browser), &extractor, url, Scope::Core, Duration::from_secs(5))
            .await
            .unwrap();
        assert!(!loaded.rendered);
        assert!(browser.calls.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn a_failing_renderer_leaves_the_static_result() {
        let url = "https://a.org/app";
        let plain = MockFetcher::new(&[(url, SHELL)]);
        let browser = MockFetcher::new(&[]); // fails: unknown URL
        let extractor = Arc::new(Extractor::new());
        let loaded = load_page(&plain, Some(&browser), &extractor, url, Scope::Core, Duration::from_secs(5))
            .await
            .unwrap();
        assert!(!loaded.rendered);
        assert!(loaded.result.needs_render);
        assert_eq!(browser.calls.lock().unwrap().len(), 1);
    }

    /// Serves whatever is in `body` right now.
    struct Flip(std::sync::Mutex<String>);

    #[async_trait]
    impl PageFetcher for Flip {
        async fn fetch(&self, url: &str) -> std::result::Result<FetchedPage, FetchError> {
            Ok(FetchedPage {
                final_url: url.to_string(),
                html: self.0.lock().unwrap().clone(),
            })
        }
    }

    #[tokio::test]
    async fn page_lifetime_grows_while_a_page_is_stable_and_shrinks_when_it_changes() {
        let cache = Arc::new(MemoryCache::new(10));
        let page = Arc::new(Flip(std::sync::Mutex::new("<p>one</p>".into())));
        let f = CachedFetcher::with_ttl_range(
            page.clone(),
            cache.clone(),
            Duration::from_millis(40),
            Duration::from_millis(10),
            Duration::from_millis(150),
        );
        let ttl = |c: Arc<MemoryCache>| async move {
            let b = c.get("pagehist:https://a.org/x").await.unwrap();
            serde_json::from_slice::<PageHistory>(&b).unwrap().ttl_ms
        };
        f.fetch("https://a.org/x").await.unwrap();
        assert_eq!(ttl(cache.clone()).await, 40);

        tokio::time::sleep(Duration::from_millis(60)).await;
        f.fetch("https://a.org/x").await.unwrap();
        assert_eq!(ttl(cache.clone()).await, 80, "unchanged: doubled");

        tokio::time::sleep(Duration::from_millis(100)).await;
        f.fetch("https://a.org/x").await.unwrap();
        assert_eq!(ttl(cache.clone()).await, 150, "capped at the maximum");

        *page.0.lock().unwrap() = "<p>two</p>".into();
        tokio::time::sleep(Duration::from_millis(170)).await;
        f.fetch("https://a.org/x").await.unwrap();
        assert_eq!(ttl(cache.clone()).await, 75, "changed: halved");
    }

    #[tokio::test]
    async fn cached_fetcher_hits_the_network_once() {
        let mock = Arc::new(MockFetcher::new(&[(
            "https://a.org/1",
            &article("One", &[P1]),
        )]));
        let cached = CachedFetcher::new(
            mock.clone(),
            Arc::new(MemoryCache::new(10)),
            Duration::from_secs(60),
        );
        let first = cached.fetch("https://a.org/1").await.unwrap();
        let second = cached.fetch("https://a.org/1").await.unwrap();
        assert_eq!(first.html, second.html);
        assert_eq!(second.final_url, "https://a.org/1");
        assert_eq!(mock.calls.lock().unwrap().len(), 1);
        assert!(
            cached.fetch("https://a.org/missing").await.is_err(),
            "failures are not cached"
        );
    }
}
