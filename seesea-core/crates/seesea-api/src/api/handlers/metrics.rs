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

//! 指标处理器
//!
//! 处理指标和统计相关的 API 请求

use axum::{
    Json,
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde_json::json;

use crate::api::on::ApiState;
use crate::api::types::{ApiEngineInfo, ApiStatsResponse};

/// 处理统计信息请求
#[utoipa::path(
    get,
    path = "/stats",
    responses(
        (status = 200, description = "获取成功", body = ApiStatsResponse),
    ),
    tag = "metrics"
)]
pub async fn handle_stats(State(state): State<ApiState>) -> Response {
    let stats = state.search.get_stats().await;
    let api_stats = ApiStatsResponse::from_search_stats(&stats);

    (StatusCode::OK, Json(api_stats)).into_response()
}

/// 处理引擎列表请求
#[utoipa::path(
    get,
    path = "/engines",
    responses(
        (status = 200, description = "获取成功"),
    ),
    tag = "metrics"
)]
pub async fn handle_engines_list(State(state): State<ApiState>) -> Response {
    let engines = state.search.list_engines();

    let engine_infos: Vec<ApiEngineInfo> = engines
        .into_iter()
        .map(|name| ApiEngineInfo {
            name: name.clone(),
            description: format!("{name} 搜索引擎"),
            engine_type: "general".to_string(),
            enabled: true,
            capabilities: vec!["web".to_string()],
        })
        .collect();

    (StatusCode::OK, Json(engine_infos)).into_response()
}

/// 处理版本信息请求
#[utoipa::path(
    get,
    path = "/version",
    responses(
        (status = 200, description = "获取成功"),
    ),
    tag = "metrics"
)]
pub async fn handle_version(State(state): State<ApiState>) -> Response {
    let version_info = json!({
        "version": state.version,
        "name": "SeeSea",
        "description": "隐私保护型元搜索引擎"
    });

    (StatusCode::OK, Json(version_info)).into_response()
}

/// 处理指标请求（Prometheus格式）
#[utoipa::path(
    get,
    path = "/metrics",
    responses(
        (status = 200, description = "获取成功", content_type = "text/plain"),
        (status = 503, description = "指标未启用"),
    ),
    tag = "metrics"
)]
pub async fn handle_metrics(State(state): State<ApiState>) -> Response {
    if let Some(metrics) = state.metrics.get_prometheus_metrics() {
        (StatusCode::OK, metrics).into_response()
    } else {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            "Metrics not enabled".to_string(),
        )
            .into_response()
    }
}

/// 处理实时指标请求（JSON格式）
#[utoipa::path(
    get,
    path = "/metrics/realtime",
    responses(
        (status = 200, description = "获取成功"),
    ),
    tag = "metrics"
)]
pub async fn handle_realtime_metrics(State(state): State<ApiState>) -> Response {
    let metrics = state.metrics.get_realtime_metrics().await;
    (StatusCode::OK, Json(metrics)).into_response()
}
