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

//! 健康检查处理器
//!
//! 处理健康检查相关的 API 请求

use axum::{
    Json,
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
};

use crate::api::on::ApiState;
use crate::api::types::ApiHealthResponse;

/// 健康检查接口
///
/// 返回服务健康状态和版本信息
#[utoipa::path(
    get,
    path = "/health",
    responses(
        (status = 200, description = "服务健康", body = ApiHealthResponse),
    ),
    tag = "health"
)]
pub async fn handle_health(State(state): State<ApiState>) -> Response {
    let engines = state.search.list_engines();

    let health = ApiHealthResponse {
        status: "healthy".to_string(),
        version: state.version.clone(),
        available_engines: engines.len(),
        total_engines: engines.len(),
    };

    (StatusCode::OK, Json(health)).into_response()
}
