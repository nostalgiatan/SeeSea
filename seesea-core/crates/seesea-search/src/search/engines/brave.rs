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

//! Brave Search 官方 API 来源
//!
//! 需要 API 密钥，从环境变量 [`BRAVE_API_KEY_ENV`] 读取；未设置时不会注册。
//! 响应格式依据官方文档编写，固定样本是按文档手写的，尚未用真实密钥验证过。

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

/// 环境变量：Brave Search API 密钥
pub const BRAVE_API_KEY_ENV: &str = "SEESEA_BRAVE_API_KEY";

const ENDPOINT: &str = "https://api.search.brave.com/res/v1/web/search";

/// Brave 单次请求最多返回的条数
const MAX_COUNT: usize = 20;
/// Brave 的 offset 是页索引，上限 9
const MAX_OFFSET: usize = 9;

pub struct BraveEngine {
    info: EngineInfo,
    api_key: String,
    client: Arc<seesea_net::client::HttpClient>,
}

impl BraveEngine {
    pub fn with_client(api_key: &str, client: Arc<seesea_net::client::HttpClient>) -> Self {
        Self { info: engine_info(), api_key: api_key.to_string(), client }
    }

    /// 读取 [`BRAVE_API_KEY_ENV`]；未设置或为空时返回 `None`
    pub fn from_env(client: Arc<seesea_net::client::HttpClient>) -> Option<Self> {
        let key = std::env::var(BRAVE_API_KEY_ENV).ok()?;
        let key = key.trim();
        (!key.is_empty()).then(|| Self::with_client(key, client))
    }

    fn request_url(&self, query: &SearchQuery) -> String {
        let count = query.page_size.clamp(1, MAX_COUNT);
        let offset = query.page.saturating_sub(1).min(MAX_OFFSET);
        let mut url = format!(
            "{ENDPOINT}?q={}&count={count}&offset={offset}",
            urlencoding::encode(&query.query)
        );
        if let Some(f) = freshness(query.time_range.as_ref()) {
            url.push_str("&freshness=");
            url.push_str(f);
        }
        if let Some(lang) = query.language.as_deref().filter(|l| !l.is_empty() && *l != "auto") {
            url.push_str("&search_lang=");
            url.push_str(&urlencoding::encode(lang));
        }
        url
    }
}

fn engine_info() -> EngineInfo {
    EngineInfo {
        name: "Brave".to_string(),
        engine_type: EngineType::General,
        description: "Brave Search 官方 API".to_string(),
        status: EngineStatus::Active,
        categories: vec!["general".to_string(), "web".to_string()],
        capabilities: EngineCapabilities {
            result_types: vec![ResultType::Web],
            supported_params: vec!["time_range".to_string(), "language".to_string()],
            max_page_size: MAX_COUNT,
            supports_pagination: true,
            supports_time_range: true,
            supports_language_filter: true,
            supports_region_filter: false,
            supports_safe_search: false,
            rate_limit: None,
        },
        about: AboutInfo {
            website: Some("https://search.brave.com".to_string()),
            wikidata_id: Some("Q22906900".to_string()),
            official_api_documentation: Some("https://api-dashboard.search.brave.com/app/documentation".to_string()),
            use_official_api: true,
            require_api_key: true,
            results: "JSON".to_string(),
        },
        shortcut: Some("br".to_string()),
        timeout: Some(10),
        disabled: false,
        inactive: false,
        version: Some("1.0.0".to_string()),
        last_checked: None,
        using_tor_proxy: false,
        display_error_messages: true,
        tokens: Vec::new(),
        max_page: MAX_OFFSET + 1,
    }
}

fn freshness(range: Option<&TimeRange>) -> Option<&'static str> {
    match range? {
        TimeRange::Any => None,
        TimeRange::Hour | TimeRange::Day => Some("pd"),
        TimeRange::Week => Some("pw"),
        TimeRange::Month => Some("pm"),
        TimeRange::Year => Some("py"),
    }
}

#[derive(Deserialize)]
struct Response {
    #[serde(default)]
    web: Option<Web>,
}

#[derive(Deserialize)]
struct Web {
    #[serde(default)]
    results: Vec<Item>,
}

#[derive(Deserialize)]
struct Item {
    #[serde(default)]
    title: String,
    url: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    page_age: Option<String>,
    #[serde(default)]
    language: Option<String>,
    #[serde(default)]
    profile: Option<Profile>,
    #[serde(default)]
    thumbnail: Option<Thumbnail>,
    #[serde(default)]
    extra_snippets: Vec<String>,
}

#[derive(Deserialize)]
struct Profile {
    #[serde(default)]
    name: String,
}

#[derive(Deserialize)]
struct Thumbnail {
    #[serde(default)]
    src: String,
}

/// Brave 的摘要里带 `<strong>` 之类的高亮标签，去掉后再用
fn strip_tags(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut in_tag = false;
    for c in s.chars() {
        match c {
            '<' => in_tag = true,
            '>' if in_tag => in_tag = false,
            _ if !in_tag => out.push(c),
            _ => {}
        }
    }
    out.replace("&amp;", "&").replace("&quot;", "\"").replace("&#x27;", "'").replace("&lt;", "<").replace("&gt;", ">")
}

/// 解析 Brave 的 JSON 响应；不触网，方便用固定样本测试
pub fn parse_response(body: &str) -> Result<SearchResult, Box<dyn Error + Send + Sync>> {
    let resp: Response = serde_json::from_str(body)?;
    let items = resp
        .web
        .map(|w| w.results)
        .unwrap_or_default()
        .into_iter()
        .filter(|r| !r.url.is_empty())
        .map(|r| {
            let mut metadata = HashMap::new();
            if let Some(lang) = r.language.filter(|l| !l.is_empty()) {
                metadata.insert("language".to_string(), lang);
            }
            if !r.extra_snippets.is_empty() {
                let extra: Vec<String> = r.extra_snippets.iter().map(|s| strip_tags(s)).collect();
                metadata.insert("extra_snippets".to_string(), extra.join("\n"));
            }
            SearchResultItem {
                title: strip_tags(&r.title),
                content: strip_tags(&r.description),
                display_url: None,
                site_name: r.profile.map(|p| p.name).filter(|n| !n.is_empty()),
                score: 0.0,
                result_type: ResultType::Web,
                thumbnail: r.thumbnail.map(|t| t.src).filter(|s| !s.is_empty()),
                published_date: r.page_age.as_deref().and_then(parse_date),
                template: None,
                metadata,
                url: r.url,
            }
        })
        .collect();
    Ok(SearchResult {
        engine_name: "Brave".to_string(),
        total_results: None,
        elapsed_ms: 0,
        items,
        pagination: None,
        suggestions: Vec::new(),
        metadata: HashMap::new(),
    })
}

fn parse_date(s: &str) -> Option<chrono::DateTime<chrono::Utc>> {
    use chrono::{DateTime, NaiveDateTime, Utc};
    if let Ok(d) = DateTime::parse_from_rfc3339(s) {
        return Some(d.with_timezone(&Utc));
    }
    NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S").ok().map(|n| n.and_utc())
}

#[async_trait]
impl SearchEngine for BraveEngine {
    fn info(&self) -> &EngineInfo {
        &self.info
    }

    async fn search(&self, query: &SearchQuery) -> Result<SearchResult, Box<dyn Error + Send + Sync>> {
        let start = Instant::now();
        let mut headers = HashMap::new();
        headers.insert("X-Subscription-Token".to_string(), self.api_key.clone());
        headers.insert("Accept".to_string(), "application/json".to_string());
        let options = RequestOptions { timeout_secs: self.info.timeout, headers: Some(headers), ..Default::default() };
        let response = self.client.get(&self.request_url(query), Some(options)).await?;
        let status = response.status();
        if !status.is_success() {
            return Err(format!("Brave API returned {status}").into());
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

    const SAMPLE: &str = include_str!("fixtures/brave_sample.json");

    #[test]
    fn parses_results_and_strips_highlight_tags() {
        let r = parse_response(SAMPLE).unwrap();
        assert_eq!(r.items.len(), 2);
        let first = &r.items[0];
        assert_eq!(first.title, "Tokio - An asynchronous Rust runtime");
        assert_eq!(first.content, "Tokio is an asynchronous runtime for the Rust programming language.");
        assert_eq!(first.site_name.as_deref(), Some("Tokio"));
        assert_eq!(first.thumbnail.as_deref(), Some("https://imgs.example/tokio.png"));
        assert_eq!(first.published_date.unwrap().to_rfc3339(), "2025-02-10T08:00:00+00:00");
        assert_eq!(first.metadata["extra_snippets"], "Build reliable network applications.");
        assert!(r.items[1].published_date.is_none());
    }

    #[test]
    fn empty_or_missing_web_section_gives_no_items() {
        assert!(parse_response(r#"{"type":"search"}"#).unwrap().items.is_empty());
        assert!(parse_response("not json").is_err());
    }

    #[test]
    fn clamps_paging_and_maps_freshness() {
        let client = Arc::new(seesea_net::client::HttpClient::new(Default::default()).unwrap());
        let engine = BraveEngine::with_client("k", client);
        let q = SearchQuery { query: "a b".into(), page_size: 100, page: 50, time_range: Some(TimeRange::Week), ..Default::default() };
        let url = engine.request_url(&q);
        assert!(url.contains("q=a%20b"), "{url}");
        assert!(url.contains("count=20") && url.contains("offset=9") && url.contains("freshness=pw"), "{url}");
    }
}
