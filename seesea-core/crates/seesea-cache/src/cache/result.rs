use crate::cache::manager::CacheManager;
use crate::cache::types::CacheImplConfig;
use dashmap::DashMap;
use seesea_errors::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::RwLock;

/// 结果缓存
/// 用于缓存搜索结果、计算结果等
#[derive(Debug, Clone)]
pub struct ResultCache {
    inner: Arc<ResultCacheInner>,
}

struct ResultCacheInner {
    /// 缓存数据
    data: DashMap<String, CachedResult>,
    /// 缓存配置
    config: CacheImplConfig,
    /// 缓存统计
    stats: RwLock<CacheStats>,
    /// 缓存管理器（用于作用域支持）
    manager: Option<Arc<CacheManager>>,
}

impl std::fmt::Debug for ResultCacheInner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ResultCacheInner")
            .field("data", &self.data)
            .field("config", &self.config)
            .field("stats", &self.stats)
            .field("manager", &self.manager.as_ref().map(|_| "CacheManager"))
            .finish()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CachedResult {
    /// 结果数据
    data: Vec<u8>,
    /// 过期时间
    expires_at: Option<SystemTime>,
    /// 创建时间
    created_at: SystemTime,
    /// 访问次数
    access_count: u64,
    /// 最后访问时间
    last_accessed: SystemTime,
}

#[derive(Debug, Default)]
struct CacheStats {
    total_hits: u64,
    total_misses: u64,
    total_evictions: u64,
    total_inserts: u64,
    total_updates: u64,
    total_deletes: u64,
}

impl ResultCache {
    /// 创建新的结果缓存实例
    pub fn new(config: CacheImplConfig) -> Self {
        let inner = Arc::new(ResultCacheInner {
            data: DashMap::new(),
            config,
            stats: RwLock::new(CacheStats::default()),
            manager: None,
        });

        Self { inner }
    }

    /// 创建新的结果缓存实例（带缓存管理器支持作用域）
    pub fn with_manager(config: CacheImplConfig, manager: Arc<CacheManager>) -> Self {
        let inner = Arc::new(ResultCacheInner {
            data: DashMap::new(),
            config,
            stats: RwLock::new(CacheStats::default()),
            manager: Some(manager),
        });

        Self { inner }
    }

    /// 获取缓存配置
    pub fn config(&self) -> &CacheImplConfig {
        &self.inner.config
    }

    /// 设置缓存项（支持作用域）
    pub async fn set(&self, key: String, value: Vec<u8>, ttl: Option<Duration>) -> Result<()> {
        let expires_at = ttl.map(|duration| SystemTime::now() + duration);
        let created_at = SystemTime::now();

        let cached_result = CachedResult {
            data: value,
            expires_at,
            created_at,
            access_count: 0,
            last_accessed: created_at,
        };

        self.inner.data.insert(key, cached_result);

        let mut stats = self.inner.stats.write().await;
        stats.total_inserts += 1;

        Ok(())
    }

    /// 设置缓存项（带作用域）
    pub async fn set_with_scope(
        &self,
        scope: &str,
        key: &str,
        value: Vec<u8>,
        ttl: Option<Duration>,
    ) -> Result<()> {
        if let Some(ref manager) = self.inner.manager {
            let full_key = format!("{}:{}", scope, key);
            return manager.set(scope, full_key, value, ttl).map_err(|e| {
                seesea_errors::ErrorInfo::new(500, format!("Failed to set cache with scope: {}", e))
            });
        }

        self.set(key.to_string(), value, ttl).await
    }

    /// 设置搜索结果缓存（带引擎作用域）
    pub async fn set_search_result(
        &self,
        key: &str,
        engine_name: &str,
        value: &seesea_derive::types::SearchResult,
        ttl: Option<Duration>,
    ) -> Result<()> {
        let serialized = serde_json::to_vec(value).map_err(|e| {
            seesea_errors::ErrorInfo::new(500, format!("Failed to serialize search result: {}", e))
        })?;

        let scope = format!("search:{}", engine_name);
        self.set_with_scope(&scope, key, serialized, ttl).await
    }

    /// 获取搜索结果缓存（带引擎作用域）
    pub async fn get_search_result(
        &self,
        key: &str,
        engine_name: &str,
    ) -> Result<Option<seesea_derive::types::SearchResult>> {
        let scope = format!("search:{}", engine_name);
        if let Some(data) = self.get_with_scope(&scope, key).await? {
            let result = serde_json::from_slice(&data).map_err(|e| {
                seesea_errors::ErrorInfo::new(
                    500,
                    format!("Failed to deserialize search result: {}", e),
                )
            })?;
            Ok(Some(result))
        } else {
            Ok(None)
        }
    }

    /// 全文搜索缓存结果
    pub async fn search_fulltext(
        &self,
        _keywords: &[String],
        _include_stale: bool,
        _max_results: Option<usize>,
    ) -> Result<Vec<(String, seesea_derive::types::SearchResultItem)>> {
        // 简化实现：返回空结果
        // 实际实现需要访问底层缓存并进行全文搜索
        Ok(Vec::new())
    }

    /// 获取缓存项
    pub async fn get(&self, key: &str) -> Result<Option<Vec<u8>>> {
        if let Some(mut entry) = self.inner.data.get_mut(key) {
            // 检查是否过期
            if let Some(expires_at) = entry.expires_at
                && SystemTime::now() > expires_at
            {
                drop(entry);
                self.inner.data.remove(key);

                let mut stats = self.inner.stats.write().await;
                stats.total_evictions += 1;
                stats.total_misses += 1;

                return Ok(None);
            }

            // 更新访问统计
            entry.access_count += 1;
            entry.last_accessed = SystemTime::now();

            let mut stats = self.inner.stats.write().await;
            stats.total_hits += 1;

            Ok(Some(entry.data.clone()))
        } else {
            let mut stats = self.inner.stats.write().await;
            stats.total_misses += 1;
            Ok(None)
        }
    }

    /// 获取缓存项（带作用域）
    pub async fn get_with_scope(&self, scope: &str, key: &str) -> Result<Option<Vec<u8>>> {
        if let Some(ref manager) = self.inner.manager {
            let full_key = format!("{}:{}", scope, key);
            return manager
                .get(scope, &full_key)
                .map_err(|e| {
                    seesea_errors::ErrorInfo::new(
                        500,
                        format!("Failed to get cache with scope: {}", e),
                    )
                })
                .map(|opt| opt.map(|v| v.to_vec()));
        }

        self.get(key).await
    }

    /// 删除缓存项
    pub async fn delete(&self, key: &str) -> Result<bool> {
        let existed = self.inner.data.remove(key).is_some();

        if existed {
            let mut stats = self.inner.stats.write().await;
            stats.total_deletes += 1;
        }

        Ok(existed)
    }

    /// 清空缓存
    pub async fn clear(&self) -> Result<()> {
        self.inner.data.clear();

        let mut stats = self.inner.stats.write().await;
        stats.total_evictions += self.inner.data.len() as u64;

        Ok(())
    }

    /// 获取缓存统计信息
    pub async fn stats(&self) -> Result<CacheStatsSnapshot> {
        let stats = self.inner.stats.read().await;
        Ok(CacheStatsSnapshot {
            total_hits: stats.total_hits,
            total_misses: stats.total_misses,
            total_evictions: stats.total_evictions,
            total_inserts: stats.total_inserts,
            total_updates: stats.total_updates,
            total_deletes: stats.total_deletes,
            current_size: self.inner.data.len() as u64,
        })
    }

    /// 获取缓存大小
    pub fn len(&self) -> usize {
        self.inner.data.len()
    }

    /// 检查缓存是否为空
    pub fn is_empty(&self) -> bool {
        self.inner.data.is_empty()
    }

    /// 清理过期缓存项
    pub async fn cleanup_expired(&self) -> Result<u64> {
        let now = SystemTime::now();
        let mut expired_count = 0u64;

        self.inner.data.retain(|_key, entry| {
            if let Some(expires_at) = entry.expires_at {
                if now > expires_at {
                    expired_count += 1;
                    false
                } else {
                    true
                }
            } else {
                true
            }
        });

        if expired_count > 0 {
            let mut stats = self.inner.stats.write().await;
            stats.total_evictions += expired_count;
        }

        Ok(expired_count)
    }

    /// 获取所有键
    pub fn keys(&self) -> Vec<String> {
        self.inner
            .data
            .iter()
            .map(|entry| entry.key().clone())
            .collect()
    }

    /// 检查键是否存在（不过期检查）
    pub fn contains_key(&self, key: &str) -> bool {
        self.inner.data.contains_key(key)
    }

    /// 批量设置缓存项
    pub async fn set_batch(&self, items: Vec<(String, Vec<u8>, Option<Duration>)>) -> Result<()> {
        for (key, value, ttl) in items {
            self.set(key, value, ttl).await?;
        }
        Ok(())
    }

    /// 批量获取缓存项
    pub async fn get_batch(&self, keys: &[String]) -> Result<Vec<Option<Vec<u8>>>> {
        let mut results = Vec::with_capacity(keys.len());
        for key in keys {
            results.push(self.get(key).await?);
        }
        Ok(results)
    }

    /// 批量删除缓存项
    pub async fn delete_batch(&self, keys: &[String]) -> Result<u64> {
        let mut deleted_count = 0u64;
        for key in keys {
            if self.delete(key).await? {
                deleted_count += 1;
            }
        }
        Ok(deleted_count)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStatsSnapshot {
    pub total_hits: u64,
    pub total_misses: u64,
    pub total_evictions: u64,
    pub total_inserts: u64,
    pub total_updates: u64,
    pub total_deletes: u64,
    pub current_size: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cache::types::{CacheImplConfig, CacheMode};
    use seesea_config::common::SafeSearchLevel;
    use seesea_derive::types::EngineType;
    use serial_test::serial;
    use std::collections::HashMap;

    fn temp_result_cache() -> ResultCache {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let counter = COUNTER.fetch_add(1, Ordering::SeqCst);

        let temp_dir = std::env::temp_dir();
        let db_path = temp_dir.join(format!("test_result_cache_{}", counter));

        let config = CacheImplConfig {
            db_path: db_path.to_string_lossy().to_string(),
            secondary_path: None,
            is_secondary: false,
            default_ttl_secs: 3600,
            max_size_bytes: 1024 * 1024,
            enabled: true,
            compression: false,
            mode: CacheMode::HighThroughput,
            enable_bloom_filter: false,
            bloom_filter_expected_elements: 1000,
            bloom_filter_false_positive_rate: 0.01,
        };

        ResultCache::new(config)
    }

    #[tokio::test]
    #[serial]
    async fn test_basic_cache_operations() {
        let cache = temp_result_cache();

        // 测试设置和获取
        let key = "test_key";
        let value = b"test_value".to_vec();

        cache
            .set(key.to_string(), value.clone(), None)
            .await
            .expect("设置缓存失败");
        let result = cache.get(key).await.expect("获取缓存失败");

        assert!(result.is_some());
        assert_eq!(result.unwrap(), value);
    }

    #[tokio::test]
    #[serial]
    async fn test_cache_expiration() {
        let cache = temp_result_cache();

        // 测试过期
        let key = "expire_key";
        let value = b"expire_value".to_vec();

        cache
            .set(key.to_string(), value, Some(Duration::from_millis(100)))
            .await
            .expect("设置缓存失败");

        // 立即获取应该存在
        let result = cache.get(key).await.expect("获取缓存失败");
        assert!(result.is_some());

        // 等待过期
        tokio::time::sleep(Duration::from_millis(150)).await;

        // 过期后获取应该不存在
        let result = cache.get(key).await.expect("获取缓存失败");
        assert!(result.is_none());
    }

    #[tokio::test]
    #[serial]
    async fn test_cache_stats() {
        let cache = temp_result_cache();

        // 测试统计
        let key = "stats_key";
        let value = b"stats_value".to_vec();

        cache
            .set(key.to_string(), value, None)
            .await
            .expect("设置缓存失败");
        let _ = cache.get(key).await.expect("获取缓存失败");
        let _ = cache.get("non_existent").await.expect("获取缓存失败");

        let stats = cache.stats().await.expect("获取统计失败");
        assert_eq!(stats.total_inserts, 1);
        assert_eq!(stats.total_hits, 1);
        assert_eq!(stats.total_misses, 1);
    }

    #[tokio::test]
    #[serial]
    async fn test_batch_operations() {
        let cache = temp_result_cache();

        // 测试批量操作
        let items = vec![
            ("batch_key1".to_string(), b"batch_value1".to_vec(), None),
            ("batch_key2".to_string(), b"batch_value2".to_vec(), None),
            ("batch_key3".to_string(), b"batch_value3".to_vec(), None),
        ];

        cache.set_batch(items).await.expect("批量设置失败");

        let keys = vec![
            "batch_key1".to_string(),
            "batch_key2".to_string(),
            "non_existent".to_string(),
        ];
        let results = cache.get_batch(&keys).await.expect("批量获取失败");

        assert_eq!(results.len(), 3);
        assert!(results[0].is_some());
        assert!(results[1].is_some());
        assert!(results[2].is_none());
    }

    #[tokio::test]
    #[serial]
    async fn test_cleanup_expired() {
        let cache = temp_result_cache();

        // 测试清理过期项
        let items = vec![
            (
                "expire1".to_string(),
                b"value1".to_vec(),
                Some(Duration::from_millis(50)),
            ),
            (
                "expire2".to_string(),
                b"value2".to_vec(),
                Some(Duration::from_millis(50)),
            ),
            ("keep".to_string(), b"value3".to_vec(), None),
        ];

        cache.set_batch(items).await.expect("批量设置失败");

        // 等待过期
        tokio::time::sleep(Duration::from_millis(100)).await;

        // 清理过期项
        let cleaned = cache.cleanup_expired().await.expect("清理过期项失败");
        assert_eq!(cleaned, 2);

        // 验证
        assert!(cache.get("expire1").await.expect("获取失败").is_none());
        assert!(cache.get("expire2").await.expect("获取失败").is_none());
        assert!(cache.get("keep").await.expect("获取失败").is_some());
    }
}
