//! 搜索引擎抽象骨架模块
//!
//! 提供搜索引擎的基础 trait 定义和核心类型。
//! 
//! 本模块基于 Python searxng 引擎接口设计，提供了一个完整、丰富的搜索引擎抽象层。
//! 使用内部 error 模块进行错误处理，确保零依赖和最佳性能。

pub mod error;
pub mod types;
pub mod engine;
pub mod result;
pub mod query;
pub mod client;
pub mod rate_limit;
pub mod macros;

// 重新导出主要类型
pub use error::{DeriveError, Result};
pub use types::*;
pub use engine::*;
pub use result::*;
pub use query::*;
pub use client::{HttpClient, ClientBuilder, ClientConfig};
pub use rate_limit::{RateLimiter, RateLimiterConfig};