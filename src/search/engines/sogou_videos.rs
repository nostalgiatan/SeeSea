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

use crate::derive::{
    EngineCapabilities, EngineInfo, EngineStatus, EngineType,
    ResultType, SearchEngine, SearchQuery, SearchResult,
    SearchResultItem, AboutInfo, RequestResponseEngine, RequestParams,
};
use crate::net::client::HttpClient;
use crate::net::types::{NetworkConfig, RequestOptions};
use super::utils::build_query_string_owned;

pub struct SogouVideosEngine {
    info: EngineInfo,
    client: Arc<HttpClient>,
}

impl SogouVideosEngine {
    pub fn new() -> Self {
        let client = HttpClient::new(NetworkConfig::default())
            .unwrap_or_else(|_| panic!("Failed to create HTTP client"));
        Self::with_client(Arc::new(client))
    }

    pub fn with_client(client: Arc<HttpClient>) -> Self {
        Self {
            info: EngineInfo {
                name: "Sogou Videos".to_string(),
                engine_type: EngineType::Video,
                description: "Sogou Videos - Chinese video search engine".to_string(),
                status: EngineStatus::Active,
                categories: vec!["videos".to_string()],
                capabilities: EngineCapabilities {
                    result_types: vec![ResultType::Video],
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
                    website: Some("https://v.sogou.com/".to_string()),
                    wikidata_id: Some("Q7554565".to_string()), // Same as Sogou main
                    official_api_documentation: None,
                    use_official_api: false,
                    require_api_key: false,
                    results: "HTML".to_string(),
                },
                shortcut: Some("sogou vid".to_string()),
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

        // Sogou video results - typical pattern for video listings
        let result_selector = Selector::parse("div.video-box")
            .or_else(|_| Selector::parse("div[class*=\"video\"]"))
            .or_else(|_| Selector::parse("li.vr-item"))
            .or_else(|_| Selector::parse("div.video-item"))
            .expect("valid selector");

        for result in document.select(&result_selector) {
            // Extract title from video title element
            let title_selector = Selector::parse("h3 a")
                .or_else(|_| Selector::parse("h4 a"))
                .or_else(|_| Selector::parse("a.video-title"))
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

            // Extract video URL
            let video_url = title_elem.value().attr("href")
                .unwrap_or("")
                .to_string();

            if video_url.is_empty() {
                continue;
            }

            // Extract thumbnail image
            let img_selector = Selector::parse("img")
                .expect("valid selector");
            let thumbnail_url = result.select(&img_selector).next()
                .and_then(|img| {
                    img.value().attr("src")
                        .or_else(|| img.value().attr("data-src"))
                        .or_else(|| img.value().attr("data-original"))
                })
                .map(|s| {
                    if s.starts_with("//") {
                        format!("https:{}", s)
                    } else if !s.starts_with("http") {
                        format!("https://v.sogou.com{}", s)
                    } else {
                        s.to_string()
                    }
                });

            // Extract description/content
            let content_selector = Selector::parse("p.desc")
                .or_else(|_| Selector::parse("p.video-desc"))
                .or_else(|_| Selector::parse("span.txt"))
                .expect("valid selector");
            let content = result.select(&content_selector).next()
                .map(|c| c.text().collect::<String>().trim().to_string())
                .unwrap_or_default();

            // Extract duration if available
            let duration_selector = Selector::parse("span.duration")
                .or_else(|_| Selector::parse("span.time"))
                .expect("valid selector");
            let duration = result.select(&duration_selector).next()
                .map(|d| d.text().collect::<String>().trim().to_string())
                .filter(|d| !d.is_empty());

            let mut metadata = HashMap::new();
            if let Some(dur) = duration {
                metadata.insert("duration".to_string(), dur);
            }

            items.push(SearchResultItem {
                title,
                url: video_url.clone(),
                content,
                display_url: Some(video_url),
                site_name: None,
                score: 1.0,
                result_type: ResultType::Video,
                thumbnail: thumbnail_url,
                published_date: None,
                template: None,
                metadata,
            });
        }

        Ok(items)
    }
}

impl Default for SogouVideosEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SearchEngine for SogouVideosEngine {
    fn info(&self) -> &EngineInfo {
        &self.info
    }

    async fn search(&self, query: &SearchQuery) -> Result<SearchResult, Box<dyn Error + Send + Sync>> {
        <Self as RequestResponseEngine>::search(self, query).await
    }

    async fn is_available(&self) -> bool {
        self.client.get("https://v.sogou.com", None).await.is_ok()
    }
}

#[async_trait]
impl RequestResponseEngine for SogouVideosEngine {
    type Response = String;

    fn request(&self, query: &str, params: &mut RequestParams) -> Result<(), Box<dyn Error + Send + Sync>> {
        // Sogou video search URL
        let query_params = vec![
            ("query", query.to_string()),
            ("page", params.pageno.to_string()),
        ];

        // Build URL with optimized query string
        let query_string = build_query_string_owned(query_params.into_iter());

        params.url = Some(format!("https://v.sogou.com/v?{}", query_string));
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