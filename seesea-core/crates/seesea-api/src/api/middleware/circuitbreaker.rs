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

//! 熔断器中间件
//!
//! 提供熔断保护功能

use axum::{
    extract::Request,
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
};
use seesea_config::CircuitBreakerConfig as Config;
use std::sync::Arc;
use std::sync::atomic::{AtomicU8, AtomicU64, Ordering};
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// 熔断器状态
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CircuitState {
    /// 关闭状态（正常）
    Closed = 0,
    /// 打开状态（熔断）
    Open = 1,
    /// 半开状态（测试恢复）
    HalfOpen = 2,
}

impl From<u8> for CircuitState {
    fn from(value: u8) -> Self {
        match value {
            0 => CircuitState::Closed,
            1 => CircuitState::Open,
            2 => CircuitState::HalfOpen,
            _ => CircuitState::Closed,
        }
    }
}

/// 熔断器状态管理
pub struct CircuitBreakerState {
    /// 当前状态
    state: AtomicU8,
    /// 失败计数
    failure_count: AtomicU64,
    /// 成功计数（半开状态）
    success_count: AtomicU64,
    /// 最后状态变更时间
    last_state_change: Arc<RwLock<Instant>>,
    /// 配置
    config: Config,
}

impl CircuitBreakerState {
    /// 创建新的熔断器状态
    pub fn new(config: Config) -> Self {
        Self {
            state: AtomicU8::new(CircuitState::Closed as u8),
            failure_count: AtomicU64::new(0),
            success_count: AtomicU64::new(0),
            last_state_change: Arc::new(RwLock::new(Instant::now())),
            config,
        }
    }

    /// 获取当前状态
    pub fn get_state(&self) -> CircuitState {
        CircuitState::from(self.state.load(Ordering::SeqCst))
    }

    /// 记录成功
    pub async fn record_success(&self) {
        match self.get_state() {
            CircuitState::HalfOpen => {
                let count = self.success_count.fetch_add(1, Ordering::SeqCst) + 1;
                if count >= self.config.success_threshold {
                    self.transition_to_closed().await;
                }
            }
            CircuitState::Closed => {
                // 重置失败计数
                self.failure_count.store(0, Ordering::SeqCst);
            }
            _ => {}
        }
    }

    /// 记录失败
    pub async fn record_failure(&self) {
        match self.get_state() {
            CircuitState::Closed => {
                let count = self.failure_count.fetch_add(1, Ordering::SeqCst) + 1;
                if count >= self.config.failure_threshold {
                    self.transition_to_open().await;
                }
            }
            CircuitState::HalfOpen => {
                self.transition_to_open().await;
            }
            _ => {}
        }
    }

    /// 检查是否允许请求
    pub async fn allow_request(&self) -> bool {
        match self.get_state() {
            CircuitState::Closed => true,
            CircuitState::Open => {
                // 检查是否超时，可以尝试半开
                let last_change = self.last_state_change.read().await;
                if last_change.elapsed() > Duration::from_secs(self.config.timeout) {
                    drop(last_change);
                    self.transition_to_half_open().await;
                    true
                } else {
                    false
                }
            }
            CircuitState::HalfOpen => true,
        }
    }

    /// 转换到关闭状态
    async fn transition_to_closed(&self) {
        self.state
            .store(CircuitState::Closed as u8, Ordering::SeqCst);
        self.failure_count.store(0, Ordering::SeqCst);
        self.success_count.store(0, Ordering::SeqCst);
        *self.last_state_change.write().await = Instant::now();
        tracing::info!("Circuit breaker transitioned to CLOSED state");
    }

    /// 转换到打开状态
    async fn transition_to_open(&self) {
        self.state.store(CircuitState::Open as u8, Ordering::SeqCst);
        self.failure_count.store(0, Ordering::SeqCst);
        self.success_count.store(0, Ordering::SeqCst);
        *self.last_state_change.write().await = Instant::now();
        tracing::warn!("Circuit breaker transitioned to OPEN state");
    }

    /// 转换到半开状态
    async fn transition_to_half_open(&self) {
        self.state
            .store(CircuitState::HalfOpen as u8, Ordering::SeqCst);
        self.success_count.store(0, Ordering::SeqCst);
        *self.last_state_change.write().await = Instant::now();
        tracing::info!("Circuit breaker transitioned to HALF-OPEN state");
    }
}

/// 熔断器中间件
pub async fn circuit_breaker_middleware(
    axum::extract::State(state): axum::extract::State<Arc<CircuitBreakerState>>,
    req: Request,
    next: Next,
) -> Response {
    if !state.config.enabled {
        return next.run(req).await;
    }

    // 检查是否允许请求
    if !state.allow_request().await {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            serde_json::json!({
                "code": "CIRCUIT_BREAKER_OPEN",
                "message": "服务暂时不可用，请稍后再试"
            })
            .to_string(),
        )
            .into_response();
    }

    // 执行请求
    let response = next.run(req).await;

    // 根据响应状态记录成功或失败
    if response.status().is_server_error() {
        state.record_failure().await;
    } else {
        state.record_success().await;
    }

    response
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_circuit_breaker_config_default() {
        let config = Config::default();
        assert_eq!(config.failure_threshold, 5);
        assert_eq!(config.success_threshold, 2);
        assert_eq!(config.timeout, 60);
        assert!(config.enabled);
    }

    #[tokio::test]
    async fn test_circuit_breaker_state_creation() {
        let config = Config::default();
        let state = CircuitBreakerState::new(config);
        assert_eq!(state.get_state(), CircuitState::Closed);
    }

    #[tokio::test]
    async fn test_circuit_breaker_transitions() {
        let config = Config {
            failure_threshold: 3,
            success_threshold: 2,
            timeout: 1,
            enabled: true,
        };
        let state = CircuitBreakerState::new(config);

        // 初始状态应该是关闭
        assert_eq!(state.get_state(), CircuitState::Closed);
        assert!(state.allow_request().await);

        // 记录失败次数直到打开
        state.record_failure().await;
        state.record_failure().await;
        state.record_failure().await;
        assert_eq!(state.get_state(), CircuitState::Open);

        // 打开状态不允许请求
        assert!(!state.allow_request().await);
    }
}
