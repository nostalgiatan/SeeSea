//! Polls due feeds, looks for feeds on sites that appeared in searches, and keeps the store
//! within its retention and size limits. Every failure is confined to the feed or site that
//! caused it: a round never returns an error and never panics through to the caller.

use crate::feed;
use crate::registry::Status;
use crate::{Index, Item};
use futures::stream::{self, StreamExt};
use seesea_pipeline::content::PageFetcher;
use serde::Serialize;
use std::sync::Arc;
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct PollConfig {
    pub concurrency: usize,
    pub feeds_per_round: usize,
    pub timeout: Duration,
    pub max_body_bytes: usize,
    pub items_per_feed: usize,
    /// Sites searched for feeds per round (demand-driven discovery).
    pub discoveries_per_round: usize,
    pub retention_days: i64,
    pub quota_bytes: u64,
}

impl Default for PollConfig {
    fn default() -> Self {
        Self {
            concurrency: 6,
            feeds_per_round: 40,
            timeout: Duration::from_secs(15),
            max_body_bytes: 2 * 1024 * 1024,
            items_per_feed: 100,
            discoveries_per_round: 3,
            retention_days: 30,
            quota_bytes: 512 * 1024 * 1024,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, PartialEq, Eq)]
pub struct PollReport {
    pub polled: usize,
    pub failed: usize,
    pub added: usize,
    pub duplicates: usize,
    pub sites_searched: usize,
    pub feeds_found: usize,
    pub shards_pruned: usize,
}

pub struct Poller {
    pub index: Arc<Index>,
    fetcher: Arc<dyn PageFetcher>,
    /// A headless browser, tried for a site's front page when plain HTML advertises nothing.
    render: Option<Arc<dyn PageFetcher>>,
    cfg: PollConfig,
}

impl Poller {
    pub fn new(index: Arc<Index>, fetcher: Arc<dyn PageFetcher>, cfg: PollConfig) -> Self {
        Self { index, fetcher, render: None, cfg }
    }

    pub fn renderer(mut self, render: Arc<dyn PageFetcher>) -> Self {
        self.render = Some(render);
        self
    }

    async fn get(&self, fetcher: &Arc<dyn PageFetcher>, url: &str) -> Result<String, String> {
        let page = tokio::time::timeout(self.cfg.timeout, fetcher.fetch(url))
            .await
            .map_err(|_| "timed out".to_string())?
            .map_err(|e| e.to_string())?;
        if page.html.len() > self.cfg.max_body_bytes {
            return Err(format!("{} bytes is too large for a feed", page.html.len()));
        }
        Ok(page.html)
    }

    async fn poll_feed(&self, url: &str) -> Result<(feed::Parsed, usize, usize), String> {
        let body = self.get(&self.fetcher, url).await?;
        let mut parsed = feed::parse(&body, url, url)?;
        parsed.items.sort_by(|a, b| b.published.cmp(&a.published));
        let oldest = crate::now() - self.cfg.retention_days * 86_400;
        parsed.items.retain(|i| i.published.is_none_or(|p| p >= oldest));
        parsed.items.truncate(self.cfg.items_per_feed);
        let host = url::Url::parse(url).ok().and_then(|u| u.host_str().map(String::from)).unwrap_or_default();
        let items: Vec<Item> = parsed.items.iter().cloned().map(|i| Item { source: host.clone(), ..i }).collect();
        let index = Arc::clone(&self.index);
        let r = tokio::task::spawn_blocking(move || index.store.ingest(&items))
            .await
            .map_err(|e| e.to_string())?
            .map_err(|e| e.to_string())?;
        Ok((parsed, r.added, r.duplicates))
    }

    /// Feeds a site offers: advertised ones, else the usual places; only those that parse and hold items.
    pub async fn discover(&self, site: &str) -> Vec<(String, feed::Parsed)> {
        let mut candidates = match self.get(&self.fetcher, site).await {
            Ok(html) => feed::advertised(&html, site),
            Err(_) => Vec::new(),
        };
        if candidates.is_empty()
            && let Some(render) = &self.render
            && let Ok(html) = self.get(render, site).await
        {
            candidates = feed::advertised(&html, site);
        }
        if candidates.is_empty() {
            candidates = feed::usual_places(site);
        }
        candidates.truncate(8);
        let mut found = Vec::new();
        for url in candidates {
            if let Ok(body) = self.get(&self.fetcher, &url).await
                && let Ok(p) = feed::parse(&body, &url, &url)
                && !p.items.is_empty()
            {
                found.push((url, p));
            }
        }
        found
    }

    /// Register what `discover` finds. `origin` is `user` for a person's request (trusted at once)
    /// and `discovered` for automatic finds (on probation).
    pub async fn add_site(&self, site: &str, origin: &str) -> Vec<String> {
        let status = if origin == "user" { Status::Active } else { Status::Probation };
        let mut added = Vec::new();
        for (url, _) in self.discover(site).await {
            if self.index.registry.add(&url, origin, status).unwrap_or(false) {
                added.push(url);
            }
        }
        added
    }

    pub async fn poll_once(&self) -> PollReport {
        let mut report = PollReport::default();
        let due = self.index.registry.due(crate::now(), self.cfg.feeds_per_round).unwrap_or_default();
        let results: Vec<_> = stream::iter(due)
            .map(|f| async move {
                let r = self.poll_feed(&f.url).await;
                (f.url, r)
            })
            .buffer_unordered(self.cfg.concurrency.max(1))
            .collect()
            .await;
        for (url, r) in results {
            report.polled += 1;
            match r {
                Ok((parsed, added, dup)) => {
                    report.added += added;
                    report.duplicates += dup;
                    let _ = self.index.registry.record(&url, Ok((added, Some(&parsed.title))));
                }
                Err(e) => {
                    report.failed += 1;
                    let _ = self.index.registry.record(&url, Err(&e));
                }
            }
        }
        for host in self.index.registry.pending_hosts(self.cfg.discoveries_per_round).unwrap_or_default() {
            report.sites_searched += 1;
            let n = self.add_site(&format!("https://{host}/"), "discovered").await.len();
            report.feeds_found += n;
            let _ = self.index.registry.host_done(&host, n);
        }
        let index = Arc::clone(&self.index);
        let (days, quota) = (self.cfg.retention_days, self.cfg.quota_bytes);
        report.shards_pruned = tokio::task::spawn_blocking(move || index.store.prune(days, quota)).await.unwrap_or(0);
        report
    }

    /// Poll forever, once per `every`.
    pub async fn run(&self, every: Duration) {
        loop {
            let _ = self.poll_once().await;
            tokio::time::sleep(every).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use seesea_pipeline::content::FetchedPage;
    use std::collections::HashMap;
    use std::sync::Mutex;

    struct Pages(Mutex<HashMap<String, String>>);

    #[async_trait]
    impl PageFetcher for Pages {
        async fn fetch(&self, url: &str) -> Result<FetchedPage, seesea_fetch::FetchError> {
            self.0
                .lock()
                .unwrap()
                .get(url)
                .map(|h| FetchedPage { final_url: url.into(), html: h.clone() })
                .ok_or(seesea_fetch::FetchError::Status(404))
        }
    }

    fn rss(items: &[(&str, &str)]) -> String {
        let body: String = items
            .iter()
            .map(|(u, t)| format!("<item><title>{t}</title><link>{u}</link><description>about {t}</description></item>"))
            .collect();
        format!("<rss version=\"2.0\"><channel><title>Site</title>{body}</channel></rss>")
    }

    fn setup(pages: &[(&str, String)]) -> (tempfile::TempDir, Poller) {
        let dir = tempfile::tempdir().unwrap();
        let fetcher = Arc::new(Pages(Mutex::new(pages.iter().map(|(u, b)| (u.to_string(), b.clone())).collect())));
        let poller = Poller::new(Index::open(dir.path()), fetcher, PollConfig::default());
        (dir, poller)
    }

    #[tokio::test]
    async fn a_round_ingests_new_items_once_and_isolates_broken_feeds() {
        let good = rss(&[("https://g.org/1", "Rust news one"), ("https://g.org/2", "Rust news two")]);
        let (_d, p) = setup(&[("https://g.org/feed", good), ("https://h.org/feed", "<html>not a feed</html>".into())]);
        for u in ["https://g.org/feed", "https://h.org/feed", "https://gone.org/feed"] {
            p.index.registry.add(u, "user", Status::Active).unwrap();
        }
        let r = p.poll_once().await;
        assert_eq!((r.polled, r.failed, r.added), (3, 2, 2), "{r:?}");
        assert_eq!(p.index.store.search("rust news", None, 5).unwrap().len(), 2);
        let feeds = p.index.registry.list().unwrap();
        assert_eq!(feeds.iter().filter(|f| f.fails == 1).count(), 2);
        assert_eq!(feeds.iter().find(|f| f.url == "https://g.org/feed").unwrap().title, "Site");
        // Nothing is due right after, so a second round does nothing; forcing it finds only duplicates.
        assert_eq!(p.poll_once().await.polled, 0);
    }

    #[tokio::test]
    async fn sites_are_searched_for_feeds_advertised_or_in_the_usual_places() {
        let front = r#"<head><link rel="alternate" type="application/rss+xml" href="/custom.xml"></head>"#;
        let (_d, p) = setup(&[
            ("https://a.org/", front.into()),
            ("https://a.org/custom.xml", rss(&[("https://a.org/1", "Alpha story")])),
            ("https://b.org/", "<html>no links</html>".into()),
            ("https://b.org/atom.xml", rss(&[("https://b.org/1", "Beta story")])),
            ("https://c.org/", "<html>nothing</html>".into()),
        ]);
        assert_eq!(p.add_site("https://a.org/", "user").await, vec!["https://a.org/custom.xml"]);
        for h in ["b.org", "c.org"] {
            p.index.registry.queue_host(h).unwrap();
        }
        let r = p.poll_once().await;
        assert_eq!((r.sites_searched, r.feeds_found), (2, 1), "{r:?}");
        let feeds = p.index.registry.list().unwrap();
        let b = feeds.iter().find(|f| f.url == "https://b.org/atom.xml").unwrap();
        assert_eq!((b.origin.as_str(), b.status), ("discovered", Status::Probation));
        assert!(p.index.registry.pending_hosts(5).unwrap().is_empty(), "each site is tried once");
        p.poll_once().await; // the feed found for b.org is polled once it exists
        assert_eq!(p.index.store.search("alpha beta story", None, 5).unwrap().len(), 2);
    }

    #[tokio::test]
    async fn oversized_bodies_are_refused() {
        let big = format!("<rss><channel>{}</channel></rss>", "x".repeat(3 * 1024 * 1024));
        let (_d, p) = setup(&[("https://big.org/feed", big)]);
        p.index.registry.add("https://big.org/feed", "user", Status::Active).unwrap();
        assert_eq!(p.poll_once().await.failed, 1);
    }
}
