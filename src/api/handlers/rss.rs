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

//! RSS API 处理器
//!
//! 处理 RSS feed 相关的 API 请求

use axum::{
    extract::{State, Json},
    response::{IntoResponse, Response},
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use crate::api::on::ApiState;
use crate::api::types::ApiErrorResponse;

/// RSS Feed 请求
#[derive(Debug, Deserialize)]
pub struct RssFetchRequest {
    /// Feed URL
    pub url: String,
    /// 最大项目数
    #[serde(default = "default_max_items")]
    pub max_items: Option<usize>,
    /// 过滤关键词
    #[serde(default)]
    pub filter_keywords: Vec<String>,
}

fn default_max_items() -> Option<usize> {
    Some(50)
}

/// RSS Feed 响应
#[derive(Debug, Serialize)]
pub struct RssFeedResponse {
    pub meta: RssFeedMeta,
    pub items: Vec<RssFeedItemResponse>,
}

#[derive(Debug, Serialize)]
pub struct RssFeedMeta {
    pub title: Option<String>,
    pub description: Option<String>,
    pub link: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct RssFeedItemResponse {
    pub title: String,
    pub link: String,
    pub description: Option<String>,
    pub author: Option<String>,
    pub published: Option<String>,
    pub categories: Vec<String>,
}

/// 模板添加请求
#[derive(Debug, Deserialize)]
pub struct TemplateAddRequest {
    /// 模板名称
    pub name: String,
    /// 要添加的分类（为空则添加全部）
    #[serde(default)]
    pub categories: Vec<String>,
}

/// 模板添加响应
#[derive(Debug, Serialize)]
pub struct TemplateAddResponse {
    /// 添加的feed数量
    pub count: usize,
    /// 成功添加的分类
    pub categories: Vec<String>,
}

/// 处理获取RSS feeds列表请求
pub async fn handle_rss_feeds_list(
    State(_state): State<ApiState>,
) -> Response {
    // TODO: 实现获取所有RSS feeds列表
    let response = serde_json::json!({
        "feeds": [],
        "total": 0
    });
    
    (StatusCode::OK, Json(response)).into_response()
}

/// 处理获取特定RSS feed请求
pub async fn handle_rss_fetch(
    State(_state): State<ApiState>,
    Json(_request): Json<RssFetchRequest>,
) -> Response {
    // TODO: 实现RSS feed获取逻辑
    let error = ApiErrorResponse {
        code: "NOT_IMPLEMENTED".to_string(),
        message: "RSS fetch not yet implemented".to_string(),
        details: None,
    };
    
    (StatusCode::NOT_IMPLEMENTED, Json(error)).into_response()
}

/// 处理获取RSS模板列表请求
pub async fn handle_rss_templates_list(
    State(_state): State<ApiState>,
) -> Response {
    // TODO: 实现RSS模板列表
    let templates = vec!["xinhua"];
    
    (StatusCode::OK, Json(templates)).into_response()
}

/// 处理从模板添加RSS feeds请求
pub async fn handle_rss_template_add(
    State(_state): State<ApiState>,
    Json(_request): Json<TemplateAddRequest>,
) -> Response {
    // TODO: 实现从模板添加feeds
    let error = ApiErrorResponse {
        code: "NOT_IMPLEMENTED".to_string(),
        message: "Template add not yet implemented".to_string(),
        details: None,
    };
    
    (StatusCode::NOT_IMPLEMENTED, Json(error)).into_response()
}
