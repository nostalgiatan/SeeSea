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

#![cfg_attr(test, allow(clippy::field_reassign_with_default))]

//! SeeSea - 看海看得远，看得广
//!
//! 一个基于 Rust 实现的隐私保护型元搜索引擎，专注于提供高性能、隐私优先的多模态搜索服务
//!
//! 核心特性
//!
//! - **隐私优先**：支持 Tor 网络、TLS 指纹混淆、DNS over HTTPS 等多层隐私保护
//! - **多模态搜索**：整合网页搜索、RSS 聚合、浏览器自动化三种数据获取方式
//! - **高性能架构**：基于 Rust 异步编程，支持高并发搜索请求
//! - **智能缓存**：语义级缓存系统，支持向量相似性匹配和智能去重
//! - **强大的向量存储**：基于 Qdrant 的高效向量存储，支持相似性搜索和元数据过滤
//! - **多引擎聚合**：支持 12+ 专业搜索引擎，覆盖通用、图片、视频、新闻等多种搜索场景
//! - **Python SDK**：强大的 Python 绑定，支持灵活的引擎扩展和集成
//!
//! ## 架构概览
//!
//! SeeSea 采用模块化设计，主要包含以下核心模块：
//!
//! - **config**：配置管理系统，支持多环境配置和动态更新
//! - **cache**：智能缓存系统，支持语义匹配和向量相似性搜索
//! - **derive**：核心数据结构和 trait 定义，包括搜索引擎、查询、结果等
//! - **net**：网络通信模块，支持隐私保护和多种 HTTP 客户端
//! - **search**：搜索核心逻辑，包括查询解析、结果聚合和排序
//! - **api**：REST API 接口，提供完整的搜索服务
//! - **rss**：RSS 聚合和订阅管理
//! - **errors**：统一的错误处理系统
//!
//! ## 快速开始
//!
//! ```rust,no_run
//! use seesea_core::search::{SearchEngine, SearchQuery};
//! use seesea_core::derive::{EngineType, SafeSearchLevel};
//! use std::collections::HashMap;
//!
//! // 创建查询
//! let query = SearchQuery {
//!     query: "Rust 编程".to_string(),
//!     engine_type: EngineType::General,
//!     language: Some("zh-CN".to_string()),
//!     region: None,
//!     page_size: 10,
//!     page: 1,
//!     safe_search: SafeSearchLevel::Moderate,
//!     time_range: None,
//!     params: HashMap::new(),
//! };
//!
//! // 执行搜索 (需要异步上下文和搜索引擎实例)
//! // let result = engine.search(&query).await;
//! ```

// Allow non-snake-case for crate name
#![allow(non_snake_case)]

/// 错误处理模块，定义统一的错误类型和处理机制
pub use seesea_errors as errors;

/// 配置管理模块，支持多环境配置和动态更新
pub use seesea_config as config;

/// 智能缓存模块，支持语义匹配和向量相似性搜索
pub use seesea_cache as cache;

/// 核心数据结构和 trait 定义模块 (外部 seesea-derive 库)
pub use seesea_derive as derive;

/// 网络通信模块，支持隐私保护和多种 HTTP 客户端
pub use seesea_net as net;

/// 搜索核心逻辑模块，包括查询解析、结果聚合和排序
pub use seesea_search as search;

/// REST API 接口模块，提供完整的搜索服务
pub use seesea_api as api;

/// RSS 聚合和订阅管理模块
pub use seesea_rss as rss;

/// 向量存储模块，基于 Qdrant 提供高效的向量存储和相似性搜索
pub use seesea_vector_store as vector_store;

/// 系统调控中心模块，负责统一管理系统资源和动态调整
pub use seesea_sys as sys;

/// 文本清洗模块，用于处理和优化搜索结果文本
pub use seesea_cleaner as cleaner;

/// 热点数据模块，用于获取和解析多平台热点数据
pub use seesea_hot as hot;

// 重新导出常用的网络配置类型
pub use seesea_api::api::network::{ExternalNetworkConfig, InternalNetworkConfig, NetworkMode};

// 重新导出服务器配置类型
pub use seesea_api::api::on::ServerConfig;

/// 统一的错误类型别名，用于简化错误处理
pub type Error = errors::ErrorInfo;

/// 统一的结果类型别名，用于简化错误处理
pub type Result<T> = errors::Result<T>;

// 自动初始化系统调控中心
use once_cell::sync::OnceCell;

#[allow(dead_code)]
static INIT_GUARD: OnceCell<()> = OnceCell::new();

/// 确保系统调控中心已初始化
#[allow(dead_code)]
fn ensure_init() {
    INIT_GUARD.get_or_init(|| {
        tracing::info!("SeeSea 库加载，自动初始化系统调控中心");
        // 检查当前是否已经存在 Tokio 运行时
        match tokio::runtime::Handle::try_current() {
            // 如果已经存在运行时，使用现有的运行时
            Ok(handle) => {
                handle.block_on(async {
                    let _controller = sys::controller::get_global_system_controller();
                });
            }
            // 如果不存在运行时，创建一个新的运行时
            Err(_) => {
                let _ = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .map(|rt| {
                        rt.block_on(async {
                            let _controller = sys::controller::get_global_system_controller();
                        });
                    });
            }
        }
    });
}

// 重新导出主要类型，方便外部使用
pub use cache::CacheInterface;
pub use config::{ConfigManager, SeeSeaConfig};
pub use derive::{
    EngineInfo, QueryBuilder, ResultParser, RssFeed, RssFeedItem, RssFeedQuery, RssFeedSource,
    SearchEngine, SearchQuery, SearchResult,
};
pub use seesea_cache::cache::types::{CacheImplConfig, CacheMode};
pub use seesea_config::config::ConfigError;

pub use seesea_net::{HttpClient, NetworkConfig, NetworkInterface};
pub use sys::controller::get_global_system_controller;
pub use vector_store::{Document, VectorStore, VectorStoreConfig, VectorStoreResult};
