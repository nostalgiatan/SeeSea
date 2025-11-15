//! API 处理器模块
//!
//! 包含各种 API 请求的处理逻辑

pub mod search;
pub mod health;
pub mod config;
pub mod metrics;

pub use search::*;
pub use health::*;
pub use config::*;
pub use metrics::*;
