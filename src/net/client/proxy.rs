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

//! 代理支持模块
//!
//! 提供 HTTP、SOCKS5、Tor 等代理配置

use crate::error::Result;
use crate::net::types::{ProxyConfig, ProxyType};
use reqwest::ClientBuilder;

/// 配置代理
///
/// # 参数
///
/// * `builder` - reqwest ClientBuilder
/// * `config` - 代理配置
///
/// # 返回
///
/// 配置好代理的 ClientBuilder
pub fn configure_proxy(builder: ClientBuilder, config: &ProxyConfig) -> Result<ClientBuilder> {
    if !config.enabled {
        return Ok(builder);
    }

    let proxy_url = match config.proxy_type {
        ProxyType::Http => format!("http://{}", config.address),
        ProxyType::Https => format!("https://{}", config.address),
        ProxyType::Socks5 => format!("socks5://{}", config.address),
        ProxyType::Tor => {
            // Tor 默认使用 SOCKS5 代理，通常在 127.0.0.1:9050
            format!("socks5://{}", config.address)
        }
    };

    let mut proxy = reqwest::Proxy::all(&proxy_url)
        .map_err(|e| crate::error::network_error(format!("Failed to create proxy: {}", e)))?;

    // 如果有认证信息，添加认证
    if let (Some(username), Some(password)) = (&config.username, &config.password) {
        proxy = proxy.basic_auth(username, password);
    }

    Ok(builder.proxy(proxy))
}

/// 检测代理是否可用
///
/// # 参数
///
/// * `config` - 代理配置
///
/// # 返回
///
/// 如果代理可用返回 Ok(true)，否则返回 Ok(false) 或错误
pub async fn check_proxy(config: &ProxyConfig) -> Result<bool> {
    if !config.enabled {
        return Ok(false);
    }

    // 创建临时客户端测试代理
    let builder = ClientBuilder::new();
    let builder = configure_proxy(builder, config)?;
    
    let client = builder
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| crate::error::network_error(format!("Failed to build test client: {}", e)))?;

    // 尝试访问一个简单的 URL 来测试代理
    match client.get("https://www.google.com").send().await {
        Ok(_) => Ok(true),
        Err(_) => Ok(false),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_configure_proxy_disabled() {
        let mut config = ProxyConfig::default();
        config.enabled = false;
        
        let builder = ClientBuilder::new();
        let result = configure_proxy(builder, &config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_configure_proxy_http() {
        let mut config = ProxyConfig::default();
        config.enabled = true;
        config.proxy_type = ProxyType::Http;
        config.address = "127.0.0.1:8080".to_string();
        
        let builder = ClientBuilder::new();
        let result = configure_proxy(builder, &config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_configure_proxy_with_auth() {
        let mut config = ProxyConfig::default();
        config.enabled = true;
        config.username = Some("user".to_string());
        config.password = Some("pass".to_string());
        
        let builder = ClientBuilder::new();
        let result = configure_proxy(builder, &config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_configure_tor_proxy() {
        let mut config = ProxyConfig::default();
        config.enabled = true;
        config.proxy_type = ProxyType::Tor;
        config.address = "127.0.0.1:9050".to_string();
        
        let builder = ClientBuilder::new();
        let result = configure_proxy(builder, &config);
        assert!(result.is_ok());
    }
}
