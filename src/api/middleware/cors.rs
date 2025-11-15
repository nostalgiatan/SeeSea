//! CORS 中间件
//!
//! 处理跨域资源共享 (CORS)

use axum::http::{header, Method, HeaderValue};
use tower_http::cors::{CorsLayer, Any};

/// 创建 CORS 中间件
///
/// # Returns
///
/// 返回配置好的 CORS 层
pub fn create_cors_layer() -> CorsLayer {
    CorsLayer::new()
        .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
        .allow_origin(Any)
        .allow_headers([header::CONTENT_TYPE, header::AUTHORIZATION])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cors_layer_creation() {
        let _layer = create_cors_layer();
        // CORS layer created successfully
    }
}
