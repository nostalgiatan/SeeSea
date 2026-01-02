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

use async_trait::async_trait;
use rand::Rng;
use std::collections::HashMap;
use std::error::Error;
use std::sync::Arc;

use super::utils::build_query_string_owned;
use seesea_derive::{
    AboutInfo, EngineCapabilities, EngineInfo, EngineStatus, EngineType, RequestParams,
    RequestResponseEngine, ResultType, SearchResultItem,
};

// 使用宏定义引擎结构体和基本方法
define_engine! {
    BilibiliEngine,
    EngineInfo {
        name: "Bilibili".to_string(),
        engine_type: EngineType::Video,
        description: "Bilibili - Chinese video sharing website".to_string(),
        status: EngineStatus::Active,
        categories: vec!["videos".to_string()],
        capabilities: EngineCapabilities {
            result_types: vec![ResultType::Video],
            supported_params: vec!["page".to_string()],
            max_page_size: 20,
            supports_pagination: true,
            supports_time_range: false,
            supports_language_filter: false,
            supports_region_filter: false,
            supports_safe_search: false,
            rate_limit: Some(30),
        },
        about: AboutInfo {
            website: Some("https://www.bilibili.com".to_string()),
            wikidata_id: Some("Q3077586".to_string()),
            official_api_documentation: None,
            use_official_api: false,
            require_api_key: false,
            results: "JSON".to_string(),
        },
        shortcut: Some("bili".to_string()),
        timeout: Some(10),
        disabled: false,
        inactive: false,
        version: Some("1.0.0".to_string()),
        last_checked: None,
        using_tor_proxy: false,
        display_error_messages: true,
        tokens: Vec::new(),
        max_page: 10,
    }
}

impl BilibiliEngine {
    fn parse_json_results(
        json_str: &str,
    ) -> Result<Vec<SearchResultItem>, Box<dyn Error + Send + Sync>> {
        use serde_json::Value;

        let json: Value = serde_json::from_str(json_str)?;
        let mut items = Vec::with_capacity(20);

        if let Some(data) = json.get("data")
            && let Some(results) = data.get("result")
            && let Some(result_array) = results.as_array()
        {
            for item in result_array {
                let raw_title = item
                    .get("title")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();

                // 提取keywords并清理HTML标签
                let (title, keywords) = extract_keywords_and_clean_html(raw_title);

                if title.is_empty() {
                    continue;
                }

                let url = item
                    .get("arcurl")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();

                if url.is_empty() {
                    continue;
                }

                let thumbnail = item.get("pic").and_then(|v| v.as_str()).map(|s| {
                    if s.starts_with("//") || !s.starts_with("http") {
                        format!("https:{s}")
                    } else {
                        s.to_string()
                    }
                });

                let content = item
                    .get("description")
                    .and_then(|v| v.as_str())
                    .map(strip_html_entities)
                    .unwrap_or_default();

                let author = item.get("author").and_then(|v| v.as_str()).unwrap_or("");

                let video_id = item.get("aid").and_then(|v| v.as_i64()).unwrap_or(0);

                let published_date = item
                    .get("pubdate")
                    .and_then(|v| v.as_i64())
                    .and_then(|ts| chrono::DateTime::from_timestamp(ts, 0));

                let duration_str = item.get("duration").and_then(|v| v.as_str()).unwrap_or("");

                let iframe_url = format!(
                    "https://player.bilibili.com/player.html?aid={video_id}&high_quality=1&autoplay=false&danmaku=0"
                );

                let mut metadata = HashMap::new();
                metadata.insert("author".to_string(), author.to_string());
                metadata.insert("length".to_string(), duration_str.to_string());
                metadata.insert("iframe_src".to_string(), iframe_url);

                // 添加keywords到metadata
                if !keywords.is_empty() {
                    metadata.insert("keywords".to_string(), keywords.join(","));
                }

                items.push(SearchResultItem {
                    title,
                    url: url.clone(),
                    content,
                    display_url: Some(url),
                    site_name: Some("Bilibili".to_string()),
                    score: 1.0,
                    result_type: ResultType::Video,
                    thumbnail,
                    published_date,
                    template: Some("videos.html".to_string()),
                    metadata,
                });
            }
        }

        Ok(items)
    }

    fn generate_bilibili_cookies() -> HashMap<String, String> {
        let mut rng = rand::thread_rng();

        let buvid3: String = (0..16)
            .map(|_| {
                let chars = b"0123456789abcdef";
                chars[rng.gen_range(0..16)] as char
            })
            .collect::<String>()
            + "infoc";

        HashMap::from([
            ("innersign".to_string(), "0".to_string()),
            ("buvid3".to_string(), buvid3),
            ("i-wanna-go-back".to_string(), "-1".to_string()),
            ("b_ut".to_string(), "7".to_string()),
            ("FEED_LIVE_VERSION".to_string(), "V8".to_string()),
            ("header_theme_version".to_string(), "undefined".to_string()),
            ("home_feed_column".to_string(), "4".to_string()),
        ])
    }
}

#[async_trait]
impl RequestResponseEngine for BilibiliEngine {
    type Response = String;

    fn request(
        &self,
        query: &str,
        params: &mut RequestParams,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        let base_url = "https://api.bilibili.com/x/web-interface/search/type";

        let query_params = vec![
            ("__refresh__", "true".to_string()),
            ("page", params.page.to_string()),
            ("page_size", "20".to_string()),
            ("single_column", "0".to_string()),
            ("keyword", query.to_string()),
            ("search_type", "video".to_string()),
        ];

        // Build URL with optimized query string
        let query_string = build_query_string_owned(query_params);

        params.url = Some(format!("{base_url}?{query_string}"));
        params.method = "GET".to_string();

        // Set cookies
        params.cookies = Self::generate_bilibili_cookies();

        // 添加必要的请求头
        params.headers.insert("User-Agent".to_string(), "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36".to_string());
        params.headers.insert(
            "Referer".to_string(),
            "https://www.bilibili.com/".to_string(),
        );
        params
            .headers
            .insert("Origin".to_string(), "https://www.bilibili.com".to_string());
        params.headers.insert(
            "Accept".to_string(),
            "application/json, text/plain, */*".to_string(),
        );
        params
            .headers
            .insert("Accept-Language".to_string(), "zh-CN,zh;q=0.9".to_string());
        params
            .headers
            .insert("X-Requested-With".to_string(), "XMLHttpRequest".to_string());
        params
            .headers
            .insert("Connection".to_string(), "keep-alive".to_string());

        Ok(())
    }

    async fn fetch(
        &self,
        params: &RequestParams,
    ) -> Result<Self::Response, Box<dyn Error + Send + Sync>> {
        // 使用通用引擎的fetch方法
        self.generic.fetch(params).await
    }

    fn response(
        &self,
        resp: Self::Response,
    ) -> Result<Vec<SearchResultItem>, Box<dyn Error + Send + Sync>> {
        Self::parse_json_results(&resp)
    }
}

// Helper function to strip HTML entities
fn strip_html_entities(text: &str) -> String {
    // Basic HTML entity stripping - this is simplified
    text.replace("&quot;", "\"")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&#39;", "'")
        .replace("&nbsp;", " ")
}

/// 提取keywords并清理HTML标签
fn extract_keywords_and_clean_html(html: &str) -> (String, Vec<String>) {
    // 使用正则表达式提取keyword高亮
    let keyword_regex = regex::Regex::new(r#"<em\s+class=["']?keyword["']?>([^<]+)</em>"#)
        .unwrap_or_else(|_| regex::Regex::new(r"<em[^>]*>([^<]+)</em>").unwrap());

    let mut keywords = Vec::new();
    let mut cleaned_html = html.to_string();

    // 提取所有keywords
    for caps in keyword_regex.captures_iter(html) {
        if let Some(keyword_match) = caps.get(1) {
            let keyword = strip_html_entities(keyword_match.as_str())
                .trim()
                .to_string();
            if !keyword.is_empty() {
                keywords.push(keyword);
            }
        }
    }

    // 移除所有HTML标签
    cleaned_html = regex::Regex::new(r#"<[^>]*>"#)
        .unwrap()
        .replace_all(&cleaned_html, "")
        .to_string();

    // 清理多余的空白和HTML实体
    cleaned_html = strip_html_entities(&cleaned_html);
    cleaned_html = cleaned_html
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");

    (cleaned_html, keywords)
}
