//! 搜索引擎抽象骨架模块
//!
//! 提供搜索引擎的基础 trait 定义和核心类型。
//! 
//! 本模块定义了搜索引擎开发的抽象框架，包括：
//! - 核心数据类型（SearchQuery, SearchResult, EngineInfo 等）
//! - 搜索引擎 trait 体系（SearchEngine, BaseEngine, ConfigurableEngine 等）
//! - 结果和查询处理的抽象接口
//! - 便利开发宏
//!
//! ## 设计原则
//!
//! - **抽象优先**: 使用关联类型和泛型避免具体实现依赖
//! - **模块分离**: HTTP 客户端在 net/client，缓存在 cache/ 模块
//! - **可扩展**: trait 支持灵活的功能组合

pub mod types;
pub mod engine;
pub mod result;
pub mod query;
pub mod macros;

// 重新导出主要类型
pub use types::*;
pub use engine::*;
pub use result::*;
pub use query::*;