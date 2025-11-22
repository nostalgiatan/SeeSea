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

pub struct SogouEngine {
    info: EngineInfo,
    client: Arc<HttpClient>,
}

impl SogouEngine {
    pub fn new() -> Self {
        let client = HttpClient::new(NetworkConfig::default())
            .unwrap_or_else(|_| panic!("Failed to create HTTP client"));
        Self::with_client(Arc::new(client))
    }

    pub fn with_client(client: Arc<HttpClient>) -> Self {
        Self {
            info: EngineInfo {
                name: "Sogou".to_string(),
                engine_type: EngineType::General,
                description: "Sogou - Chinese search engine".to_string(),
                status: EngineStatus::Active,
                categories: vec!["general".to_string()],
                capabilities: EngineCapabilities {
                    result_types: vec![ResultType::Web],
                    supported_params: vec!["time_range".to_string()],
                    max_page_size: 10,
                    supports_pagination: true,
                    supports_time_range: true,
                    supports_language_filter: false,
                    supports_region_filter: false,
                    supports_safe_search: false,
                    rate_limit: Some(60),
                },
                about: AboutInfo {
                    website: Some("https://www.sogou.com/".to_string()),
                    wikidata_id: Some("Q7554565".to_string()),
                    official_api_documentation: None,
                    use_official_api: false,
                    require_api_key: false,
                    results: "HTML".to_string(),
                },
                shortcut: Some("sogou".to_string()),
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

        // Python: for item in dom.xpath('//div[contains(@class, "vrwrap")]'):
        let result_selector = Selector::parse("div.vrwrap")
            .or_else(|_| Selector::parse("div[class*=\"vrwrap\"]"))
            .expect("valid selector");

        for result in document.select(&result_selector) {
            // Python: title = extract_text(item.xpath('.//h3[contains(@class, "vr-title")]/a'))
            let title_selector = Selector::parse("h3.vr-title a")
                .or_else(|_| Selector::parse("h3[class*=\"vr-title\"] a"))
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

            // Python: url = extract_text(item.xpath('.//h3[contains(@class, "vr-title")]/a/@href'))
            let url_elem = title_elem;
            let url = url_elem.value().attr("href")
                .unwrap_or("")
                .to_string();

            // Handle redirect URLs: if url.startswith("/link?url="):
            if url.starts_with("/link?url=") {
                // In real implementation, we might need to resolve this redirect
                // For now, construct the full URL
                continue; // Skip redirects for now as they require special handling
            }

            if url.is_empty() {
                continue;
            }

            // Python: content = extract_text(item.xpath('.//div[contains(@class, "text-layout")]//p[contains(@class, "star-wiki")]'))
            // if not content: content = extract_text(item.xpath('.//div[contains(@class, "fz-mid space-txt")]'))
            let content = result.select(&Selector::parse("div.text-layout p.star-wiki").expect("valid selector")).next()
                .map(|c| c.text().collect::<String>().trim().to_string())
                .or_else(|| {
                    result.select(&Selector::parse("div.fz-mid.space-txt").expect("valid selector")).next()
                        .map(|c| c.text().collect::<String>().trim().to_string())
                })
                .unwrap_or_default();

            items.push(SearchResultItem {
                title,
                url: url.clone(),
                content,
                display_url: Some(url),
                site_name: None,
                score: 1.0,
                result_type: ResultType::Web,
                thumbnail: None,
                published_date: None,
                template: None,
                metadata: HashMap::new(),
            });
        }

        Ok(items)
    }
}

impl Default for SogouEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SearchEngine for SogouEngine {
    fn info(&self) -> &EngineInfo {
        &self.info
    }

    async fn search(&self, query: &SearchQuery) -> Result<SearchResult, Box<dyn Error + Send + Sync>> {
        <Self as RequestResponseEngine>::search(self, query).await
    }

    async fn is_available(&self) -> bool {
        self.client.get("https://www.sogou.com", None).await.is_ok()
    }
}

#[async_trait]
impl RequestResponseEngine for SogouEngine {
    type Response = String;

    fn request(&self, query: &str, params: &mut RequestParams) -> Result<(), Box<dyn Error + Send + Sync>> {
        // Python: base_url = "https://www.sogou.com"
        // query_params = {"query": query, "page": params["pageno"]}
        let mut query_params = vec![
            ("query", query.to_string()),
            ("page", params.pageno.to_string()),
        ];

        // Add time range filter if specified
        // Python: time_range_dict = {'day': 'inttime_day', 'week': 'inttime_week', 'month': 'inttime_month', 'year': 'inttime_year'}
        if let Some(ref tr) = params.time_range {
            let s_from = match tr.as_str() {
                "day" => "inttime_day",
                "week" => "inttime_week",
                "month" => "inttime_month",
                "year" => "inttime_year",
                _ => "",
            };
            if !s_from.is_empty() {
                query_params.push(("s_from", s_from.to_string()));
                query_params.push(("tsn", "1".to_string()));
            }
        }

        // Build URL with optimized query string
        let query_string = build_query_string_owned(query_params.into_iter());

        params.url = Some(format!("https://www.sogou.com/web?{}", query_string));
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