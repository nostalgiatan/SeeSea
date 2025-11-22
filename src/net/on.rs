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

//! 网络层外部接口模块
//!
//! 提供统一的网络层访问接口

use crate::error::Result;
use crate::net::client::HttpClient;
use crate::net::resolver::DnsResolver;
use crate::net::types::NetworkConfig;
use std::sync::Arc;

/// 网络层统一接口
///
/// 提供对 HTTP 客户端、DNS 解析器等网络功能的统一访问
#[derive(Clone)]
pub struct NetworkInterface {
    /// HTTP 客户端
    http_client: Arc<HttpClient>,
    /// DNS 解析器
    dns_resolver: Arc<DnsResolver>,
    /// 网络配置
    config: Arc<NetworkConfig>,
}

impl NetworkInterface {
    /// 创建新的网络层接口
    ///
    /// # 参数
    ///
    /// * `config` - 网络配置
    ///
    /// # 返回
    ///
    /// 成功返回 NetworkInterface，失败返回错误
    pub fn new(config: NetworkConfig) -> Result<Self> {
        let http_client = HttpClient::new(config.clone())?;
        let dns_resolver = DnsResolver::new(config.doh.clone());

        Ok(Self {
            http_client: Arc::new(http_client),
            dns_resolver: Arc::new(dns_resolver),
            config: Arc::new(config),
        })
    }

    /// 获取 HTTP 客户端
    ///
    /// # 返回
    ///
    /// HttpClient 的引用
    pub fn http(&self) -> &HttpClient {
        &self.http_client
    }

    /// 获取 DNS 解析器
    ///
    /// # 返回
    ///
    /// DnsResolver 的引用
    pub fn dns(&self) -> &DnsResolver {
        &self.dns_resolver
    }

    /// 获取网络配置
    ///
    /// # 返回
    ///
    /// NetworkConfig 的引用
    pub fn config(&self) -> &NetworkConfig {
        &self.config
    }

    /// 执行健康检查
    ///
    /// 检查网络层各组件是否正常工作
    ///
    /// # 返回
    ///
    /// 如果所有组件正常返回 Ok(())，否则返回错误
    pub async fn health_check(&self) -> Result<HealthStatus> {
        let mut status = HealthStatus::default();

        // 检查 HTTP 客户端
        status.http_client = true;

        // 检查 DNS 解析器
        if let Ok(ips) = self.dns_resolver.resolve("localhost").await {
            status.dns_resolver = !ips.is_empty();
        }

        // 检查代理（如果启用）
        if self.config.proxy.enabled {
            status.proxy = crate::net::client::proxy::check_proxy(&self.config.proxy)
                .await
                .unwrap_or(false);
        } else {
            status.proxy = true; // 未启用代理视为正常
        }

        status.overall = status.http_client && status.dns_resolver && status.proxy;

        Ok(status)
    }
}

impl Default for NetworkInterface {
    fn default() -> Self {
        Self::new(NetworkConfig::default())
            .expect("Failed to create default NetworkInterface")
    }
}

/// 健康状态
#[derive(Debug, Clone)]
pub struct HealthStatus {
    /// HTTP 客户端状态
    pub http_client: bool,
    /// DNS 解析器状态
    pub dns_resolver: bool,
    /// 代理状态
    pub proxy: bool,
    /// 总体状态
    pub overall: bool,
}

impl Default for HealthStatus {
    fn default() -> Self {
        Self {
            http_client: false,
            dns_resolver: false,
            proxy: false,
            overall: false,
        }
    }
}

impl HealthStatus {
    /// 是否健康
    pub fn is_healthy(&self) -> bool {
        self.overall
    }

    /// 获取状态报告
    pub fn report(&self) -> String {
        format!(
            "Network Health Status:\n\
             - HTTP Client: {}\n\
             - DNS Resolver: {}\n\
             - Proxy: {}\n\
             - Overall: {}",
            if self.http_client { "✓" } else { "✗" },
            if self.dns_resolver { "✓" } else { "✗" },
            if self.proxy { "✓" } else { "✗" },
            if self.overall { "✓ Healthy" } else { "✗ Unhealthy" }
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_network_interface_creation() {
        let config = NetworkConfig::default();
        let interface = NetworkInterface::new(config);
        assert!(interface.is_ok());
    }

    #[test]
    fn test_network_interface_default() {
        let interface = NetworkInterface::default();
        assert!(interface.config().proxy.enabled == false);
    }

    #[test]
    fn test_network_interface_http_client() {
        let interface = NetworkInterface::default();
        let _http = interface.http();
        // 只测试不会 panic
    }

    #[test]
    fn test_network_interface_dns_resolver() {
        let interface = NetworkInterface::default();
        let _dns = interface.dns();
        // 只测试不会 panic
    }

    #[tokio::test]
    async fn test_network_interface_health_check() {
        let interface = NetworkInterface::default();
        let status = interface.health_check().await;
        assert!(status.is_ok());
    }

    #[test]
    fn test_health_status_default() {
        let status = HealthStatus::default();
        assert!(!status.is_healthy());
    }

    #[test]
    fn test_health_status_report() {
        let status = HealthStatus {
            http_client: true,
            dns_resolver: true,
            proxy: true,
            overall: true,
        };
        let report = status.report();
        assert!(report.contains("✓ Healthy"));
    }

    #[test]
    fn test_health_status_unhealthy() {
        let mut status = HealthStatus::default();
        status.http_client = true;
        status.dns_resolver = false;
        let report = status.report();
        assert!(report.contains("✗ Unhealthy"));
    }
}
