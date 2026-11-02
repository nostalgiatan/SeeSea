//! The index as a search source: lets the pipeline fuse local fresh items with web results.

use crate::Index;
use async_trait::async_trait;
use seesea_pipeline::error::{FailureKind, SourceError};
use seesea_pipeline::{RawHit, Source, SourceQuery, TimeRange};
use seesea_pipeline::semantic::TextEmbedder;
use std::collections::HashMap;
use std::sync::Arc;

/// Items farther than this share of the bits from the query are not a semantic match.
const MAX_DISTANCE_SHARE: f64 = 0.42;

pub struct IndexSource {
    index: Arc<Index>,
    embedder: Option<Arc<dyn TextEmbedder>>,
}

impl IndexSource {
    /// Also look items up by meaning: the query is embedded and compared with the stored bits.
    pub fn embedder(mut self, e: Arc<dyn TextEmbedder>) -> Self {
        self.embedder = Some(e);
        self
    }

    pub fn new(index: Arc<Index>) -> Self {
        Self { index, embedder: None }
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

    fn auxiliary(&self) -> bool {
        true
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
        let embedder = self.embedder.clone();
        let rows = tokio::task::spawn_blocking(move || -> Result<Vec<crate::Stored>, String> {
            let words = index.store.search(&query, since, limit).map_err(|e| e.to_string())?;
            let Some(bits) = embedder.and_then(|e| crate::vectors::query_bits(&index, &*e, &query)) else { return Ok(words) };
            let max = (bits.len() as f64 * 8.0 * MAX_DISTANCE_SHARE) as u32;
            let meaning = index.store.nearest(&bits, since, limit, max).map_err(|e| e.to_string())?;
            // Reciprocal-rank fusion of the two lists.
            let mut score: HashMap<String, (f64, crate::Stored)> = HashMap::new();
            let lists = [words, meaning.into_iter().map(|(s, _)| s).collect()];
            for list in lists {
                for (rank, s) in list.into_iter().enumerate() {
                    score.entry(s.url.clone()).or_insert((0.0, s)).0 += 1.0 / (60.0 + rank as f64);
                }
            }
            let mut merged: Vec<(f64, crate::Stored)> = score.into_values().collect();
            merged.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
            Ok(merged.into_iter().take(limit).map(|(_, s)| s).collect())
        })
        .await
        .map_err(|e| SourceError::new(FailureKind::Other, e.to_string()))?
        .map_err(|e| SourceError::new(FailureKind::Other, e))?;
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
