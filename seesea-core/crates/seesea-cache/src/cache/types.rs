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

//! 缓存类型定义
//!
//! 定义缓存相关的类型和配置

use serde::{Deserialize, Serialize};
use std::time::Duration;

pub use seesea_config::{
    CacheBackend, CacheConfig, CompressionAlgorithm, CompressionConfig, EvictionPolicy,
    ShardingConfig, ShardingStrategy,
};

/// 缓存值类型
pub type CacheValue = Vec<u8>;

/// 缓存模式
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum CacheMode {
    /// 低延迟模式，适合读多写少
    LowLatency,
    /// 高吞吐量模式，适合大量写入
    HighThroughput,
    /// 平衡模式，读写均衡
    #[default]
    Balanced,
    /// 内存优化模式，适合内存受限环境
    LowMemory,
    /// 高性能模式，数据持久化到磁盘
    HighPerformance,
}

/// 缓存实现配置（兼容层，使用集中配置）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheImplConfig {
    /// 数据库路径（用于持久化模式）
    pub db_path: String,
    /// Secondary 实例路径（用于多进程访问）
    pub secondary_path: Option<String>,
    /// 是否作为 Secondary 实例打开（只读，用于多进程访问）
    pub is_secondary: bool,
    /// 默认TTL（秒）
    pub default_ttl_secs: u64,
    /// 最大缓存大小（字节）
    pub max_size_bytes: u64,
    /// 是否启用缓存
    pub enabled: bool,
    /// 是否启用压缩
    pub compression: bool,
    /// 缓存模式
    pub mode: CacheMode,
    /// 是否启用布隆过滤器
    pub enable_bloom_filter: bool,
    /// 布隆过滤器期望元素数量
    pub bloom_filter_expected_elements: u64,
    /// 布隆过滤器误报率
    pub bloom_filter_false_positive_rate: f64,
}

impl Default for CacheImplConfig {
    fn default() -> Self {
        Self {
            db_path: "./cache".to_string(),
            secondary_path: None,
            is_secondary: false,
            default_ttl_secs: 3600,
            max_size_bytes: 1024 * 1024 * 1024,
            enabled: true,
            compression: false,
            mode: CacheMode::default(),
            enable_bloom_filter: false,
            bloom_filter_expected_elements: 10000,
            bloom_filter_false_positive_rate: 0.01,
        }
    }
}

impl From<CacheConfig> for CacheImplConfig {
    fn from(config: CacheConfig) -> Self {
        Self {
            db_path: config.database_path.to_string_lossy().to_string(),
            secondary_path: None,
            is_secondary: false,
            default_ttl_secs: config.ttl,
            max_size_bytes: config.max_size,
            enabled: config.enable_result_cache,
            compression: config.compression.enabled,
            mode: CacheMode::Balanced,
            enable_bloom_filter: false,
            bloom_filter_expected_elements: 10000,
            bloom_filter_false_positive_rate: 0.01,
        }
    }
}

impl CacheImplConfig {
    pub fn new(db_path: String) -> Self {
        Self {
            db_path,
            ..Default::default()
        }
    }

    pub fn with_ttl(mut self, ttl_secs: u64) -> Self {
        self.default_ttl_secs = ttl_secs;
        self
    }

    pub fn with_max_size(mut self, max_size_bytes: u64) -> Self {
        self.max_size_bytes = max_size_bytes;
        self
    }

    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    pub fn with_compression(mut self, compression: bool) -> Self {
        self.compression = compression;
        self
    }

    pub fn with_mode(mut self, mode: CacheMode) -> Self {
        self.mode = mode;
        self
    }

    pub fn with_bloom_filter(mut self, expected_elements: u64, false_positive_rate: f64) -> Self {
        self.enable_bloom_filter = true;
        self.bloom_filter_expected_elements = expected_elements;
        self.bloom_filter_false_positive_rate = false_positive_rate;
        self
    }

    pub fn default_ttl(&self) -> Duration {
        Duration::from_secs(self.default_ttl_secs)
    }

    pub fn as_primary(mut self) -> Self {
        self.is_secondary = false;
        self.secondary_path = None;
        self
    }

    pub fn as_secondary(mut self, secondary_path: String) -> Self {
        self.is_secondary = true;
        self.secondary_path = Some(secondary_path);
        self
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.default_ttl_secs == 0 {
            return Err("默认TTL不能为0".to_string());
        }
        if self.max_size_bytes == 0 {
            return Err("最大缓存大小不能为0".to_string());
        }
        if self.bloom_filter_false_positive_rate <= 0.0
            || self.bloom_filter_false_positive_rate >= 1.0
        {
            return Err("布隆过滤器误报率必须在(0, 1)范围内".to_string());
        }
        if self.bloom_filter_expected_elements == 0 {
            return Err("布隆过滤器期望元素数量不能为0".to_string());
        }
        Ok(())
    }
}

/// 缓存作用域配置（兼容层）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheScopeConfig {
    /// 作用域名称
    pub name: String,
    /// 是否启用
    pub enabled: bool,
    /// 自定义TTL（如果为None则使用默认TTL）
    pub custom_ttl_secs: Option<u64>,
    /// 自定义最大大小（如果为None则使用默认最大大小）
    pub custom_max_size_bytes: Option<u64>,
    /// 是否启用压缩
    pub compression: bool,
    /// 缓存模式（如果为None则使用默认模式）
    pub mode: Option<CacheMode>,
}

impl Default for CacheScopeConfig {
    fn default() -> Self {
        Self {
            name: "default".to_string(),
            enabled: true,
            custom_ttl_secs: None,
            custom_max_size_bytes: None,
            compression: false,
            mode: None,
        }
    }
}

impl CacheScopeConfig {
    pub fn new(name: String) -> Self {
        Self {
            name,
            ..Default::default()
        }
    }

    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    pub fn with_custom_ttl(mut self, ttl_secs: u64) -> Self {
        self.custom_ttl_secs = Some(ttl_secs);
        self
    }

    pub fn with_custom_max_size(mut self, max_size_bytes: u64) -> Self {
        self.custom_max_size_bytes = Some(max_size_bytes);
        self
    }

    pub fn with_compression(mut self, compression: bool) -> Self {
        self.compression = compression;
        self
    }

    pub fn with_mode(mut self, mode: CacheMode) -> Self {
        self.mode = Some(mode);
        self
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.name.is_empty() {
            return Err("作用域名称不能为空".to_string());
        }
        if let Some(ttl) = self.custom_ttl_secs
            && ttl == 0
        {
            return Err("自定义TTL不能为0".to_string());
        }
        if let Some(size) = self.custom_max_size_bytes
            && size == 0
        {
            return Err("自定义最大大小不能为0".to_string());
        }
        Ok(())
    }
}

/// 缓存键配置（兼容层）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheKeyConfig {
    /// 键前缀
    pub prefix: String,
    /// 键后缀
    pub suffix: String,
    /// 键分隔符
    pub separator: String,
    /// 是否启用哈希
    pub enable_hash: bool,
    /// 哈希算法
    pub hash_algorithm: String,
}

impl Default for CacheKeyConfig {
    fn default() -> Self {
        Self {
            prefix: "".to_string(),
            suffix: "".to_string(),
            separator: ":".to_string(),
            enable_hash: false,
            hash_algorithm: "sha256".to_string(),
        }
    }
}

impl CacheKeyConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_prefix(mut self, prefix: String) -> Self {
        self.prefix = prefix;
        self
    }

    pub fn with_suffix(mut self, suffix: String) -> Self {
        self.suffix = suffix;
        self
    }

    pub fn with_separator(mut self, separator: String) -> Self {
        self.separator = separator;
        self
    }

    pub fn with_enable_hash(mut self, enable_hash: bool) -> Self {
        self.enable_hash = enable_hash;
        self
    }

    pub fn with_hash_algorithm(mut self, hash_algorithm: String) -> Self {
        self.hash_algorithm = hash_algorithm;
        self
    }

    pub fn build_key(&self, key: &str) -> String {
        let mut result = String::new();

        if !self.prefix.is_empty() {
            result.push_str(&self.prefix);
            result.push_str(&self.separator);
        }

        if self.enable_hash {
            let hash = format!("{:x}", key.bytes().map(|b| b as u32).sum::<u32>());
            result.push_str(&hash);
        } else {
            result.push_str(key);
        }

        if !self.suffix.is_empty() {
            result.push_str(&self.separator);
            result.push_str(&self.suffix);
        }

        result
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.separator.is_empty() {
            return Err("键分隔符不能为空".to_string());
        }
        if self.enable_hash && self.hash_algorithm.is_empty() {
            return Err("启用哈希时哈希算法不能为空".to_string());
        }
        Ok(())
    }
}

/// 延迟统计信息
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LatencyStats {
    /// 平均延迟（微秒）
    pub avg_latency_us: u64,
    /// 最小延迟（微秒）
    pub min_latency_us: u64,
    /// 最大延迟（微秒）
    pub max_latency_us: u64,
    /// 延迟百分位数（微秒）
    pub p50_latency_us: u64,
    pub p90_latency_us: u64,
    pub p95_latency_us: u64,
    pub p99_latency_us: u64,
}

/// 热键信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HotKeyInfo {
    /// 键名称
    pub key: String,
    /// 访问次数
    pub access_count: u64,
    /// 最后访问时间
    pub last_accessed: std::time::SystemTime,
}

impl HotKeyInfo {
    /// 创建新的热键信息
    pub fn new(key: String) -> Self {
        Self {
            key,
            access_count: 0,
            last_accessed: std::time::SystemTime::now(),
        }
    }
}

/// 缓存统计信息
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CacheStats {
    /// 总命中次数
    pub total_hits: u64,
    /// 总未命中次数
    pub total_misses: u64,
    /// 总插入次数
    pub total_inserts: u64,
    /// 总删除次数
    pub total_deletes: u64,
    /// 总更新次数
    pub total_updates: u64,
    /// 总驱逐次数
    pub total_evictions: u64,
    /// 当前缓存大小
    pub current_size: usize,
    /// 延迟统计
    pub latency_stats: LatencyStats,
    /// 热键列表
    pub hot_keys: Vec<HotKeyInfo>,
}

/// 缓存条目元数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntryMetadata {
    /// 键
    pub key: String,
    /// 创建时间
    pub created_at: std::time::SystemTime,
    /// 最后访问时间
    pub last_accessed: std::time::SystemTime,
    /// 访问次数
    pub access_count: u64,
    /// TTL（秒）
    pub ttl_secs: u64,
    /// 大小（字节）
    pub size_bytes: usize,
    /// 是否已过期
    pub is_expired: bool,
}

impl CacheEntryMetadata {
    /// 创建新的缓存条目元数据
    pub fn new(key: String, ttl_secs: u64, size_bytes: usize) -> Self {
        let now = std::time::SystemTime::now();
        Self {
            key,
            created_at: now,
            last_accessed: now,
            access_count: 0,
            ttl_secs,
            size_bytes,
            is_expired: false,
        }
    }

    /// 更新访问信息
    pub fn update_access(&mut self) {
        self.last_accessed = std::time::SystemTime::now();
        self.access_count += 1;
    }

    /// 检查是否过期
    pub fn check_expired(&mut self) -> bool {
        if self.ttl_secs == 0 {
            return false;
        }

        let now = std::time::SystemTime::now();
        match now.duration_since(self.created_at) {
            Ok(duration) => {
                self.is_expired = duration.as_secs() > self.ttl_secs;
                self.is_expired
            }
            Err(_) => {
                self.is_expired = false;
                false
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use seesea_config::CacheConfig;

    #[test]
    fn test_cache_config_default() {
        let config = CacheConfig::default();
        assert_eq!(config.ttl, 3600);
        assert_eq!(config.max_size, 1024 * 1024 * 1024);
    }

    #[test]
    fn test_cache_impl_config_builder() {
        let config = CacheImplConfig::new("./test_db".to_string())
            .with_ttl(7200)
            .with_max_size(2048 * 1024 * 1024)
            .with_enabled(false)
            .with_compression(true)
            .with_mode(CacheMode::HighPerformance)
            .with_bloom_filter(5000, 0.001);

        assert_eq!(config.db_path, "./test_db");
        assert_eq!(config.default_ttl_secs, 7200);
        assert_eq!(config.max_size_bytes, 2048 * 1024 * 1024);
        assert_eq!(config.enabled, false);
        assert_eq!(config.compression, true);
        assert_eq!(config.mode, CacheMode::HighPerformance);
        assert_eq!(config.enable_bloom_filter, true);
        assert_eq!(config.bloom_filter_expected_elements, 5000);
        assert_eq!(config.bloom_filter_false_positive_rate, 0.001);
    }

    #[test]
    fn test_cache_impl_config_validate() {
        let mut config = CacheImplConfig::default();
        assert!(config.validate().is_ok());

        config.default_ttl_secs = 0;
        assert!(config.validate().is_err());

        config.default_ttl_secs = 3600;
        config.max_size_bytes = 0;
        assert!(config.validate().is_err());

        config.max_size_bytes = 1024 * 1024;
        config.bloom_filter_false_positive_rate = 0.0;
        assert!(config.validate().is_err());

        config.bloom_filter_false_positive_rate = 1.0;
        assert!(config.validate().is_err());

        config.bloom_filter_false_positive_rate = 0.5;
        config.bloom_filter_expected_elements = 0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_cache_scope_config_builder() {
        let config = CacheScopeConfig::new("test_scope".to_string())
            .with_enabled(false)
            .with_custom_ttl(1800)
            .with_custom_max_size(512 * 1024 * 1024)
            .with_compression(true)
            .with_mode(CacheMode::LowMemory);

        assert_eq!(config.name, "test_scope");
        assert_eq!(config.enabled, false);
        assert_eq!(config.custom_ttl_secs, Some(1800));
        assert_eq!(config.custom_max_size_bytes, Some(512 * 1024 * 1024));
        assert_eq!(config.compression, true);
        assert_eq!(config.mode, Some(CacheMode::LowMemory));
    }

    #[test]
    fn test_cache_scope_config_validate() {
        let mut config = CacheScopeConfig::default();
        assert!(config.validate().is_ok());

        config.name = "".to_string();
        assert!(config.validate().is_err());

        config.name = "test".to_string();
        config.custom_ttl_secs = Some(0);
        assert!(config.validate().is_err());

        config.custom_ttl_secs = Some(3600);
        config.custom_max_size_bytes = Some(0);
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_cache_key_config_builder() {
        let config = CacheKeyConfig::new()
            .with_prefix("app".to_string())
            .with_suffix("v1".to_string())
            .with_separator("/".to_string())
            .with_enable_hash(true)
            .with_hash_algorithm("md5".to_string());

        assert_eq!(config.prefix, "app");
        assert_eq!(config.suffix, "v1");
        assert_eq!(config.separator, "/");
        assert_eq!(config.enable_hash, true);
        assert_eq!(config.hash_algorithm, "md5");
    }

    #[test]
    fn test_cache_key_config_build_key() {
        let config = CacheKeyConfig::new()
            .with_prefix("app".to_string())
            .with_suffix("v1".to_string());

        let key = config.build_key("user:123");
        assert_eq!(key, "app:user:123:v1");

        let config = CacheKeyConfig::new().with_enable_hash(true);

        let key = config.build_key("test");
        assert!(!key.is_empty());
    }

    #[test]
    fn test_cache_key_config_validate() {
        let mut config = CacheKeyConfig::default();
        assert!(config.validate().is_ok());

        config.separator = "".to_string();
        assert!(config.validate().is_err());

        config.separator = ":".to_string();
        config.enable_hash = true;
        config.hash_algorithm = "".to_string();
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_latency_stats_default() {
        let stats = LatencyStats::default();
        assert_eq!(stats.avg_latency_us, 0);
        assert_eq!(stats.min_latency_us, 0);
        assert_eq!(stats.max_latency_us, 0);
    }

    #[test]
    fn test_hot_key_info_new() {
        let info = HotKeyInfo::new("test_key".to_string());
        assert_eq!(info.key, "test_key");
        assert_eq!(info.access_count, 0);
    }

    #[test]
    fn test_cache_stats_default() {
        let stats = CacheStats::default();
        assert_eq!(stats.total_hits, 0);
        assert_eq!(stats.total_misses, 0);
        assert_eq!(stats.total_inserts, 0);
        assert_eq!(stats.total_deletes, 0);
        assert_eq!(stats.total_updates, 0);
        assert_eq!(stats.total_evictions, 0);
        assert_eq!(stats.current_size, 0);
    }

    #[test]
    fn test_cache_entry_metadata_new() {
        let metadata = CacheEntryMetadata::new("test_key".to_string(), 3600, 1024);
        assert_eq!(metadata.key, "test_key");
        assert_eq!(metadata.ttl_secs, 3600);
        assert_eq!(metadata.size_bytes, 1024);
        assert_eq!(metadata.access_count, 0);
        assert!(!metadata.is_expired);
    }

    #[test]
    fn test_cache_entry_metadata_update_access() {
        let mut metadata = CacheEntryMetadata::new("test_key".to_string(), 3600, 1024);
        metadata.update_access();
        assert_eq!(metadata.access_count, 1);
    }

    #[test]
    fn test_cache_entry_metadata_check_expired() {
        let mut metadata = CacheEntryMetadata::new("test_key".to_string(), 0, 1024);
        assert!(!metadata.check_expired());

        let mut metadata = CacheEntryMetadata::new("test_key".to_string(), 1, 1024);
        assert!(!metadata.check_expired());
    }
}
