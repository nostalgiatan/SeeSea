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

//! # 缓存模块
//!
//! 缓存模块是 SeeSea 的核心组件之一，提供基于 RocksDB 嵌入式数据库的高性能缓存系统，支持多种缓存类型和智能语义匹配。
//!
//! ## 模块架构
//!
//! 缓存模块采用分层设计，主要包含以下核心组件：
//!
//! - **types**：核心类型定义，包括缓存配置、模式和统计信息
//! - **manager**：缓存管理器，负责缓存的生命周期管理和统计
//! - **result**：搜索结果缓存，存储和管理搜索结果
//! - **metadata**：元数据缓存，存储搜索引擎的元数据信息
//! - **rss**：RSS 缓存，存储和管理 RSS 订阅内容
//! - **semantic**：语义缓存，提供基于向量相似度的智能缓存匹配
//! - **semantic_cache**：语义缓存的具体实现
//! - **bloom**：布隆过滤器，用于高效的存在性检查
//! - **scope**：作用域缓存，支持基于作用域的缓存管理
//! - **on**：主要接口定义，提供统一的缓存访问接口
//!
//! ## 缓存类型
//!
//! SeeSea 支持多种类型的缓存，满足不同场景的需求：
//!
//! - **搜索结果缓存**：存储搜索引擎返回的结果，支持按查询和引擎分类
//! - **引擎元数据缓存**：存储搜索引擎的元数据，如支持的功能、速率限制等
//! - **RSS 订阅缓存**：存储 RSS 订阅源的内容，支持定时更新
//! - **语义相似度缓存**：基于向量相似度的智能缓存，支持语义级别的缓存命中
//! - **作用域缓存**：支持基于不同作用域的缓存管理，如用户、会话等
//!
//! ## 核心特性
//!
//! - **高性能**：基于 RocksDB 嵌入式数据库，提供毫秒级读写性能
//! - **持久化**：数据持久化到磁盘，重启不丢失
//! - **过期管理**：支持 TTL 过期时间和自动清理机制
//! - **语义搜索**：基于向量相似度的智能缓存命中，提高缓存利用率
//! - **统计信息**：提供详细的缓存统计数据，包括命中率、大小、访问次数等
//! - **类型安全**：使用强类型接口，避免运行时错误
//! - **零拷贝**：最小化内存分配，优化性能
//! - **可配置**：支持多种缓存模式和配置选项
//! - **线程安全**：支持多线程并发访问
//! - **多进程支持**：支持多进程并发访问，解决锁冲突问题
//!
//! ## 缓存模式
//!
//! - **HighThroughput**：高吞吐量模式，优化读写性能
//! - **LowMemory**：低内存模式，优化内存使用
//! - **Balanced**：平衡模式，兼顾性能和内存使用
//!
//! ## 使用示例
//!
//! ```rust,no_run
//! use seesea::seesea_seesea_cache::{CacheInterface, CacheImplConfig, CacheMode};
//!
//! // 创建缓存配置
//! let config = CacheImplConfig {
//!     db_path: ".seesea/cache.db".to_string(),
//!     default_ttl_secs: 3600, // 默认过期时间为 1 小时
//!     max_size_bytes: 1024 * 1024 * 1024, // 最大缓存大小为 1GB
//!     enabled: true,
//!     compression: false,
//!     mode: CacheMode::HighThroughput, // 高吞吐量模式
//! };
//!
//! // 创建缓存接口实例
//! let cache = CacheInterface::new(config)?;
//!
//! // 使用不同类型的缓存
//! let results_cache = cache.results(); // 搜索结果缓存
//! let metadata_cache = cache.metadata(); // 元数据缓存
//! let rss_cache = cache.rss(); // RSS 缓存
//! let semantic_cache = cache.semantic(); // 语义缓存
//! let scope_cache = cache.scope(); // 作用域缓存
//!
//! // 获取缓存统计信息
//! let stats = cache.manager().stats();
//! println!("缓存命中率: {:.2}%", stats.hit_rate() * 100.0);
//! println!("缓存大小: {:.2} MB", stats.total_size_bytes() as f64 / 1024.0 / 1024.0);
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

/// 布隆过滤器模块，用于高效的存在性检查
pub mod bloom;

/// 核心类型定义模块，包括缓存配置、模式和统计信息
pub mod types;

/// 缓存管理器模块，负责缓存的生命周期管理和统计
pub mod manager;

/// 搜索结果缓存模块，存储和管理搜索结果
pub mod result;

/// 元数据缓存模块，存储搜索引擎的元数据信息
pub mod metadata;

/// RSS 缓存模块，存储和管理 RSS 订阅内容
pub mod rss;

/// 语义缓存模块，提供基于向量相似度的智能缓存匹配
pub mod semantic;

/// 语义缓存的具体实现模块
pub mod semantic_cache;

/// 主要接口定义模块，提供统一的缓存访问接口
pub mod on;

/// 作用域缓存模块，支持基于作用域的缓存管理
pub mod scope;

// 重新导出主要类型，方便外部使用

/// 缓存配置和模式类型
pub use types::{
    CacheEntryMetadata, // 缓存条目元数据
    CacheImplConfig,    // 缓存实现配置
    CacheMode,          // 缓存模式（高吞吐量、低内存、平衡）
    CacheStats,         // 缓存统计信息
};

/// 缓存管理器和错误类型
pub use manager::{
    CacheError,   // 缓存错误类型
    CacheManager, // 缓存管理器
    Result,       // 缓存结果类型
};

/// 搜索结果缓存
pub use result::ResultCache;

/// 引擎元数据缓存
pub use metadata::MetadataCache;

/// RSS 订阅缓存
pub use rss::RssCache;

/// 语义缓存相关类型
pub use semantic::{
    QueryVector,      // 查询向量类型
    SimpleVectorizer, // 简单向量器，用于生成查询向量
};

/// 语义缓存实现
pub use semantic_cache::{
    SemanticCache,       // 语义缓存
    SemanticCacheConfig, // 语义缓存配置
};

/// 主要缓存接口
pub use on::CacheInterface;

/// 作用域缓存
pub use scope::ScopeCache;
