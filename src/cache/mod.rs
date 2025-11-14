//! sled 缓存模块
//!
//! 提供基于 sled 的高性能缓存功能，包括：
//! - 搜索结果缓存
//! - 引擎元数据缓存
//! - 通用键值缓存
//!
//! # 特性
//!
//! - **高性能**：基于 sled 嵌入式数据库，提供毫秒级读写性能
//! - **持久化**：数据持久化到磁盘，重启不丢失
//! - **过期管理**：支持 TTL 过期时间和自动清理
//! - **统计信息**：提供命中率、大小等统计数据
//! - **类型安全**：使用强类型接口，避免运行时错误
//! - **零拷贝**：最小化内存分配，优化性能
//!
//! # 使用示例
//!
//! ```rust,no_run
//! use seesea::cache::{CacheInterface, CacheImplConfig, CacheMode};
//!
//! // 创建缓存接口
//! let config = CacheImplConfig {
//!     db_path: "./data/cache.db".to_string(),
//!     default_ttl_secs: 3600,
//!     max_size_bytes: 1024 * 1024 * 1024, // 1GB
//!     enabled: true,
//!     compression: false,
//!     mode: CacheMode::HighThroughput,
//! };
//!
//! let cache = CacheInterface::new(config)?;
//!
//! // 使用搜索结果缓存
//! let results_cache = cache.results();
//!
//! // 使用元数据缓存
//! let metadata_cache = cache.metadata();
//!
//! // 获取统计信息
//! let stats = cache.manager().stats();
//! println!("命中率: {:.2}%", stats.hit_rate() * 100.0);
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

pub mod types;
pub mod manager;
pub mod result;
pub mod metadata;
pub mod on;

// 重新导出主要类型
pub use types::{CacheImplConfig, CacheMode, CacheStats, CacheEntryMetadata};
pub use manager::{CacheManager, CacheError, Result};
pub use result::ResultCache;
pub use metadata::MetadataCache;
pub use on::CacheInterface;
