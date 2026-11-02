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

//! # 配置管理模块
//!
//! 配置管理模块是 SeeSea 的核心组件之一，负责处理所有配置相关的功能。

pub mod cache;
pub mod common;
pub mod network;
pub mod paths;
pub mod search;
pub mod types;

pub use cache::{
    CacheBackend, CacheConfig, CacheKeyStrategy, CacheMonitoringConfig, CacheTypeConfig,
    CompressionAlgorithm, CompressionConfig, ConsistentHashConfig, DnsCacheConfig, EvictionPolicy,
    HashAlgorithm, MemoryConfig as CacheMemoryConfig, ReadFrom, RedisClusterConfig, RedisConfig,
    RedisNode, RedisTlsConfig, ResultCacheConfig, ShardingConfig, ShardingStrategy,
};
pub use common::{
    AuthType, ConfigValidationResult, EngineLoadingMode, FingerprintLevel, LogLevel, ProxyType,
    SafeSearchLevel,
};
pub use network::{
    DnsConfig, NetworkConfig, PoolConfig, ProxyConfig, RequestOptions, TlsConfig, TlsVersion,
};
pub use paths::{PlatformPaths, get_platform_paths, init_platform_paths};
pub use search::{EngineListConfig, EngineMode, SearchConfig};
pub use types::Environment;
