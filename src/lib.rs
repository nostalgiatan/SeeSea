//! SeeSea - 看海看得远，看得广
//!
//! 一个基于 Rust 实现的隐私保护型元搜索引擎

pub mod config;

// 搜索引擎抽象骨架模块（可选特性）
#[cfg(feature = "derive")]
pub mod derive;

// 重新导出主要类型
pub use config::*;

// 重新导出 derive 模块类型（可选特性）
#[cfg(feature = "derive")]
pub use derive::*;

// 导入error模块供其他模块使用
pub mod search;

// 公开error_derive宏
#[cfg(feature = "derive")]
pub extern crate error_derive;
#[cfg(feature = "derive")]
pub use error_derive::Error;