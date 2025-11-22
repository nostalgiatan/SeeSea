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

//! 缓存配置类型定义

use crate::config::common::ConfigValidationResult;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// 缓存配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    /// 缓存后端类型
    pub backend: CacheBackend,
    /// 数据库路径（仅文件后端）
    pub database_path: PathBuf,
    /// 缓存过期时间（秒）
    pub ttl: u64,
    /// 最大缓存大小（字节）
    pub max_size: u64,
    /// 是否启用结果缓存
    pub enable_result_cache: bool,
    /// 是否启用元数据缓存
    pub enable_metadata_cache: bool,
    /// 是否启用 DNS 缓存
    pub enable_dns_cache: bool,
    /// 缓存刷新间隔（秒）
    pub refresh_interval: u64,
    /// 缓存策略
    pub eviction_policy: EvictionPolicy,
    /// 压缩配置
    pub compression: CompressionConfig,
    /// 分片配置
    pub sharding: ShardingConfig,
    /// 监控配置
    pub monitoring: CacheMonitoringConfig,
}

/// 缓存后端类型
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CacheBackend {
    /// Sled 嵌入式数据库
    Sled,
    /// Redis 缓存
    Redis,
    /// 内存缓存
    Memory,
    /// 混合缓存（内存 + 磁盘）
    Hybrid,
    /// 自定义后端
    Custom,
}

/// 淘汰策略
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvictionPolicy {
    /// 最近最少使用
    Lru,
    /// 最近最不常使用
    Lfu,
    /// 先进先出
    Fifo,
    /// 随机淘汰
    Random,
    /// 基于 TTL
    Ttl,
    /// 混合策略
    Hybrid,
}

/// 压缩配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressionConfig {
    /// 是否启用压缩
    pub enabled: bool,
    /// 压缩算法
    pub algorithm: CompressionAlgorithm,
    /// 压缩阈值（字节）
    pub threshold: usize,
    /// 压缩级别（1-9）
    pub level: u32,
}

/// 压缩算法
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CompressionAlgorithm {
    /// 不压缩
    None,
    /// LZ4 快速压缩
    Lz4,
    /// Zlib 标准压缩
    Zlib,
    /// ZSTD 高性能压缩
    Zstd,
    /// Snappy 平衡压缩
    Snappy,
}

/// 分片配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShardingConfig {
    /// 是否启用分片
    pub enabled: bool,
    /// 分片数量
    pub shard_count: usize,
    /// 分片策略
    pub strategy: ShardingStrategy,
    /// 一致性哈希配置
    pub consistent_hash: Option<ConsistentHashConfig>,
}

/// 分片策略
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ShardingStrategy {
    /// 基于哈希
    Hash,
    /// 一致性哈希
    ConsistentHash,
    /// 范围分片
    Range,
    /// 轮询分片
    RoundRobin,
}

/// 一致性哈希配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsistentHashConfig {
    /// 虚拟节点数量
    pub virtual_nodes: usize,
    /// 哈希算法
    pub hash_algorithm: HashAlgorithm,
}

/// 哈希算法
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HashAlgorithm {
    /// MurmurHash3
    Murmur3,
    /// CRC32
    Crc32,
    /// FNV-1a
    Fnv1a,
    /// xxHash
    XxHash,
}

/// 缓存监控配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheMonitoringConfig {
    /// 是否启用监控
    pub enabled: bool,
    /// 指标收集间隔（秒）
    pub metrics_interval: u64,
    /// 是否启用慢查询日志
    pub enable_slow_query_log: bool,
    /// 慢查询阈值（毫秒）
    pub slow_query_threshold: u64,
    /// 是否启用性能分析
    pub enable_profiling: bool,
}

/// Redis 后端配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedisConfig {
    /// Redis 服务器地址
    pub host: String,
    /// Redis 端口
    pub port: u16,
    /// 数据库编号
    pub database: u8,
    /// 密码（可选）
    pub password: Option<String>,
    /// 连接池大小
    pub pool_size: usize,
    /// 连接超时时间（秒）
    pub timeout: u64,
    /// 是否启用 TLS
    pub use_tls: bool,
    /// TLS 配置
    pub tls_config: Option<RedisTlsConfig>,
    /// 集群配置
    pub cluster_config: Option<RedisClusterConfig>,
}

/// Redis TLS 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedisTlsConfig {
    /// CA 证书路径
    pub ca_cert_path: Option<PathBuf>,
    /// 客户端证书路径
    pub client_cert_path: Option<PathBuf>,
    /// 客户端私钥路径
    pub client_key_path: Option<PathBuf>,
    /// 是否验证主机名
    pub verify_hostname: bool,
}

/// Redis 集群配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedisClusterConfig {
    /// 集群节点列表
    pub nodes: Vec<RedisNode>,
    /// 读取策略
    pub read_from: ReadFrom,
    /// 最大重定向次数
    pub max_redirects: usize,
}

/// Redis 节点配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedisNode {
    /// 节点地址
    pub host: String,
    /// 节点端口
    pub port: u16,
    /// 节点权重
    pub weight: f32,
}

/// 读取策略
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReadFrom {
    /// 从主节点读取
    Master,
    /// 从从节点读取
    Slave,
    /// 优先从主节点读取
    PreferredMaster,
    /// 优先从从节点读取
    PreferredSlave,
}

/// 内存缓存配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryConfig {
    /// 最大条目数
    pub max_entries: usize,
    /// 分段数量（用于并发）
    pub segments: usize,
    /// 是否启用统计
    pub enable_stats: bool,
    /// 并发级别
    pub concurrency_level: usize,
}

/// 缓存类型特定配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheTypeConfig {
    /// 结果缓存配置
    pub result_cache: ResultCacheConfig,
    /// 元数据缓存配置
    pub metadata_cache: MetadataCacheConfig,
    /// DNS 缓存配置
    pub dns_cache: DnsCacheConfig,
}

/// 结果缓存配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResultCacheConfig {
    /// 自定义 TTL（覆盖全局设置）
    pub ttl: Option<u64>,
    /// 缓存键策略
    pub key_strategy: CacheKeyStrategy,
    /// 是否缓存空结果
    pub cache_empty_results: bool,
    /// 最大结果大小（字节）
    pub max_result_size: usize,
}

/// 元数据缓存配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetadataCacheConfig {
    /// 自定义 TTL
    pub ttl: Option<u64>,
    /// 缓存引擎状态
    pub cache_engine_status: bool,
    /// 缓存引擎配置
    pub cache_engine_config: bool,
    /// 缓存统计信息
    pub cache_statistics: bool,
}

/// DNS 缓存配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnsCacheConfig {
    /// 自定义 TTL
    pub ttl: Option<u64>,
    /// 缓存成功解析
    pub cache_success: bool,
    /// 缓存失败解析
    pub cache_failures: bool,
    /// 失败缓存 TTL
    pub failure_ttl: Option<u64>,
}

/// 缓存键策略
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CacheKeyStrategy {
    /// 基于查询和引擎
    QueryAndEngine,
    /// 基于查询哈希
    QueryHash,
    /// 基于完整请求
    FullRequest,
    /// 自定义策略
    Custom,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            backend: CacheBackend::Sled,
            database_path: PathBuf::from("./cache/seesea.db"),
            ttl: 3600, // 1 hour
            max_size: 1024 * 1024 * 1024, // 1GB
            enable_result_cache: true,
            enable_metadata_cache: true,
            enable_dns_cache: true,
            refresh_interval: 300, // 5 minutes
            eviction_policy: EvictionPolicy::Ttl,
            compression: CompressionConfig::default(),
            sharding: ShardingConfig::default(),
            monitoring: CacheMonitoringConfig::default(),
        }
    }
}

impl CacheConfig {
    /// 验证缓存配置
    pub fn validate(&self) -> ConfigValidationResult {
        let mut result = ConfigValidationResult::success();

        // 检查缓存大小
        if self.max_size == 0 {
            result.add_error("最大缓存大小必须大于 0".to_string());
        }

        // 检查 TTL
        if self.ttl == 0 {
            result.add_error("缓存 TTL 必须大于 0".to_string());
        }

        // 检查刷新间隔
        if self.refresh_interval == 0 {
            result.add_error("缓存刷新间隔必须大于 0".to_string());
        }

        if self.refresh_interval >= self.ttl {
            result.add_warning("缓存刷新间隔大于等于 TTL，缓存可能永远不会过期".to_string());
        }

        // 检查压缩配置
        if let Some(compression) = Some(&self.compression) {
            if compression.enabled && compression.threshold == 0 {
                result.add_error("压缩阈值必须大于 0".to_string());
            }

            if compression.level == 0 || compression.level > 9 {
                result.add_error("压缩级别必须在 1-9 之间".to_string());
            }
        }

        // 检查分片配置
        if let Some(sharding) = Some(&self.sharding) {
            if sharding.enabled && sharding.shard_count == 0 {
                result.add_error("分片数量必须大于 0".to_string());
            }

            if sharding.shard_count > 1000 {
                result.add_warning("分片数量过多可能影响性能".to_string());
            }
        }

        result
    }

    /// 获取缓存类型配置
    pub fn get_type_config(&self) -> CacheTypeConfig {
        CacheTypeConfig {
            result_cache: ResultCacheConfig::default(),
            metadata_cache: MetadataCacheConfig::default(),
            dns_cache: DnsCacheConfig::default(),
        }
    }

    /// 检查是否为分布式后端
    pub fn is_distributed(&self) -> bool {
        matches!(self.backend, CacheBackend::Redis)
    }

    /// 检查是否为内存后端
    pub fn is_memory_backend(&self) -> bool {
        matches!(self.backend, CacheBackend::Memory)
    }
}

impl Default for CompressionConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            algorithm: CompressionAlgorithm::Lz4,
            threshold: 1024, // 1KB
            level: 3,
        }
    }
}

impl Default for ShardingConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            shard_count: 16,
            strategy: ShardingStrategy::Hash,
            consistent_hash: None,
        }
    }
}

impl Default for CacheMonitoringConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            metrics_interval: 60, // 1 minute
            enable_slow_query_log: true,
            slow_query_threshold: 1000, // 1 second
            enable_profiling: false,
        }
    }
}

impl Default for ResultCacheConfig {
    fn default() -> Self {
        Self {
            ttl: None,
            key_strategy: CacheKeyStrategy::QueryAndEngine,
            cache_empty_results: false,
            max_result_size: 1024 * 1024, // 1MB
        }
    }
}

impl Default for MetadataCacheConfig {
    fn default() -> Self {
        Self {
            ttl: None,
            cache_engine_status: true,
            cache_engine_config: true,
            cache_statistics: true,
        }
    }
}

impl Default for DnsCacheConfig {
    fn default() -> Self {
        Self {
            ttl: None,
            cache_success: true,
            cache_failures: true,
            failure_ttl: Some(300), // 5 minutes
        }
    }
}

impl Default for RedisConfig {
    fn default() -> Self {
        Self {
            host: "localhost".to_string(),
            port: 6379,
            database: 0,
            password: None,
            pool_size: 10,
            timeout: 30,
            use_tls: false,
            tls_config: None,
            cluster_config: None,
        }
    }
}

impl Default for MemoryConfig {
    fn default() -> Self {
        Self {
            max_entries: 10000,
            segments: 16,
            enable_stats: true,
            concurrency_level: 4,
        }
    }
}