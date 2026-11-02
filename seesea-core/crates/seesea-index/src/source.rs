//! The index as a search source: lets the pipeline fuse local fresh items with web results.

use crate::Index;
use async_trait::async_trait;
use seesea_pipeline::error::{FailureKind, SourceError};
use seesea_pipeline::{RawHit, Source, SourceQuery, TimeRange};
use std::sync::Arc;

pub struct IndexSource {
    index: Arc<Index>,
}

impl IndexSource {
    pub fn new(index: Arc<Index>) -> Self {
        Self { index }
    }
}

#[async_trait]
impl Source for IndexSource {
    fn name(&self) -> &str {
        "local"
    }

    fn weight(&self) -> f64 {
        0.9
    }

    async fn search(&self, q: &SourceQuery) -> Result<Vec<RawHit>, SourceError> {
        let day = 86_400;
        let since = q.time_range.map(|r| {
            crate::now()
                - match r {
                    TimeRange::Day => day,
                    TimeRange::Week => 7 * day,
                    TimeRange::Month => 30 * day,
                    TimeRange::Year => 365 * day,
                }
        });
        let (index, query, limit) = (Arc::clone(&self.index), q.query.clone(), q.limit.max(1));
        let rows = tokio::task::spawn_blocking(move || index.store.search(&query, since, limit))
            .await
            .map_err(|e| SourceError::new(FailureKind::Other, e.to_string()))?
            .map_err(|e| SourceError::new(FailureKind::Other, e.to_string()))?;
        Ok(rows
            .into_iter()
            .map(|s| RawHit {
                url: s.url,
                title: s.title,
                snippet: s.lead,
                published: s.published.and_then(|p| chrono::DateTime::from_timestamp(p, 0)),
                upstream: Vec::new(),
            })
            .collect())
    }
}
