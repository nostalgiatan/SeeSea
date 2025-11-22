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

//! 缓存外部接口
//!
//! 提供缓存模块的公共 API 接口

use crate::cache::manager::{CacheManager, Result};
use crate::cache::metadata::MetadataCache;
use crate::cache::result::ResultCache;
use crate::cache::rss::RssCache;
use crate::cache::semantic_cache::{SemanticCache, SemanticCacheConfig};
use crate::cache::types::CacheImplConfig;
use std::sync::Arc;

/// 统一的缓存接口
///
/// 提供对所有缓存功能的统一访问
pub struct CacheInterface {
    /// 缓存管理器
    manager: Arc<CacheManager>,
    /// 语义缓存配置
    semantic_config: SemanticCacheConfig,
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
    /// use seesea::cache::{CacheInterface, CacheImplConfig};
    ///
    /// let config = CacheImplConfig::default();
    /// let cache = CacheInterface::new(config)?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn new(config: CacheImplConfig) -> Result<Self> {
        let manager = CacheManager::instance(config)?;

        Ok(Self {
            manager,
            semantic_config: SemanticCacheConfig::default(),
        })
    }

    /// 设置语义缓存配置
    pub fn with_semantic_config(mut self, config: SemanticCacheConfig) -> Self {
        self.semantic_config = config;
        self
    }

    /// 获取搜索结果缓存
    pub fn results(&self) -> ResultCache {
        ResultCache::new(Arc::clone(&self.manager))
    }

    /// 获取元数据缓存
    pub fn metadata(&self) -> MetadataCache {
        MetadataCache::new(Arc::clone(&self.manager))
    }

    /// 获取 RSS 缓存
    pub fn rss(&self) -> RssCache {
        RssCache::new(Arc::clone(&self.manager))
    }

    /// 获取语义缓存
    pub fn semantic(&self) -> SemanticCache {
        SemanticCache::new(Arc::clone(&self.manager), self.semantic_config.clone())
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
        
        let config = CacheImplConfig {
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
        
        let config = CacheImplConfig {
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

