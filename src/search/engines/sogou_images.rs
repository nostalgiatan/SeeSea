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

pub struct SogouImagesEngine {
    info: EngineInfo,
    client: Arc<HttpClient>,
}

impl SogouImagesEngine {
    pub fn new() -> Self {
        let client = HttpClient::new(NetworkConfig::default())
            .unwrap_or_else(|_| panic!("Failed to create HTTP client"));
        Self::with_client(Arc::new(client))
    }

    pub fn with_client(client: Arc<HttpClient>) -> Self {
        Self {
            info: EngineInfo {
                name: "Sogou Images".to_string(),
                engine_type: EngineType::Image,
                description: "Sogou Images - Chinese image search engine".to_string(),
                status: EngineStatus::Active,
                categories: vec!["images".to_string()],
                capabilities: EngineCapabilities {
                    result_types: vec![ResultType::Image],
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
                    website: Some("https://pic.sogou.com/".to_string()),
                    wikidata_id: Some("Q7554565".to_string()), // Same as Sogou main
                    official_api_documentation: None,
                    use_official_api: false,
                    require_api_key: false,
                    results: "HTML".to_string(),
                },
                shortcut: Some("sogou img".to_string()),
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
        let mut items = Vec::with_capacity(20);

        // Sogou image results are typically in div elements with specific classes
        let result_selector = Selector::parse("div.img-box")
            .or_else(|_| Selector::parse("div[class*=\"img\"]"))
            .or_else(|_| Selector::parse("li.img-item"))
            .expect("valid selector");

        for result in document.select(&result_selector) {
            // Extract image URL from thumbnail
            let img_selector = Selector::parse("img")
                .expect("valid selector");
            let img_elem = result.select(&img_selector).next();

            if img_elem.is_none() {
                continue;
            }

            let img_elem = img_elem.unwrap();
            let img_url = img_elem.value().attr("src")
                .or_else(|| img_elem.value().attr("data-src"))
                .or_else(|| img_elem.value().attr("data-original"))
                .unwrap_or("")
                .to_string();

            if img_url.is_empty() {
                continue;
            }

            // Extract title from alt text or nearby elements
            let title = img_elem.value().attr("alt")
                .map(|s| s.to_string())
                .unwrap_or_else(|| {
                    result.text().collect::<String>().trim().to_string()
                });

            // Extract link to the source page
            let link_selector = Selector::parse("a")
                .expect("valid selector");
            let source_url = result.select(&link_selector).next()
                .and_then(|a| a.value().attr("href"))
                .map(|s| s.to_string())
                .unwrap_or_default();

            // Handle protocol-relative URLs
            let thumbnail_url = if img_url.starts_with("//") {
                format!("https:{}", &img_url)
            } else if !img_url.starts_with("http") {
                format!("https://pic.sogou.com{}", &img_url)
            } else {
                img_url.clone()
            };

            items.push(SearchResultItem {
                title,
                url: source_url.clone(),
                content: String::new(),
                display_url: if !source_url.is_empty() { Some(source_url) } else { Some(img_url.clone()) },
                site_name: None,
                score: 1.0,
                result_type: ResultType::Image,
                thumbnail: Some(thumbnail_url),
                published_date: None,
                template: None,
                metadata: {
                    let mut meta = HashMap::new();
                    meta.insert("image_url".to_string(), img_url);
                    meta
                },
            });
        }

        Ok(items)
    }
}

impl Default for SogouImagesEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SearchEngine for SogouImagesEngine {
    fn info(&self) -> &EngineInfo {
        &self.info
    }

    async fn search(&self, query: &SearchQuery) -> Result<SearchResult, Box<dyn Error + Send + Sync>> {
        <Self as RequestResponseEngine>::search(self, query).await
    }

    async fn is_available(&self) -> bool {
        self.client.get("https://pic.sogou.com", None).await.is_ok()
    }
}

#[async_trait]
impl RequestResponseEngine for SogouImagesEngine {
    type Response = String;

    fn request(&self, query: &str, params: &mut RequestParams) -> Result<(), Box<dyn Error + Send + Sync>> {
        // Sogou images search URL
        let query_params = vec![
            ("query", query.to_string()),
            ("start", ((params.pageno - 1) * 20).to_string()),
        ];

        // Build URL with optimized query string
        let query_string = build_query_string_owned(query_params.into_iter());

        params.url = Some(format!("https://pic.sogou.com/pics?{}", query_string));
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