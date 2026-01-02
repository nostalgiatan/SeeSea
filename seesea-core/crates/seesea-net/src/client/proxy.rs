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

//! 代理支持模块
//!
//! 提供 HTTP、SOCKS5、Tor 等代理配置

use reqwest::ClientBuilder;
use seesea_config::{ProxyConfig, ProxyType};
use seesea_errors::Result;

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
        ProxyType::Socks4 => format!("socks4://{}", config.address),
        ProxyType::Socks5 => format!("socks5://{}", config.address),
    };

    let mut proxy = reqwest::Proxy::all(&proxy_url)
        .map_err(|e| seesea_errors::proxy_error(&format!("Failed to create proxy: {e}")))?;

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
/// 成功返回 true，失败返回错误
pub fn check_proxy(config: &ProxyConfig) -> Result<bool> {
    // 简单的代理可用性检测
    // 这里可以实现更复杂的检测逻辑
    if !config.enabled {
        return Ok(false);
    }

    // 检查地址格式是否有效
    if config.address.is_empty() {
        return Err(seesea_errors::proxy_error("Proxy address is empty"));
    }

    // 检查端口是否有效
    let parts: Vec<&str> = config.address.split(':').collect();
    if parts.len() != 2 {
        return Err(seesea_errors::proxy_error("Invalid proxy address format"));
    }

    if let Ok(port) = parts[1].parse::<u16>() {
        if port == 0 {
            return Err(seesea_errors::proxy_error("Invalid proxy port"));
        }
    } else {
        return Err(seesea_errors::proxy_error("Invalid proxy port"));
    }

    Ok(true)
}
