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

use crate::api::on::ApiState;
use crate::api::types::ApiErrorResponse;
use axum::{
    extract::{Json, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};

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
pub async fn handle_rss_feeds_list(State(_state): State<ApiState>) -> Response {
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
    Json(request): Json<RssFetchRequest>,
) -> Response {
    use crate::rss::parser::RssParser;

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
    let feed = match parser.parse(&feed_content) {
        Ok(feed) => feed,
        Err(e) => {
            let error = ApiErrorResponse {
                code: "PARSE_ERROR".to_string(),
                message: format!("Failed to parse RSS feed: {}", e),
                details: None,
            };
            return (StatusCode::UNPROCESSABLE_ENTITY, Json(error)).into_response();
        }
    };

    // 转换为响应格式
    let meta = RssFeedMeta {
        title: Some(feed.meta.title),
        description: feed.meta.description,
        link: Some(feed.meta.link),
    };

    let max_items = request.max_items.unwrap_or(50);
    let items: Vec<RssFeedItemResponse> = feed
        .items
        .into_iter()
        .filter(|item| {
            // 如果有过滤关键词，检查标题或描述是否包含
            if request.filter_keywords.is_empty() {
                true
            } else {
                request.filter_keywords.iter().any(|keyword| {
                    item.title.to_lowercase().contains(&keyword.to_lowercase())
                        || item.description.as_ref().is_some_and(|desc| {
                            desc.to_lowercase().contains(&keyword.to_lowercase())
                        })
                })
            }
        })
        .take(max_items)
        .map(|item| RssFeedItemResponse {
            title: item.title,
            link: item.link,
            description: item.description,
            author: item.author,
            published: item.pub_date,
            categories: item.categories,
        })
        .collect();

    let response = RssFeedResponse { meta, items };

    (StatusCode::OK, Json(response)).into_response()
}

/// 处理获取RSS模板列表请求
pub async fn handle_rss_templates_list(State(_state): State<ApiState>) -> Response {
    // 动态读取 rss/template 目录下的所有 .see 文件
    let template_dir = "rss/template";
    let mut templates = Vec::new();

    match std::fs::read_dir(template_dir) {
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
        }
        Err(e) => {
            tracing::warn!("Failed to read RSS template directory: {}", e);
            // 如果读取失败，返回空列表
        }
    }

    // 按字母顺序排序
    templates.sort();

    (StatusCode::OK, Json(templates)).into_response()
}

/// 处理从模板添加RSS feeds请求
pub async fn handle_rss_template_add(
    State(_state): State<ApiState>,
    Json(request): Json<TemplateAddRequest>,
) -> Response {
    // 读取模板文件
    let template_path = format!("rss/template/{}.rss.see", request.name);

    let template_content = match std::fs::read_to_string(&template_path) {
        Ok(content) => content,
        Err(e) => {
            let error = ApiErrorResponse {
                code: "TEMPLATE_NOT_FOUND".to_string(),
                message: format!("Template '{}' not found", request.name),
                details: Some(e.to_string()),
            };
            return (StatusCode::NOT_FOUND, Json(error)).into_response();
        }
    };

    // 解析模板内容（简单的行解析）
    let mut added_feeds = Vec::new();

    for line in template_content.lines() {
        let line = line.trim();
        // 跳过空行和注释
        if line.is_empty() || line.starts_with('#') || line.starts_with("//") {
            continue;
        }

        // 解析格式: URL [分类]
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.is_empty() {
            continue;
        }

        let url = parts[0];
        let category = if parts.len() > 1 {
            parts[1].to_string()
        } else {
            "默认".to_string()
        };

        // 如果指定了特定分类，只添加匹配的
        if !request.categories.is_empty() && !request.categories.contains(&category) {
            continue;
        }

        // 添加到列表（实际应用中这里应该保存到数据库）
        added_feeds.push(url.to_string());
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
