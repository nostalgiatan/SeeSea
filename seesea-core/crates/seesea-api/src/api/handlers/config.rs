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

//! 配置处理器
//!
//! 处理配置相关的 API 请求

use axum::{
    extract::{Json, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde_json::json;

use crate::api::on::ApiState;

/// 处理魔法链接生成请求
pub async fn handle_magic_link_generate(
    State(state): State<ApiState>,
    Json(params): Json<serde_json::Value>,
) -> Response {
    let purpose = params
        .get("purpose")
        .and_then(|v| v.as_str())
        .unwrap_or("general")
        .to_string();

    let token = state.magic_link.generate_token(purpose);

    (
        StatusCode::OK,
        Json(json!({
            "token": token,
            "expires_in": 300,
            "url": format!("/api/search?magic_token={}", token)
        })),
    )
        .into_response()
}
