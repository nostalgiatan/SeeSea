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

//! # 网络模块
//!
//! 网络模块是 SeeSea 的核心组件之一，提供隐私优先的网络通信功能，支持多种隐私保护机制和灵活的配置选项。
//!
//! ## 模块架构
//!
//! 网络模块采用分层设计，主要包含以下核心组件：
//!
//! - **client**：HTTP/HTTPS 客户端实现，支持多种隐私保护特性
//! - **privacy**：隐私保护机制，包括 TLS 指纹混淆、请求头伪造等
//! - **resolver**：DNS 解析器，支持 DNS over HTTPS (DoH)
//! - **config**：网络配置，包括代理、超时、连接池等选项
//! - **interface**：网络接口定义，提供统一的网络访问接口
//! - **retry**：请求重试机制，提高网络请求的可靠性
//! - **metrics**：网络指标监控，提供详细的网络性能统计
//!
//! ## 核心功能
//!
//! ### HTTP/HTTPS 客户端
//! - 支持 HTTP/1.1 和 HTTP/2
//! - 连接池管理，优化性能
//! - 超时控制和请求取消
//! - 支持多种认证方式
//!
//! ### 隐私保护机制
//! - **TLS 指纹混淆**：防止基于 TLS 指纹的设备识别
//! - **User-Agent 轮换**：自动轮换 User-Agent，避免指纹追踪
//! - **请求头伪造**：生成随机化的请求头，增强隐私保护
//! - **代理支持**：支持 HTTP、SOCKS5 和 Tor 代理
//! - **DNS over HTTPS (DoH)**：加密 DNS 查询，防止 DNS 泄露
//! - **请求时序随机化**：随机化请求间隔，避免流量分析
//! - **Cookie 隔离**：为不同请求隔离 Cookie，防止跨站追踪
//!
//! ### 代理支持
//! - HTTP 代理
//! - SOCKS5 代理
//! - Tor 网络集成
//! - 代理链支持
//! - 动态代理切换
//!
//! ### 可靠性机制
//! - 智能重试策略
//! - 故障转移支持
//! - 连接健康检查
//! - 自动恢复机制
//!
//! ## 使用示例
//!
//! ```rust,no_run
//! use seesea::seesea_seesea_net::{HttpClient, NetworkConfig, PrivacyConfig};
//!
//! // 创建隐私配置
//! let privacy_config = PrivacyConfig {
//!     tls_fingerprint_obfuscation: true,
//!     user_agent_rotation: true,
//!     doh_providers: vec!["cloudflare", "google"],
//!     ..Default::default()
//! };
//!
//! // 创建网络配置
//! let network_config = NetworkConfig {
//!     timeout_secs: 30,
//!     connection_pool_size: 50,
//!     privacy: privacy_config,
//!     ..Default::default()
//! };
//!
//! // 创建HTTP客户端
//! let client = HttpClient::new(network_config)?;
//!
//! // 发送GET请求
//! let response = client.get("https://example.com", None).await?;
//!
//! // 发送POST请求
//! let post_data = serde_json::json!({"key": "value"});
//! let response = client.post("https://example.com/api", Some(post_data)).await?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

/// HTTP/HTTPS 客户端模块，支持多种隐私保护特性
pub mod client;

/// 隐私保护机制模块，包括 TLS 指纹混淆、请求头伪造等
pub mod privacy;

/// DNS 解析器模块，支持 DNS over HTTPS (DoH)
pub mod resolver;

/// 网络接口定义模块，提供统一的网络访问接口
pub mod interface;

/// 请求重试机制模块，提高网络请求的可靠性
pub mod retry;

/// 网络指标监控模块，提供详细的网络性能统计
pub mod metrics;

// 导出核心配置类型
pub use seesea_config::*;

// 导出网络接口
pub use interface::NetworkInterface;

// 导出HTTP客户端
pub use client::HttpClient;
