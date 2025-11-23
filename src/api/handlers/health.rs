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

//! 健康检查处理器
//!
//! 处理健康检查相关的 API 请求

use axum::{
    extract::State,
    response::{IntoResponse, Response},
    http::StatusCode,
    Json,
};

use crate::api::on::ApiState;
use crate::api::types::ApiHealthResponse;

/// 处理健康检查请求
pub async fn handle_health(
    State(state): State<ApiState>,
) -> Response {
    let engines = state.search.list_engines();
    
    let health = ApiHealthResponse {
        status: "healthy".to_string(),
        version: state.version.clone(),
        available_engines: engines.len(),
        total_engines: engines.len(),
    };
    
    (StatusCode::OK, Json(health)).into_response()
}
