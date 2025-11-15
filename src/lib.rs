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
};
pub use net::{NetworkInterface, NetworkConfig, HttpClient};
pub mod search;
pub mod api;
