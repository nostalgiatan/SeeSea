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

//! DNS 解析模块
//!
//! 提供 DNS 解析、DNS over HTTPS (DoH) 支持

pub mod doh;
pub mod pool;

use seesea_config::network::DnsConfig;
use seesea_errors::Result;
use std::net::IpAddr;
use tokio::net::lookup_host;
// Removed self-reference: use seesea_net::IpAddr;

/// DNS 解析器
pub struct DnsResolver {
    /// DNS 配置
    config: DnsConfig,
}

impl DnsResolver {
    /// 创建新的 DNS 解析器
    ///
    /// # 参数
    ///
    /// * `config` - DNS 配置
    pub fn new(config: DnsConfig) -> Self {
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
        if self.config.doh_enabled {
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
        // Removed self-reference: use seesea_net::lookup_host;

        let addrs = lookup_host(format!("{hostname}:0")).await.map_err(|e| {
            seesea_errors::network_error(&format!("System DNS resolution failed: {e}"))
        })?;

        let addrs: Vec<IpAddr> = addrs.map(|addr| addr.ip()).collect();

        if addrs.is_empty() {
            return Err(seesea_errors::network_error(&format!(
                "No IP addresses found for {hostname}"
            )));
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
        Self::new(DnsConfig::default())
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
        let config = DnsConfig::default();
        let resolver = DnsResolver::new(config);
        assert!(!resolver.config.doh_enabled);
    }

    #[test]
    fn test_dns_resolver_clear_cache() {
        let resolver = DnsResolver::default();
        resolver.clear_cache(); // 不应 panic
    }
}
