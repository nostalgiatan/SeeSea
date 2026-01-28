// Copyright (C) 2025 nostalgiatan
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

//! # 配置管理模块
//!
//! 配置管理模块是 SeeSea 的核心组件之一，负责处理所有配置相关的功能，包括配置加载、验证、合并和动态更新。
//!
//! ## 模块架构
//!
//! 配置管理模块采用分层设计，主要包含以下核心组件：
//!
//! - **common**：通用类型定义，包括配置验证结果、引擎基础配置等
//! - **general**：通用配置，包括应用名称、版本、环境等
//! - **server**：服务器配置，包括监听地址、端口、TLS 配置等
//! - **search**：搜索配置，包括默认引擎、超时时间、并发数等
//! - **privacy**：隐私配置，包括 Tor 网络、指纹保护、用户代理轮换等
//! - **cache**：缓存配置，包括缓存类型、TTL、大小限制等
//! - **api**：API 配置，包括认证方式、速率限制、CORS 配置等
//! - **logging**：日志配置，包括日志级别、格式、输出方式等
//! - **engines**：引擎配置，包括各搜索引擎的具体配置
//! - **types**：核心类型定义，包括环境类型等
//! - **config**：主配置类型，整合所有子配置
//! - **on**：公共接口，包括配置管理器和全局配置访问
//! - **loader**：配置加载器，负责从各种源加载配置
//! - **validator**：配置验证器，负责验证配置的有效性
//!
//! ## 配置加载流程
//!
//! 1. 从默认配置文件加载基础配置
//! 2. 从环境变量加载覆盖配置
//! 3. 从命令行参数加载最终覆盖配置
//! 4. 验证配置的有效性
//! 5. 初始化全局配置实例
//! 6. 提供配置访问接口
//!
//! ## 配置源优先级
//!
//! 配置按照以下优先级从高到低加载：
//! 1. 命令行参数
//! 2. 环境变量
//! 3. 本地配置文件（local.toml）
//! 4. 环境配置文件（production.toml, development.toml 等）
//! 5. 默认配置
//!
//! ## 使用示例
//!
//! ```rust
//! use seesea::seesea_seesea_config::{init_config, get_global_config};
//!
//! // 初始化配置
//! init_config(None).await?;
//!
//! // 获取全局配置
//! let config = get_global_config();
//!
//! // 使用配置
//! let search_config = &config.search;
//! println!("默认搜索引擎: {:?}", search_config.default_engines);
//! ```

/// 通用类型定义模块，包括配置验证结果、引擎基础配置等
pub mod common;

/// 通用配置模块，包括应用名称、版本、环境等
pub mod general;

/// 服务器配置模块，包括监听地址、端口、TLS 配置等
pub mod server;

/// 搜索配置模块，包括默认引擎、超时时间、并发数等
pub mod search;

/// 隐私配置模块，包括 Tor 网络、指纹保护、用户代理轮换等
pub mod privacy;

/// 缓存配置模块，包括缓存类型、TTL、大小限制等
pub mod cache;

/// API 配置模块，包括认证方式、速率限制、CORS 配置等
pub mod api;

/// 日志配置模块，包括日志级别、格式、输出方式等
pub mod logging;

/// 引擎配置模块，包括各搜索引擎的具体配置
pub mod engines;

/// 向量数据库配置模块，包括向量数据库的配置
pub mod vector_store;

/// 平台路径配置模块
pub mod paths;

/// RSS 配置模块，包括 RSS Feed 配置
pub mod rss;

/// 核心类型定义模块，包括环境类型等
pub mod types;

/// 主配置类型模块，整合所有子配置
#[allow(clippy::module_inception)]
pub mod config;

/// 公共接口模块，包括配置管理器和全局配置访问
pub mod on;

/// 配置加载器模块，负责从各种源加载配置
pub mod loader;

/// 配置验证器模块，负责验证配置的有效性
pub mod validator;

// 重新导出关键公共类型，方便外部使用

/// 配置验证结果和通用配置类型
pub use common::{
    AuthType,                       // 认证类型
    BaseEngineConfig,               // 引擎基础配置
    ConfigValidationResult,         // 配置验证结果类型
    EngineLoadingMode,              // 引擎加载模式
    EngineType as CommonEngineType, // 引擎类型
    FingerprintLevel,               // 指纹保护级别
    LogFormat,                      // 日志格式
    LogLevel,                       // 日志级别
    LogOutput,                      // 日志输出方式
};

/// 服务器配置，包括监听地址、端口、TLS 配置等
pub use server::ServerConfig;

/// 搜索配置，包括默认引擎、超时时间、并发数等
pub use crate::SearchConfig;

/// 隐私配置，包括 Tor 网络、指纹保护、用户代理轮换等
pub use privacy::PrivacyConfig;

/// 缓存配置，包括缓存类型、TTL、大小限制等
pub use crate::CacheConfig;

/// API 配置，包括认证方式、速率限制、CORS 配置等
pub use crate::ApiConfig;

/// 日志配置，包括日志级别、格式、输出方式等
pub use logging::LoggingConfig;

/// 引擎配置，包括各搜索引擎的具体配置
pub use engines::EnginesConfig;

/// RSS 配置，包括 RSS Feed 配置
pub use rss::RssConfig;

/// 环境类型，包括开发、测试、生产等
pub use types::Environment;

/// 向量数据库配置
pub use crate::{DynamicAdjustmentConfig, VectorStoreConfig, VectorStoreStatsConfig};

/// 平台路径配置
pub use paths::{PlatformPaths, get_platform_paths, init_platform_paths};

/// 主配置类型和相关结果类型
pub use crate::{
    ConfigError,      // 配置错误类型
    ConfigLoadResult, // 配置加载结果
    ConfigSource,     // 配置源类型
    ConfigSummary,    // 配置摘要
    SeeSeaConfig,     // 主配置类型，整合所有子配置
};

/// 配置管理器和全局配置访问接口
pub use on::{
    ConfigManager,        // 配置管理器，负责配置的加载、更新和访问
    get_global_config,    // 获取全局配置实例
    init_config,          // 初始化配置
    init_config_with_env, // 使用指定环境初始化配置
};

/// 配置加载器，负责从各种源加载配置
pub use loader::ConfigLoader;

/// 配置验证器和验证函数
pub use validator::{
    ConfigValidator, // 配置验证器 trait
    validate_config, // 验证配置的函数
};
