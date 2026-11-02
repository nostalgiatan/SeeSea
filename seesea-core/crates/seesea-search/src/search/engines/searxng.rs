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

//! SearXNG 上游来源
//!
//! 调用自部署 SearXNG 实例的 JSON 接口（`/search?format=json`），一次拿到它聚合的多个引擎的结果。
//! 实例需要在 `settings.yml` 的 `search.formats` 里启用 `json`。
//!
//! 每条结果保留 SearXNG 报告的命中引擎（`engines`）和各引擎名次（`positions`），放在
//! `metadata` 里：多个独立引擎同时命中同一页面，是后续融合排序的质量信号。

use async_trait::async_trait;
use std::collections::HashMap;
use std::error::Error;
use std::sync::Arc;
use std::time::Instant;

use seesea_config::RequestOptions;
use seesea_derive::{
    AboutInfo, EngineCapabilities, EngineInfo, EngineStatus, EngineType, ResultType, SearchEngine,
    SearchQuery, SearchResult, SearchResultItem, TimeRange,
};
use serde::Deserialize;

/// 环境变量：SearXNG 实例地址，例如 `http://127.0.0.1:8888`
pub const SEARXNG_URL_ENV: &str = "SEESEA_SEARXNG_URL";

/// SearXNG 引擎
pub struct SearxngEngine {
    info: EngineInfo,
    base_url: String,
    client: Arc<seesea_net::client::HttpClient>,
}

impl SearxngEngine {
    /// 使用共享 HTTP 客户端创建；`base_url` 末尾的斜杠会被去掉
    pub fn with_client(base_url: &str, client: Arc<seesea_net::client::HttpClient>) -> Self {
        Self {
            info: engine_info(),
            base_url: base_url.trim_end_matches('/').to_string(),
            client,
        }
    }

    /// 读取 [`SEARXNG_URL_ENV`]；未设置或为空时返回 `None`
    pub fn from_env(client: Arc<seesea_net::client::HttpClient>) -> Option<Self> {
        let url = std::env::var(SEARXNG_URL_ENV).ok()?;
        let url = url.trim();
        (!url.is_empty()).then(|| Self::with_client(url, client))
    }

    fn request_url(&self, query: &SearchQuery) -> String {
        let mut url = format!(
            "{}/search?format=json&q={}&pageno={}&categories={}",
            self.base_url,
            urlencoding::encode(&query.query),
            query.page.max(1),
            categories(query.engine_type),
        );
        if let Some(range) = time_range(query.time_range.as_ref()) {
            url.push_str("&time_range=");
            url.push_str(range);
        }
        if let Some(lang) = query.language.as_deref().filter(|l| !l.is_empty() && *l != "auto") {
            url.push_str("&language=");
            url.push_str(&urlencoding::encode(lang));
        }
        url
    }
}

fn engine_info() -> EngineInfo {
    EngineInfo {
        name: "SearXNG".to_string(),
        engine_type: EngineType::General,
        description: "自部署 SearXNG 实例聚合的多引擎结果".to_string(),
        status: EngineStatus::Active,
        categories: vec!["general".to_string(), "web".to_string()],
        capabilities: EngineCapabilities {
            result_types: vec![ResultType::Web, ResultType::News, ResultType::Image, ResultType::Video],
            supported_params: vec!["time_range".to_string(), "language".to_string()],
            max_page_size: 20,
            supports_pagination: true,
            supports_time_range: true,
            supports_language_filter: true,
            supports_region_filter: false,
            supports_safe_search: false,
            rate_limit: None,
        },
        about: AboutInfo {
            website: Some("https://docs.searxng.org".to_string()),
            wikidata_id: None,
            official_api_documentation: Some("https://docs.searxng.org/dev/search_api.html".to_string()),
            use_official_api: true,
            require_api_key: false,
            results: "JSON".to_string(),
        },
        shortcut: Some("sx".to_string()),
        timeout: Some(15),
        disabled: false,
        inactive: false,
        version: Some("1.0.0".to_string()),
        last_checked: None,
        using_tor_proxy: false,
        display_error_messages: true,
        tokens: Vec::new(),
        max_page: 20,
    }
}

fn categories(t: EngineType) -> &'static str {
    match t {
        EngineType::Image => "images",
        EngineType::Video => "videos",
        EngineType::News => "news",
        EngineType::Academic => "science",
        EngineType::Code => "it",
        EngineType::Music => "music",
        _ => "general",
    }
}

/// SearXNG 没有"过去一小时"，向上取到一天
fn time_range(range: Option<&TimeRange>) -> Option<&'static str> {
    match range? {
        TimeRange::Any => None,
        TimeRange::Hour | TimeRange::Day => Some("day"),
        TimeRange::Week => Some("week"),
        TimeRange::Month => Some("month"),
        TimeRange::Year => Some("year"),
    }
}

#[derive(Deserialize)]
struct Response {
    #[serde(default)]
    number_of_results: f64,
    #[serde(default)]
    results: Vec<Item>,
    #[serde(default)]
    suggestions: Vec<String>,
    #[serde(default)]
    unresponsive_engines: Vec<Vec<String>>,
}

#[derive(Deserialize)]
struct Item {
    url: String,
    #[serde(default)]
    title: String,
    #[serde(default)]
    content: String,
    #[serde(default)]
    engines: Vec<String>,
    #[serde(default)]
    positions: Vec<u32>,
    #[serde(default)]
    score: f64,
    #[serde(default)]
    category: String,
    #[serde(default, rename = "publishedDate")]
    published_date: Option<String>,
    #[serde(default)]
    thumbnail: Option<String>,
    #[serde(default)]
    img_src: Option<String>,
}

/// 解析 SearXNG 的 JSON 响应；不触网，方便用固定样本测试
pub fn parse_response(body: &str) -> Result<SearchResult, Box<dyn Error + Send + Sync>> {
    let resp: Response = serde_json::from_str(body)?;
    let items = resp
        .results
        .into_iter()
        .filter(|r| !r.url.is_empty())
        .map(|r| {
            let mut metadata = HashMap::new();
            if !r.engines.is_empty() {
                metadata.insert("engines".to_string(), r.engines.join(","));
            }
            if !r.positions.is_empty() {
                let positions: Vec<String> = r.positions.iter().map(u32::to_string).collect();
                metadata.insert("positions".to_string(), positions.join(","));
            }
            if !r.category.is_empty() {
                metadata.insert("category".to_string(), r.category.clone());
            }
            let result_type = match r.category.as_str() {
                "news" => ResultType::News,
                "images" => ResultType::Image,
                "videos" => ResultType::Video,
                "science" => ResultType::Academic,
                "it" => ResultType::Code,
                _ => ResultType::Web,
            };
            SearchResultItem {
                title: r.title,
                content: r.content,
                display_url: None,
                site_name: None,
                score: r.score,
                result_type,
                thumbnail: r.thumbnail.or(r.img_src),
                published_date: r.published_date.as_deref().and_then(parse_date),
                template: None,
                metadata,
                url: r.url,
            }
        })
        .collect();

    let mut metadata = HashMap::new();
    if !resp.unresponsive_engines.is_empty() {
        let names: Vec<String> = resp
            .unresponsive_engines
            .iter()
            .filter_map(|e| e.first().cloned())
            .collect();
        metadata.insert("unresponsive_engines".to_string(), names.join(","));
    }
    Ok(SearchResult {
        engine_name: "SearXNG".to_string(),
        total_results: (resp.number_of_results > 0.0).then_some(resp.number_of_results as usize),
        elapsed_ms: 0,
        items,
        pagination: None,
        suggestions: resp.suggestions,
        metadata,
    })
}

/// SearXNG 的日期没有统一格式：有带时区的 RFC 3339，也有不带时区的 `2025-03-04T10:15:00`
fn parse_date(s: &str) -> Option<chrono::DateTime<chrono::Utc>> {
    use chrono::{DateTime, NaiveDateTime, Utc};
    if let Ok(d) = DateTime::parse_from_rfc3339(s) {
        return Some(d.with_timezone(&Utc));
    }
    NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S")
        .ok()
        .map(|n| n.and_utc())
}

#[async_trait]
impl SearchEngine for SearxngEngine {
    fn info(&self) -> &EngineInfo {
        &self.info
    }

    async fn search(
        &self,
        query: &SearchQuery,
    ) -> Result<SearchResult, Box<dyn Error + Send + Sync>> {
        let start = Instant::now();
        let url = self.request_url(query);
        let options = RequestOptions {
            timeout_secs: self.info.timeout,
            ..Default::default()
        };
        let response = self.client.get(&url, Some(options)).await?;
        let status = response.status();
        if !status.is_success() {
            return Err(format!("SearXNG returned {status}; is the json format enabled?").into());
        }
        let body = response.text().await?;
        let mut result = parse_response(&body)?;
        result.elapsed_ms = start.elapsed().as_millis() as u64;
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = include_str!("fixtures/searxng_sample.json");

    #[test]
    fn parses_results_with_engine_agreement() {
        let r = parse_response(SAMPLE).unwrap();
        assert_eq!(r.items.len(), 3);
        assert_eq!(r.total_results, None, "number_of_results = 0 means unknown");
        assert_eq!(r.suggestions, vec!["tokio vs async-std", "rust futures"]);
        assert_eq!(r.metadata.get("unresponsive_engines").map(String::as_str), Some("startpage"));

        let first = &r.items[0];
        assert_eq!(first.url, "https://tokio.rs/");
        assert_eq!(first.metadata["engines"], "duckduckgo,brave,google");
        assert_eq!(first.metadata["positions"], "1,2,1");
        assert!(first.published_date.is_none());

        let second = &r.items[1];
        assert_eq!(second.metadata["engines"], "brave");
        assert_eq!(second.thumbnail.as_deref(), Some("https://blog.example.org/img/async.png"));
        let date = second.published_date.expect("naive timestamps are accepted");
        assert_eq!(date.to_rfc3339(), "2025-03-04T10:15:00+00:00");
    }

    #[test]
    fn tolerates_missing_optional_fields() {
        let r = parse_response(SAMPLE).unwrap();
        let bare = &r.items[2];
        assert_eq!(bare.content, "");
        assert_eq!(bare.score, 0.0);
    }

    #[test]
    fn skips_results_without_url_and_rejects_non_json() {
        let r = parse_response(r#"{"results":[{"url":"","title":"x"},{"url":"https://a.example/","title":"a"}]}"#).unwrap();
        assert_eq!(r.items.len(), 1);
        assert!(parse_response("<html>blocked</html>").is_err());
    }

    #[test]
    fn maps_query_to_parameters() {
        assert_eq!(categories(EngineType::News), "news");
        assert_eq!(categories(EngineType::Code), "it");
        assert_eq!(time_range(Some(&TimeRange::Hour)), Some("day"));
        assert_eq!(time_range(Some(&TimeRange::Any)), None);
        assert_eq!(time_range(None), None);
    }
}
