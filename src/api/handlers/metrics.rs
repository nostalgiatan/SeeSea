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

//! 指标处理器
//!
//! 处理指标和统计相关的 API 请求

use axum::{
    extract::State,
    response::{IntoResponse, Response},
    http::StatusCode,
    Json,
};
use serde_json::json;

use crate::api::on::ApiState;
use crate::api::types::{ApiStatsResponse, ApiEngineInfo};

/// 处理统计信息请求
pub async fn handle_stats(
    State(state): State<ApiState>,
) -> Response {
    let stats = state.search.get_stats().await;
    let api_stats = ApiStatsResponse::from_search_stats(&stats);

    (StatusCode::OK, Json(api_stats)).into_response()
}

/// 处理引擎列表请求
pub async fn handle_engines_list(
    State(state): State<ApiState>,
) -> Response {
    let engines = state.search.list_engines();
    
    let engine_infos: Vec<ApiEngineInfo> = engines
        .into_iter()
        .map(|name| ApiEngineInfo {
            name: name.clone(),
            description: format!("{} 搜索引擎", name),
            engine_type: "general".to_string(),
            enabled: true,
            capabilities: vec!["web".to_string()],
        })
        .collect();
    
    (StatusCode::OK, Json(engine_infos)).into_response()
}

/// 处理版本信息请求
pub async fn handle_version(
    State(state): State<ApiState>,
) -> Response {
    let version_info = json!({
        "version": state.version,
        "name": "SeeSea",
        "description": "隐私保护型元搜索引擎"
    });
    
    (StatusCode::OK, Json(version_info)).into_response()
}

/// 处理指标请求（Prometheus格式）
pub async fn handle_metrics(
    State(state): State<ApiState>,
) -> Response {
    if let Some(metrics) = state.metrics.get_prometheus_metrics() {
        (StatusCode::OK, metrics).into_response()
    } else {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            "Metrics not enabled".to_string()
        ).into_response()
    }
}

/// 处理实时指标请求（JSON格式）
pub async fn handle_realtime_metrics(
    State(state): State<ApiState>,
) -> Response {
    let metrics = state.metrics.get_realtime_metrics().await;
    (StatusCode::OK, Json(metrics)).into_response()
}
