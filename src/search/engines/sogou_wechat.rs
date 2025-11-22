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

use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use std::error::Error;
use regex::Regex;

use crate::derive::{
    EngineCapabilities, EngineInfo, EngineStatus, EngineType,
    ResultType, SearchEngine, SearchQuery, SearchResult,
    SearchResultItem, AboutInfo, RequestResponseEngine, RequestParams,
};
use crate::net::client::HttpClient;
use crate::net::types::{NetworkConfig, RequestOptions};
use super::utils::build_query_string_owned;

pub struct SogouWeChatEngine {
    info: EngineInfo,
    client: Arc<HttpClient>,
}

impl SogouWeChatEngine {
    pub fn new() -> Self {
        let client = HttpClient::new(NetworkConfig::default())
            .unwrap_or_else(|_| panic!("Failed to create HTTP client"));
        Self::with_client(Arc::new(client))
    }

    pub fn with_client(client: Arc<HttpClient>) -> Self {
        Self {
            info: EngineInfo {
                name: "Sogou WeChat".to_string(),
                engine_type: EngineType::News,
                description: "Sogou WeChat - Search WeChat articles".to_string(),
                status: EngineStatus::Active,
                categories: vec!["news".to_string(), "social".to_string()],
                capabilities: EngineCapabilities {
                    result_types: vec![ResultType::News],
                    supported_params: vec!["page".to_string()],
                    max_page_size: 10,
                    supports_pagination: true,
                    supports_time_range: false,
                    supports_language_filter: false,
                    supports_region_filter: false,
                    supports_safe_search: false,
                    rate_limit: Some(30),
                },
                about: AboutInfo {
                    website: Some("https://weixin.sogou.com/".to_string()),
                    wikidata_id: Some("Q7554565".to_string()), // Same as Sogou main
                    official_api_documentation: None,
                    use_official_api: false,
                    require_api_key: false,
                    results: "HTML".to_string(),
                },
                shortcut: Some("sogou wx".to_string()),
                timeout: Some(10),
                disabled: false,
                inactive: false,
                version: Some("1.0.0".to_string()),
                last_checked: None,
                using_tor_proxy: false,
                display_error_messages: true,
                tokens: Vec::new(),
                max_page: 10,
            },
            client,
        }
    }

    fn parse_html_results(html: &str) -> Result<Vec<SearchResultItem>, Box<dyn Error + Send + Sync>> {
        use scraper::{Html, Selector};

        if html.is_empty() {
            return Ok(Vec::new());
        }

        let document = Html::parse_document(html);
        let mut items = Vec::with_capacity(10);

        let result_selector = Selector::parse("li[id*=\"sogou_vr_\"]")
            .or_else(|_| Selector::parse("li.results-item"))
            .or_else(|_| Selector::parse("div.results"))
            .expect("valid selector");

        for result in document.select(&result_selector) {
            let title_selector = Selector::parse("h3 a")
                .expect("valid selector");
            let title_elem = result.select(&title_selector).next();

            if title_elem.is_none() {
                continue;
            }

            let title_elem = title_elem.unwrap();
            let title = title_elem.text().collect::<String>().trim().to_string();

            if title.is_empty() {
                continue;
            }
 
            let url = title_elem.value().attr("href")
                .unwrap_or("")
                .to_string();

            // Handle redirect URLs: if url.startswith("/link?url="):
            if url.starts_with("/link?url=") {
                // For now, skip redirects as they require special handling
                continue;
            }

            if url.is_empty() {
                continue;
            }

            let content_selector = Selector::parse("p.txt-info")
                .or_else(|_| Selector::parse("p[class*=\"txt-info\"]"))
                .expect("valid selector");
            let content = result.select(&content_selector).next()
                .map(|c| c.text().collect::<String>().trim().to_string())
                .unwrap_or_default();

            let thumbnail = result.select(&Selector::parse("div.img-box a img").expect("valid selector")).next()
                .and_then(|img| img.value().attr("src"))
                .map(|s| {
                    if s.starts_with("//") {
                        format!("https:{}", s)
                    } else if !s.starts_with("http") {
                        format!("https://weixin.sogou.com{}", s)
                    } else {
                        s.to_string()
                    }
                });

            let mut published_date = None;
            let script_selector = Selector::parse("script")
                .expect("valid selector");
            for script in result.select(&script_selector) {
                let script_text = script.text().collect::<String>();
                if script_text.contains("timeConvert") {
                    if let Ok(re) = Regex::new(r"timeConvert\('(\d+)'\)") {
                        if let Some(caps) = re.captures(&script_text) {
                            if let Some(timestamp) = caps.get(1) {
                                if let Ok(ts) = timestamp.as_str().parse::<i64>() {
                                    // Convert Unix timestamp to datetime (simplified)
                                    published_date = Some(chrono::DateTime::from_timestamp(ts, 0));
                                }
                            }
                        }
                    }
                }
            }

            items.push(SearchResultItem {
                title,
                url: url.clone(),
                content,
                display_url: Some(url),
                site_name: Some("WeChat".to_string()),
                score: 1.0,
                result_type: ResultType::News,
                thumbnail,
                published_date: published_date.flatten(),
                template: None,
                metadata: {
                    let mut meta = HashMap::new();
                    meta.insert("source".to_string(), "WeChat".to_string());
                    meta
                },
            });
        }

        Ok(items)
    }
}

impl Default for SogouWeChatEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SearchEngine for SogouWeChatEngine {
    fn info(&self) -> &EngineInfo {
        &self.info
    }

    async fn search(&self, query: &SearchQuery) -> Result<SearchResult, Box<dyn Error + Send + Sync>> {
        <Self as RequestResponseEngine>::search(self, query).await
    }

    async fn is_available(&self) -> bool {
        self.client.get("https://weixin.sogou.com", None).await.is_ok()
    }
}

#[async_trait]
impl RequestResponseEngine for SogouWeChatEngine {
    type Response = String;

    fn request(&self, query: &str, params: &mut RequestParams) -> Result<(), Box<dyn Error + Send + Sync>> {

        let query_params = vec![
            ("query", query.to_string()),
            ("page", params.pageno.to_string()),
            ("type", "2".to_string()), // Type 2 for WeChat articles
        ];

        // Build URL with optimized query string
        let query_string = build_query_string_owned(query_params.into_iter());

        params.url = Some(format!("https://weixin.sogou.com/weixin?{}", query_string));
        params.method = "GET".to_string();

        Ok(())
    }

    async fn fetch(&self, params: &RequestParams) -> Result<Self::Response, Box<dyn Error + Send + Sync>> {
        let url = params.url.as_ref().ok_or("URL not set")?;

        let mut options = RequestOptions::default();
        // 使用配置的默认超时时间

        for (key, value) in &params.headers {
            options.headers.push((key.clone(), value.clone()));
        }

        let response = self.client.get(url, Some(options)).await
            .map_err(|e| format!("Request failed: {}", e))?;

        response.text().await.map_err(|e| format!("Failed to read response: {}", e).into())
    }

    fn response(&self, resp: Self::Response) -> Result<Vec<SearchResultItem>, Box<dyn Error + Send + Sync>> {
        Self::parse_html_results(&resp)
    }
}