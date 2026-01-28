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

//! 热榜缓存模块
//!
//! 提供热榜数据的缓存功能，默认5分钟过期。
//! 支持单平台缓存和全平台缓存。
//! 统一使用 cache 模块的 ScopeCache 实现。

use crate::types::HotTrendResult;
use once_cell::sync::Lazy;
use seesea_cache::CacheInterface;
use seesea_cache::cache::scope::ScopeCache;
use seesea_cache::cache::types::{CacheImplConfig, CacheMode};
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, info, warn};

/// 默认缓存过期时间（5分钟）
pub const DEFAULT_CACHE_TTL_SECS: u64 = 300;

/// 热榜缓存作用域名称
const HOT_TREND_SCOPE: &str = "hot_trend";

/// 全平台缓存键
const ALL_PLATFORMS_KEY: &str = "__all_platforms__";

/// 全局缓存接口
static CACHE_INTERFACE: Lazy<Arc<CacheInterface>> = Lazy::new(|| {
    let config = CacheImplConfig {
        db_path: "D:/Program Files/SeeSea/.cache".to_string(),
        secondary_path: None,
        is_secondary: false,
        default_ttl_secs: DEFAULT_CACHE_TTL_SECS,
        max_size_bytes: 100 * 1024 * 1024,
        enabled: true,
        compression: false,
        mode: CacheMode::HighThroughput,
        enable_bloom_filter: false,
        bloom_filter_expected_elements: 1000,
        bloom_filter_false_positive_rate: 0.01,
    };

    match CacheInterface::new(config) {
        Ok(cache) => {
            info!("热榜统一缓存初始化成功");
            Arc::new(cache)
        }
        Err(e) => {
            warn!("热榜统一缓存初始化失败: {}，使用默认配置", e);
            Arc::new(
                CacheInterface::new(CacheImplConfig::new(
                    "D:/Program Files/SeeSea/.cache".to_string(),
                ))
                .unwrap(),
            )
        }
    }
});

/// 热榜缓存管理器
///
/// 基于统一的 cache 模块实现，使用 ScopeCache 进行数据存储。
pub struct HotTrendCache {
    /// 作用域缓存访问器
    scope_cache: ScopeCache,
    /// 缓存过期时间
    ttl: Duration,
}

impl HotTrendCache {
    /// 创建新的热榜缓存
    pub fn new(ttl_secs: Option<u64>) -> Self {
        let ttl = Duration::from_secs(ttl_secs.unwrap_or(DEFAULT_CACHE_TTL_SECS));
        let cache_manager = CACHE_INTERFACE
            .manager()
            .expect("Failed to get cache manager");
        let scope_cache = ScopeCache::new(cache_manager, HOT_TREND_SCOPE.to_string());
        info!(
            "热榜缓存初始化，TTL: {:?}，作用域: {}",
            ttl, HOT_TREND_SCOPE
        );
        Self { scope_cache, ttl }
    }

    /// 生成平台缓存键
    fn platform_key(platform_id: &str) -> String {
        format!("platform:{}", platform_id)
    }

    /// 获取单平台缓存
    pub async fn get_platform(&self, platform_id: &str) -> Option<HotTrendResult> {
        let key = Self::platform_key(platform_id);
        match self.scope_cache.get(&key) {
            Ok(Some(data)) => match serde_json::from_slice::<HotTrendResult>(&data[..]) {
                Ok(result) => {
                    debug!("热榜缓存命中: {}", platform_id);
                    Some(result)
                }
                Err(e) => {
                    warn!("热榜缓存反序列化失败: {}", e);
                    None
                }
            },
            Ok(None) => None,
            Err(e) => {
                warn!("热榜缓存读取失败: {}", e);
                None
            }
        }
    }

    /// 设置单平台缓存
    pub async fn set_platform(&self, platform_id: &str, data: HotTrendResult) {
        let key = Self::platform_key(platform_id);
        match serde_json::to_vec(&data) {
            Ok(bytes) => {
                if let Err(e) = self.scope_cache.set(key, bytes, Some(self.ttl)) {
                    warn!("热榜缓存写入失败: {}", e);
                } else {
                    debug!("热榜缓存已更新: {}", platform_id);
                }
            }
            Err(e) => {
                warn!("热榜缓存序列化失败: {}", e);
            }
        }
    }

    /// 获取全平台缓存
    pub async fn get_all_platforms(&self) -> Option<Vec<HotTrendResult>> {
        match self.scope_cache.get(ALL_PLATFORMS_KEY) {
            Ok(Some(data)) => match serde_json::from_slice::<Vec<HotTrendResult>>(&data[..]) {
                Ok(results) => {
                    debug!("全平台热榜缓存命中");
                    Some(results)
                }
                Err(e) => {
                    warn!("全平台热榜缓存反序列化失败: {}", e);
                    None
                }
            },
            Ok(None) => None,
            Err(e) => {
                warn!("全平台热榜缓存读取失败: {}", e);
                None
            }
        }
    }

    /// 设置全平台缓存
    pub async fn set_all_platforms(&self, data: Vec<HotTrendResult>) {
        match serde_json::to_vec(&data) {
            Ok(bytes) => {
                if let Err(e) =
                    self.scope_cache
                        .set(ALL_PLATFORMS_KEY.to_string(), bytes, Some(self.ttl))
                {
                    warn!("全平台热榜缓存写入失败: {}", e);
                } else {
                    debug!("全平台热榜缓存已更新");
                }
            }
            Err(e) => {
                warn!("全平台热榜缓存序列化失败: {}", e);
            }
        }
    }

    /// 清除单平台缓存
    pub async fn invalidate_platform(&self, platform_id: &str) {
        let key = Self::platform_key(platform_id);
        if let Err(e) = self.scope_cache.delete(&key) {
            warn!("热榜缓存清除失败: {}", e);
        } else {
            debug!("热榜缓存已清除: {}", platform_id);
        }
    }

    /// 清除所有缓存
    pub async fn invalidate_all(&self) {
        if let Err(e) = self.scope_cache.clear() {
            warn!("热榜缓存清除失败: {}", e);
        } else {
            info!("所有热榜缓存已清除");
        }
    }

    /// 清理过期缓存
    pub async fn cleanup_expired(&self) {
        if let Err(e) = self.scope_cache.cleanup_expired() {
            warn!("热榜缓存清理失败: {}", e);
        } else {
            debug!("过期热榜缓存已清理");
        }
    }

    /// 获取缓存统计信息
    pub async fn stats(&self) -> HotTrendCacheStats {
        // 统一缓存模式下，通过 manager 获取统计
        HotTrendCacheStats {
            platform_cache_count: 0, // 统一缓存不再单独统计
            has_all_platforms_cache: self.get_all_platforms().await.is_some(),
            ttl_secs: self.ttl.as_secs(),
        }
    }
}

/// 缓存统计信息
#[derive(Debug, Clone)]
pub struct HotTrendCacheStats {
    /// 单平台缓存数量
    pub platform_cache_count: usize,
    /// 是否有全平台缓存
    pub has_all_platforms_cache: bool,
    /// 缓存过期时间（秒）
    pub ttl_secs: u64,
}

/// 全局热榜缓存实例
static HOT_TREND_CACHE: once_cell::sync::Lazy<Arc<HotTrendCache>> =
    once_cell::sync::Lazy::new(|| Arc::new(HotTrendCache::new(None)));

/// 获取全局热榜缓存
pub fn get_hot_trend_cache() -> Arc<HotTrendCache> {
    HOT_TREND_CACHE.clone()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::HotTrendItem;

    #[tokio::test]
    async fn test_cache_set_get() {
        let cache = HotTrendCache::new(Some(60));

        let result = HotTrendResult {
            platform_id: "test".to_string(),
            platform_name: "测试平台".to_string(),
            status: "success".to_string(),
            items: vec![HotTrendItem {
                title: "测试标题".to_string(),
                url: "https://example.com".to_string(),
                mobile_url: None,
                rank: Some(1),
                hot_value: None,
                hot_index: None,
                source: None,
                publish_time: None,
            }],
        };

        cache.set_platform("test", result.clone()).await;

        let cached = cache.get_platform("test").await;
        assert!(cached.is_some());
        assert_eq!(cached.unwrap().platform_id, "test");
    }

    #[tokio::test]
    async fn test_cache_expiration() {
        let cache = HotTrendCache::new(Some(1)); // 1秒过期

        let result = HotTrendResult {
            platform_id: "test".to_string(),
            platform_name: "测试平台".to_string(),
            status: "success".to_string(),
            items: vec![],
        };

        cache.set_platform("test", result).await;

        // 等待过期
        tokio::time::sleep(Duration::from_secs(2)).await;

        let cached = cache.get_platform("test").await;
        assert!(cached.is_none());
    }
}
