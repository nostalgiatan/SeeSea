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

//! 魔法链接中间件
//!
//! 提供一次性魔法链接认证功能
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

use axum::{
    extract::Request,
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
};
use chrono::Utc;
use dashmap::DashMap;
use seesea_config::MagicLinkConfig as Config;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::sync::Arc;
use std::time::{Duration, Instant};
use uuid::Uuid;

/// 魔法链接信息
#[derive(Debug, Clone)]
struct MagicLinkInfo {
    /// 创建时间
    created_at: Instant,
    /// 用途描述
    purpose: String,
    /// 是否已使用
    used: bool,
}

/// 魔法链接状态
pub struct MagicLinkState {
    /// 有效的魔法链接映射
    links: Arc<DashMap<String, MagicLinkInfo>>,
    /// 配置
    config: Config,
}

impl MagicLinkState {
    /// 创建新的魔法链接状态
    pub fn new(config: Config) -> Self {
        Self {
            links: Arc::new(DashMap::new()),
            config,
        }
    }

    /// 生成新的魔法链接令牌
    pub fn generate_token(&self, purpose: String) -> String {
        // 生成随机UUID
        let uuid = Uuid::new_v4().to_string();

        // 使用密钥、UUID和时间戳计算安全哈希作为令牌
        let mut hasher = Sha256::new();
        hasher.update(uuid.as_bytes());
        hasher.update(self.config.secret.as_bytes());
        hasher.update(Utc::now().timestamp().to_string().as_bytes());
        let token = format!("{:x}", hasher.finalize());

        let info = MagicLinkInfo {
            created_at: Instant::now(),
            purpose,
            used: false,
        };

        self.links.insert(token.clone(), info);

        // 启动清理任务
        let links = self.links.clone();
        let expiration = self.config.expiration;
        let token_clone = token.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_secs(expiration + 60)).await;
            links.remove(&token_clone);
        });

        token
    }

    /// 验证魔法链接令牌
    pub fn verify_token(&self, token: &str) -> Result<String, String> {
        if let Some(mut entry) = self.links.get_mut(token) {
            let info = entry.value_mut();

            // 检查是否过期
            if info.created_at.elapsed() > Duration::from_secs(self.config.expiration) {
                return Err("Magic link expired".to_string());
            }

            // 检查是否已使用
            if info.used {
                return Err("Magic link already used".to_string());
            }

            // 标记为已使用
            info.used = true;

            Ok(info.purpose.clone())
        } else {
            Err("Invalid magic link".to_string())
        }
    }

    /// 清理过期的链接
    pub fn cleanup_expired(&self) {
        let expiration = Duration::from_secs(self.config.expiration);
        self.links
            .retain(|_, info| info.created_at.elapsed() < expiration + Duration::from_secs(60));
    }

    /// 获取活跃链接数量
    pub fn active_links_count(&self) -> usize {
        self.links.len()
    }
}

/// 魔法链接查询参数
#[derive(Debug, Deserialize)]
struct MagicLinkQuery {
    #[serde(rename = "magic_token")]
    token: Option<String>,
}

/// 魔法链接中间件
pub async fn magic_link_middleware(
    axum::extract::State(state): axum::extract::State<Arc<MagicLinkState>>,
    req: Request,
    next: Next,
) -> Response {
    if !state.config.enabled {
        return next.run(req).await;
    }

    // 检查查询参数中的magic_token
    let uri = req.uri();
    let query_str = uri.query().unwrap_or("");

    tracing::debug!("Magic link middleware: query_str = {}", query_str);

    if let Ok(query) = serde_urlencoded::from_str::<MagicLinkQuery>(query_str) {
        tracing::debug!(
            "Magic link middleware: parsed query, token = {:?}",
            query.token
        );
        if let Some(token) = &query.token {
            tracing::info!("Magic link middleware: verifying token = {}", token);
            match state.verify_token(token) {
                Ok(_purpose) => {
                    // 魔法链接验证成功，添加标记到请求扩展
                    // 这样后续的认证中间件可以跳过
                    tracing::info!("Magic link verified successfully");
                    return next.run(req).await;
                }
                Err(e) => {
                    tracing::warn!("Magic link verification failed: {}", e);
                    return (
                        StatusCode::UNAUTHORIZED,
                        serde_json::json!({
                            "code": "MAGIC_LINK_INVALID",
                            "message": format!("魔法链接无效: {}", e)
                        })
                        .to_string(),
                    )
                        .into_response();
                }
            }
        } else {
            tracing::debug!("Magic link middleware: no token in query");
        }
    } else {
        tracing::debug!("Magic link middleware: failed to parse query");
    }

    // 没有魔法链接，继续正常流程
    tracing::debug!("Magic link middleware: no magic token found, continuing");
    next.run(req).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_magic_link_config_default() {
        let config = Config::default();
        assert!(config.enabled);
        assert_eq!(config.expiration, 300);
    }

    #[tokio::test]
    async fn test_magic_link_generation_and_verification() {
        let config = Config {
            enabled: true,
            expiration: 300,
            secret: "test_secret".to_string(),
        };
        let state = MagicLinkState::new(config);

        let token = state.generate_token("test_purpose".to_string());
        assert!(!token.is_empty());

        // 首次验证应该成功
        let result = state.verify_token(&token);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "test_purpose");

        // 再次验证应该失败（已使用）
        let result = state.verify_token(&token);
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_magic_link_invalid_token() {
        let config = Config::default();
        let state = MagicLinkState::new(config);

        let result = state.verify_token("invalid_token");
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_magic_link_cleanup() {
        let config = Config::default();
        let state = MagicLinkState::new(config);

        let _token = state.generate_token("test".to_string());
        assert_eq!(state.active_links_count(), 1);

        state.cleanup_expired();
        // 应该还在，因为还没过期
        assert_eq!(state.active_links_count(), 1);
    }
}
