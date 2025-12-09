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

//! 网络层配置类型定义模块
//!
//! 本模块定义了网络层所需的核心配置类型，包括：
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

/// DNS 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnsConfig {
    /// 是否启用 DNS over HTTPS (DoH)
    pub doh_enabled: bool,
    /// DoH 服务器列表
    pub doh_servers: Vec<String>,
    /// 是否使用系统 DNS 作为后备
    pub fallback_to_system: bool,
}

impl Default for DnsConfig {
    fn default() -> Self {
        Self {
            doh_enabled: false,
            doh_servers: vec![
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
    /// 连接超时时间（秒）
    pub connect_timeout_secs: u64,
    /// 读取超时时间（秒）
    pub read_timeout_secs: u64,
    /// 写入超时时间（秒）
    pub write_timeout_secs: u64,
    /// 是否启用 HTTP/2
    pub http2_only: bool,
    /// 是否启用 TCP_NODELAY
    pub tcp_nodelay: bool,
    /// TCP 保活间隔（秒）
    pub tcp_keepalive_interval_secs: Option<u64>,
    /// TCP 保活重试次数
    pub tcp_keepalive_retries: Option<u32>,
    /// 健康检查间隔（秒）
    pub health_check_interval_secs: Option<u64>,
    /// 连接最大生命周期（秒）
    pub max_lifetime_secs: Option<u64>,
    /// 连接获取超时时间（秒）
    pub connection_acquisition_timeout_secs: u64,
    /// 是否启用连接泄漏检测
    pub enable_connection_leak_detection: bool,
    /// 连接泄漏检测超时时间（秒）
    pub leak_detection_timeout_secs: u64,
    /// 是否启用动态调整连接池大小
    pub dynamic_resizing_enabled: bool,
    /// 每个主机的最小连接数
    pub min_connections_per_host: usize,
    /// 退避因子
    pub backoff_factor: f64,
    /// 最大重试次数
    pub max_retries: usize,
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self {
            max_idle_connections: 500,    // 增加到500
            max_connections_per_host: 50, // 增加到50
            idle_timeout_secs: 300,       // 增加到5分钟
            connect_timeout_secs: 10,     // 10秒
            read_timeout_secs: 30,        // 30秒
            write_timeout_secs: 30,       // 30秒
            http2_only: false,
            tcp_nodelay: true,                       // 启用 TCP_NODELAY
            tcp_keepalive_interval_secs: None,       // 使用系统默认
            tcp_keepalive_retries: None,             // 使用系统默认
            health_check_interval_secs: None,        // 默认关闭健康检查
            max_lifetime_secs: None,                 // 无最大生命周期限制
            connection_acquisition_timeout_secs: 10, // 10秒
            enable_connection_leak_detection: false, // 默认关闭泄漏检测
            leak_detection_timeout_secs: 300,        // 5分钟
            dynamic_resizing_enabled: false,         // 默认关闭动态调整
            min_connections_per_host: 1,             // 每个主机至少1个连接
            backoff_factor: 1.5,                     // 退避因子
            max_retries: 3,                          // 最大重试次数
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
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NetworkConfig {
    /// 代理配置
    pub proxy: ProxyConfig,
    /// TLS 配置
    pub tls: TlsConfig,
    /// DNS 配置
    pub dns: DnsConfig,
    /// 隐私配置
    pub privacy: PrivacyConfig,
    /// 连接池配置
    pub pool: PoolConfig,
}

impl NetworkConfig {
    /// 从项目级配置创建网络层配置
    ///
    /// # 参数
    ///
    /// * `config` - 项目级配置
    ///
    /// # 返回
    ///
    /// 网络层配置
    pub fn from_project_config(config: &crate::config::SeeSeaConfig) -> Self {
        // 转换代理配置
        let proxy_config = if let Some(proxy) = config.privacy.proxy_chain.first() {
            ProxyConfig {
                proxy_type: match proxy.proxy_type {
                    crate::config::common::ProxyType::Http => ProxyType::Http,
                    crate::config::common::ProxyType::Https => ProxyType::Https,
                    crate::config::common::ProxyType::Socks5 => ProxyType::Socks5,
                    crate::config::common::ProxyType::Socks4 => ProxyType::Socks5, // Socks4 映射为 Socks5
                },
                address: format!("{}:{}", proxy.address, proxy.port),
                username: proxy.username.clone(),
                password: proxy.password.clone(),
                enabled: proxy.enabled,
            }
        } else {
            ProxyConfig::default()
        };

        // 转换 TLS 配置
        let tls_config = TlsConfig {
            verify_certificates: true, // 项目级配置中没有直接对应，使用默认值
            use_sni: true,             // 项目级配置中没有直接对应，使用默认值
            fingerprint_level: match config.privacy.fingerprint_protection.protection_level {
                crate::config::common::FingerprintLevel::None => TlsFingerprintLevel::None,
                crate::config::common::FingerprintLevel::Basic => TlsFingerprintLevel::Basic,
                crate::config::common::FingerprintLevel::Advanced => TlsFingerprintLevel::Advanced,
                crate::config::common::FingerprintLevel::Maximum => TlsFingerprintLevel::Full,
            },
            min_version: String::from("1.2"), // 项目级配置中没有直接对应，使用默认值
            custom_cert_path: None,           // 项目级配置中没有直接对应，使用默认值
        };

        // 转换 DNS 配置
        let dns_config = DnsConfig {
            doh_enabled: config.privacy.dns_config.enabled,
            doh_servers: config
                .privacy
                .dns_config
                .servers
                .iter()
                .filter(|server| server.enabled)
                .map(|server| server.url.clone())
                .collect(),
            fallback_to_system: true, // 项目级配置中没有直接对应，使用默认值
        };

        // 转换隐私配置
        let privacy_config = PrivacyConfig {
            user_agent_strategy: match config.privacy.user_agent_rotation.rotation_strategy {
                crate::config::privacy::UaRotationStrategy::Random => UserAgentStrategy::Random,
                _ => UserAgentStrategy::Realistic, // 其他策略映射为 Realistic
            },
            custom_user_agent: None,   // 项目级配置中没有直接对应，使用默认值
            fake_headers: true,        // 项目级配置中没有直接对应，使用默认值
            fake_referer: true,        // 项目级配置中没有直接对应，使用默认值
            remove_fingerprints: true, // 项目级配置中没有直接对应，使用默认值
        };

        // 转换连接池配置
        let pool_config = PoolConfig {
            max_idle_connections: 500,         // 项目级配置中没有直接对应，使用默认值
            max_connections_per_host: 50,      // 项目级配置中没有直接对应，使用默认值
            idle_timeout_secs: 300,            // 项目级配置中没有直接对应，使用默认值
            connect_timeout_secs: 10,          // 项目级配置中没有直接对应，使用默认值
            read_timeout_secs: 30,             // 项目级配置中没有直接对应，使用默认值
            write_timeout_secs: 30,            // 项目级配置中没有直接对应，使用默认值
            http2_only: false,                 // 项目级配置中没有直接对应，使用默认值
            tcp_nodelay: true,                 // 项目级配置中没有直接对应，使用默认值
            tcp_keepalive_interval_secs: None, // 项目级配置中没有直接对应，使用默认值
            tcp_keepalive_retries: None,       // 项目级配置中没有直接对应，使用默认值
            // 使用默认值初始化新增字段
            health_check_interval_secs: None,
            max_lifetime_secs: None,
            connection_acquisition_timeout_secs: 10,
            enable_connection_leak_detection: false,
            leak_detection_timeout_secs: 300,
            dynamic_resizing_enabled: false,
            min_connections_per_host: 1,
            backoff_factor: 1.5,
            max_retries: 3,
        };

        Self {
            proxy: proxy_config,
            tls: tls_config,
            dns: dns_config,
            privacy: privacy_config,
            pool: pool_config,
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
