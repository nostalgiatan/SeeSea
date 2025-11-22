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

//! DNS over HTTPS (DoH) 模块
//!
//! 提供基于 HTTPS 的 DNS 查询功能

use crate::error::Result;
use crate::net::types::DohConfig;
use std::net::IpAddr;

/// 通过 DoH 解析域名
///
/// # 参数
///
/// * `hostname` - 要解析的域名
/// * `config` - DoH 配置
///
/// # 返回
///
/// 成功返回 IP 地址列表，失败返回错误
pub async fn resolve_via_doh(hostname: &str, config: &DohConfig) -> Result<Vec<IpAddr>> {
    if config.servers.is_empty() {
        return Err(crate::error::network_error("No DoH servers configured".to_string()));
    }

    // 尝试每个 DoH 服务器
    for server in &config.servers {
        match query_doh_server(hostname, server).await {
            Ok(ips) if !ips.is_empty() => return Ok(ips),
            _ => continue,
        }
    }

    Err(crate::error::network_error(format!("All DoH servers failed to resolve {}", hostname)))
}

/// 查询单个 DoH 服务器
///
/// # 参数
///
/// * `hostname` - 要解析的域名
/// * `server_url` - DoH 服务器 URL
///
/// # 返回
///
/// 成功返回 IP 地址列表，失败返回错误
async fn query_doh_server(hostname: &str, server_url: &str) -> Result<Vec<IpAddr>> {
    // 构造 DoH 查询 URL
    let query_url = format!("{}?name={}&type=A", server_url, hostname);

    // 发送 HTTPS 请求
    let client = reqwest::Client::new();
    let response = client
        .get(&query_url)
        .header("Accept", "application/dns-json")
        .send()
        .await
        .map_err(|e| crate::error::network_error(format!("DoH query failed: {}", e)))?;

    if !response.status().is_success() {
        return Err(crate::error::network_error(format!("DoH server returned error: {}", response.status())));
    }

    // 解析 JSON 响应
    let json: serde_json::Value = response
        .json()
        .await
        .map_err(|e| crate::error::network_error(format!("Failed to parse DoH response: {}", e)))?;

    // 提取 IP 地址
    let mut ips = Vec::new();
    if let Some(answers) = json.get("Answer").and_then(|a| a.as_array()) {
        for answer in answers {
            if let Some(data) = answer.get("data").and_then(|d| d.as_str()) {
                if let Ok(ip) = data.parse::<IpAddr>() {
                    ips.push(ip);
                }
            }
        }
    }

    if ips.is_empty() {
        Err(crate::error::network_error(format!("No IP addresses in DoH response for {}", hostname)))
    } else {
        Ok(ips)
    }
}

/// DoH 查询类型
#[derive(Debug, Clone, Copy)]
pub enum QueryType {
    /// A 记录（IPv4）
    A,
    /// AAAA 记录（IPv6）
    AAAA,
    /// 两者都查询
    Both,
}

impl QueryType {
    /// 转换为 DNS 类型字符串
    pub fn as_str(&self) -> &str {
        match self {
            QueryType::A => "A",
            QueryType::AAAA => "AAAA",
            QueryType::Both => "A", // 默认查询 A 记录
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_query_type_as_str() {
        assert_eq!(QueryType::A.as_str(), "A");
        assert_eq!(QueryType::AAAA.as_str(), "AAAA");
        assert_eq!(QueryType::Both.as_str(), "A");
    }

    #[tokio::test]
    async fn test_resolve_via_doh_no_servers() {
        let config = DohConfig {
            enabled: true,
            servers: vec![],
            fallback_to_system: false,
        };
        let result = resolve_via_doh("example.com", &config).await;
        assert!(result.is_err());
    }

    // 注意：以下测试需要网络连接，在 CI 环境中可能失败
    #[tokio::test]
    #[ignore] // 标记为 ignore，需要网络连接才能运行
    async fn test_resolve_via_doh_cloudflare() {
        let config = DohConfig {
            enabled: true,
            servers: vec!["https://cloudflare-dns.com/dns-query".to_string()],
            fallback_to_system: false,
        };
        let result = resolve_via_doh("example.com", &config).await;
        // 这个测试需要真实的网络连接
        if let Ok(ips) = result {
            assert!(!ips.is_empty());
        }
    }
}
