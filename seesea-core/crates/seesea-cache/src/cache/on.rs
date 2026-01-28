use crate::cache::manager::CacheManager;
use crate::cache::result::ResultCache;
use crate::cache::rss::RssCache;
use crate::cache::scope::ScopeCache;
use crate::cache::types::CacheImplConfig;
use rocksdb::DB;
use seesea_errors::{ErrorInfo, Result};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

/// 缓存接口层
/// 提供统一的缓存操作接口，支持多种缓存后端
#[derive(Clone)]
pub struct CacheInterface {
    inner: Arc<CacheInterfaceInner>,
}

struct CacheInterfaceInner {
    db: Arc<DB>,
    config: CacheImplConfig,
    metadata: Arc<RwLock<CacheMetadata>>,
    results: Arc<ResultCache>,
    manager: Arc<CacheManager>,
}

#[derive(Debug, Default)]
struct CacheMetadata {
    total_hits: u64,
    total_misses: u64,
    total_evictions: u64,
    total_inserts: u64,
    total_updates: u64,
    total_deletes: u64,
}

/// 缓存结果结构体
#[derive(Debug, Default)]
pub struct CacheResults {
    // 保留结构体以便将来扩展
}

impl CacheResults {
    /// 全文搜索缓存结果
    pub fn search_fulltext(
        &self,
        _keywords: &[String],
        _include_stale: bool,
        _max_results: Option<usize>,
    ) -> Result<Vec<(String, seesea_derive::types::SearchResultItem)>> {
        // 简化实现：返回空结果，实际实现需要访问底层缓存
        Ok(Vec::new())
    }

    /// 获取缓存项
    pub fn get(
        &self,
        _key: &str,
        _engine_name: &str,
    ) -> Result<Option<seesea_derive::types::SearchResult>> {
        // 简化实现：返回None，实际实现需要访问底层缓存
        Ok(None)
    }

    /// 设置缓存项
    pub fn set(
        &self,
        _key: &str,
        _engine_name: &str,
        _value: &seesea_derive::types::SearchResult,
        _ttl: Option<std::time::Duration>,
    ) -> Result<()> {
        // 简化实现：成功返回，实际实现需要写入底层缓存
        Ok(())
    }
}

impl CacheInterface {
    /// 创建新的缓存接口实例
    pub fn new(config: CacheImplConfig) -> Result<Self> {
        let manager_config = CacheImplConfig {
            db_path: config.db_path.clone(),
            secondary_path: config.secondary_path.clone(),
            is_secondary: config.is_secondary,
            default_ttl_secs: config.default_ttl_secs,
            max_size_bytes: config.max_size_bytes,
            enabled: config.enabled,
            compression: config.compression,
            mode: config.mode,
            enable_bloom_filter: config.enable_bloom_filter,
            bloom_filter_expected_elements: config.bloom_filter_expected_elements,
            bloom_filter_false_positive_rate: config.bloom_filter_false_positive_rate,
        };

        let manager = CacheManager::instance(manager_config)
            .map_err(|e| ErrorInfo::new(500, format!("Failed to create cache manager: {}", e)))?;

        let results = ResultCache::with_manager(config.clone(), Arc::clone(&manager));

        let inner = Arc::new(CacheInterfaceInner {
            db: Arc::clone(manager.db()),
            config,
            metadata: Arc::new(RwLock::new(CacheMetadata::default())),
            results: Arc::new(results),
            manager,
        });

        Ok(Self { inner })
    }

    /// 获取缓存元数据
    pub async fn metadata(&self) -> CacheMetadataSnapshot {
        let metadata = self.inner.metadata.read().await;
        CacheMetadataSnapshot {
            total_hits: metadata.total_hits,
            total_misses: metadata.total_misses,
            total_evictions: metadata.total_evictions,
            total_inserts: metadata.total_inserts,
            total_updates: metadata.total_updates,
            total_deletes: metadata.total_deletes,
        }
    }

    /// 获取结果缓存
    pub fn results(&self) -> Arc<ResultCache> {
        Arc::clone(&self.inner.results)
    }

    /// 获取数据库实例
    pub fn db(&self) -> Arc<DB> {
        Arc::clone(&self.inner.db)
    }

    /// 获取配置
    pub fn config(&self) -> &CacheImplConfig {
        &self.inner.config
    }

    /// 插入数据
    pub async fn insert(&self, key: &str, value: &[u8]) -> Result<()> {
        self.inner
            .db
            .put(key.as_bytes(), value)
            .map_err(|e| ErrorInfo::new(500, format!("Failed to insert data: {}", e)))?;

        let mut metadata = self.inner.metadata.write().await;
        metadata.total_inserts += 1;

        Ok(())
    }

    /// 获取数据
    pub async fn get(&self, key: &str) -> Result<Option<Vec<u8>>> {
        match self.inner.db.get(key.as_bytes()) {
            Ok(Some(value)) => {
                let mut metadata = self.inner.metadata.write().await;
                metadata.total_hits += 1;
                Ok(Some(value.to_vec()))
            }
            Ok(None) => {
                let mut metadata = self.inner.metadata.write().await;
                metadata.total_misses += 1;
                Ok(None)
            }
            Err(e) => Err(ErrorInfo::new(500, format!("Failed to get data: {}", e))),
        }
    }

    /// 删除数据
    pub async fn delete(&self, key: &str) -> Result<()> {
        self.inner
            .db
            .delete(key.as_bytes())
            .map_err(|e| ErrorInfo::new(500, format!("Failed to delete data: {}", e)))?;

        let mut metadata = self.inner.metadata.write().await;
        metadata.total_deletes += 1;

        Ok(())
    }

    /// 清空缓存
    pub async fn clear(&self) -> Result<()> {
        // RocksDB 不支持直接清空所有数据，需要遍历删除
        let iter = self.inner.db.iterator(rocksdb::IteratorMode::Start);
        for item in iter {
            let (key, _) =
                item.map_err(|e| ErrorInfo::new(500, format!("Failed to iterate: {}", e)))?;
            self.inner
                .db
                .delete(key)
                .map_err(|e| ErrorInfo::new(500, format!("Failed to delete: {}", e)))?;
        }

        let mut metadata = self.inner.metadata.write().await;
        metadata.total_evictions += 1;

        Ok(())
    }

    /// 获取缓存大小
    pub fn size(&self) -> Result<u64> {
        // RocksDB 获取磁盘大小需要使用不同的方法
        // 这里返回一个近似值
        Ok(0)
    }

    /// 获取作用域缓存
    pub fn scope(&self, scope_name: &str) -> ScopeCache {
        ScopeCache::new(Arc::clone(&self.inner.manager), scope_name.to_string())
    }

    /// 获取RSS缓存
    pub fn rss(&self) -> RssCache {
        RssCache::new(Arc::clone(&self.inner.manager))
    }

    /// 清空所有缓存（包括所有作用域）
    pub async fn clear_all(&self) -> Result<()> {
        // RocksDB 不支持直接清空所有数据，需要遍历删除
        let iter = self.inner.db.iterator(rocksdb::IteratorMode::Start);
        for item in iter {
            let (key, _) =
                item.map_err(|e| ErrorInfo::new(500, format!("Failed to iterate: {}", e)))?;
            self.inner
                .db
                .delete(key)
                .map_err(|e| ErrorInfo::new(500, format!("Failed to delete: {}", e)))?;
        }
        Ok(())
    }

    /// 获取缓存管理器（返回CacheManager实例）
    pub fn manager(&self) -> Result<Arc<CacheManager>> {
        Ok(Arc::clone(&self.inner.manager))
    }

    /// 清理过期缓存
    pub async fn cleanup(&self) -> Result<usize> {
        // 获取缓存管理器并清理过期缓存
        let manager = self.manager()?;
        manager
            .cleanup_expired(None)
            .map_err(|e| ErrorInfo::new(500, format!("Failed to cleanup expired cache: {}", e)))
    }

    /// 获取缓存条目元数据
    pub fn get_metadata(
        &self,
        _scope: &str,
        _key: &str,
    ) -> Result<Option<crate::cache::types::CacheEntryMetadata>> {
        // 注意：由于迁移到 RocksDB 后移除了元数据列族，此方法暂时不可用
        // 如果需要元数据功能，可以考虑在数据值中嵌入元数据或使用其他方案
        Ok(None)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheMetadataSnapshot {
    pub total_hits: u64,
    pub total_misses: u64,
    pub total_evictions: u64,
    pub total_inserts: u64,
    pub total_updates: u64,
    pub total_deletes: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cache::types::CacheMode;

    #[test]
    fn test_cache_interface_creation() {
        let temp_dir = std::env::temp_dir();
        let db_path = temp_dir.join(format!("test_cache_interface_{}", std::process::id()));

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

        let interface = CacheInterface::new(config);
        assert!(interface.is_ok());
    }

    #[test]
    fn test_cache_interface_results_and_metadata() {
        let temp_dir = std::env::temp_dir();
        let db_path = temp_dir.join(format!("test_cache_interface_2_{}", std::process::id()));

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

        let interface = CacheInterface::new(config).expect("创建缓存接口失败");

        // 测试可以获取子缓存
        let _ = interface.results();
        let _ = interface.metadata();

        // 清理
        let _ = std::fs::remove_dir_all(&db_path);
    }
}
