//! 日志中间件
//!
//! 记录 API 请求和响应日志

use axum::{
    body::Body,
    http::Request,
    middleware::Next,
    response::Response,
};
use std::time::Instant;

/// 日志中间件处理器
///
/// # Arguments
///
/// * `req` - HTTP 请求
/// * `next` - 下一个中间件
///
/// # Returns
///
/// 返回 HTTP 响应
pub async fn logging_middleware(
    req: Request<Body>,
    next: Next,
) -> Response {
    let start = Instant::now();
    let method = req.method().clone();
    let uri = req.uri().clone();
    
    // 处理请求
    let response = next.run(req).await;
    
    let elapsed = start.elapsed();
    let status = response.status();
    
    // 记录日志
    tracing::info!(
        method = %method,
        uri = %uri,
        status = %status,
        elapsed_ms = elapsed.as_millis(),
        "API request processed"
    );
    
    response
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_logging_middleware_exists() {
        // Test that the middleware function is callable
        // Actual testing would require setting up a full axum app
    }
}
