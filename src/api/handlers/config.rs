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

//! 配置处理器
//!
//! 处理配置相关的 API 请求

use axum::{
    extract::{State, Json},
    response::{IntoResponse, Response},
    http::StatusCode,
};
use serde_json::json;

use crate::api::on::ApiState;

/// 处理魔法链接生成请求
pub async fn handle_magic_link_generate(
    State(state): State<ApiState>,
    Json(params): Json<serde_json::Value>,
) -> Response {
    let purpose = params.get("purpose")
        .and_then(|v| v.as_str())
        .unwrap_or("general")
        .to_string();
    
    let token = state.magic_link.generate_token(purpose);
    
    (StatusCode::OK, Json(json!({
        "token": token,
        "expires_in": 300,
        "url": format!("/api/search?magic_token={}", token)
    }))).into_response()
}
