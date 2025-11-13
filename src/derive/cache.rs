//! 缓存模块
//!
//! 本模块提供了搜索结果缓存功能，用于提高性能和减少对搜索引擎的请求。
//! 支持内存缓存和基于时间的过期策略。

use crate::derive::error::{CacheOperation, DeriveError, Result};
use crate::derive::types::SearchResult;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

/// 缓存项
#[derive(Debug, Clone)]
struct CacheEntry {
    /// 缓存的搜索结果
    result: SearchResult,
    /// 创建时间
    created_at: Instant,
    /// 过期时间（从创建时开始计算）
    ttl: Duration,
}

impl CacheEntry {
    /// 检查缓存项是否过期
    fn is_expired(&self) -> bool {
        Instant::now().duration_since(self.created_at) > self.ttl
    }
}

/// 缓存配置
#[derive(Debug, Clone)]
pub struct CacheConfig {
    /// 默认过期时间（秒）
    pub default_ttl_secs: u64,
    /// 最大缓存条目数
    pub max_entries: usize,
    /// 是否启用缓存
    pub enabled: bool,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            default_ttl_secs: 300, // 5分钟
            max_entries: 1000,
            enabled: true,
        }
    }
}

/// 内存缓存实现
///
/// 使用 HashMap 和 RwLock 实现的线程安全内存缓存
pub struct MemoryCache {
    /// 缓存存储
    storage: Arc<RwLock<HashMap<String, CacheEntry>>>,
    /// 缓存配置
    config: CacheConfig,
}

impl MemoryCache {
    /// 创建新的内存缓存
    ///
    /// # 参数
    ///
    /// * `config` - 缓存配置
    pub fn new(config: CacheConfig) -> Self {
        Self {
            storage: Arc::new(RwLock::new(HashMap::new())),
            config,
        }
    }

    /// 获取缓存项
    ///
    /// # 参数
    ///
    /// * `key` - 缓存键
    ///
    /// # 返回值
    ///
    /// 如果存在且未过期，返回 Some(SearchResult)，否则返回 None
    pub fn get(&self, key: &str) -> Result<Option<SearchResult>> {
        if !self.config.enabled {
            return Ok(None);
        }

        let storage = self.storage.read().map_err(|e| DeriveError::Cache {
            message: format!("获取缓存读锁失败: {}", e),
            operation: CacheOperation::Read,
        })?;

        match storage.get(key) {
            Some(entry) if !entry.is_expired() => {
                tracing::debug!("缓存命中: {}", key);
                Ok(Some(entry.result.clone()))
            }
            Some(_) => {
                tracing::debug!("缓存已过期: {}", key);
                Ok(None)
            }
            None => {
                tracing::debug!("缓存未命中: {}", key);
                Ok(None)
            }
        }
    }

    /// 设置缓存项
    ///
    /// # 参数
    ///
    /// * `key` - 缓存键
    /// * `result` - 搜索结果
    /// * `ttl` - 过期时间（可选，使用默认值如果不提供）
    pub fn set(&self, key: String, result: SearchResult, ttl: Option<Duration>) -> Result<()> {
        if !self.config.enabled {
            return Ok(());
        }

        let mut storage = self.storage.write().map_err(|e| DeriveError::Cache {
            message: format!("获取缓存写锁失败: {}", e),
            operation: CacheOperation::Write,
        })?;

        // 如果达到最大条目数，清理过期条目
        if storage.len() >= self.config.max_entries {
            self.evict_expired(&mut storage);
            
            // 如果清理后仍然满，删除最旧的条目
            if storage.len() >= self.config.max_entries {
                if let Some(oldest_key) = storage.keys().next().cloned() {
                    storage.remove(&oldest_key);
                }
            }
        }

        let entry = CacheEntry {
            result,
            created_at: Instant::now(),
            ttl: ttl.unwrap_or_else(|| Duration::from_secs(self.config.default_ttl_secs)),
        };

        storage.insert(key.clone(), entry);
        tracing::debug!("缓存已设置: {}", key);
        
        Ok(())
    }

    /// 删除缓存项
    ///
    /// # 参数
    ///
    /// * `key` - 缓存键
    pub fn remove(&self, key: &str) -> Result<()> {
        let mut storage = self.storage.write().map_err(|e| DeriveError::Cache {
            message: format!("获取缓存写锁失败: {}", e),
            operation: CacheOperation::Delete,
        })?;

        storage.remove(key);
        tracing::debug!("缓存已删除: {}", key);
        
        Ok(())
    }

    /// 清空所有缓存
    pub fn clear(&self) -> Result<()> {
        let mut storage = self.storage.write().map_err(|e| DeriveError::Cache {
            message: format!("获取缓存写锁失败: {}", e),
            operation: CacheOperation::Delete,
        })?;

        storage.clear();
        tracing::debug!("缓存已清空");
        
        Ok(())
    }

    /// 获取缓存大小
    pub fn len(&self) -> Result<usize> {
        let storage = self.storage.read().map_err(|e| DeriveError::Cache {
            message: format!("获取缓存读锁失败: {}", e),
            operation: CacheOperation::Read,
        })?;

        Ok(storage.len())
    }

    /// 检查缓存是否为空
    pub fn is_empty(&self) -> Result<bool> {
        Ok(self.len()? == 0)
    }

    /// 清理所有过期的缓存项
    pub fn cleanup(&self) -> Result<usize> {
        let mut storage = self.storage.write().map_err(|e| DeriveError::Cache {
            message: format!("获取缓存写锁失败: {}", e),
            operation: CacheOperation::Delete,
        })?;

        let count = self.evict_expired(&mut storage);
        tracing::debug!("清理了 {} 个过期缓存项", count);
        
        Ok(count)
    }

    /// 内部方法：清理过期条目
    ///
    /// # 返回值
    ///
    /// 返回清理的条目数
    fn evict_expired(&self, storage: &mut HashMap<String, CacheEntry>) -> usize {
        let initial_len = storage.len();
        storage.retain(|_, entry| !entry.is_expired());
        initial_len - storage.len()
    }

    /// 生成缓存键
    ///
    /// 根据查询参数生成唯一的缓存键
    ///
    /// # 参数
    ///
    /// * `engine_name` - 引擎名称
    /// * `query` - 查询字符串
    /// * `page` - 页码
    /// * `params` - 额外参数（可选）
    pub fn generate_key(
        engine_name: &str,
        query: &str,
        page: usize,
        params: Option<&HashMap<String, String>>,
    ) -> String {
        let mut key = format!("{}:{}:{}", engine_name, query, page);
        
        if let Some(params) = params {
            let mut sorted_params: Vec<_> = params.iter().collect();
            sorted_params.sort_by_key(|(k, _)| *k);
            
            for (k, v) in sorted_params {
                key.push(':');
                key.push_str(k);
                key.push('=');
                key.push_str(v);
            }
        }
        
        key
    }
}

impl Default for MemoryCache {
    fn default() -> Self {
        Self::new(CacheConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::derive::types::{EngineType, PaginationInfo, SearchQuery};
    use crate::config::common::SafeSearchLevel;
    use std::collections::HashMap;

    fn create_test_result() -> SearchResult {
        SearchResult {
            engine_name: "TestEngine".to_string(),
            total_results: Some(100),
            elapsed_ms: 50,
            items: vec![],
            pagination: None,
            suggestions: vec![],
            metadata: HashMap::new(),
        }
    }

    #[test]
    fn test_cache_config_default() {
        let config = CacheConfig::default();
        assert_eq!(config.default_ttl_secs, 300);
        assert_eq!(config.max_entries, 1000);
        assert!(config.enabled);
    }

    #[test]
    fn test_memory_cache_creation() {
        let cache = MemoryCache::new(CacheConfig::default());
        assert!(cache.is_empty().is_ok());
    }

    #[test]
    fn test_cache_set_and_get() {
        let cache = MemoryCache::new(CacheConfig::default());
        let result = create_test_result();
        
        assert!(cache.set("test_key".to_string(), result.clone(), None).is_ok());
        
        let cached = cache.get("test_key");
        assert!(cached.is_ok());
        assert!(cached.as_ref().ok().and_then(|o| o.as_ref()).is_some());
        
        let cached_result = cached.unwrap().unwrap();
        assert_eq!(cached_result.engine_name, "TestEngine");
    }

    #[test]
    fn test_cache_miss() {
        let cache = MemoryCache::new(CacheConfig::default());
        let result = cache.get("nonexistent");
        
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[test]
    fn test_cache_expiration() {
        let cache = MemoryCache::new(CacheConfig::default());
        let result = create_test_result();
        
        // 设置1秒过期
        let ttl = Duration::from_secs(0); // 立即过期
        assert!(cache.set("test_key".to_string(), result, Some(ttl)).is_ok());
        
        // 应该已经过期
        std::thread::sleep(Duration::from_millis(10));
        let cached = cache.get("test_key");
        assert!(cached.is_ok());
        assert!(cached.unwrap().is_none());
    }

    #[test]
    fn test_cache_remove() {
        let cache = MemoryCache::new(CacheConfig::default());
        let result = create_test_result();
        
        assert!(cache.set("test_key".to_string(), result, None).is_ok());
        assert!(cache.get("test_key").unwrap().is_some());
        
        assert!(cache.remove("test_key").is_ok());
        assert!(cache.get("test_key").unwrap().is_none());
    }

    #[test]
    fn test_cache_clear() {
        let cache = MemoryCache::new(CacheConfig::default());
        let result = create_test_result();
        
        assert!(cache.set("key1".to_string(), result.clone(), None).is_ok());
        assert!(cache.set("key2".to_string(), result, None).is_ok());
        
        assert_eq!(cache.len().unwrap(), 2);
        
        assert!(cache.clear().is_ok());
        assert_eq!(cache.len().unwrap(), 0);
        assert!(cache.is_empty().unwrap());
    }

    #[test]
    fn test_cache_max_entries() {
        let config = CacheConfig {
            default_ttl_secs: 300,
            max_entries: 2,
            enabled: true,
        };
        let cache = MemoryCache::new(config);
        let result = create_test_result();
        
        assert!(cache.set("key1".to_string(), result.clone(), None).is_ok());
        assert!(cache.set("key2".to_string(), result.clone(), None).is_ok());
        assert_eq!(cache.len().unwrap(), 2);
        
        // 添加第三个应该触发清理
        assert!(cache.set("key3".to_string(), result, None).is_ok());
        assert_eq!(cache.len().unwrap(), 2);
    }

    #[test]
    fn test_cache_disabled() {
        let config = CacheConfig {
            default_ttl_secs: 300,
            max_entries: 1000,
            enabled: false,
        };
        let cache = MemoryCache::new(config);
        let result = create_test_result();
        
        // 设置不应该存储任何东西
        assert!(cache.set("key1".to_string(), result, None).is_ok());
        assert_eq!(cache.len().unwrap(), 0);
        
        // 获取应该返回 None
        assert!(cache.get("key1").unwrap().is_none());
    }

    #[test]
    fn test_generate_cache_key() {
        let key1 = MemoryCache::generate_key("engine1", "query", 1, None);
        assert_eq!(key1, "engine1:query:1");
        
        let mut params = HashMap::new();
        params.insert("lang".to_string(), "en".to_string());
        params.insert("safe".to_string(), "1".to_string());
        
        let key2 = MemoryCache::generate_key("engine1", "query", 1, Some(&params));
        // 参数应该按字母顺序排列
        assert!(key2.contains("lang=en"));
        assert!(key2.contains("safe=1"));
    }

    #[test]
    fn test_cache_cleanup() {
        let cache = MemoryCache::new(CacheConfig::default());
        let result = create_test_result();
        
        // 添加一些会立即过期的项
        let ttl = Duration::from_secs(0);
        assert!(cache.set("key1".to_string(), result.clone(), Some(ttl)).is_ok());
        assert!(cache.set("key2".to_string(), result.clone(), Some(ttl)).is_ok());
        
        // 添加一个不会过期的项
        let long_ttl = Duration::from_secs(3600);
        assert!(cache.set("key3".to_string(), result, Some(long_ttl)).is_ok());
        
        std::thread::sleep(Duration::from_millis(10));
        
        // 清理应该删除2个过期项
        let cleaned = cache.cleanup().unwrap();
        assert_eq!(cleaned, 2);
        assert_eq!(cache.len().unwrap(), 1);
    }
}
