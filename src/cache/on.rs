//! 缓存外部接口
//!
//! 提供缓存模块的公共 API 接口

use crate::cache::manager::{CacheError, CacheManager, Result};
use crate::cache::metadata::MetadataCache;
use crate::cache::result::ResultCache;
use crate::cache::types::CacheConfig;
use std::sync::Arc;

/// 统一的缓存接口
///
/// 提供对所有缓存功能的统一访问
pub struct CacheInterface {
    /// 缓存管理器
    manager: Arc<CacheManager>,
}

impl CacheInterface {
    /// 创建缓存接口
    ///
    /// # 参数
    ///
    /// * `config` - 缓存配置
    ///
    /// # 返回值
    ///
    /// 返回缓存接口实例或错误
    ///
    /// # 示例
    ///
    /// ```rust,no_run
    /// use seesea::cache::{CacheInterface, CacheConfig};
    ///
    /// let config = CacheConfig::default();
    /// let cache = CacheInterface::new(config)?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn new(config: CacheConfig) -> Result<Self> {
        let manager = Arc::new(CacheManager::new(config)?);

        Ok(Self { manager })
    }

    /// 获取搜索结果缓存
    pub fn results(&self) -> ResultCache {
        ResultCache::new(Arc::clone(&self.manager))
    }

    /// 获取元数据缓存
    pub fn metadata(&self) -> MetadataCache {
        MetadataCache::new(Arc::clone(&self.manager))
    }

    /// 获取缓存管理器引用
    pub fn manager(&self) -> &CacheManager {
        &self.manager
    }

    /// 清空所有缓存
    pub fn clear_all(&self) -> Result<()> {
        self.manager.clear()
    }

    /// 刷新缓存到磁盘
    pub fn flush(&self) -> Result<()> {
        self.manager.flush()
    }

    /// 清理过期条目
    pub fn cleanup(&self) -> Result<usize> {
        self.manager.cleanup_expired()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cache::types::CacheMode;

    #[test]
    fn test_cache_interface_creation() {
        let temp_dir = std::env::temp_dir();
        let db_path = temp_dir.join(format!("test_cache_interface_{}", std::process::id()));
        
        let config = CacheConfig {
            db_path: db_path.to_string_lossy().to_string(),
            default_ttl_secs: 3600,
            max_size_bytes: 1024 * 1024,
            enabled: true,
            compression: false,
            mode: CacheMode::HighThroughput,
        };

        let interface = CacheInterface::new(config);
        assert!(interface.is_ok());
    }

    #[test]
    fn test_cache_interface_results_and_metadata() {
        let temp_dir = std::env::temp_dir();
        let db_path = temp_dir.join(format!("test_cache_interface_2_{}", std::process::id()));
        
        let config = CacheConfig {
            db_path: db_path.to_string_lossy().to_string(),
            default_ttl_secs: 3600,
            max_size_bytes: 1024 * 1024,
            enabled: true,
            compression: false,
            mode: CacheMode::HighThroughput,
        };

        let interface = CacheInterface::new(config).expect("创建缓存接口失败");
        
        // 测试可以获取子缓存
        let _ = interface.results();
        let _ = interface.metadata();
        let _ = interface.manager();
    }
}

