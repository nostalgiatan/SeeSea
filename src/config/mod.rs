//! SeeSea 配置管理模块
//!
//! 提供完整的配置管理功能

// 通用类型定义
pub mod common;

// 子模块
pub mod general;
pub mod server;
pub mod search;
pub mod privacy;
pub mod cache;
pub mod api;
pub mod logging;
pub mod engines;

// 核心类型定义
pub mod types;

// 主配置类型
pub mod config;

// 公共接口
pub mod on;
pub mod loader;
pub mod validator;

// 重新导出关键公共类型
pub use common::{
    ConfigValidationResult, BaseEngineConfig,
    EngineLoadingMode, LogLevel, LogFormat, LogOutput,
    EngineType as CommonEngineType,
    AuthType, FingerprintLevel,
};
pub use server::ServerConfig;
pub use search::SearchConfig;
pub use privacy::PrivacyConfig;
pub use cache::CacheConfig;
pub use api::ApiConfig;
pub use logging::LoggingConfig;
pub use engines::EnginesConfig;
pub use types::Environment;
pub use config::{SeeSeaConfig, ConfigLoadResult, ConfigSummary, ConfigError, ConfigSource};
pub use on::{ConfigManager, get_global_config, init_config, init_config_with_env};
pub use loader::ConfigLoader;
pub use validator::{ConfigValidator, validate_config};