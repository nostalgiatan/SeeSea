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

//! # API 模块
//!
//! API 模块是 SeeSea 的核心组件之一，提供完整的 RESTful API 接口，用于公开搜索引擎的功能，便于外部系统集成和扩展。
//!
//! ## 模块架构
//!
//! API 模块采用分层设计，主要包含以下核心组件：
//!
//! - **types**：核心类型定义，包括 API 请求、响应和配置等
//! - **on**：主要接口定义，提供统一的 API 服务访问接口
//! - **handlers**：请求处理器，负责处理各种 API 请求
//! - **middleware**：中间件，包括认证、日志、CORS 等功能
//! - **metrics**：指标监控，提供详细的 API 性能统计
//! - **network**：网络相关功能，包括服务器配置和启动等
//!
//! ## 核心功能
//!
//! ### RESTful API 接口
//! - 搜索接口：支持多引擎搜索、图片搜索、视频搜索等
//! - RSS 接口：支持 RSS 订阅管理、内容获取和解析
//! - 缓存接口：支持缓存管理、统计和清理
//! - 配置接口：支持动态配置更新和查询
//! - 健康检查接口：支持服务健康状态监控
//! - 指标接口：支持 Prometheus 指标导出
//!
//! ### 中间件支持
//! - **认证中间件**：支持多种认证方式（API Key、JWT 等）
//! - **日志中间件**：详细的请求日志记录
//! - **CORS 中间件**：支持跨域资源共享配置
//! - **速率限制中间件**：防止 API 滥用
//! - **错误处理中间件**：统一的错误响应格式
//!
//! ### 性能与可靠性
//! - 异步处理：基于 Tokio 和 Axum 实现高并发处理
//! - 连接池：优化数据库和网络连接
//! - 超时控制：防止请求长时间阻塞
//! - 优雅关闭：支持服务的优雅启动和关闭
//! - 指标监控：详细的性能指标和请求统计
//!
//! ## API 设计原则
//!
//! - **RESTful 设计**：遵循 REST 架构风格
//! - **统一响应格式**：所有 API 返回统一的响应结构
//! - **详细的错误信息**：提供清晰的错误码和错误描述
//! - **版本控制**：支持 API 版本管理
//! - **文档化**：提供完整的 API 文档
//! - **安全性**：内置多种安全机制
//!
//! ## 使用示例
//!
//! ```rust,no_run
//! use seesea::seesea_seesea_api::{ApiServer, ApiConfig};
//!
//! // 创建 API 配置
//! let api_config = ApiConfig {
//!     host: "0.0.0.0".to_string(),
//!     port: 8080,
//!     enable_cors: true,
//!     enable_metrics: true,
//!     ..Default::default()
//! };
//!
//! // 创建并启动 API 服务器
//! let server = ApiServer::new(api_config)?;
//! server.start().await?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

/// 核心类型定义模块，包括 API 请求、响应和配置等
pub mod types;

/// 主要接口定义模块，提供统一的 API 服务访问接口
pub mod on;

/// 请求处理器模块，负责处理各种 API 请求
pub mod handlers;

/// 中间件模块，包括认证、日志、CORS 等功能
pub mod middleware;

/// 指标监控模块，提供详细的 API 性能统计
pub mod metrics;

/// 网络相关功能模块，包括服务器配置和启动等
pub mod network;

/// 动态路由模块，实现基于前缀树的高效动态路由匹配机制
pub mod dynamic_router;

/// OpenAPI 文档模块
pub mod openapi;

/// Swagger UI 路由模块
pub mod swagger;

/// 内部管理 API 类型定义模块
pub mod internal_types;

/// 运行时配置管理模块
pub mod runtime_config;

// 导出核心类型和功能，方便外部使用

/// API 核心类型，包括请求、响应和配置等
pub use types::*;

/// 内部管理 API 类型
pub use internal_types::*;

/// 运行时配置
pub use runtime_config::{RuntimeConfig, RuntimeConfigManager};

/// API 主要接口，包括 API 服务器和相关功能
pub use on::*;

/// API 指标监控功能
pub use metrics::*;

/// API 网络相关功能，包括服务器配置和启动
pub use network::*;

/// API 动态路由功能，支持高效路由匹配
pub use dynamic_router::*;
