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

//! 日志中间件
//!
//! 记录 API 请求和响应日志
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

use axum::{body::Body, http::Request, middleware::Next, response::Response};
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
pub async fn logging_middleware(req: Request<Body>, next: Next) -> Response {
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

    #[test]
    fn test_logging_middleware_exists() {
        // Test that the middleware function is callable
        // Actual testing would require setting up a full axum app
    }
}
