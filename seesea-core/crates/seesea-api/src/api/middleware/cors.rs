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
//! CORS 中间件
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.
//! 处理跨域资源共享 (CORS)

use axum::http::{HeaderValue, Method};
use tower_http::cors::{AllowOrigin, Any, CorsLayer};

/// 创建 CORS 中间件
///
/// # Arguments
///
/// * `allowed_origins` - 允许的源列表
///
/// # Returns
///
/// 返回配置好的 CORS 层
pub fn create_cors_layer(allowed_origins: Vec<String>) -> CorsLayer {
    // 如果允许的源包含 "*"，则允许所有源
    if allowed_origins.contains(&"*".to_string()) {
        return CorsLayer::very_permissive();
    }

    // 否则转换为 HeaderValue 列表
    let allowed_origins: Vec<HeaderValue> = allowed_origins
        .iter()
        .filter_map(|origin| HeaderValue::from_str(origin).ok())
        .collect();

    CorsLayer::new()
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::DELETE,
            Method::OPTIONS,
            Method::HEAD,
            Method::PATCH,
        ])
        .allow_origin(AllowOrigin::list(allowed_origins))
        .allow_headers(Any)
        .expose_headers(Any)
        .max_age(std::time::Duration::from_secs(86400))
}

/// 创建默认的 CORS 中间件（允许所有源，用于开发环境）
///
/// 使用 `very_permissive()` 预设，这会：
/// - 允许所有源
/// - 允许所有方法
/// - 允许所有头部
/// - 暴露所有头部
/// - 允许凭证（当使用 mirror_request 时）
///
/// # Returns
///
/// 返回配置好的 CORS 层
pub fn default_cors_layer() -> CorsLayer {
    CorsLayer::very_permissive()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cors_layer_creation() {
        let _layer = default_cors_layer();
        // CORS layer created successfully
    }

    #[test]
    fn test_custom_cors_layer() {
        let _layer = create_cors_layer(vec!["https://example.com".to_string()]);
        // Custom CORS layer created successfully
    }
}
