// Copyright 2025 nostalgiatan
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! 网络层类型定义模块
//!
//! 本模块定义了网络层所需的核心类型，包括：
//! - 代理配置
//! - TLS 配置
//! - DNS 配置
//! - 隐私设置
//! - 请求选项

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// 代理类型枚举
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProxyType {
    /// HTTP 代理
    Http,
    /// HTTPS 代理
    Https,
    /// SOCKS5 代理
    Socks5,
    /// Tor 代理
    Tor,
}

/// 代理配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyConfig {
    /// 代理类型
    pub proxy_type: ProxyType,
    /// 代理地址（例如: "127.0.0.1:9050"）
    pub address: String,
    /// 认证用户名（可选）
    pub username: Option<String>,
    /// 认证密码（可选）
    pub password: Option<String>,
    /// 是否启用
    pub enabled: bool,
}

impl Default for ProxyConfig {
    fn default() -> Self {
        Self {
            proxy_type: ProxyType::Http,
            address: String::from("127.0.0.1:8080"),
            username: None,
            password: None,
            enabled: false,
        }
    }
}

/// TLS 指纹混淆级别
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TlsFingerprintLevel {
    /// 无混淆
    None,
    /// 基础混淆
    Basic,
    /// 高级混淆
    Advanced,
    /// 完全随机化
    Full,
}

/// TLS 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TlsConfig {
    /// 是否验证证书
    pub verify_certificates: bool,
    /// 是否使用 SNI
    pub use_sni: bool,
    /// 指纹混淆级别
    pub fingerprint_level: TlsFingerprintLevel,
    /// 支持的 TLS 版本（最小版本）
    pub min_version: String,
    /// 自定义证书路径（可选）
    pub custom_cert_path: Option<String>,
}

impl Default for TlsConfig {
    fn default() -> Self {
        Self {
            verify_certificates: true,
            use_sni: true,
            fingerprint_level: TlsFingerprintLevel::Basic,
            min_version: String::from("1.2"),
            custom_cert_path: None,
        }
    }
}

/// DNS over HTTPS (DoH) 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DohConfig {
    /// 是否启用 DoH
    pub enabled: bool,
    /// DoH 服务器列表
    pub servers: Vec<String>,
    /// 是否使用系统 DNS 作为后备
    pub fallback_to_system: bool,
}

impl Default for DohConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            servers: vec![
                String::from("https://cloudflare-dns.com/dns-query"),
                String::from("https://dns.google/dns-query"),
            ],
            fallback_to_system: true,
        }
    }
}

/// User-Agent 轮换策略
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UserAgentStrategy {
    /// 固定 User-Agent
    Fixed,
    /// 随机轮换
    Random,
    /// 模拟真实浏览器
    Realistic,
    /// 自定义
    Custom,
}

/// 隐私保护配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrivacyConfig {
    /// User-Agent 轮换策略
    pub user_agent_strategy: UserAgentStrategy,
    /// 自定义 User-Agent（当策略为 Custom 时使用）
    pub custom_user_agent: Option<String>,
    /// 是否伪造请求头
    pub fake_headers: bool,
    /// 是否启用 Referer 伪造
    pub fake_referer: bool,
    /// 是否移除指纹特征
    pub remove_fingerprints: bool,
}

impl Default for PrivacyConfig {
    fn default() -> Self {
        Self {
            user_agent_strategy: UserAgentStrategy::Realistic,
            custom_user_agent: None,
            fake_headers: true,
            fake_referer: true,
            remove_fingerprints: true,
        }
    }
}

/// 连接池配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolConfig {
    /// 最大空闲连接数
    pub max_idle_connections: usize,
    /// 每个主机的最大连接数
    pub max_connections_per_host: usize,
    /// 空闲连接超时时间（秒）
    pub idle_timeout_secs: u64,
    /// 是否启用 HTTP/2
    pub http2_only: bool,
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self {
            max_idle_connections: 500,        // 增加到500
            max_connections_per_host: 50,     // 增加到50
            idle_timeout_secs: 300,           // 增加到5分钟
            http2_only: false,
        }
    }
}

/// HTTP 请求选项
#[derive(Debug, Clone)]
pub struct RequestOptions {
    /// 请求超时时间
    pub timeout: Duration,
    /// 连接超时时间
    pub connect_timeout: Duration,
    /// 是否跟随重定向
    pub follow_redirects: bool,
    /// 最大重定向次数
    pub max_redirects: usize,
    /// 是否启用压缩
    pub compression: bool,
    /// 自定义请求头
    pub headers: Vec<(String, String)>,
}

impl Default for RequestOptions {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(30),
            connect_timeout: Duration::from_secs(10),
            follow_redirects: true,
            max_redirects: 10,
            compression: true,
            headers: Vec::new(),
        }
    }
}

/// 网络层配置（总配置）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    /// 代理配置
    pub proxy: ProxyConfig,
    /// TLS 配置
    pub tls: TlsConfig,
    /// DNS 配置
    pub doh: DohConfig,
    /// 隐私配置
    pub privacy: PrivacyConfig,
    /// 连接池配置
    pub pool: PoolConfig,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            proxy: ProxyConfig::default(),
            tls: TlsConfig::default(),
            doh: DohConfig::default(),
            privacy: PrivacyConfig::default(),
            pool: PoolConfig::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_proxy_config_default() {
        let config = ProxyConfig::default();
        assert!(!config.enabled);
        assert_eq!(config.proxy_type, ProxyType::Http);
    }

    #[test]
    fn test_tls_config_default() {
        let config = TlsConfig::default();
        assert!(config.verify_certificates);
        assert_eq!(config.fingerprint_level, TlsFingerprintLevel::Basic);
    }

    #[test]
    fn test_privacy_config_default() {
        let config = PrivacyConfig::default();
        assert_eq!(config.user_agent_strategy, UserAgentStrategy::Realistic);
        assert!(config.fake_headers);
    }

    #[test]
    fn test_network_config_default() {
        let config = NetworkConfig::default();
        assert!(!config.proxy.enabled);
        assert!(config.tls.verify_certificates);
    }

    #[test]
    fn test_request_options_default() {
        let opts = RequestOptions::default();
        assert_eq!(opts.timeout, Duration::from_secs(30));
        assert!(opts.follow_redirects);
        assert!(opts.compression);
    }
}
