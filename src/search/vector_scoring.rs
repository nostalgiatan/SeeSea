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
//! 使用向量嵌入进行语义相关性计算，结合 SIMD 加速和缓存优化。
//! 支持标准模式（BM25 + 轻量向量）和 Pro 模式（高质量向量）。

use crate::cleaner::simd_utils::simd_cosine_similarity;
use crate::derive::{SearchQuery, SearchResultItem};
use crate::sys::controller::get_global_system_controller;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{RwLock, Semaphore};

#[cfg(feature = "python")]
use crate::python_bindings::py_embedding_callback::{embed_text, get_embedding_callback};

/// 向量缓存
pub struct VectorCache {
    /// 缓存映射：文本哈希 -> 向量
    cache: RwLock<HashMap<u64, Vec<f32>>>,
    /// 最大缓存大小
    max_size: usize,
}

impl VectorCache {
    /// 创建新的向量缓存
    pub fn new(max_size: usize) -> Self {
        Self {
            cache: RwLock::new(HashMap::new()),
            max_size,
        }
    }

    /// 计算文本哈希
    fn hash_text(text: &str) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        text.hash(&mut hasher);
        hasher.finish()
    }

    /// 获取缓存的向量
    pub async fn get(&self, text: &str) -> Option<Vec<f32>> {
        let hash = Self::hash_text(text);
        let cache = self.cache.read().await;
        cache.get(&hash).cloned()
    }

    /// 存储向量到缓存
    pub async fn set(&self, text: &str, vector: Vec<f32>) {
        let hash = Self::hash_text(text);
        let mut cache = self.cache.write().await;

        // 如果缓存满了，清除一半（简单 LRU 策略）
        if cache.len() >= self.max_size {
            let keys_to_remove: Vec<u64> = cache.keys().take(self.max_size / 2).cloned().collect();
            for key in keys_to_remove {
                cache.remove(&key);
            }
        }

        cache.insert(hash, vector);
    }

    /// 清空缓存
    pub async fn clear(&self) {
        let mut cache = self.cache.write().await;
        cache.clear();
    }

    /// 获取缓存大小
    pub async fn len(&self) -> usize {
        let cache = self.cache.read().await;
        cache.len()
    }

    /// 检查缓存是否为空
    pub async fn is_empty(&self) -> bool {
        let cache = self.cache.read().await;
        cache.is_empty()
    }
}

/// 全局向量缓存
static VECTOR_CACHE: once_cell::sync::Lazy<Arc<VectorCache>> =
    once_cell::sync::Lazy::new(|| Arc::new(VectorCache::new(10000)));

/// 获取全局向量缓存
pub fn get_vector_cache() -> Arc<VectorCache> {
    VECTOR_CACHE.clone()
}

/// 向量化评分权重配置
#[derive(Debug, Clone)]
pub struct VectorScoringWeights {
    /// 向量相似度权重
    pub vector_similarity: f64,
    /// BM25 标题权重
    pub bm25_title: f64,
    /// BM25 内容权重
    pub bm25_content: f64,
    /// 引擎权威度权重
    pub engine_authority: f64,
    /// 位置权重
    pub position: f64,
}

impl Default for VectorScoringWeights {
    fn default() -> Self {
        Self {
            vector_similarity: 0.40, // 向量相似度最重要
            bm25_title: 0.25,        // 标题 BM25
            bm25_content: 0.20,      // 内容 BM25
            engine_authority: 0.10,  // 引擎权威度
            position: 0.05,          // 原始位置
        }
    }
}

/// Pro 模式权重（更依赖向量）
impl VectorScoringWeights {
    pub fn pro_mode() -> Self {
        Self {
            vector_similarity: 0.55, // Pro 模式更依赖向量
            bm25_title: 0.15,
            bm25_content: 0.15,
            engine_authority: 0.10,
            position: 0.05,
        }
    }
}

/// 向量化相关性评分器
#[allow(dead_code)]
pub struct VectorScorer {
    /// 评分权重
    weights: VectorScoringWeights,
    /// 并发控制信号量
    semaphore: Arc<Semaphore>,
    /// 向量缓存
    cache: Arc<VectorCache>,
}

impl VectorScorer {
    /// 创建新的向量评分器
    pub fn new(weights: VectorScoringWeights, max_concurrency: usize) -> Self {
        Self {
            weights,
            semaphore: Arc::new(Semaphore::new(max_concurrency)),
            cache: get_vector_cache(),
        }
    }

    /// 从系统控制器获取配置创建
    pub fn from_system_controller() -> Self {
        let controller = get_global_system_controller();
        let config = controller.config();

        // 根据系统负载动态调整并发（使用默认并发数的一半）
        let max_concurrency = std::cmp::max(2, config.adjustment_interval_ms as usize / 500);

        Self::new(VectorScoringWeights::default(), max_concurrency)
    }

    /// 获取或计算文本向量
    #[cfg(feature = "python")]
    async fn get_or_compute_vector(&self, text: &str) -> Option<Vec<f32>> {
        // 先检查缓存
        if let Some(vector) = self.cache.get(text).await {
            return Some(vector);
        }

        // 获取并发许可
        let _permit = self.semaphore.acquire().await.ok()?;

        // 计算向量
        match embed_text(text) {
            Ok(vector) => {
                // 存入缓存
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

    /// 计算向量相似度（使用 SIMD 加速）
    pub fn compute_similarity(vec1: &[f32], vec2: &[f32]) -> f32 {
        if vec1.len() != vec2.len() || vec1.is_empty() {
            return 0.0;
        }
        simd_cosine_similarity(vec1, vec2)
    }

    /// 批量计算相似度
    pub fn batch_similarity(query_vec: &[f32], item_vecs: &[Vec<f32>]) -> Vec<f32> {
        item_vecs
            .iter()
            .map(|v| Self::compute_similarity(query_vec, v))
            .collect()
    }

    /// 计算综合评分
    pub async fn score_item(
        &self,
        item: &SearchResultItem,
        query: &SearchQuery,
        query_vector: Option<&Vec<f32>>,
        engine_name: &str,
        position: usize,
    ) -> f64 {
        let mut score = 0.0;

        // 1. 向量相似度（如果有查询向量）
        if let Some(q_vec) = query_vector {
            // 获取标题向量
            if let Some(title_vec) = self.get_or_compute_vector(&item.title).await {
                let similarity = Self::compute_similarity(q_vec, &title_vec);
                score += self.weights.vector_similarity * similarity as f64;
            }
        }

        // 2. BM25 评分（复用现有逻辑）
        let title_score = super::scoring::exact_match_bonus_optimized(&item.title, &query.query);
        let content_score =
            super::scoring::exact_match_bonus_optimized(&item.content, &query.query);
        score += self.weights.bm25_title * title_score;
        score += self.weights.bm25_content * content_score;

        // 3. 引擎权威度
        let authority = super::scoring::get_engine_authority(engine_name);
        score += self.weights.engine_authority * authority;

        // 4. 位置评分
        let pos_score = super::scoring::position_score(position);
        score += self.weights.position * pos_score;

        score.clamp(0.0, 1.0)
    }

    /// 批量评分结果
    pub async fn score_results(
        &self,
        items: &mut [SearchResultItem],
        query: &SearchQuery,
        engine_name: &str,
    ) {
        // 首先获取查询向量
        let query_vector = self.get_or_compute_vector(&query.query).await;

        // 并发计算每个结果的评分
        for (position, item) in items.iter_mut().enumerate() {
            item.score = self
                .score_item(item, query, query_vector.as_ref(), engine_name, position)
                .await;
        }

        // 按分数排序
        items.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
    }
}

/// 检查向量化评分是否可用
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
    fn test_batch_similarity() {
        let query = vec![1.0, 2.0, 3.0, 4.0];
        let items = vec![vec![1.0, 2.0, 3.0, 4.0], vec![4.0, 3.0, 2.0, 1.0]];
        let similarities = VectorScorer::batch_similarity(&query, &items);
        assert_eq!(similarities.len(), 2);
        assert!((similarities[0] - 1.0).abs() < 1e-6);
    }
}
