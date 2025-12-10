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

//! 指标收集中间件
//!
//! 记录每个请求的性能指标

use axum::{body::Body, extract::State, http::Request, middleware::Next, response::Response};
use std::sync::Arc;
use std::time::Instant;

use crate::api::metrics::MetricsCollector;

/// 指标收集中间件
///
/// 记录每个请求的响应时间和成功/失败状态
pub async fn metrics_middleware(
    State(metrics): State<Arc<MetricsCollector>>,
    request: Request<Body>,
    next: Next,
) -> Response {
    let start = Instant::now();

    // 增加活跃连接数
    metrics.increment_active_connections().await;

    // 执行请求
    let response = next.run(request).await;

    // 减少活跃连接数
    metrics.decrement_active_connections().await;

    // 计算响应时间
    let duration = start.elapsed();
    let response_time_ms = duration.as_secs_f64() * 1000.0;

    // 判断请求是否成功（2xx 或 3xx 状态码视为成功）
    let success = response.status().is_success() || response.status().is_redirection();

    // 记录请求指标
    metrics.record_request(success, response_time_ms).await;

    response
}
