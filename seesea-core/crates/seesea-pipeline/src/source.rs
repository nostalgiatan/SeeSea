//! What the pipeline needs from a search source.

use crate::error::SourceError;
use crate::model::{Attribution, TimeRange};
use async_trait::async_trait;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone)]
pub struct SourceQuery {
    pub query: String,
    pub limit: usize,
    pub page: usize,
    pub time_range: Option<TimeRange>,
    pub language: Option<String>,
}

/// One result as a source reports it.
#[derive(Debug, Clone, Default)]
pub struct RawHit {
    pub url: String,
    pub title: String,
    pub snippet: String,
    pub published: Option<DateTime<Utc>>,
    /// Further engines behind the same result, when the source is itself an aggregator (SearXNG).
    /// Each one counts as an independent source in fusion.
    pub upstream: Vec<Attribution>,
}

#[async_trait]
pub trait Source: Send + Sync {
    fn name(&self) -> &str;

    /// Weight of this source's ranking in fusion; 1.0 is neutral.
    fn weight(&self) -> f64 {
        1.0
    }

    async fn search(&self, query: &SourceQuery) -> Result<Vec<RawHit>, SourceError>;
}
