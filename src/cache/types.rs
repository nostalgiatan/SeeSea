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

//! 缓存类型定义
//!
//! 定义缓存模块的核心类型和数据结构

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// 缓存键类型
///
/// 使用字符串作为缓存键，支持灵活的键命名
pub type CacheKey = String;

/// 缓存值类型
///
/// 缓存的值以字节数组形式存储，支持任意可序列化的数据
pub type CacheValue = Vec<u8>;

/// 缓存实现配置
///
/// 定义缓存的全局配置参数
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheImplConfig {
    /// 缓存数据库路径
    pub db_path: String,
    /// 默认过期时间（秒）
    pub default_ttl_secs: u64,
    /// 最大缓存大小（字节）
    pub max_size_bytes: u64,
    /// 是否启用缓存
    pub enabled: bool,
    /// 是否启用压缩
    pub compression: bool,
    /// 缓存模式
    pub mode: CacheMode,
}

impl Default for CacheImplConfig {
    fn default() -> Self {
        Self {
            db_path: "./data/cache.db".to_string(),
            default_ttl_secs: 3600, // 1小时
            max_size_bytes: 1024 * 1024 * 1024, // 1GB
            enabled: true,
            compression: false,
            mode: CacheMode::HighThroughput,
        }
    }
}

impl CacheImplConfig {
    /// 从配置模块的 CacheConfig 创建
    pub fn from_config(config: &crate::config::cache::types::CacheConfig) -> Self {
        Self {
            db_path: config.database_path.to_string_lossy().to_string(),
            default_ttl_secs: config.ttl,
            max_size_bytes: config.max_size,
            enabled: config.enable_result_cache || config.enable_metadata_cache,
            compression: config.compression.enabled,
            mode: match config.backend {
                crate::config::cache::types::CacheBackend::Sled => CacheMode::HighThroughput,
                crate::config::cache::types::CacheBackend::Memory => CacheMode::LowLatency,
                _ => CacheMode::HighThroughput,
            },
        }
    }
}

/// 缓存模式
///
/// 定义缓存的性能优化模式
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CacheMode {
    /// 低延迟模式（更快的读写，更多内存使用）
    LowLatency,
    /// 高吞吐量模式（平衡性能和内存）
    HighThroughput,
    /// 低内存模式（节省内存，较慢）
    LowMemory,
}

/// 缓存统计信息
///
/// 记录缓存的运行统计数据
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CacheStats {
    /// 命中次数
    pub hits: u64,
    /// 未命中次数
    pub misses: u64,
    /// 写入次数
    pub writes: u64,
    /// 删除次数
    pub deletes: u64,
    /// 总键数
    pub total_keys: u64,
    /// 估算的总大小（字节）
    pub estimated_size_bytes: u64,
    /// 过期清理次数
    pub evictions: u64,
}

impl CacheStats {
    /// 计算缓存命中率
    ///
    /// # 返回值
    ///
    /// 返回 0.0 到 1.0 之间的命中率，如果没有请求则返回 0.0
    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 {
            0.0
        } else {
            self.hits as f64 / total as f64
        }
    }
}

/// 缓存条目元数据
///
/// 存储每个缓存条目的附加信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntryMetadata {
    /// 创建时间（Unix 时间戳）
    pub created_at: u64,
    /// 过期时间（Unix 时间戳，None 表示永不过期）
    pub expires_at: Option<u64>,
    /// 访问次数
    pub access_count: u64,
    /// 最后访问时间（Unix 时间戳）
    pub last_accessed_at: u64,
    /// 数据大小（字节）
    pub size_bytes: usize,
}

impl CacheEntryMetadata {
    /// 创建新的缓存条目元数据
    ///
    /// # 参数
    ///
    /// * `ttl` - 生存时间，None 表示永不过期
    /// * `size_bytes` - 数据大小
    pub fn new(ttl: Option<Duration>, size_bytes: usize) -> Self {
        let now = current_timestamp();
        Self {
            created_at: now,
            expires_at: ttl.map(|d| now + d.as_secs()),
            access_count: 0,
            last_accessed_at: now,
            size_bytes,
        }
    }

    /// 检查条目是否过期
    pub fn is_expired(&self) -> bool {
        if let Some(expires_at) = self.expires_at {
            current_timestamp() >= expires_at
        } else {
            false
        }
    }

    /// 更新访问信息
    pub fn update_access(&mut self) {
        self.access_count += 1;
        self.last_accessed_at = current_timestamp();
    }
}

/// 获取当前 Unix 时间戳（秒）
#[inline]
fn current_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::CacheConfig;

    #[test]
    fn test_cache_config_default() {
        let config = CacheConfig::default();
        assert_eq!(config.ttl, 3600);
        assert_eq!(config.max_size, 1024 * 1024 * 1024);
        assert!(config.enable_result_cache);
        // Compression is a struct, not a bool
    }

    #[test]
    fn test_cache_stats_hit_rate() {
        let mut stats = CacheStats::default();
        assert_eq!(stats.hit_rate(), 0.0);

        stats.hits = 75;
        stats.misses = 25;
        assert_eq!(stats.hit_rate(), 0.75);

        stats.hits = 100;
        stats.misses = 0;
        assert_eq!(stats.hit_rate(), 1.0);
    }

    #[test]
    fn test_cache_entry_metadata_expiration() {
        let metadata = CacheEntryMetadata::new(Some(Duration::from_secs(1)), 100);
        assert!(!metadata.is_expired());

        std::thread::sleep(Duration::from_millis(1100));
        assert!(metadata.is_expired());
    }

    #[test]
    fn test_cache_entry_metadata_no_expiration() {
        let metadata = CacheEntryMetadata::new(None, 100);
        assert!(!metadata.is_expired());
    }

    #[test]
    fn test_cache_entry_metadata_access() {
        let mut metadata = CacheEntryMetadata::new(None, 100);
        assert_eq!(metadata.access_count, 0);

        metadata.update_access();
        assert_eq!(metadata.access_count, 1);

        metadata.update_access();
        assert_eq!(metadata.access_count, 2);
    }
}
