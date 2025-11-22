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
//! 一个基于 Rust 实现的隐私保护型元搜索引擎

// Allow non-snake-case for crate name
#![allow(non_snake_case)]

// 重新导出 error crate
pub use error as error_crate;

pub mod error;
pub mod config;
pub mod cache;
pub mod derive;
pub mod net;

// 创建便利的 Error 和 Result 类型别名
pub type Error = error_crate::ErrorInfo;
pub type Result<T> = error_crate::Result<T>;

// 重新导出主要类型
pub use config::{SeeSeaConfig, ConfigManager, ConfigError};
pub use cache::{CacheInterface, CacheImplConfig, CacheMode};
pub use derive::{
    SearchEngine, SearchQuery, SearchResult, EngineInfo,
    QueryBuilder, ResultParser,
    RssFeed, RssFeedItem, RssFeedQuery, RssFeedSource,
};
pub use net::{NetworkInterface, NetworkConfig, HttpClient};
pub mod search;
pub mod api;
pub mod rss;

#[cfg(feature = "python")]
pub mod python_bindings;

// Python module definition
#[cfg(feature = "python")]
use pyo3::prelude::*;

#[cfg(feature = "python")]
#[pymodule]
fn seesea_core(_py: Python, m: &Bound<'_, PyModule>) -> PyResult<()> {
    use python_bindings::{py_search, py_api, py_config, py_cache, py_rss, py_browser, py_engine_registry};

    m.add_class::<py_search::PySearchClient>()?;
    m.add_class::<py_api::PyApiServer>()?;
    m.add_class::<py_config::PyConfig>()?;
    m.add_class::<py_cache::PyCacheStats>()?;
    m.add_class::<py_cache::PyCacheInterface>()?;
    m.add_class::<py_rss::PyRssClient>()?;
    m.add_class::<py_browser::PyBrowserConfig>()?;
    m.add_class::<py_browser::PyBrowserEngineClient>()?;
    
    // 引擎注册表函数（不再暴露类，只暴露函数）
    m.add_function(wrap_pyfunction!(py_engine_registry::register_engine, m)?)?;
    m.add_function(wrap_pyfunction!(py_engine_registry::unregister_engine, m)?)?;
    m.add_function(wrap_pyfunction!(py_engine_registry::list_engines, m)?)?;
    m.add_function(wrap_pyfunction!(py_engine_registry::has_engine, m)?)?;

    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    m.add("__doc__", "SeeSea - Privacy-focused metasearch engine with RSS and browser engine support")?;

    Ok(())
}
