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

//! sled 缓存模块
//!
//! 提供基于 sled 的高性能缓存功能，包括：
//! - 搜索结果缓存
//! - 引擎元数据缓存
//! - RSS feed 缓存
//! - 语义相似度缓存
//! - 通用键值缓存
//!
//! # 特性
//!
//! - **高性能**：基于 sled 嵌入式数据库，提供毫秒级读写性能
//! - **持久化**：数据持久化到磁盘，重启不丢失
//! - **过期管理**：支持 TTL 过期时间和自动清理
//! - **语义搜索**：基于向量相似度的智能缓存命中
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
//!     db_path: ".seesea/cache.db".to_string(),
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
//! // 使用 RSS 缓存
//! let rss_cache = cache.rss();
//!
//! // 使用语义缓存
//! let semantic_cache = cache.semantic();
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
pub mod rss;
pub mod semantic;
pub mod semantic_cache;
pub mod on;

// 重新导出主要类型
pub use types::{CacheImplConfig, CacheMode, CacheStats, CacheEntryMetadata};
pub use manager::{CacheManager, CacheError, Result};
pub use result::ResultCache;
pub use metadata::MetadataCache;
pub use rss::RssCache;
pub use semantic::{SimpleVectorizer, QueryVector};
pub use semantic_cache::{SemanticCache, SemanticCacheConfig};
pub use on::CacheInterface;
