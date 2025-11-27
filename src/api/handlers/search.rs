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

//! 搜索处理器
//!
//! 处理搜索相关的 API 请求

use axum::{
    extract::{State, Query, Json},
    response::{IntoResponse, Response},
    http::StatusCode,
};

use crate::api::on::ApiState;
use crate::api::types::{ApiSearchRequest, ApiSearchResponse, ApiSearchResultItem, ApiErrorResponse};
use crate::search::SearchRequest;

/// 处理 GET 搜索请求
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
    let search_query = params.to_search_query()
        .map_err(|e| format!("参数错误: {}", e))?;

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
    };

    // 执行搜索
    let response = state.search.search(&request).await?;
    
    // 转换结果 - 收集所有结果
    let mut results = Vec::new();
    for search_result in &response.results {
        for item in &search_result.items {
            results.push(ApiSearchResultItem {
                title: item.title.clone(),
                url: item.url.clone(),
                description: Some(item.content.clone()),
                engine: search_result.engine_name.clone(),
                score: Some(item.score),
            });
        }
    }
    
    // 按分数降序排序，确保最相关的结果在前面
    results.sort_by(|a, b| {
        let score_a = a.score.unwrap_or(0.0);
        let score_b = b.score.unwrap_or(0.0);
        score_b.partial_cmp(&score_a).unwrap_or(std::cmp::Ordering::Equal)
    });
    
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
