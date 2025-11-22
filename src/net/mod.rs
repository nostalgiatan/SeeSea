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
