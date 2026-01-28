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

//! RSS API 处理器
//!
//! 处理 RSS feed 相关的 API 请求

use crate::api::handlers::static_files::get_rss_template_dir;
use crate::api::on::ApiState;
use crate::api::types::ApiErrorResponse;

use axum::{
    extract::{Json, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use seesea_rss::RssParser;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use utoipa::ToSchema;

/// 存储的 RSS Feed
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct StoredFeed {
    /// Feed 名称/分类
    pub name: String,
    /// Feed URL
    pub url: String,
    /// 添加时间戳
    pub added_at: u64,
}

/// RSS 存储管理器
#[derive(Clone)]
pub struct RssStorage {
    feeds: Arc<RwLock<HashMap<String, StoredFeed>>>,
}

impl RssStorage {
    /// 创建新的存储
    pub fn new() -> Self {
        Self {
            feeds: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 添加 feed
    pub async fn add_feed(&self, name: String, url: String) {
        let mut feeds = self.feeds.write().await;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        feeds.insert(
            name.clone(),
            StoredFeed {
                name,
                url,
                added_at: now,
            },
        );
    }

    /// 获取所有 feeds
    pub async fn get_all_feeds(&self) -> Vec<StoredFeed> {
        let feeds = self.feeds.read().await;
        feeds.values().cloned().collect()
    }

    /// 获取 feeds 数量
    pub async fn count(&self) -> usize {
        let feeds = self.feeds.read().await;
        feeds.len()
    }
}

impl Default for RssStorage {
    fn default() -> Self {
        Self::new()
    }
}

/// RSS Feed 请求
#[derive(Debug, Deserialize, utoipa::ToSchema)]
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
#[derive(Debug, Serialize, ToSchema)]
pub struct RssFeedResponse {
    pub meta: RssFeedMeta,
    pub items: Vec<RssFeedItemResponse>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct RssFeedMeta {
    pub title: Option<String>,
    pub description: Option<String>,
    pub link: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct RssFeedItemResponse {
    pub title: String,
    pub link: String,
    pub description: Option<String>,
    pub author: Option<String>,
    pub published: Option<String>,
    pub categories: Vec<String>,
}

/// 模板添加请求
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct TemplateAddRequest {
    /// 模板名称
    pub name: String,
    /// 要添加的分类（为空则添加全部）
    #[serde(default)]
    pub categories: Vec<String>,
}

/// 模板添加响应
#[derive(Debug, Serialize, ToSchema)]
pub struct TemplateAddResponse {
    /// 是否成功
    pub success: bool,
    /// 响应消息
    pub message: String,
    /// 成功添加的feeds
    pub added_feeds: Vec<String>,
    /// 失败的feeds
    pub failed_feeds: Vec<String>,
}

/// 处理获取RSS feeds列表请求
#[utoipa::path(
    get,
    path = "/rss/feeds",
    responses(
        (status = 200, description = "获取成功"),
    ),
    tag = "rss"
)]
pub async fn handle_rss_feeds_list(State(state): State<ApiState>) -> Response {
    // 从 ApiState 获取 RSS 存储
    let feeds = state.rss_storage.get_all_feeds().await;

    let response = serde_json::json!({
        "feeds": feeds,
        "total": feeds.len()
    });

    (StatusCode::OK, Json(response)).into_response()
}

/// 处理获取特定RSS feed请求
#[utoipa::path(
    post,
    path = "/rss/fetch",
    request_body = RssFetchRequest,
    responses(
        (status = 200, description = "获取成功"),
        (status = 400, description = "参数错误", body = ApiErrorResponse),
        (status = 502, description = "网络错误", body = ApiErrorResponse),
    ),
    tag = "rss"
)]
pub async fn handle_rss_fetch(
    State(_state): State<ApiState>,
    Json(request): Json<RssFetchRequest>,
) -> Response {
    // use crate::rss::parser::RssParser;

    // 获取 RSS feed 内容
    let feed_content = match reqwest::get(&request.url).await {
        Ok(response) => match response.text().await {
            Ok(text) => text,
            Err(e) => {
                let error = ApiErrorResponse {
                    code: "FETCH_ERROR".to_string(),
                    message: format!("Failed to read RSS feed content: {}", e),
                    details: None,
                };
                return (StatusCode::BAD_GATEWAY, Json(error)).into_response();
            }
        },
        Err(e) => {
            let error = ApiErrorResponse {
                code: "NETWORK_ERROR".to_string(),
                message: format!("Failed to fetch RSS feed: {}", e),
                details: None,
            };
            return (StatusCode::BAD_GATEWAY, Json(error)).into_response();
        }
    };

    // 解析 RSS feed
    let parser = RssParser::new();
    let feed = match parser.parse_rss2(&feed_content) {
        Ok(feed) => feed,
        Err(e) => {
            let error = ApiErrorResponse {
                code: "PARSE_ERROR".to_string(),
                message: format!("Failed to parse RSS feed: {}", e),
                details: None,
            };
            return (StatusCode::BAD_REQUEST, Json(error)).into_response();
        }
    };

    // 应用过滤器和限制
    let mut items = feed.items;

    // 根据关键词过滤
    if !request.filter_keywords.is_empty() {
        items.retain(|item| {
            let title = item.title.to_lowercase();
            let description = item.description.as_deref().unwrap_or("").to_lowercase();
            request.filter_keywords.iter().any(|keyword| {
                let keyword_lower = keyword.to_lowercase();
                title.contains(&keyword_lower) || description.contains(&keyword_lower)
            })
        });
    }

    // 限制项目数量
    if let Some(max_items) = request.max_items {
        items.truncate(max_items);
    }

    let response = RssFeedResponse {
        meta: RssFeedMeta {
            title: Some(feed.meta.title),
            description: Some(feed.meta.description.unwrap_or_default()),
            link: Some(feed.meta.link),
        },
        items: items
            .into_iter()
            .map(|item| RssFeedItemResponse {
                title: item.title,
                link: item.link,
                description: item.description,
                author: item.author,
                published: item.pub_date,
                categories: item.categories,
            })
            .collect(),
    };

    Json(response).into_response()
}

/// 处理获取RSS模板列表请求
#[utoipa::path(
    get,
    path = "/rss/templates",
    responses(
        (status = 200, description = "获取成功"),
    ),
    tag = "rss"
)]
pub async fn handle_rss_templates_list(State(_state): State<ApiState>) -> Response {
    // 动态读取 rss/template 目录下的所有 .see 文件
    let template_dir = get_rss_template_dir();
    let mut templates = Vec::new();

    tracing::debug!("Looking for RSS templates in: {}", template_dir.display());

    match std::fs::read_dir(&template_dir) {
        Ok(entries) => {
            for entry in entries.flatten() {
                if let Ok(file_type) = entry.file_type()
                    && file_type.is_file()
                    && let Some(file_name) = entry.file_name().to_str()
                {
                    // 只包含 .see 文件，并去掉扩展名
                    if file_name.ends_with(".rss.see") {
                        let template_name = file_name.trim_end_matches(".rss.see");
                        templates.push(template_name.to_string());
                    }
                }
            }
            tracing::info!(
                "Found {} RSS templates in {}",
                templates.len(),
                template_dir.display()
            );
        }
        Err(e) => {
            tracing::warn!(
                "Failed to read RSS template directory {}: {}",
                template_dir.display(),
                e
            );
            // 如果读取失败，返回空列表
        }
    }

    // 按字母顺序排序
    templates.sort();

    (StatusCode::OK, Json(templates)).into_response()
}

/// 处理从模板添加RSS feeds请求
#[utoipa::path(
    post,
    path = "/rss/template/add",
    request_body = TemplateAddRequest,
    responses(
        (status = 200, description = "添加成功"),
        (status = 404, description = "模板不存在", body = ApiErrorResponse),
    ),
    tag = "rss"
)]
pub async fn handle_rss_template_add(
    State(state): State<ApiState>,
    Json(request): Json<TemplateAddRequest>,
) -> Response {
    // 读取模板文件
    let template_dir = get_rss_template_dir();
    let template_path = template_dir.join(format!("{}.rss.see", request.name));

    tracing::debug!("Looking for RSS template at: {}", template_path.display());

    let template_content = match std::fs::read_to_string(&template_path) {
        Ok(content) => {
            tracing::info!("Loaded RSS template from: {}", template_path.display());
            content
        }
        Err(e) => {
            let error = ApiErrorResponse {
                code: "TEMPLATE_NOT_FOUND".to_string(),
                message: format!("Template '{}' not found", request.name),
                details: Some(format!("Path: {}, Error: {}", template_path.display(), e)),
            };
            return (StatusCode::NOT_FOUND, Json(error)).into_response();
        }
    };

    // 解析模板内容（INI 格式）
    let mut added_feeds = Vec::new();
    let mut current_section = String::new();

    for line in template_content.lines() {
        let line = line.trim();
        // 跳过空行和注释
        if line.is_empty() || line.starts_with('#') || line.starts_with("//") {
            continue;
        }

        // 检查是否是节标题 [section]
        if line.starts_with('[') && line.ends_with(']') {
            current_section = line[1..line.len() - 1].to_string();
            continue;
        }

        // 只在 [feeds] 节中解析 feeds
        if current_section != "feeds" {
            continue;
        }

        // 解析格式: key = "URL" 或 key = URL
        if let Some((key, value)) = line.split_once('=') {
            let key = key.trim();
            let value = value.trim().trim_matches('"').trim_matches('\'');

            // 如果指定了特定分类，只添加匹配的
            if !request.categories.is_empty() && !request.categories.contains(&key.to_string()) {
                continue;
            }

            // 添加到存储
            if !value.is_empty() && (value.starts_with("http://") || value.starts_with("https://"))
            {
                state
                    .rss_storage
                    .add_feed(key.to_string(), value.to_string())
                    .await;
                added_feeds.push(format!("{}: {}", key, value));
            }
        }
    }

    let response = TemplateAddResponse {
        success: true,
        message: format!(
            "Successfully loaded {} feeds from template '{}'",
            added_feeds.len(),
            request.name
        ),
        added_feeds,
        failed_feeds: vec![],
    };

    (StatusCode::OK, Json(response)).into_response()
}
