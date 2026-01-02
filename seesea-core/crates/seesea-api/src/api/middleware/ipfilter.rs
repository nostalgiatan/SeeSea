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

//! IP过滤中间件
//!
//! 提供IP黑名单和白名单功能
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

use axum::{
    extract::Request,
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
};
use dashmap::DashMap;
use seesea_config::IpFilterConfig as Config;
use std::net::IpAddr;
use std::sync::Arc;

/// IP过滤状态
pub struct IpFilterState {
    /// 黑名单
    blacklist: Arc<DashMap<IpAddr, String>>,
    /// 白名单
    whitelist: Arc<DashMap<IpAddr, String>>,
    /// 配置
    config: Config,
}

impl IpFilterState {
    /// 创建新的IP过滤状态
    pub fn new(config: Config) -> Self {
        Self {
            blacklist: Arc::new(DashMap::new()),
            whitelist: Arc::new(DashMap::new()),
            config,
        }
    }

    /// 添加IP到黑名单
    pub fn add_to_blacklist(&self, ip: IpAddr, reason: String) {
        tracing::info!("IP {} added to blacklist: {}", ip, &reason);
        self.blacklist.insert(ip, reason);
    }

    /// 从黑名单移除IP
    pub fn remove_from_blacklist(&self, ip: &IpAddr) {
        self.blacklist.remove(ip);
        tracing::info!("IP {} removed from blacklist", ip);
    }

    /// 添加IP到白名单
    pub fn add_to_whitelist(&self, ip: IpAddr, reason: String) {
        tracing::info!("IP {} added to whitelist: {}", ip, &reason);
        self.whitelist.insert(ip, reason);
    }

    /// 从白名单移除IP
    pub fn remove_from_whitelist(&self, ip: &IpAddr) {
        self.whitelist.remove(ip);
        tracing::info!("IP {} removed from whitelist", ip);
    }

    /// 检查IP是否被允许
    pub fn is_allowed(&self, ip: &IpAddr) -> bool {
        if self.config.whitelist_mode {
            // 白名单模式：只有在白名单中的IP才允许
            self.whitelist.contains_key(ip)
        } else {
            // 黑名单模式：不在黑名单中的IP都允许
            !self.blacklist.contains_key(ip)
        }
    }

    /// 获取黑名单大小
    pub fn blacklist_size(&self) -> usize {
        self.blacklist.len()
    }

    /// 获取白名单大小
    pub fn whitelist_size(&self) -> usize {
        self.whitelist.len()
    }
}

/// IP过滤中间件
pub async fn ip_filter_middleware(
    axum::extract::State(state): axum::extract::State<Arc<IpFilterState>>,
    req: Request,
    next: Next,
) -> Response {
    if !state.config.enabled {
        return next.run(req).await;
    }

    // 提取客户端IP
    if let Some(ip) = extract_client_ip(&req)
        && !state.is_allowed(&ip)
    {
        return (
            StatusCode::FORBIDDEN,
            serde_json::json!({
                "code": "IP_BLOCKED",
                "message": "您的IP地址已被封禁"
            })
            .to_string(),
        )
            .into_response();
    }

    next.run(req).await
}

/// 提取客户端IP
fn extract_client_ip(req: &Request) -> Option<IpAddr> {
    // 尝试从X-Forwarded-For获取
    if let Some(forwarded) = req.headers().get("x-forwarded-for")
        && let Ok(forwarded_str) = forwarded.to_str()
        && let Some(ip_str) = forwarded_str.split(',').next()
        && let Ok(ip) = ip_str.trim().parse()
    {
        return Some(ip);
    }

    // 尝试从X-Real-IP获取
    if let Some(real_ip) = req.headers().get("x-real-ip")
        && let Ok(ip_str) = real_ip.to_str()
        && let Ok(ip) = ip_str.parse()
    {
        return Some(ip);
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ip_filter_config_default() {
        let config = Config::default();
        assert!(!config.whitelist_mode);
        assert!(config.enabled);
    }

    #[test]
    fn test_ip_filter_blacklist() {
        let config = Config::default();
        let state = IpFilterState::new(config);

        let ip: IpAddr = "192.168.1.1".parse().unwrap();
        assert!(state.is_allowed(&ip));

        state.add_to_blacklist(ip, "Test ban".to_string());
        assert!(!state.is_allowed(&ip));

        state.remove_from_blacklist(&ip);
        assert!(state.is_allowed(&ip));
    }

    #[test]
    fn test_ip_filter_whitelist() {
        let config = Config {
            whitelist_mode: true,
            ..Default::default()
        };
        let state = IpFilterState::new(config);

        let ip: IpAddr = "192.168.1.1".parse().unwrap();
        assert!(!state.is_allowed(&ip));

        state.add_to_whitelist(ip, "Test allow".to_string());
        assert!(state.is_allowed(&ip));

        state.remove_from_whitelist(&ip);
        assert!(!state.is_allowed(&ip));
    }
}
