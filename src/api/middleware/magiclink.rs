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

//! 魔法链接中间件
//!
//! 提供一次性魔法链接认证功能

use axum::{
    extract::Request,
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
};
use chrono::Utc;
use dashmap::DashMap;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::sync::Arc;
use std::time::{Duration, Instant};
use uuid::Uuid;

/// 魔法链接配置
#[derive(Debug, Clone)]
pub struct MagicLinkConfig {
    /// 是否启用
    pub enabled: bool,
    
    /// 链接有效期（秒）
    pub expiration: u64,
    
    /// 密钥
    pub secret: String,
}

impl Default for MagicLinkConfig {
    fn default() -> Self {
        // Warning: Default secret should be changed in production
        tracing::warn!("Using default MagicLink secret - CHANGE THIS IN PRODUCTION!");
        
        Self {
            enabled: true,
            expiration: 300, // 5 minutes
            secret: format!("magic_link_default_secret_{}", Uuid::new_v4()),
        }
    }
}

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
    config: MagicLinkConfig,
}

impl MagicLinkState {
    /// 创建新的魔法链接状态
    pub fn new(config: MagicLinkConfig) -> Self {
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
        self.links.retain(|_, info| {
            info.created_at.elapsed() < expiration + Duration::from_secs(60)
        });
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
    
    if let Ok(query) = serde_urlencoded::from_str::<MagicLinkQuery>(query_str) {
        if let Some(token) = query.token {
            match state.verify_token(&token) {
                Ok(_purpose) => {
                    // 魔法链接验证成功，添加标记到请求扩展
                    // 这样后续的认证中间件可以跳过
                    tracing::info!("Magic link verified successfully");
                    return next.run(req).await;
                }
                Err(e) => {
                    return (
                        StatusCode::UNAUTHORIZED,
                        serde_json::json!({
                            "code": "MAGIC_LINK_INVALID",
                            "message": format!("魔法链接无效: {}", e)
                        }).to_string()
                    ).into_response();
                }
            }
        }
    }

    // 没有魔法链接，继续正常流程
    next.run(req).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_magic_link_config_default() {
        let config = MagicLinkConfig::default();
        assert!(config.enabled);
        assert_eq!(config.expiration, 300);
    }

    #[test]
    fn test_magic_link_generation_and_verification() {
        let config = MagicLinkConfig {
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

    #[test]
    fn test_magic_link_invalid_token() {
        let config = MagicLinkConfig::default();
        let state = MagicLinkState::new(config);

        let result = state.verify_token("invalid_token");
        assert!(result.is_err());
    }

    #[test]
    fn test_magic_link_cleanup() {
        let config = MagicLinkConfig::default();
        let state = MagicLinkState::new(config);

        let _token = state.generate_token("test".to_string());
        assert_eq!(state.active_links_count(), 1);
        
        state.cleanup_expired();
        // 应该还在，因为还没过期
        assert_eq!(state.active_links_count(), 1);
    }
}
