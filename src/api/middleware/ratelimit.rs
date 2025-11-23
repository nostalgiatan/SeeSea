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

//! 限流中间件
//!
//! 提供 API 请求速率限制功能

use axum::{
    extract::Request,
    http::{HeaderValue, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use dashmap::DashMap;
use governor::{
    clock::DefaultClock,
    state::{direct::NotKeyed, InMemoryState},
    Quota, RateLimiter,
};
use std::net::IpAddr;
use std::num::NonZeroU32;
use std::sync::Arc;

/// 限流配置
#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    /// 每秒请求数限制
    pub requests_per_second: u32,
    
    /// 突发请求容量
    pub burst_size: u32,

    /// 是否启用
    pub enabled: bool,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            requests_per_second: 100,
            burst_size: 200,
            enabled: true,
        }
    }
}

/// 限流器状态
pub struct RateLimiterState {
    /// 全局限流器
    global_limiter: Arc<RateLimiter<NotKeyed, InMemoryState, DefaultClock>>,
    /// IP级别限流器映射
    ip_limiters: Arc<DashMap<IpAddr, Arc<RateLimiter<NotKeyed, InMemoryState, DefaultClock>>>>,
    /// 配置
    config: RateLimitConfig,
}

impl RateLimiterState {
    /// 创建新的限流器状态
    pub fn new(config: RateLimitConfig) -> Self {
        let quota = Quota::per_second(
            NonZeroU32::new(config.requests_per_second).unwrap_or(NonZeroU32::new(100).unwrap())
        )
        .allow_burst(
            NonZeroU32::new(config.burst_size).unwrap_or(NonZeroU32::new(200).unwrap())
        );
        
        let global_limiter = Arc::new(RateLimiter::direct(quota));
        
        Self {
            global_limiter,
            ip_limiters: Arc::new(DashMap::new()),
            config,
        }
    }

    /// 获取或创建IP限流器
    fn get_or_create_limiter(&self, ip: IpAddr) -> Arc<RateLimiter<NotKeyed, InMemoryState, DefaultClock>> {
        self.ip_limiters
            .entry(ip)
            .or_insert_with(|| {
                // 每个IP的限流为全局的10%，但至少1请求/秒
                let per_ip_rate = std::cmp::max(1, self.config.requests_per_second / 10);
                let per_ip_burst = std::cmp::max(2, self.config.burst_size / 10);
                
                let quota = Quota::per_second(
                    NonZeroU32::new(per_ip_rate).unwrap()
                )
                .allow_burst(
                    NonZeroU32::new(per_ip_burst).unwrap()
                );
                Arc::new(RateLimiter::direct(quota))
            })
            .clone()
    }
}

/// 限流中间件
pub async fn rate_limit_middleware(
    axum::extract::State(state): axum::extract::State<Arc<RateLimiterState>>,
    req: Request,
    next: Next,
) -> Response {
    if !state.config.enabled {
        return next.run(req).await;
    }

    // 检查全局限流
    if state.global_limiter.check().is_err() {
        return create_rate_limit_response();
    }

    // 提取客户端IP
    if let Some(ip) = extract_client_ip(&req) {
        let limiter = state.get_or_create_limiter(ip);
        if limiter.check().is_err() {
            return create_rate_limit_response();
        }
    }

    next.run(req).await
}

/// 创建限流响应
fn create_rate_limit_response() -> Response {
    let mut response = (
        StatusCode::TOO_MANY_REQUESTS,
        serde_json::json!({
            "code": "RATE_LIMIT_EXCEEDED",
            "message": "请求过于频繁，请稍后再试"
        }).to_string()
    ).into_response();
    
    response.headers_mut().insert(
        "Retry-After",
        HeaderValue::from_static("60"),
    );
    
    response
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
    fn test_rate_limit_config_default() {
        let config = RateLimitConfig::default();
        assert_eq!(config.requests_per_second, 100);
        assert_eq!(config.burst_size, 200);
        assert!(config.enabled);
    }

    #[test]
    fn test_rate_limiter_state_creation() {
        let config = RateLimitConfig::default();
        let _state = RateLimiterState::new(config);
    }
}
