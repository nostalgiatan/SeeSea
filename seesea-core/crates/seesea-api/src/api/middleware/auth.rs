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
//! 认证中间件
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.
//! 提供 API 认证功能

use axum::{
    extract::Request,
    http::{StatusCode, header::AUTHORIZATION},
    middleware::Next,
    response::{IntoResponse, Response},
};
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
use seesea_config::api::AuthConfig as Config;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

/// 认证配置适配器
#[derive(Debug, Clone)]
pub struct AuthConfig {
    /// 是否启用认证
    pub enabled: bool,

    /// JWT 密钥
    pub jwt_secret: String,

    /// JWT 过期时间（秒）
    pub jwt_expiration: u64,

    /// API 密钥列表
    pub api_keys: Vec<String>,

    /// Query 参数名称
    pub query_param_name: String,
}

impl From<Config> for AuthConfig {
    fn from(config: Config) -> Self {
        let query_param_name = if !config.api_key.query_param.is_empty() {
            config.api_key.query_param.clone()
        } else {
            "magic_token".to_string()
        };

        Self {
            enabled: config.enabled,
            jwt_secret: config.jwt.secret,
            jwt_expiration: config.jwt.expiry,
            api_keys: config
                .api_key
                .api_keys
                .iter()
                .map(|k| k.key_hash.clone())
                .collect(),
            query_param_name,
        }
    }
}

impl Default for AuthConfig {
    fn default() -> Self {
        // Warning: Default secret should be changed in production
        tracing::warn!("Using default JWT secret - CHANGE THIS IN PRODUCTION!");

        Self {
            enabled: false,
            jwt_secret: format!("jwt_default_secret_{}", Uuid::new_v4()),
            jwt_expiration: 3600, // 1 hour
            api_keys: Vec::new(),
            query_param_name: "magic_token".to_string(),
        }
    }
}

/// JWT Claims
#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    /// 主题（用户ID或标识）
    pub sub: String,
    /// 过期时间
    pub exp: u64,
    /// 签发时间
    pub iat: u64,
}

/// 认证状态
pub struct AuthState {
    /// 配置
    config: AuthConfig,
    /// 编码密钥
    encoding_key: EncodingKey,
    /// 解码密钥
    decoding_key: DecodingKey,
}

impl AuthState {
    /// 创建新的认证状态
    pub fn new(config: AuthConfig) -> Self {
        let encoding_key = EncodingKey::from_secret(config.jwt_secret.as_bytes());
        let decoding_key = DecodingKey::from_secret(config.jwt_secret.as_bytes());

        Self {
            config,
            encoding_key,
            decoding_key,
        }
    }

    /// 生成JWT令牌
    pub fn generate_token(&self, subject: String) -> Result<String, jsonwebtoken::errors::Error> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let claims = Claims {
            sub: subject,
            exp: now + self.config.jwt_expiration,
            iat: now,
        };

        encode(&Header::default(), &claims, &self.encoding_key)
    }

    /// 验证JWT令牌
    pub fn verify_token(&self, token: &str) -> Result<Claims, jsonwebtoken::errors::Error> {
        let token_data = decode::<Claims>(token, &self.decoding_key, &Validation::default())?;

        Ok(token_data.claims)
    }

    /// 验证API密钥
    pub fn verify_api_key(&self, api_key: &str) -> bool {
        self.config.api_keys.iter().any(|k| k == api_key)
    }

    /// 验证认证头
    pub fn verify_auth_header(&self, auth_header: &str) -> Result<Claims, String> {
        // Bearer token
        if let Some(token) = auth_header.strip_prefix("Bearer ") {
            return self
                .verify_token(token)
                .map_err(|e| format!("Invalid JWT token: {e}"));
        }

        // API Key
        if let Some(api_key) = auth_header.strip_prefix("ApiKey ") {
            if self.verify_api_key(api_key) {
                // 为API Key创建虚拟Claims
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs();
                return Ok(Claims {
                    sub: "api_key".to_string(),
                    exp: now + 3600,
                    iat: now,
                });
            }
            return Err("Invalid API key".to_string());
        }

        Err("Invalid authorization format".to_string())
    }
}

/// JWT认证中间件
pub async fn jwt_auth_middleware(
    axum::extract::State(state): axum::extract::State<Arc<AuthState>>,
    req: Request,
    next: Next,
) -> Response {
    if !state.config.enabled {
        return next.run(req).await;
    }

    // 首先检查 query 参数中的 API key
    let uri = req.uri();
    let query_params: Vec<(String, String)> = uri
        .query()
        .map(|q| {
            q.split('&')
                .filter_map(|pair| {
                    let mut parts = pair.splitn(2, '=');
                    Some((
                        parts.next()?.to_string(),
                        parts.next().unwrap_or("").to_string(),
                    ))
                })
                .collect()
        })
        .unwrap_or_default();

    // 从配置中获取 query 参数名（默认为 "magic_token"）
    let query_param_name = &state.config.query_param_name;

    // 查找 API key 参数
    let api_key = query_params
        .iter()
        .find(|(k, _)| k == query_param_name)
        .map(|(_, v)| v.clone());

    if let Some(key) = api_key {
        // 验证 API key
        if state.config.api_keys.contains(&key) {
            // 认证成功，继续处理请求
            return next.run(req).await;
        } else {
            return (
                StatusCode::UNAUTHORIZED,
                serde_json::json!({
                    "code": "AUTH_FAILED",
                    "message": "无效的 API 密钥"
                })
                .to_string(),
            )
                .into_response();
        }
    }

    // 检查Authorization头
    let auth_header = req
        .headers()
        .get(AUTHORIZATION)
        .and_then(|h| h.to_str().ok());

    if let Some(auth_header) = auth_header {
        match state.verify_auth_header(auth_header) {
            Ok(_claims) => {
                // 认证成功，继续处理请求
                return next.run(req).await;
            }
            Err(e) => {
                return (
                    StatusCode::UNAUTHORIZED,
                    serde_json::json!({
                        "code": "AUTH_FAILED",
                        "message": format!("认证失败: {}", e)
                    })
                    .to_string(),
                )
                    .into_response();
            }
        }
    }

    // 没有Authorization头或API key
    (
        StatusCode::UNAUTHORIZED,
        serde_json::json!({
            "code": "AUTH_REQUIRED",
            "message": format!("需要认证（提供 {} 参数或 Authorization 头）", query_param_name)
        })
        .to_string(),
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auth_config_default() {
        let config = AuthConfig::default();
        assert!(!config.enabled);
        assert_eq!(config.api_keys.len(), 0);
        assert_eq!(config.jwt_expiration, 3600);
    }

    #[test]
    fn test_jwt_generation_and_verification() {
        let config = AuthConfig {
            enabled: true,
            jwt_secret: "test_secret".to_string(),
            jwt_expiration: 3600,
            api_keys: vec![],
        };
        let state = AuthState::new(config);

        let token = state.generate_token("test_user".to_string()).unwrap();
        let claims = state.verify_token(&token).unwrap();

        assert_eq!(claims.sub, "test_user");
    }

    #[test]
    fn test_api_key_verification() {
        let config = AuthConfig {
            enabled: true,
            jwt_secret: "test_secret".to_string(),
            jwt_expiration: 3600,
            api_keys: vec!["test_key".to_string(), "another_key".to_string()],
        };
        let state = AuthState::new(config);

        assert!(state.verify_api_key("test_key"));
        assert!(state.verify_api_key("another_key"));
        assert!(!state.verify_api_key("invalid_key"));
    }

    #[test]
    fn test_auth_header_verification() {
        let config = AuthConfig {
            enabled: true,
            jwt_secret: "test_secret".to_string(),
            jwt_expiration: 3600,
            api_keys: vec!["valid_key".to_string()],
        };
        let state = AuthState::new(config);

        // Test JWT token
        let token = state.generate_token("test_user".to_string()).unwrap();
        let auth_header = format!("Bearer {token}");
        assert!(state.verify_auth_header(&auth_header).is_ok());

        // Test API key
        let auth_header = "ApiKey valid_key";
        assert!(state.verify_auth_header(auth_header).is_ok());

        // Test invalid API key
        let auth_header = "ApiKey invalid_key";
        assert!(state.verify_auth_header(auth_header).is_err());

        // Test invalid format
        let auth_header = "Invalid format";
        assert!(state.verify_auth_header(auth_header).is_err());
    }
}
