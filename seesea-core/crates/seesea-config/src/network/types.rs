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

//! 网络配置主类型

use crate::network::{DnsConfig, PoolConfig, ProxyConfig, TlsConfig};
use crate::privacy::PrivacyConfig;
use serde::{Deserialize, Serialize};

/// 网络配置主结构
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NetworkConfig {
    /// 代理配置
    pub proxy: ProxyConfig,
    /// TLS 配置
    pub tls: TlsConfig,
    /// DNS 配置
    pub dns: DnsConfig,
    /// 连接池配置
    pub pool: PoolConfig,
    /// 隐私配置
    pub privacy: PrivacyConfig,
}

impl NetworkConfig {
    /// 创建默认配置
    pub fn new() -> Self {
        Self::default()
    }
}

/// 请求选项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestOptions {
    /// 超时时间（秒）
    pub timeout_secs: Option<u64>,
    /// 最大重试次数
    pub max_retries: Option<u32>,
    /// 重试间隔（秒）
    pub retry_delay_secs: Option<u64>,
    /// 是否跟随重定向
    pub follow_redirects: Option<bool>,
    /// 最大重定向次数
    pub max_redirects: Option<u32>,
    /// 用户代理
    pub user_agent: Option<String>,
    /// 额外的请求头
    pub headers: Option<std::collections::HashMap<String, String>>,
    /// 是否验证证书
    pub verify_certificates: Option<bool>,
    /// 代理配置（覆盖全局配置）
    pub proxy: Option<ProxyConfig>,
}

impl Default for RequestOptions {
    fn default() -> Self {
        Self {
            timeout_secs: Some(30),
            max_retries: Some(3),
            retry_delay_secs: Some(1),
            follow_redirects: Some(true),
            max_redirects: Some(10),
            user_agent: None,
            headers: None,
            verify_certificates: None,
            proxy: None,
        }
    }
}
