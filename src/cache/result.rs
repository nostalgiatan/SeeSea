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

//! 搜索结果缓存
//!
//! 提供搜索结果的专门缓存功能

use crate::cache::manager::{CacheManager, CacheError};
use crate::derive::types::{SearchQuery, SearchResult};
use std::sync::Arc;
use std::time::Duration;

type Result<T> = std::result::Result<T, CacheError>;

/// 搜索结果缓存键前缀
const RESULT_KEY_PREFIX: &str = "result:";

/// 搜索结果缓存
///
/// 封装 CacheManager，提供搜索结果专用的缓存接口
pub struct ResultCache {
    manager: Arc<CacheManager>,
}

impl ResultCache {
    /// 创建搜索结果缓存实例
    ///
    /// # 参数
    ///
    /// * `manager` - 缓存管理器（Arc包装）
    pub fn new(manager: Arc<CacheManager>) -> Self {
        Self { manager }
    }

    /// 生成搜索结果缓存键
    ///
    /// # 参数
    ///
    /// * `query` - 搜索查询
    /// * `engine_name` - 引擎名称
    ///
    /// # 返回值
    ///
    /// 返回唯一的缓存键字符串
    pub fn generate_key(query: &SearchQuery, engine_name: &str) -> String {
        // 使用 hash 生成唯一键，避免键过长
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        query.query.hash(&mut hasher);
        query.page.hash(&mut hasher);
        query.page_size.hash(&mut hasher);
        query.language.hash(&mut hasher);
        query.region.hash(&mut hasher);
        engine_name.hash(&mut hasher);

        format!("{}{:x}", RESULT_KEY_PREFIX, hasher.finish())
    }

    /// 获取缓存的搜索结果
    ///
    /// # 参数
    ///
    /// * `query` - 搜索查询
    /// * `engine_name` - 引擎名称
    ///
    /// # 返回值
    ///
    /// 返回缓存的搜索结果，如果不存在或已过期则返回 None
    pub fn get(&self, query: &SearchQuery, engine_name: &str) -> Result<Option<SearchResult>> {
        let key = Self::generate_key(query, engine_name);
        
        match self.manager.get(&key)? {
            Some(data) => {
                // 反序列化搜索结果
                let result: SearchResult = bincode::serde::decode_from_slice(&data, bincode::config::standard())
                    .map(|(res, _)| res)
                    .map_err(|e| {
                        CacheError::SerializationError(format!("反序列化搜索结果失败: {}", e))
                    })?;
                Ok(Some(result))
            }
            None => Ok(None),
        }
    }

    /// 检查缓存是否过期
    ///
    /// # 参数
    ///
    /// * `query` - 搜索查询
    /// * `engine_name` - 引擎名称
    /// * `timeline` - 时间线阈值（秒）
    ///
    /// # 返回值
    ///
    /// 如果缓存存在但已超过时间线，返回 Some(true)
    /// 如果缓存存在且未过期，返回 Some(false)
    /// 如果缓存不存在，返回 None
    pub fn is_stale(&self, query: &SearchQuery, engine_name: &str, timeline: u64) -> Result<Option<bool>> {
        let key = Self::generate_key(query, engine_name);
        
        // 获取缓存元数据
        if let Some(metadata) = self.manager.get_metadata(&key)? {
            use std::time::{SystemTime, UNIX_EPOCH};
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            
            let age = now.saturating_sub(metadata.created_at);
            Ok(Some(age >= timeline))
        } else {
            Ok(None)
        }
    }

    /// 缓存搜索结果
    ///
    /// # 参数
    ///
    /// * `query` - 搜索查询
    /// * `engine_name` - 引擎名称
    /// * `result` - 搜索结果
    /// * `ttl` - 生存时间，None 表示使用默认值
    pub fn set(
        &self,
        query: &SearchQuery,
        engine_name: &str,
        result: &SearchResult,
        ttl: Option<Duration>,
    ) -> Result<()> {
        let key = Self::generate_key(query, engine_name);
        
        // 序列化搜索结果
        let data = bincode::serde::encode_to_vec(result, bincode::config::standard()).map_err(|e| {
            CacheError::SerializationError(format!("序列化搜索结果失败: {}", e))
        })?;

        self.manager.set(key, data, ttl)
    }

    /// 删除缓存的搜索结果
    ///
    /// # 参数
    ///
    /// * `query` - 搜索查询
    /// * `engine_name` - 引擎名称
    pub fn delete(&self, query: &SearchQuery, engine_name: &str) -> Result<bool> {
        let key = Self::generate_key(query, engine_name);
        self.manager.delete(&key)
    }

    /// 清空所有搜索结果缓存
    pub fn clear_all(&self) -> Result<()> {
        self.manager.clear()
    }

    /// 获取底层缓存管理器引用
    pub fn manager(&self) -> &CacheManager {
        &self.manager
    }

    /// 全文搜索 - 在所有缓存的搜索结果中查找包含关键词的项目
    ///
    /// # 参数
    ///
    /// * `keywords` - 搜索关键词列表
    /// * `include_stale` - 是否包含过期的缓存结果
    /// * `max_results` - 最大返回结果数（可选）
    ///
    /// # 返回值
    ///
    /// 返回匹配的搜索结果项列表
    ///
    /// # 性能说明
    ///
    /// 此方法遍历所有缓存条目进行全文搜索。对于大型缓存，建议：
    /// - 使用 max_results 参数限制返回结果数量
    /// - 定期清理过期缓存以减少数据库大小
    /// - 考虑在应用层实现结果缓存
    pub fn search_fulltext(
        &self,
        keywords: &[String],
        include_stale: bool,
        max_results: Option<usize>,
    ) -> Result<Vec<crate::derive::types::SearchResultItem>> {
        let mut matched_items = Vec::new();
        let max = max_results.unwrap_or(usize::MAX);

        // 遍历所有以 result: 开头的缓存键
        for item in self.manager.iter() {
            if matched_items.len() >= max {
                break;
            }

            let (key, value) = item.map_err(|e| {
                CacheError::DatabaseError(format!("遍历缓存失败: {}", e))
            })?;

            let key_str = String::from_utf8_lossy(&key);
            
            // 只处理搜索结果缓存
            if !key_str.starts_with(RESULT_KEY_PREFIX) {
                continue;
            }

            // 检查是否过期（如果不包含过期结果）
            if !include_stale {
                if let Some(metadata) = self.manager.get_metadata(&key_str)? {
                    if metadata.is_expired() {
                        continue;
                    }
                }
            }

            // 反序列化搜索结果
            let result: SearchResult = match bincode::serde::decode_from_slice(&value, bincode::config::standard()) {
                Ok((res, _)) => res,
                Err(_) => continue, // 跳过损坏的数据
            };

            // 在结果项中搜索关键词
            for item in result.items {
                if matched_items.len() >= max {
                    break;
                }

                // 检查标题、内容和 URL 是否包含任何关键词
                let matches = keywords.iter().any(|keyword| {
                    let keyword_lower = keyword.to_lowercase();
                    item.title.to_lowercase().contains(&keyword_lower)
                        || item.content.to_lowercase().contains(&keyword_lower)
                        || item.url.to_lowercase().contains(&keyword_lower)
                });

                if matches {
                    matched_items.push(item);
                }
            }
        }

        Ok(matched_items)
    }

    /// 按查询字符串搜索缓存的结果
    ///
    /// # 参数
    ///
    /// * `query_pattern` - 查询字符串模式（部分匹配）
    /// * `include_stale` - 是否包含过期的缓存结果
    /// * `max_results` - 最大返回结果数（可选）
    ///
    /// # 返回值
    ///
    /// 返回匹配的搜索结果列表
    pub fn search_by_query(
        &self,
        query_pattern: &str,
        include_stale: bool,
        max_results: Option<usize>,
    ) -> Result<Vec<SearchResult>> {
        let mut matched_results = Vec::new();
        let max = max_results.unwrap_or(usize::MAX);
        let pattern_lower = query_pattern.to_lowercase();

        // 遍历所有以 result: 开头的缓存键
        for item in self.manager.iter() {
            if matched_results.len() >= max {
                break;
            }

            let (key, value) = item.map_err(|e| {
                CacheError::DatabaseError(format!("遍历缓存失败: {}", e))
            })?;

            let key_str = String::from_utf8_lossy(&key);
            
            // 只处理搜索结果缓存
            if !key_str.starts_with(RESULT_KEY_PREFIX) {
                continue;
            }

            // 检查是否过期（如果不包含过期结果）
            if !include_stale {
                if let Some(metadata) = self.manager.get_metadata(&key_str)? {
                    if metadata.is_expired() {
                        continue;
                    }
                }
            }

            // 反序列化搜索结果
            let result: SearchResult = match bincode::serde::decode_from_slice(&value, bincode::config::standard()) {
                Ok((res, _)) => res,
                Err(_) => continue, // 跳过损坏的数据
            };

            // 注意：由于使用哈希键，无法直接匹配原始查询
            // 我们需要在结果项中查找模式
            let has_match = result.items.iter().any(|item| {
                item.title.to_lowercase().contains(&pattern_lower)
                    || item.content.to_lowercase().contains(&pattern_lower)
            });

            if has_match {
                matched_results.push(result);
            }
        }

        Ok(matched_results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cache::types::{CacheImplConfig, CacheMode};
    use crate::derive::types::EngineType;
    use crate::config::common::SafeSearchLevel;
    use std::collections::HashMap;
    use serial_test::serial;

    fn temp_result_cache() -> ResultCache {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        
        let temp_dir = std::env::temp_dir();
        let unique_id = COUNTER.fetch_add(1, Ordering::SeqCst);
        let db_path = temp_dir.join(format!(
            "test_result_cache_{}_{}",
            std::process::id(),
            unique_id
        ));
        
        let config = CacheImplConfig {
            db_path: db_path.to_string_lossy().to_string(),
            default_ttl_secs: 10,
            max_size_bytes: 1024 * 1024,
            enabled: true,
            compression: false,
            mode: CacheMode::HighThroughput,
        };

        let manager = CacheManager::instance(config).expect("Failed to create cache manager");
        ResultCache::new(manager)
    }

    fn sample_query() -> SearchQuery {
        SearchQuery {
            query: "rust programming".to_string(),
            engine_type: EngineType::General,
            language: Some("en".to_string()),
            region: None,
            page_size: 10,
            page: 1,
            safe_search: SafeSearchLevel::Moderate,
            time_range: None,
            params: HashMap::new(),
        }
    }

    fn sample_result() -> SearchResult {
        use crate::derive::types::{SearchResultItem, ResultType};
        
        SearchResult {
            engine_name: "TestEngine".to_string(),
            total_results: Some(1000),
            elapsed_ms: 150,
            items: vec![
                SearchResultItem {
                    title: "Test Result".to_string(),
                    url: "https://example.com".to_string(),
                    content: "Test content".to_string(),
                    display_url: Some("example.com".to_string()),
                    site_name: None,
                    score: 0.95,
                    result_type: ResultType::Web,
                    thumbnail: None,
                    published_date: None,
                    template: None,
                    metadata: HashMap::new(),
                },
            ],
            pagination: None,
            suggestions: Vec::new(),
            metadata: HashMap::new(),
        }
    }

    #[test]
    #[serial]
    fn test_result_cache_set_and_get() {
        let cache = temp_result_cache();
        let query = sample_query();
        let result = sample_result();
        let engine_name = "TestEngine";

        // 缓存结果
        cache.set(&query, engine_name, &result, None).expect("缓存搜索结果失败");

        // 获取缓存
        let cached = cache.get(&query, engine_name).unwrap_or(None);
        assert!(cached.is_some());
        
        let cached_result = cached.unwrap();
        assert_eq!(cached_result.engine_name, result.engine_name);
        assert_eq!(cached_result.items.len(), result.items.len());
    }

    #[test]
    #[serial]
    fn test_result_cache_miss() {
        let cache = temp_result_cache();
        let query = sample_query();
        let engine_name = "TestEngine";

        // 获取不存在的缓存
        let cached = cache.get(&query, engine_name).unwrap_or(None);
        assert!(cached.is_none());
    }

    #[test]
    #[serial]
    fn test_result_cache_delete() {
        let cache = temp_result_cache();
        let query = sample_query();
        let result = sample_result();
        let engine_name = "TestEngine";

        // 缓存结果
        cache.set(&query, engine_name, &result, None).expect("缓存搜索结果失败");
        assert!(cache.get(&query, engine_name).unwrap_or(None).is_some());

        // 删除缓存
        let deleted = cache.delete(&query, engine_name).unwrap_or(false);
        assert!(deleted);

        // 验证已删除
        assert!(cache.get(&query, engine_name).unwrap_or(None).is_none());
    }

    #[test]
    #[serial]
    fn test_result_cache_key_generation() {
        let query1 = sample_query();
        let query2 = sample_query();
        let engine_name = "TestEngine";

        // 相同查询应该生成相同的键
        let key1 = ResultCache::generate_key(&query1, engine_name);
        let key2 = ResultCache::generate_key(&query2, engine_name);
        assert_eq!(key1, key2);

        // 不同查询应该生成不同的键
        let mut query3 = sample_query();
        query3.page = 2;
        let key3 = ResultCache::generate_key(&query3, engine_name);
        assert_ne!(key1, key3);
    }

    #[test]
    #[serial]
    fn test_result_cache_expiration() {
        let cache = temp_result_cache();
        let query = sample_query();
        let result = sample_result();
        let engine_name = "TestEngine";

        // 设置1秒过期
        cache.set(&query, engine_name, &result, Some(Duration::from_secs(1)))
            .expect("缓存搜索结果失败");

        // 立即获取应该存在
        assert!(cache.get(&query, engine_name).unwrap_or(None).is_some());

        // 等待过期
        std::thread::sleep(Duration::from_millis(1100));

        // 获取应该返回 None
        assert!(cache.get(&query, engine_name).unwrap_or(None).is_none());
    }
}
