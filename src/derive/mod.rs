//! 搜索引擎抽象骨架模块
//!
//! 提供搜索引擎的基础 trait 定义和核心类型

pub mod types;
pub mod engine;
pub mod result;
pub mod query;

// 重新导出主要类型
pub use types::*;
pub use engine::*;
pub use result::*;
pub use query::*;