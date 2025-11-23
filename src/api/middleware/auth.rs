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

//! 认证中间件
//!
//! 提供 API 认证功能

use axum::{
    extract::Request,
    http::{header::AUTHORIZATION, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

/// 认证配置
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
        let token_data = decode::<Claims>(
            token,
            &self.decoding_key,
            &Validation::default(),
        )?;

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
            return self.verify_token(token)
                .map_err(|e| format!("Invalid JWT token: {}", e));
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
                    }).to_string()
                ).into_response();
            }
        }
    }

    // 没有Authorization头
    (
        StatusCode::UNAUTHORIZED,
        serde_json::json!({
            "code": "AUTH_REQUIRED",
            "message": "需要认证"
        }).to_string()
    ).into_response()
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
        let auth_header = format!("Bearer {}", token);
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

