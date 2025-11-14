//! # 搜索模块
//!
//! 提供多引擎搜索编排、结果聚合、查询解析等核心功能。
//!
//! 本模块基于 derive 模块的抽象骨架，参考 Python searxng 项目的引擎处理方式，
//! 实现了解耦式的搜索功能。

pub mod aggregator;
pub mod engine_manager;
pub mod engines;
pub mod on;
pub mod orchestrator;
pub mod query;
pub mod types;

pub use aggregator::*;
pub use engine_manager::*;
pub use engines::EngineRegistry;
pub use on::*;
pub use orchestrator::*;
pub use query::*;
pub use types::*;
