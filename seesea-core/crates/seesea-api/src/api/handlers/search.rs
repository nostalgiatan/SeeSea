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

//! 搜索处理器
//!
//! 处理搜索相关的 API 请求

use axum::{
    extract::{Json, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};

use crate::api::on::ApiState;
use crate::api::types::{
    ApiErrorResponse, ApiSearchRequest, ApiSearchResponse, ApiSearchResultItem,
};
use seesea_search::SearchRequest;

/// 执行搜索请求
///
/// 支持多引擎搜索，可指定引擎数量、语言、地区等参数
#[utoipa::path(
    get,
    path = "/search",
    params(
        ("query" = Option<String>, Query, description = "搜索查询字符串"),
        ("q" = Option<String>, Query, description = "搜索查询字符串（短参数名）"),
        ("engine_count" = Option<u32>, Query, description = "引擎数量"),
        ("page" = u32, Query, description = "页码"),
        ("page_size" = u32, Query, description = "每页结果数"),
        ("language" = Option<String>, Query, description = "语言过滤"),
        ("region" = Option<String>, Query, description = "地区过滤"),
        ("safe_search" = Option<String>, Query, description = "安全搜索级别"),
        ("time_range" = Option<String>, Query, description = "时间范围"),
        ("engines" = Option<String>, Query, description = "指定搜索引擎"),
        ("include_deepweb" = bool, Query, description = "是否包含深网搜索"),
    ),
    responses(
        (status = 200, description = "搜索成功", body = ApiSearchResponse),
        (status = 400, description = "参数错误", body = ApiErrorResponse),
        (status = 500, description = "服务器错误", body = ApiErrorResponse),
    ),
    tag = "search"
)]
pub async fn handle_search(
    State(state): State<ApiState>,
    Query(params): Query<ApiSearchRequest>,
) -> Response {
    match execute_search(&state, params).await {
        Ok(response) => (StatusCode::OK, Json(response)).into_response(),
        Err(e) => {
            let error = ApiErrorResponse {
                code: "SEARCH_ERROR".to_string(),
                message: "搜索失败".to_string(),
                details: Some(e.to_string()),
            };
            (StatusCode::INTERNAL_SERVER_ERROR, Json(error)).into_response()
        }
    }
}

/// 处理 POST 搜索请求
#[utoipa::path(
    post,
    path = "/search",
    request_body = ApiSearchRequest,
    responses(
        (status = 200, description = "搜索成功", body = ApiSearchResponse),
        (status = 400, description = "参数错误", body = ApiErrorResponse),
        (status = 500, description = "服务器错误", body = ApiErrorResponse),
    ),
    tag = "search"
)]
pub async fn handle_search_post(
    State(state): State<ApiState>,
    Json(params): Json<ApiSearchRequest>,
) -> Response {
    match execute_search(&state, params).await {
        Ok(response) => (StatusCode::OK, Json(response)).into_response(),
        Err(e) => {
            let error = ApiErrorResponse {
                code: "SEARCH_ERROR".to_string(),
                message: "搜索失败".to_string(),
                details: Some(e.to_string()),
            };
            (StatusCode::INTERNAL_SERVER_ERROR, Json(error)).into_response()
        }
    }
}

/// 执行搜索
async fn execute_search(
    state: &ApiState,
    params: ApiSearchRequest,
) -> Result<ApiSearchResponse, Box<dyn std::error::Error + Send + Sync>> {
    let start_time = std::time::Instant::now();

    // 转换为内部搜索查询
    let search_query = params
        .to_search_query()
        .map_err(|e| format!("参数错误: {e}"))?;

    // 获取引擎列表
    let engines = params.get_engines();

    // 创建搜索请求 - 设置合理的最大结果数以防止资源耗尽
    let request = SearchRequest {
        query: search_query,
        engines,
        timeout: None,
        max_results: Some(1000), // 限制最大结果数为1000
        force: false,
        cache_timeline: Some(3600),
        include_deepweb: params.include_deepweb, // 使用API参数
    };

    // 执行搜索
    let response = state.search.search(&request).await?;

    // 转换结果 - 收集所有结果
    // 预分配容量减少内存分配
    let total_items: usize = response.results.iter().map(|r| r.items.len()).sum();
    let mut results = Vec::with_capacity(total_items);

    for search_result in &response.results {
        for item in &search_result.items {
            // 将DateTime转换为ISO 8601字符串格式
            let published_date = item.published_date.as_ref().map(|dt| dt.to_rfc3339());

            results.push(ApiSearchResultItem {
                title: item.title.clone(),
                url: item.url.clone(),
                description: Some(item.content.clone()),
                engine: search_result.engine_name.clone(),
                score: Some(item.score),
                published_date,
            });
        }
    }

    // 不需要重复排序，因为搜索结果已经在search方法中排序过了

    let elapsed = start_time.elapsed().as_millis() as u64;

    // 获取实际的查询字符串
    let query_text = params.get_query().unwrap_or_default();

    // 返回所有结果，让前端进行分页
    let total_count = results.len();

    Ok(ApiSearchResponse {
        query: query_text,
        results,
        total_count,
        page: params.page,
        page_size: params.page_size,
        engines_used: response.engines_used,
        query_time_ms: elapsed,
        cached: response.cached,
    })
}
