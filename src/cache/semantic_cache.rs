// Copyright 2025 nostalgiatan
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! 语义缓存
//!
//! 基于向量相似度的智能缓存系统

use crate::cache::manager::{CacheManager, CacheError};
use crate::cache::semantic::{SimpleVectorizer, QueryVector};
use crate::derive::types::{SearchQuery, SearchResult, SearchResultItem};
use std::sync::Arc;
use std::time::Duration;
use std::collections::HashSet;
use serde::{Deserialize, Serialize};

type Result<T> = std::result::Result<T, CacheError>;

/// 语义缓存键前缀
const SEMANTIC_CACHE_PREFIX: &str = "semantic:";
const QUERY_VECTOR_PREFIX: &str = "qvec:";

/// 语义缓存配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticCacheConfig {
    /// 相似度阈值（0.0-1.0）
    pub similarity_threshold: f64,
    /// 最大结果数
    pub max_results_per_query: usize,
    /// 是否启用去重
    pub enable_deduplication: bool,
}

impl Default for SemanticCacheConfig {
    fn default() -> Self {
        Self {
            similarity_threshold: 0.7, // 70%相似度
            max_results_per_query: 50,
            enable_deduplication: true,
        }
    }
}

/// 缓存的查询结果条目
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CachedQueryResult {
    /// 查询文本
    query: String,
    /// 引擎名称
    engine: String,
    /// 搜索结果
    result: SearchResult,
    /// 时间戳
    timestamp: u64,
}

/// 语义缓存
pub struct SemanticCache {
    /// 缓存管理器
    manager: Arc<CacheManager>,
    /// 向量化器
    vectorizer: SimpleVectorizer,
    /// 配置
    config: SemanticCacheConfig,
}

impl SemanticCache {
    /// 创建语义缓存
    pub fn new(manager: Arc<CacheManager>, config: SemanticCacheConfig) -> Self {
        Self {
            manager,
            vectorizer: SimpleVectorizer::new(),
            config,
        }
    }

    /// 生成查询向量键
    fn generate_vector_key(&self, query_hash: &str) -> String {
        format!("{}{}", QUERY_VECTOR_PREFIX, query_hash)
    }

    /// 生成缓存键
    fn generate_cache_key(&self, query_hash: &str, engine: &str) -> String {
        format!("{}{}:{}", SEMANTIC_CACHE_PREFIX, query_hash, engine)
    }

    /// 计算查询hash
    fn hash_query(&self, query: &str) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        query.to_lowercase().hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }

    /// 存储查询向量
    fn store_query_vector(&self, query: &str, vector: &[f64]) -> Result<String> {
        let query_hash = self.hash_query(query);
        let key = self.generate_vector_key(&query_hash);
        
        let qvec = QueryVector::new(query.to_string(), vector.to_vec());
        let data = bincode::serde::encode_to_vec(&qvec, bincode::config::standard())
            .map_err(|e| CacheError::SerializationError(format!("Failed to serialize query vector: {}", e)))?;
        
        self.manager.set(key, data, None)?;
        Ok(query_hash)
    }

    /// 获取所有查询向量
    fn get_all_query_vectors(&self) -> Result<Vec<(String, QueryVector)>> {
        // Note: This is a simplified implementation
        // In production, you'd want to maintain an index
        Ok(vec![])
    }

    /// 查找相似查询
    pub fn find_similar_queries(&self, query: &str) -> Result<Vec<(String, f64)>> {
        let query_vector = self.vectorizer.vectorize(query);
        let mut similar_queries = Vec::new();

        // 获取所有已缓存的查询向量
        let cached_vectors = self.get_all_query_vectors()?;

        for (hash, qvec) in cached_vectors {
            let similarity = self.vectorizer.cosine_similarity(&query_vector, &qvec.vector);
            if similarity >= self.config.similarity_threshold {
                similar_queries.push((hash, similarity));
            }
        }

        // 按相似度排序
        similar_queries.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        Ok(similar_queries)
    }

    /// 获取缓存结果（支持语义搜索）
    pub fn get(&self, query: &SearchQuery, engine: &str) -> Result<Option<Vec<SearchResultItem>>> {
        let query_text = &query.query;
        
        // 1. 首先尝试精确匹配
        let query_hash = self.hash_query(query_text);
        let exact_key = self.generate_cache_key(&query_hash, engine);
        
        if let Some(data) = self.manager.get(&exact_key)? {
            let cached: CachedQueryResult = bincode::serde::decode_from_slice(&data, bincode::config::standard())
                .map(|(res, _)| res)
                .map_err(|e| CacheError::SerializationError(format!("Failed to deserialize: {}", e)))?;
            return Ok(Some(cached.result.items));
        }

        // 2. 查找语义相似的查询
        let similar = self.find_similar_queries(query_text)?;
        if similar.is_empty() {
            return Ok(None);
        }

        // 3. 收集所有相似查询的结果
        let mut all_items = Vec::new();
        let mut seen_urls: HashSet<String> = HashSet::new();

        for (sim_hash, _similarity) in similar.iter().take(5) {
            let sim_key = self.generate_cache_key(sim_hash, engine);
            if let Some(data) = self.manager.get(&sim_key)? {
                let (cached, _): (CachedQueryResult, _) = bincode::serde::decode_from_slice(&data, bincode::config::standard())
                    .map_err(|e| CacheError::SerializationError(format!("Failed to decode: {}", e)))?;
                
                for item in cached.result.items {
                    // 去重
                    if self.config.enable_deduplication {
                        if !seen_urls.contains(&item.url) {
                            seen_urls.insert(item.url.clone());
                            all_items.push(item);
                        }
                    } else {
                        all_items.push(item);
                    }

                    if all_items.len() >= self.config.max_results_per_query {
                        break;
                    }
                }
            }
            if all_items.len() >= self.config.max_results_per_query {
                break;
            }
        }

        if all_items.is_empty() {
            Ok(None)
        } else {
            Ok(Some(all_items))
        }
    }

    /// 查询统一缓存（同时查询搜索缓存和RSS缓存）
    ///
    /// 这个方法会同时从搜索结果缓存和RSS缓存中查询相关内容，
    /// 基于语义相似度匹配，然后合并去重返回
    pub fn query_combined(&self, query: &SearchQuery, engine: &str) -> Result<Option<Vec<SearchResultItem>>> {
        let mut all_items = Vec::new();
        let mut seen_urls: HashSet<String> = HashSet::new();

        // 1. 从搜索缓存获取结果
        if let Some(search_items) = self.get(query, engine)? {
            for item in search_items {
                if !seen_urls.contains(&item.url.to_lowercase()) {
                    seen_urls.insert(item.url.to_lowercase());
                    all_items.push(item);
                }
            }
        }

        // 2. 从RSS缓存获取相关结果
        if let Some(rss_items) = self.get_rss_items(query)? {
            for item in rss_items {
                // 跨源去重
                if !seen_urls.contains(&item.url.to_lowercase()) {
                    seen_urls.insert(item.url.to_lowercase());
                    all_items.push(item);

                    if all_items.len() >= self.config.max_results_per_query {
                        break;
                    }
                }
            }
        }

        if all_items.is_empty() {
            Ok(None)
        } else {
            Ok(Some(all_items))
        }
    }

    /// 从RSS缓存中获取与查询相关的项目
    fn get_rss_items(&self, _query: &SearchQuery) -> Result<Option<Vec<SearchResultItem>>> {
        // 获取所有RSS feeds并进行语义匹配
        // Note: 这里简化实现，实际生产环境应该维护一个RSS feed索引
        let matched_items = Vec::new();

        // 简化：这里我们可以添加一个方法来列出所有RSS feeds
        // 然后对每个feed的items进行关键词匹配
        // 由于当前RssCache没有list_all方法，我们先返回空
        // 在生产环境中，应该实现一个RSS索引系统

        Ok(if matched_items.is_empty() {
            None
        } else {
            Some(matched_items)
        })
    }

    /// 存储搜索结果
    pub fn set(
        &self,
        query: &SearchQuery,
        engine: &str,
        result: &SearchResult,
        ttl: Option<Duration>,
    ) -> Result<()> {
        let query_text = &query.query;
        
        // 1. 向量化查询并存储
        let query_vector = self.vectorizer.vectorize(query_text);
        let query_hash = self.store_query_vector(query_text, &query_vector)?;

        // 2. 存储搜索结果
        let cache_key = self.generate_cache_key(&query_hash, engine);
        
        use std::time::{SystemTime, UNIX_EPOCH};
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let cached = CachedQueryResult {
            query: query_text.clone(),
            engine: engine.to_string(),
            result: result.clone(),
            timestamp,
        };

        let data = bincode::serde::encode_to_vec(&cached, bincode::config::standard())
            .map_err(|e| CacheError::SerializationError(format!("Failed to serialize result: {}", e)))?;

        self.manager.set(cache_key, data, ttl)?;
        Ok(())
    }

    /// 清除特定查询的缓存
    pub fn clear_query(&self, query: &str, engine: &str) -> Result<()> {
        let query_hash = self.hash_query(query);
        let cache_key = self.generate_cache_key(&query_hash, engine);
        self.manager.delete(&cache_key)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cache::types::CacheImplConfig;

    #[test]
    fn test_semantic_cache_creation() {
        let config = CacheImplConfig::default();
        let manager = CacheManager::instance(config).unwrap();
        let semantic_config = SemanticCacheConfig::default();
        let _cache = SemanticCache::new(manager, semantic_config);
    }

    #[test]
    fn test_hash_query() {
        use std::time::{SystemTime, UNIX_EPOCH};
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        let mut config = CacheImplConfig::default();
        config.db_path = format!("./data/test_semantic_{}.db", timestamp);
        let manager = CacheManager::instance(config).unwrap();
        let semantic_config = SemanticCacheConfig::default();
        let cache = SemanticCache::new(manager, semantic_config);

        let hash1 = cache.hash_query("test query");
        let hash2 = cache.hash_query("test query");
        let hash3 = cache.hash_query("different query");

        assert_eq!(hash1, hash2);
        assert_ne!(hash1, hash3);
    }
}
