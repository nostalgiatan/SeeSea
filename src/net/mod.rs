//! 网络层模块
//!
//! 提供隐私保护的网络层，支持：
//! - HTTP/HTTPS 客户端封装
//! - 代理支持（HTTP、SOCKS5、Tor）
//! - TLS 配置和指纹混淆
//! - DNS over HTTPS (DoH)
//! - 隐私保护特性（User-Agent 轮换、请求头伪造等）
//! - 连接池管理

pub mod types;
pub mod client;
pub mod privacy;
pub mod resolver;
pub mod on;

// 导出核心类型
pub use types::{
    NetworkConfig, ProxyConfig, ProxyType, TlsConfig, TlsFingerprintLevel,
    DohConfig, PrivacyConfig, UserAgentStrategy, PoolConfig, RequestOptions,
};

pub use on::NetworkInterface;
pub use client::HttpClient;
