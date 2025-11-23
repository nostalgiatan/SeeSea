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

//! IP过滤中间件
//!
//! 提供IP黑名单和白名单功能

use axum::{
    extract::Request,
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
};
use dashmap::DashMap;
use std::net::IpAddr;
use std::sync::Arc;

/// IP过滤配置
#[derive(Debug, Clone)]
pub struct IpFilterConfig {
    /// 是否启用白名单模式
    pub whitelist_mode: bool,
    
    /// 是否启用
    pub enabled: bool,
}

impl Default for IpFilterConfig {
    fn default() -> Self {
        Self {
            whitelist_mode: false,
            enabled: true,
        }
    }
}

/// IP过滤状态
pub struct IpFilterState {
    /// 黑名单
    blacklist: Arc<DashMap<IpAddr, String>>,
    /// 白名单
    whitelist: Arc<DashMap<IpAddr, String>>,
    /// 配置
    config: IpFilterConfig,
}

impl IpFilterState {
    /// 创建新的IP过滤状态
    pub fn new(config: IpFilterConfig) -> Self {
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
    if let Some(ip) = extract_client_ip(&req) {
        if !state.is_allowed(&ip) {
            return (
                StatusCode::FORBIDDEN,
                serde_json::json!({
                    "code": "IP_BLOCKED",
                    "message": "您的IP地址已被封禁"
                }).to_string()
            ).into_response();
        }
    }

    next.run(req).await
}

/// 提取客户端IP
fn extract_client_ip(req: &Request) -> Option<IpAddr> {
    // 尝试从X-Forwarded-For获取
    if let Some(forwarded) = req.headers().get("x-forwarded-for") {
        if let Ok(forwarded_str) = forwarded.to_str() {
            if let Some(ip_str) = forwarded_str.split(',').next() {
                if let Ok(ip) = ip_str.trim().parse() {
                    return Some(ip);
                }
            }
        }
    }

    // 尝试从X-Real-IP获取
    if let Some(real_ip) = req.headers().get("x-real-ip") {
        if let Ok(ip_str) = real_ip.to_str() {
            if let Ok(ip) = ip_str.parse() {
                return Some(ip);
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ip_filter_config_default() {
        let config = IpFilterConfig::default();
        assert!(!config.whitelist_mode);
        assert!(config.enabled);
    }

    #[test]
    fn test_ip_filter_blacklist() {
        let config = IpFilterConfig::default();
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
        let mut config = IpFilterConfig::default();
        config.whitelist_mode = true;
        let state = IpFilterState::new(config);
        
        let ip: IpAddr = "192.168.1.1".parse().unwrap();
        assert!(!state.is_allowed(&ip));
        
        state.add_to_whitelist(ip, "Test allow".to_string());
        assert!(state.is_allowed(&ip));
        
        state.remove_from_whitelist(&ip);
        assert!(!state.is_allowed(&ip));
    }
}
