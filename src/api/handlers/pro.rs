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

//! Pro API 处理器模块
//!
//! 处理所有 Pro API 请求

use crate::api::on::ApiState;
use axum::body::Body;
use axum::{
    extract::Path,
    extract::Query,
    extract::Request,
    extract::State,
    http::{Method, Response, StatusCode},
};
#[cfg(feature = "python")]
use pyo3::prelude::*;
use serde_json::json;
use std::collections::HashMap;

/// 处理所有Pro API请求
///
/// 匹配所有以/api/pro/开头的请求，并转发到相应的动态路由处理函数
///
/// # Arguments
/// * `state` - API状态
/// * `path` - URL路径参数
/// * `method` - 请求方法
/// * `query` - 查询参数
/// * `request` - 完整请求对象
///
/// # Returns
/// * `Response<Body>` - 处理结果
#[cfg(feature = "python")]
pub async fn handle_pro_api(
    State(state): State<ApiState>,
    Path(path): Path<String>,
    method: Method,
    query: Query<HashMap<String, String>>,
    _request: Request,
) -> Response<Body> {
    // 构建完整路径
    let full_path = format!("/{path}");

    // 获取动态路由匹配器的读锁
    let router = state.dynamic_router.read().await;

    // 查找匹配的路由处理函数
    if let Some(handler) = router.match_route(&full_path, method.as_str()) {
        // 1. 提取查询参数
        let query_params: HashMap<String, String> = query.0.clone();

        // 克隆handler，以便在阻塞线程中使用
        let handler_clone = handler.clone();
        let full_path_clone = full_path.clone();
        let method_str = method.as_str().to_string();

        // 2. 在阻塞线程池中调用Python函数
        // 这样可以避免阻塞异步执行器
        let result = tokio::task::spawn_blocking(move || {
            // 使用Python::attach来调用Python回调函数
            Python::attach(|py| {
                // 调用Python回调函数
                let result =
                    handler_clone.call1(py, (full_path_clone, &method_str, &query_params))?;

                // 提取结果
                result.extract::<String>(py)
            })
        })
        .await;

        // 3. 处理返回结果
        match result {
            Ok(Ok(json_str)) => {
                // 4. 返回响应
                Response::builder()
                    .status(StatusCode::OK)
                    .header("Content-Type", "application/json")
                    .body(Body::from(json_str))
                    .unwrap()
            }
            Ok(Err(err)) => {
                // 处理Python调用错误
                Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .header("Content-Type", "application/json")
                    .body(Body::from(
                        serde_json::to_string(
                            &json!({"error": format!("Python handler error: {:?}", err)}),
                        )
                        .unwrap(),
                    ))
                    .unwrap()
            }
            Err(err) => {
                // 处理任务生成错误
                Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .header("Content-Type", "application/json")
                    .body(Body::from(
                        serde_json::to_string(
                            &json!({"error": format!("Task spawn error: {:?}", err)}),
                        )
                        .unwrap(),
                    ))
                    .unwrap()
            }
        }
    } else {
        // 没有匹配到路由，返回404
        Response::builder()
            .status(StatusCode::NOT_FOUND)
            .header("Content-Type", "application/json")
            .body(Body::from(
                serde_json::to_string(
                    &json!({"error": "Resource not found","path": full_path, "method": method.as_str()}),
                )
                .unwrap(),
            ))
            .unwrap()
    }
}

#[cfg(not(feature = "python"))]
pub async fn handle_pro_api(
    _state: State<ApiState>,
    Path(path): Path<String>,
    method: Method,
    _query: Query<HashMap<String, String>>,
    _request: Request,
) -> Response<Body> {
    // 构建完整路径
    let full_path = format!("/{path}");

    // 当python功能未启用时，返回501 Not Implemented
    Response::builder()
        .status(StatusCode::NOT_IMPLEMENTED)
        .header("Content-Type", "application/json")
        .body(Body::from(
            serde_json::to_string(
                &json!({"error": "Pro API is not implemented", "path": full_path, "method": method.as_str()}),
            )
            .unwrap(),
        ))
        .unwrap()
}
