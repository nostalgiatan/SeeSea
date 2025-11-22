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

//! DNS 解析模块
//!
//! 提供 DNS 解析、DNS over HTTPS (DoH) 支持

pub mod doh;
pub mod pool;

use crate::error::Result;
use crate::net::types::DohConfig;
use std::net::IpAddr;

/// DNS 解析器
pub struct DnsResolver {
    /// DoH 配置
    config: DohConfig,
}

impl DnsResolver {
    /// 创建新的 DNS 解析器
    ///
    /// # 参数
    ///
    /// * `config` - DoH 配置
    pub fn new(config: DohConfig) -> Self {
        Self { config }
    }

    /// 解析域名到 IP 地址
    ///
    /// # 参数
    ///
    /// * `hostname` - 要解析的域名
    ///
    /// # 返回
    ///
    /// 成功返回 IP 地址列表，失败返回错误
    pub async fn resolve(&self, hostname: &str) -> Result<Vec<IpAddr>> {
        if self.config.enabled {
            // 使用 DoH
            match doh::resolve_via_doh(hostname, &self.config).await {
                Ok(ips) => Ok(ips),
                Err(_e) if self.config.fallback_to_system => {
                    // 回退到系统 DNS
                    self.resolve_system(hostname).await
                }
                Err(e) => Err(e),
            }
        } else {
            // 使用系统 DNS
            self.resolve_system(hostname).await
        }
    }

    /// 使用系统 DNS 解析
    async fn resolve_system(&self, hostname: &str) -> Result<Vec<IpAddr>> {
        use tokio::net::lookup_host;

        let addrs: Vec<IpAddr> = lookup_host(format!("{}:0", hostname))
            .await
            .map_err(|e| crate::error::network_error(format!("System DNS resolution failed: {}", e)))?
            .map(|addr| addr.ip())
            .collect();

        if addrs.is_empty() {
            return Err(crate::error::network_error(format!("No IP addresses found for {}", hostname)));
        }

        Ok(addrs)
    }

    /// 清除 DNS 缓存
    pub fn clear_cache(&self) {
        // DNS 缓存由系统或 DoH 服务器管理
        // 这里留作扩展接口
    }
}

impl Default for DnsResolver {
    fn default() -> Self {
        Self::new(DohConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_dns_resolver_system() {
        let resolver = DnsResolver::default();
        let result = resolver.resolve("localhost").await;
        assert!(result.is_ok());
        let ips = result.unwrap();
        assert!(!ips.is_empty());
    }

    #[tokio::test]
    async fn test_dns_resolver_creation() {
        let config = DohConfig::default();
        let resolver = DnsResolver::new(config);
        assert!(!resolver.config.enabled);
    }

    #[test]
    fn test_dns_resolver_clear_cache() {
        let resolver = DnsResolver::default();
        resolver.clear_cache(); // 不应 panic
    }
}
