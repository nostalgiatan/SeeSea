//! 搜索引擎抽象骨架模块
//!
//! 提供搜索引擎的基础 trait 定义和核心类型。
//! 
//! 本模块是一个抽象骨架，只定义接口和类型，不包含具体实现。
//! 基于 Python searxng 引擎接口设计。

pub mod error;
pub mod types;
pub mod engine;
pub mod result;
pub mod query;
pub mod macros;

// 重新导出主要类型
pub use error::{DeriveError, Result};
pub use types::*;
pub use engine::*;
pub use result::*;
pub use query::*;