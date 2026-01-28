// Copyright (C) 2025 SeeSea Team
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

//! 热门搜索处理器
//!
//! 处理热门搜索相关的 API 请求，包括获取单个平台热点、获取所有平台热点、列出支持的平台等。
//! 支持5分钟缓存以减少重复请求。

use utoipa::ToSchema;

use axum::{
    extract::{Json, Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Deserialize;
use serde_json::json;

use crate::api::on::ApiState;
use crate::api::types::ApiErrorResponse;
use seesea_hot::client::AsyncHotTrendClient;
use seesea_hot::types::HotTrendResult;
use seesea_hot::{SUPPORTED_PLATFORMS, get_hot_trend_cache};

/// 热门搜索请求参数
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct ApiHotSearchRequest {
    /// 是否获取最新数据
    #[serde(default = "default_latest")]
    pub latest: bool,

    /// 平台ID列表（用于批量获取）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub platforms: Option<String>,
}

/// 默认值：是否获取最新数据
fn default_latest() -> bool {
    false
}

/// 处理获取单个平台热点请求
///
/// 获取指定平台的热门搜索内容
#[utoipa::path(
    get,
    path = "/hot/{platform_id}",
    params(
        ("platform_id" = String, Path, description = "平台ID"),
        ("latest" = bool, Query, description = "是否获取最新数据"),
    ),
    responses(
        (status = 200, description = "获取成功", body = HotTrendResult),
        (status = 500, description = "服务器错误", body = ApiErrorResponse),
    ),
    tag = "hot"
)]
pub async fn handle_hot_platform(
    State(state): State<ApiState>,
    Path(platform_id): Path<String>,
    Query(_params): Query<ApiHotSearchRequest>,
) -> Response {
    // 检查缓存
    let cache = get_hot_trend_cache();
    if let Some(cached_result) = cache.get_platform(&platform_id).await {
        return (StatusCode::OK, Json(cached_result)).into_response();
    }

    // 初始化热门搜索客户端
    let hot_client = state
        .hot_client
        .get_or_init(|| async { AsyncHotTrendClient::new(10).await.unwrap() })
        .await;

    match hot_client.fetch_platform(&platform_id).await {
        Ok(result) => {
            // 存入缓存
            cache.set_platform(&platform_id, result.clone()).await;
            (StatusCode::OK, Json(result)).into_response()
        }
        Err(e) => {
            let error = ApiErrorResponse {
                code: "HOT_SEARCH_ERROR".to_string(),
                message: "获取热门搜索失败".to_string(),
                details: Some(e.to_string()),
            };
            (StatusCode::INTERNAL_SERVER_ERROR, Json(error)).into_response()
        }
    }
}

/// 处理获取所有平台热点请求
#[utoipa::path(
    get,
    path = "/hot",
    params(
        ("latest" = bool, Query, description = "是否获取最新数据"),
    ),
    responses(
        (status = 200, description = "获取成功"),
        (status = 500, description = "服务器错误", body = ApiErrorResponse),
    ),
    tag = "hot"
)]
pub async fn handle_hot_all(
    State(state): State<ApiState>,
    Query(_params): Query<ApiHotSearchRequest>,
) -> Response {
    // 检查缓存
    let cache = get_hot_trend_cache();
    if let Some(cached_results) = cache.get_all_platforms().await {
        let total_count = cached_results.len();
        let response = json!({
            "success_count": total_count,
            "failed_count": 0,
            "cached": true,
            "results": cached_results
        });
        return (StatusCode::OK, Json(response)).into_response();
    }

    // 初始化热门搜索客户端
    let hot_client = state
        .hot_client
        .get_or_init(|| async { AsyncHotTrendClient::new(10).await.unwrap() })
        .await;

    let results = hot_client.fetch_all_platforms().await;
    let total_count = results.len();

    // 过滤成功的结果
    let success_results: Vec<HotTrendResult> = results
        .into_iter()
        .filter_map(|result| result.ok())
        .collect();

    // 存入缓存
    cache.set_all_platforms(success_results.clone()).await;

    let response = json!({
        "success_count": success_results.len(),
        "failed_count": total_count - success_results.len(),
        "cached": false,
        "results": success_results
    });

    (StatusCode::OK, Json(response)).into_response()
}

/// 处理获取多个平台热点请求
#[utoipa::path(
    get,
    path = "/hot/multiple",
    params(
        ("platforms" = Option<String>, Query, description = "平台ID列表，逗号分隔"),
        ("latest" = bool, Query, description = "是否获取最新数据"),
    ),
    responses(
        (status = 200, description = "获取成功"),
        (status = 400, description = "参数错误", body = ApiErrorResponse),
        (status = 500, description = "服务器错误", body = ApiErrorResponse),
    ),
    tag = "hot"
)]
pub async fn handle_hot_multiple(
    State(state): State<ApiState>,
    Query(params): Query<ApiHotSearchRequest>,
) -> Response {
    match params.platforms {
        Some(platforms_str) => {
            let platform_ids: Vec<String> = platforms_str
                .split(",")
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();

            if platform_ids.is_empty() {
                let error = ApiErrorResponse {
                    code: "INVALID_PARAMETERS".to_string(),
                    message: "平台列表不能为空".to_string(),
                    details: None,
                };
                return (StatusCode::BAD_REQUEST, Json(error)).into_response();
            }

            // 初始化热门搜索客户端
            let hot_client = state
                .hot_client
                .get_or_init(|| async { AsyncHotTrendClient::new(10).await.unwrap() })
                .await;

            let results = hot_client.fetch_multiple_platforms(&platform_ids).await;
            let total_count = results.len();

            // 过滤成功的结果
            let success_results: Vec<HotTrendResult> = results
                .into_iter()
                .filter_map(|result| result.ok())
                .collect();

            let response = json!({
                "success_count": success_results.len(),
                "failed_count": total_count - success_results.len(),
                "results": success_results
            });

            (StatusCode::OK, Json(response)).into_response()
        }
        None => {
            let error = ApiErrorResponse {
                code: "MISSING_PARAMETERS".to_string(),
                message: "缺少platforms参数".to_string(),
                details: None,
            };
            (StatusCode::BAD_REQUEST, Json(error)).into_response()
        }
    }
}

/// 处理列出支持的平台请求
#[utoipa::path(
    get,
    path = "/hot/platforms",
    responses(
        (status = 200, description = "获取成功"),
    ),
    tag = "hot"
)]
pub async fn handle_hot_platforms_list() -> Response {
    let platforms: Vec<serde_json::Value> = SUPPORTED_PLATFORMS
        .iter()
        .map(|(id, name)| {
            json!({
                "id": id,
                "name": name,
                "description": format!("{name} 热门搜索")
            })
        })
        .collect();

    (StatusCode::OK, Json(platforms)).into_response()
}
