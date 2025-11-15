//! # API 模块
//!
//! 提供高层次的外部 HTTP API 接口，用于公开搜索引擎的功能。
//! 所有 API 都是经过高度封装的，便于外部集成。

pub mod types;
pub mod on;
pub mod handlers;
pub mod middleware;

pub use types::*;
pub use on::*;
