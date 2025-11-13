//! 搜索结果缓存
//!
//! 提供搜索结果的专门缓存功能

use crate::cache::manager::{CacheManager, Result};
use crate::derive::types::{SearchQuery, SearchResult};
use std::sync::Arc;
use std::time::Duration;

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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cache::types::{CacheImplConfig, CacheMode};
    use crate::derive::types::{EngineType, TimeRange};
    use crate::config::common::SafeSearchLevel;
    use std::collections::HashMap;

    fn temp_result_cache() -> ResultCache {
        let temp_dir = std::env::temp_dir();
        let db_path = temp_dir.join(format!("test_result_cache_{}", std::process::id()));
        
        let config = CacheImplConfig {
            db_path: db_path.to_string_lossy().to_string(),
            default_ttl_secs: 10,
            max_size_bytes: 1024 * 1024,
            enabled: true,
            compression: false,
            mode: CacheMode::HighThroughput,
        };

        let manager = Arc::new(CacheManager::new(config).expect("创建缓存管理器失败"));
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
    fn test_result_cache_set_and_get() {
        let cache = temp_result_cache();
        let query = sample_query();
        let result = sample_result();
        let engine_name = "TestEngine";

        // 缓存结果
        cache.set(&query, engine_name, &result, None).expect("缓存搜索结果失败");

        // 获取缓存
        let cached = cache.get(&query, engine_name).expect("获取缓存失败");
        assert!(cached.is_some());
        
        let cached_result = cached.unwrap();
        assert_eq!(cached_result.engine_name, result.engine_name);
        assert_eq!(cached_result.items.len(), result.items.len());
    }

    #[test]
    fn test_result_cache_miss() {
        let cache = temp_result_cache();
        let query = sample_query();
        let engine_name = "TestEngine";

        // 获取不存在的缓存
        let cached = cache.get(&query, engine_name).expect("获取缓存失败");
        assert!(cached.is_none());
    }

    #[test]
    fn test_result_cache_delete() {
        let cache = temp_result_cache();
        let query = sample_query();
        let result = sample_result();
        let engine_name = "TestEngine";

        // 缓存结果
        cache.set(&query, engine_name, &result, None).expect("缓存搜索结果失败");
        assert!(cache.get(&query, engine_name).expect("获取缓存失败").is_some());

        // 删除缓存
        let deleted = cache.delete(&query, engine_name).expect("删除缓存失败");
        assert!(deleted);

        // 验证已删除
        assert!(cache.get(&query, engine_name).expect("获取缓存失败").is_none());
    }

    #[test]
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
    fn test_result_cache_expiration() {
        let cache = temp_result_cache();
        let query = sample_query();
        let result = sample_result();
        let engine_name = "TestEngine";

        // 设置1秒过期
        cache.set(&query, engine_name, &result, Some(Duration::from_secs(1)))
            .expect("缓存搜索结果失败");

        // 立即获取应该存在
        assert!(cache.get(&query, engine_name).expect("获取缓存失败").is_some());

        // 等待过期
        std::thread::sleep(Duration::from_millis(1100));

        // 获取应该返回 None
        assert!(cache.get(&query, engine_name).expect("获取缓存失败").is_none());
    }
}
