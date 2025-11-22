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

pub struct BingNewsEngine {
    info: EngineInfo,
    client: Arc<HttpClient>,
}

impl BingNewsEngine {
    pub fn new() -> Self {
        let client = HttpClient::new(NetworkConfig::default())
            .unwrap_or_else(|_| panic!("Failed to create HTTP client"));
        Self::with_client(Arc::new(client))
    }

    pub fn with_client(client: Arc<HttpClient>) -> Self {
        Self {
            info: EngineInfo {
                name: "Bing News".to_string(),
                engine_type: EngineType::News,
                description: "Bing News - Microsoft's news search engine".to_string(),
                status: EngineStatus::Active,
                categories: vec!["news".to_string()],
                capabilities: EngineCapabilities {
                    result_types: vec![ResultType::News],
                    supported_params: vec!["page".to_string(), "time_range".to_string()],
                    max_page_size: 10,
                    supports_pagination: true,
                    supports_time_range: true,
                    supports_language_filter: false,
                    supports_region_filter: false,
                    supports_safe_search: false,
                    rate_limit: Some(30),
                },
                about: AboutInfo {
                    website: Some("https://www.bing.com/news".to_string()),
                    wikidata_id: Some("Q2878637".to_string()),
                    official_api_documentation: Some("https://www.microsoft.com/en-us/bing/apis/bing-news-search-api".to_string()),
                    use_official_api: false,
                    require_api_key: false,
                    results: "RSS".to_string(),
                },
                shortcut: Some("bing news".to_string()),
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

        // Bing news results are typically in news cards or result items
        let result_selector = Selector::parse("div.news-card")
            .or_else(|_| Selector::parse("div[class*=\"news\"]"))
            .or_else(|_| Selector::parse("article"))
            .or_else(|_| Selector::parse("div.result"))
            .expect("valid selector");

        for result in document.select(&result_selector) {
            // Extract title from news headline
            let title_selector = Selector::parse("h3 a")
                .or_else(|_| Selector::parse("h2 a"))
                .or_else(|_| Selector::parse("a.title"))
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

            // Extract news URL
            let news_url = title_elem.value().attr("href")
                .unwrap_or("")
                .to_string();

            if news_url.is_empty() {
                continue;
            }

            // Extract thumbnail image
            let thumbnail = result.select(&Selector::parse("img").expect("valid selector")).next()
                .and_then(|img| {
                    img.value().attr("src")
                        .or_else(|| img.value().attr("data-src"))
                })
                .map(|s| {
                    if s.starts_with("//") {
                        format!("https:{}", s)
                    } else if !s.starts_with("http") {
                        format!("https://www.bing.com{}", s)
                    } else {
                        s.to_string()
                    }
                });

            // Extract description/content
            let content_selector = Selector::parse("p.snippet")
                .or_else(|_| Selector::parse("p.description"))
                .or_else(|_| Selector::parse("div.snippet"))
                .expect("valid selector");
            let content = result.select(&content_selector).next()
                .map(|c| c.text().collect::<String>().trim().to_string())
                .unwrap_or_default();

            // Extract source/publisher info
            let source = result.select(&Selector::parse("span.source")
                .or_else(|_| Selector::parse("span.provider"))
                .or_else(|_| Selector::parse("cite"))
                .expect("valid selector")).next()
                .map(|s| s.text().collect::<String>().trim().to_string())
                .unwrap_or_default();

            // Extract published date
            let published_date = result.select(&Selector::parse("span.date")
                .or_else(|_| Selector::parse("time"))
                .expect("valid selector")).next()
                .and_then(|date_elem| {
                    let date_str = date_elem.text().collect::<String>().trim().to_string();
                    // Try to parse relative dates like "2 hours ago", "1 day ago"
                    parse_relative_date(&date_str)
                });

            let mut metadata = HashMap::new();
            if !source.is_empty() {
                metadata.insert("source".to_string(), source);
            }

            items.push(SearchResultItem {
                title,
                url: news_url.clone(),
                content,
                display_url: Some(news_url),
                site_name: None,
                score: 1.0,
                result_type: ResultType::News,
                thumbnail,
                published_date,
                template: None,
                metadata,
            });
        }

        Ok(items)
    }
}

impl Default for BingNewsEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SearchEngine for BingNewsEngine {
    fn info(&self) -> &EngineInfo {
        &self.info
    }

    async fn search(&self, query: &SearchQuery) -> Result<SearchResult, Box<dyn Error + Send + Sync>> {
        <Self as RequestResponseEngine>::search(self, query).await
    }

    async fn is_available(&self) -> bool {
        self.client.get("https://www.bing.com/news", None).await.is_ok()
    }
}

#[async_trait]
impl RequestResponseEngine for BingNewsEngine {
    type Response = String;

    fn request(&self, query: &str, params: &mut RequestParams) -> Result<(), Box<dyn Error + Send + Sync>> {
        // Python: base_url = 'https://www.bing.com/news/infinitescrollajax'
        let base_url = "https://www.bing.com/news/infinitescrollajax";

        // Python: time_map = {'day': 'interval="4"', 'week': 'interval="7"', 'month': 'interval="9"'}
        let time_map = HashMap::from([
            ("day", "interval=\"4\""),
            ("week", "interval=\"7\""),
            ("month", "interval=\"9\""),
        ]);

        // Python: query_params = {'q': query, 'first': (int(params['pageno']) - 1) * 10 + 1}
        let mut query_params = vec![
            ("q", query.to_string()),
            ("first", ((params.pageno - 1) * 10 + 1).to_string()),
        ];

        // Add time range filter if specified
        // Python: if params['time_range']: query_params['qft'] = time_map[params['time_range']]
        if let Some(ref tr) = params.time_range {
            if let Some(interval) = time_map.get(tr.as_str()) {
                query_params.push(("qft", interval.to_string()));
            }
        }

        // Build URL with optimized query string
        let query_string = build_query_string_owned(query_params.into_iter());

        params.url = Some(format!("{}?{}", base_url, query_string));
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

// Helper function to parse relative dates like "2 hours ago", "1 day ago"
fn parse_relative_date(date_str: &str) -> Option<chrono::DateTime<chrono::Utc>> {
    let now = chrono::Utc::now();
    let lower = date_str.to_lowercase();

    if lower.contains("minute") {
        if let Some(minutes) = extract_number(&lower) {
            return Some(now - chrono::Duration::minutes(minutes));
        }
    } else if lower.contains("hour") {
        if let Some(hours) = extract_number(&lower) {
            return Some(now - chrono::Duration::hours(hours));
        }
    } else if lower.contains("day") {
        if let Some(days) = extract_number(&lower) {
            return Some(now - chrono::Duration::days(days));
        }
    } else if lower.contains("week") {
        if let Some(weeks) = extract_number(&lower) {
            return Some(now - chrono::Duration::weeks(weeks));
        }
    }

    None
}

// Extract number from string like "2 hours ago" -> 2
fn extract_number(s: &str) -> Option<i64> {
    s.split_whitespace()
        .find_map(|word| word.parse::<i64>().ok())
}