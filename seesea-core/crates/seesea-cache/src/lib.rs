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
//! 提供多种缓存实现，包括内存缓存、磁盘缓存、语义缓存等。

pub mod cache {
    pub mod bloom;
    pub mod manager;
    pub mod metadata;
    pub mod on;
    pub mod result;
    pub mod rss;
    pub mod scope;
    pub mod semantic;
    pub mod semantic_cache;
    pub mod types;
}

pub use cache::manager::CacheManager;
pub use cache::on::CacheInterface;
pub use cache::types::CacheStats;
pub use seesea_config::{
    CacheBackend, CacheConfig, CompressionAlgorithm, CompressionConfig, EvictionPolicy,
    ShardingConfig, ShardingStrategy,
};
