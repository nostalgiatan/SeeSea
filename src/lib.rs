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
//! ```rust
//! use seesea::{SearchEngine, SearchQuery, QueryBuilder};
//!
//! // 创建查询
//! let query = QueryBuilder::new("Rust 编程")
//!     .engine("bing")
//!     .page(1)
//!     .build();
//!
//! // 执行搜索
//! let result = engine.search(&query).await;
//! ```

// Allow non-snake-case for crate name
#![allow(non_snake_case)]

/// 错误处理模块，定义统一的错误类型和处理机制
pub mod errors;

/// 配置管理模块，支持多环境配置和动态更新
pub mod config;

/// 智能缓存模块，支持语义匹配和向量相似性搜索
pub mod cache;

/// 核心数据结构和 trait 定义模块
pub mod derive;

/// 网络通信模块，支持隐私保护和多种 HTTP 客户端
pub mod net;

/// 搜索核心逻辑模块，包括查询解析、结果聚合和排序
pub mod search;

/// REST API 接口模块，提供完整的搜索服务
pub mod api;

/// RSS 聚合和订阅管理模块
pub mod rss;

/// HTML解析模块，用于判定网页类型（SPA或HTML）
pub mod html_parser;

/// 向量存储模块，基于 Qdrant 提供高效的向量存储和相似性搜索
pub mod vector_store;

/// 系统调控中心模块，负责统一管理系统资源和动态调整
pub mod sys;

/// 文本清洗模块，用于处理和优化搜索结果文本
pub mod cleaner;

/// 热点数据模块，用于获取和解析多平台热点数据
pub mod hot;

/// Python 绑定模块，提供 Python SDK 支持
#[cfg(feature = "python")]
pub mod python_bindings;

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
pub use cache::{CacheImplConfig, CacheInterface, CacheMode};
pub use config::{ConfigError, ConfigManager, SeeSeaConfig};
pub use derive::{
    EngineInfo, QueryBuilder, ResultParser, RssFeed, RssFeedItem, RssFeedQuery, RssFeedSource,
    SearchEngine, SearchQuery, SearchResult,
};
pub use html_parser::{HtmlPageType, HtmlParser};
pub use net::{HttpClient, NetworkConfig, NetworkInterface};
pub use sys::controller::get_global_system_controller;
pub use vector_store::{
    Document, VectorStore, VectorStoreConfig, VectorStoreResult, create_vector_store,
};

// Python module definition
#[cfg(feature = "python")]
use pyo3::prelude::*;

/// Python 模块定义，用于生成 Python SDK
#[cfg(feature = "python")]
#[pymodule]
fn seesea_core(_py: Python, m: &Bound<'_, PyModule>) -> PyResult<()> {
    use python_bindings::{
        py_api, py_browser, py_cache, py_cleaner, py_config, py_date_page, py_engine_registry,
        py_hot, py_html_parser, py_net, py_object_pool, py_rss, py_search,
    };

    m.add_class::<py_search::PySearchClient>()?;
    m.add_class::<py_api::PyApiServer>()?;
    m.add_class::<py_config::PyConfig>()?;
    m.add_class::<py_cache::PyCacheStats>()?;
    m.add_class::<py_cache::PyCacheInterface>()?;
    m.add_class::<py_rss::PyRssClient>()?;
    m.add_class::<py_browser::PyBrowserConfig>()?;
    m.add_class::<py_browser::PyBrowserEngineClient>()?;
    m.add_class::<py_net::PyNetClient>()?;
    m.add_class::<py_cleaner::PyCleaner>()?;
    m.add_class::<py_cleaner::PyDataBlock>()?;
    m.add_class::<py_date_page::PyDatePage>()?;
    m.add_class::<py_date_page::PyMapItem>()?;
    m.add_class::<py_date_page::PyExtraInfoItem>()?;
    m.add_class::<py_object_pool::PyDatePageObjectPool>()?;
    m.add_class::<py_object_pool::PyDatePageObjectPoolStats>()?;
    m.add_class::<py_hot::PyHotTrendClient>()?;

    // 引擎注册表函数（不再暴露类，只暴露函数）
    m.add_function(wrap_pyfunction!(py_engine_registry::register_engine, m)?)?;
    m.add_function(wrap_pyfunction!(py_engine_registry::unregister_engine, m)?)?;
    m.add_function(wrap_pyfunction!(py_engine_registry::list_engines, m)?)?;
    m.add_function(wrap_pyfunction!(py_engine_registry::has_engine, m)?)?;

    // 网络客户端函数
    m.add_function(wrap_pyfunction!(py_net::get, m)?)?;
    m.add_function(wrap_pyfunction!(py_net::post, m)?)?;
    m.add_function(wrap_pyfunction!(py_net::get_file, m)?)?;
    m.add_function(wrap_pyfunction!(py_net::post_file, m)?)?;

    // HTML解析器函数
    m.add_function(wrap_pyfunction!(py_html_parser::determine_page_type, m)?)?;
    m.add_function(wrap_pyfunction!(py_html_parser::get_html_meta_info, m)?)?;

    // 配置初始化函数
    m.add_function(wrap_pyfunction!(py_config::init_config, m)?)?;

    // 系统控制器函数
    m.add_function(wrap_pyfunction!(
        python_bindings::py_system_controller::init_system_controller,
        m
    )?)?;
    m.add_function(wrap_pyfunction!(
        python_bindings::py_system_controller::register_component,
        m
    )?)?;
    m.add_function(wrap_pyfunction!(
        python_bindings::py_system_controller::adjust_component_concurrency,
        m
    )?)?;
    m.add_function(wrap_pyfunction!(
        python_bindings::py_system_controller::adjust_component_priority,
        m
    )?)?;
    m.add_function(wrap_pyfunction!(
        python_bindings::py_system_controller::get_system_status,
        m
    )?)?;
    m.add_function(wrap_pyfunction!(
        python_bindings::py_system_controller::set_resource_threshold,
        m
    )?)?;
    m.add_function(wrap_pyfunction!(
        python_bindings::py_system_controller::adjust_crawl4ai_concurrency,
        m
    )?)?;
    m.add_function(wrap_pyfunction!(
        python_bindings::py_system_controller::adjust_pro_processor_concurrency,
        m
    )?)?;

    // 向量数据库绑定
    m.add_class::<python_bindings::py_vector_store::PyVectorClient>()?;

    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    m.add(
        "__doc__",
        "SeeSea - Privacy-focused metasearch engine with RSS and browser engine support",
    )?;

    Ok(())
}
