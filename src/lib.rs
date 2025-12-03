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

//! SeeSea - 看海看得远，看得广
//!
//! 一个基于 Rust 实现的隐私保护型元搜索引擎，专注于提供高性能、隐私优先的多模态搜索服务
//!
//! ## 核心特性
//!
//! - **隐私优先**：支持 Tor 网络、TLS 指纹混淆、DNS over HTTPS 等多层隐私保护
//! - **多模态搜索**：整合网页搜索、RSS 聚合、浏览器自动化三种数据获取方式
//! - **高性能架构**：基于 Rust 异步编程，支持高并发搜索请求
//! - **智能缓存**：语义级缓存系统，支持向量相似性匹配和智能去重
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

/// Python 绑定模块，提供 Python SDK 支持
#[cfg(feature = "python")]
pub mod python_bindings;

/// 统一的错误类型别名，用于简化错误处理
pub type Error = errors::ErrorInfo;

/// 统一的结果类型别名，用于简化错误处理
pub type Result<T> = errors::Result<T>;

// 重新导出主要类型，方便外部使用
pub use cache::{CacheImplConfig, CacheInterface, CacheMode};
pub use config::{ConfigError, ConfigManager, SeeSeaConfig};
pub use derive::{
    EngineInfo, QueryBuilder, ResultParser, RssFeed, RssFeedItem, RssFeedQuery, RssFeedSource,
    SearchEngine, SearchQuery, SearchResult,
};
pub use html_parser::{HtmlPageType, HtmlParser};
pub use net::{HttpClient, NetworkConfig, NetworkInterface};

// Python module definition
#[cfg(feature = "python")]
use pyo3::prelude::*;

/// Python 模块定义，用于生成 Python SDK
#[cfg(feature = "python")]
#[pymodule]
fn seesea_core(_py: Python, m: &Bound<'_, PyModule>) -> PyResult<()> {
    use python_bindings::{
        py_api, py_browser, py_cache, py_config, py_engine_registry, py_html_parser, py_net,
        py_rss, py_search,
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

    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    m.add(
        "__doc__",
        "SeeSea - Privacy-focused metasearch engine with RSS and browser engine support",
    )?;

    Ok(())
}
