// Copyright (C) 2025 nostalgiatan
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

//! 向量化相关性评分模块
//!
//! 完全基于向量嵌入进行语义相关性计算，结合 SIMD 加速和缓存优化。
//! 核心评分因素：向量相似度、时间新鲜度、引擎权威度。

use chrono::{DateTime, Utc};
use seesea_cleaner::simd_utils::simd_cosine_similarity;
use seesea_derive::{SearchQuery, SearchResultItem};
use seesea_sys::controller::get_global_system_controller;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{RwLock, Semaphore};

#[cfg(feature = "python")]
use seesea_python_bindings::py_embedding_callback::{embed_text, get_embedding_callback};

/// 向量缓存
pub struct VectorCache {
    cache: RwLock<HashMap<u64, Vec<f32>>>,
    max_size: usize,
}

impl VectorCache {
    pub fn new(max_size: usize) -> Self {
        Self {
            cache: RwLock::new(HashMap::new()),
            max_size,
        }
    }

    fn hash_text(text: &str) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        text.hash(&mut hasher);
        hasher.finish()
    }

    pub async fn get(&self, text: &str) -> Option<Vec<f32>> {
        let hash = Self::hash_text(text);
        let cache = self.cache.read().await;
        cache.get(&hash).cloned()
    }

    pub async fn set(&self, text: &str, vector: Vec<f32>) {
        let hash = Self::hash_text(text);
        let mut cache = self.cache.write().await;

        if cache.len() >= self.max_size {
            let keys_to_remove: Vec<u64> = cache.keys().take(self.max_size / 2).cloned().collect();
            for key in keys_to_remove {
                cache.remove(&key);
            }
        }

        cache.insert(hash, vector);
    }

    pub async fn clear(&self) {
        let mut cache = self.cache.write().await;
        cache.clear();
    }
}

static VECTOR_CACHE: once_cell::sync::Lazy<Arc<VectorCache>> =
    once_cell::sync::Lazy::new(|| Arc::new(VectorCache::new(10000)));

pub fn get_vector_cache() -> Arc<VectorCache> {
    VECTOR_CACHE.clone()
}

/// 向量化评分权重配置
#[derive(Debug, Clone)]
pub struct VectorScoringWeights {
    pub vector_similarity: f64,
    pub time_freshness: f64,
    pub engine_authority: f64,
}

impl Default for VectorScoringWeights {
    fn default() -> Self {
        Self {
            vector_similarity: 0.70,
            time_freshness: 0.20,
            engine_authority: 0.10,
        }
    }
}

impl VectorScoringWeights {
    pub fn pro_mode() -> Self {
        Self {
            vector_similarity: 0.80,
            time_freshness: 0.15,
            engine_authority: 0.05,
        }
    }
}

/// 向量化相关性评分器
pub struct VectorScorer {
    weights: VectorScoringWeights,
    #[allow(dead_code)]
    semaphore: Arc<Semaphore>,
    #[allow(dead_code)]
    cache: Arc<VectorCache>,
}

impl VectorScorer {
    pub fn new(weights: VectorScoringWeights, max_concurrency: usize) -> Self {
        Self {
            weights,
            semaphore: Arc::new(Semaphore::new(max_concurrency)),
            cache: get_vector_cache(),
        }
    }

    pub fn from_system_controller() -> Self {
        let controller = get_global_system_controller();
        let config = controller.config();
        let max_concurrency = std::cmp::max(2, config.adjustment_interval_ms as usize / 500);
        Self::new(VectorScoringWeights::default(), max_concurrency)
    }

    #[allow(clippy::should_implement_trait)]
    pub fn default() -> Self {
        Self::new(VectorScoringWeights::default(), 4)
    }

    pub fn pro_mode() -> Self {
        Self::new(VectorScoringWeights::pro_mode(), 4)
    }

    #[cfg(feature = "python")]
    async fn get_or_compute_vector(&self, text: &str) -> Option<Vec<f32>> {
        if let Some(vector) = self.cache.get(text).await {
            return Some(vector);
        }

        let _permit = self.semaphore.acquire().await.ok()?;

        match embed_text(text) {
            Ok(vector) => {
                self.cache.set(text, vector.clone()).await;
                Some(vector)
            }
            Err(_) => None,
        }
    }

    #[cfg(not(feature = "python"))]
    async fn get_or_compute_vector(&self, _text: &str) -> Option<Vec<f32>> {
        None
    }

    pub fn compute_similarity(vec1: &[f32], vec2: &[f32]) -> f32 {
        if vec1.len() != vec2.len() || vec1.is_empty() {
            return 0.0;
        }
        simd_cosine_similarity(vec1, vec2)
    }

    pub fn compute_time_freshness(published_date: Option<DateTime<Utc>>) -> f64 {
        match published_date {
            Some(date) => {
                let now = Utc::now();
                let days_old = (now - date).num_days().max(0) as f64;

                match days_old {
                    x if x <= 1.0 => 1.0,
                    x if x <= 7.0 => 0.8,
                    x if x <= 30.0 => 0.6,
                    x if x <= 90.0 => 0.4,
                    x if x <= 365.0 => 0.2,
                    _ => 0.1,
                }
            }
            None => 0.5,
        }
    }

    pub async fn score_item(
        &self,
        item: &SearchResultItem,
        _query: &SearchQuery,
        query_vector: Option<&Vec<f32>>,
        engine_name: &str,
    ) -> f64 {
        let mut score = 0.0;

        if let (Some(q_vec), Some(title_vec)) =
            (query_vector, self.get_or_compute_vector(&item.title).await)
        {
            let similarity = Self::compute_similarity(q_vec, &title_vec);
            score += self.weights.vector_similarity * similarity as f64;
        }

        let time_freshness = Self::compute_time_freshness(item.published_date);
        score += self.weights.time_freshness * time_freshness;

        let authority = super::scoring::get_engine_authority(engine_name);
        score += self.weights.engine_authority * authority;

        score.clamp(0.0, 1.0)
    }

    pub async fn score_results(
        &self,
        items: &mut [SearchResultItem],
        query: &SearchQuery,
        engine_name: &str,
    ) {
        let query_vector = self.get_or_compute_vector(&query.query).await;

        for item in items.iter_mut() {
            item.score = self
                .score_item(item, query, query_vector.as_ref(), engine_name)
                .await;
        }

        items.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
    }
}

#[cfg(feature = "python")]
pub fn is_vector_scoring_available() -> bool {
    get_embedding_callback().is_some()
}

#[cfg(not(feature = "python"))]
pub fn is_vector_scoring_available() -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simd_similarity() {
        let vec1 = vec![1.0, 2.0, 3.0, 4.0];
        let vec2 = vec![1.0, 2.0, 3.0, 4.0];
        let similarity = VectorScorer::compute_similarity(&vec1, &vec2);
        assert!((similarity - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_different_vectors() {
        let vec1 = vec![1.0, 0.0, 0.0, 0.0];
        let vec2 = vec![0.0, 1.0, 0.0, 0.0];
        let similarity = VectorScorer::compute_similarity(&vec1, &vec2);
        assert!((similarity - 0.0).abs() < 1e-6);
    }

    #[test]
    fn test_time_freshness() {
        let now = Utc::now();
        assert_eq!(VectorScorer::compute_time_freshness(Some(now)), 1.0);
        assert_eq!(VectorScorer::compute_time_freshness(None), 0.5);
    }

    #[test]
    fn test_engine_authority() {
        assert_eq!(VectorScorer::get_engine_authority("google"), 1.0);
        assert_eq!(VectorScorer::get_engine_authority("baidu"), 0.95);
        assert!(VectorScorer::get_engine_authority("unknown") < 1.0);
    }
}
