//! Adapters from the search engines in `seesea-search` to the pipeline's [`Source`] trait.

use async_trait::async_trait;
use seesea_derive::{SearchEngine, SearchQuery, TimeRange as EngineTimeRange};
use seesea_pipeline::error::{FailureKind, SourceError};
use seesea_pipeline::{Attribution, RawHit, Source, SourceQuery, TimeRange};
use seesea_search::search::engines::{BaiduEngine, BingEngine, BraveEngine, SearxngEngine, SogouEngine};
use std::sync::Arc;

pub struct EngineSource {
    name: String,
    weight: f64,
    engine: Arc<dyn SearchEngine + Send + Sync>,
    /// The engine reports the engines behind each result (SearXNG).
    aggregator: bool,
}

impl EngineSource {
    pub fn new(name: &str, weight: f64, engine: Arc<dyn SearchEngine + Send + Sync>, aggregator: bool) -> Self {
        Self { name: name.to_string(), weight, engine, aggregator }
    }
}

/// Best-effort classification of an engine's error message.
pub fn classify(message: &str) -> FailureKind {
    let m = message.to_lowercase();
    if m.contains("timed out") || m.contains("timeout") {
        FailureKind::Timeout
    } else if ["403", "429", "captcha", "blocked", "forbidden", "too many requests"].iter().any(|k| m.contains(k)) {
        FailureKind::Blocked
    } else if ["parse", "json", "expected value", "invalid type", "missing field", "eof"].iter().any(|k| m.contains(k)) {
        FailureKind::Parse
    } else if ["status", "http", "request failed", "connect", "dns", "refused"].iter().any(|k| m.contains(k)) {
        FailureKind::Http
    } else {
        FailureKind::Other
    }
}

fn engine_time_range(t: TimeRange) -> EngineTimeRange {
    match t {
        TimeRange::Day => EngineTimeRange::Day,
        TimeRange::Week => EngineTimeRange::Week,
        TimeRange::Month => EngineTimeRange::Month,
        TimeRange::Year => EngineTimeRange::Year,
    }
}

/// SearXNG lists the engines behind a result and their ranks in two parallel comma-separated fields.
pub fn upstream_attributions(engines: &str, positions: &str) -> Vec<Attribution> {
    let ranks: Vec<u32> = positions.split(',').filter_map(|p| p.trim().parse().ok()).collect();
    engines
        .split(',')
        .map(str::trim)
        .filter(|e| !e.is_empty())
        .enumerate()
        .map(|(i, e)| Attribution { source: format!("searxng/{e}"), rank: ranks.get(i).copied().unwrap_or(10).max(1) })
        .collect()
}

#[async_trait]
impl Source for EngineSource {
    fn name(&self) -> &str {
        &self.name
    }

    fn weight(&self) -> f64 {
        self.weight
    }

    async fn search(&self, q: &SourceQuery) -> Result<Vec<RawHit>, SourceError> {
        let query = SearchQuery {
            query: q.query.clone(),
            page_size: q.limit,
            page: q.page,
            language: q.language.clone(),
            time_range: q.time_range.map(engine_time_range),
            ..Default::default()
        };
        let result = self.engine.search(&query).await.map_err(|e| {
            let message = e.to_string();
            SourceError::new(classify(&message), message)
        })?;
        Ok(result
            .items
            .into_iter()
            .filter(|i| !i.url.trim().is_empty())
            .map(|i| {
                let upstream = if self.aggregator {
                    upstream_attributions(i.metadata.get("engines").map_or("", String::as_str), i.metadata.get("positions").map_or("", String::as_str))
                } else {
                    Vec::new()
                };
                RawHit { url: i.url, title: i.title, snippet: i.content, published: i.published_date, upstream }
            })
            .collect())
    }
}

#[derive(Debug, Clone, Default)]
pub struct SourceSettings {
    pub searxng_url: Option<String>,
    pub brave_key: Option<String>,
    /// Include the built-in scraping engines (Bing, Baidu, Sogou).
    pub native_engines: bool,
}

/// Build every source the settings allow. Order is stable so output is reproducible.
pub fn build_sources(settings: &SourceSettings, client: Arc<seesea_net::client::HttpClient>) -> Vec<Arc<dyn Source>> {
    let mut out: Vec<Arc<dyn Source>> = Vec::new();
    if let Some(url) = settings.brave_key.as_deref().filter(|k| !k.is_empty()) {
        out.push(Arc::new(EngineSource::new("brave", 1.2, Arc::new(BraveEngine::with_client(url, Arc::clone(&client))), false)));
    }
    if let Some(url) = settings.searxng_url.as_deref().filter(|u| !u.is_empty()) {
        out.push(Arc::new(EngineSource::new("searxng", 1.0, Arc::new(SearxngEngine::with_client(url, Arc::clone(&client))), true)));
    }
    if settings.native_engines {
        out.push(Arc::new(EngineSource::new("bing", 1.0, Arc::new(BingEngine::with_client(Arc::clone(&client))), false)));
        out.push(Arc::new(EngineSource::new("baidu", 0.9, Arc::new(BaiduEngine::with_client(Arc::clone(&client))), false)));
        out.push(Arc::new(EngineSource::new("sogou", 0.8, Arc::new(SogouEngine::with_client(client)), false)));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_recognises_common_failures() {
        assert_eq!(classify("operation timed out"), FailureKind::Timeout);
        assert_eq!(classify("Brave API returned 429 Too Many Requests"), FailureKind::Blocked);
        assert_eq!(classify("expected value at line 1 column 1"), FailureKind::Parse);
        assert_eq!(classify("GET request failed: error sending request"), FailureKind::Http);
        assert_eq!(classify("something odd"), FailureKind::Other);
    }

    #[test]
    fn upstream_attributions_pair_engines_with_ranks() {
        let a = upstream_attributions("google, brave ,duckduckgo", "1,4");
        assert_eq!(a.len(), 3);
        assert_eq!((a[0].source.as_str(), a[0].rank), ("searxng/google", 1));
        assert_eq!((a[1].source.as_str(), a[1].rank), ("searxng/brave", 4));
        assert_eq!(a[2].rank, 10, "a missing rank is treated as a low one");
        assert!(upstream_attributions("", "").is_empty());
    }
}
