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

pub struct BingVideosEngine {
    info: EngineInfo,
    client: Arc<HttpClient>,
}

impl BingVideosEngine {
    pub fn new() -> Self {
        let client = HttpClient::new(NetworkConfig::default())
            .unwrap_or_else(|_| panic!("Failed to create HTTP client"));
        Self::with_client(Arc::new(client))
    }

    pub fn with_client(client: Arc<HttpClient>) -> Self {
        Self {
            info: EngineInfo {
                name: "Bing Videos".to_string(),
                engine_type: EngineType::Video,
                description: "Bing Videos - Microsoft's video search engine".to_string(),
                status: EngineStatus::Active,
                categories: vec!["videos".to_string(), "web".to_string()],
                capabilities: EngineCapabilities {
                    result_types: vec![ResultType::Video],
                    supported_params: vec!["page".to_string(), "time_range".to_string()],
                    max_page_size: 35,
                    supports_pagination: true,
                    supports_time_range: true,
                    supports_language_filter: false,
                    supports_region_filter: false,
                    supports_safe_search: true,
                    rate_limit: Some(30),
                },
                about: AboutInfo {
                    website: Some("https://www.bing.com/videos".to_string()),
                    wikidata_id: Some("Q4914152".to_string()),
                    official_api_documentation: Some("https://www.microsoft.com/en-us/bing/apis/bing-video-search-api".to_string()),
                    use_official_api: false,
                    require_api_key: false,
                    results: "HTML".to_string(),
                },
                shortcut: Some("bing vid".to_string()),
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
        let mut items = Vec::with_capacity(35);

        // Bing video results are typically in video cards or result items
        let result_selector = Selector::parse("div.dmcv")
            .or_else(|_| Selector::parse("div[class*=\"video\"]"))
            .or_else(|_| Selector::parse("div.videocard"))
            .or_else(|_| Selector::parse("div.mc_vtvc"))
            .expect("valid selector");

        for result in document.select(&result_selector) {
            // Extract title from video title
            let title_selector = Selector::parse("a.title")
                .or_else(|_| Selector::parse("h3 a"))
                .or_else(|_| Selector::parse("a.tit"))
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
            let thumbnail = result.select(&Selector::parse("img").expect("valid selector")).next()
                .and_then(|img| {
                    img.value().attr("src")
                        .or_else(|| img.value().attr("data-src"))
                        .or_else(|| img.value().attr("data-thumbsrc"))
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
            let content_selector = Selector::parse("p.caption")
                .or_else(|_| Selector::parse("div.mc_vtvc_meta"))
                .or_else(|_| Selector::parse("span.mc_vtvc_desc"))
                .expect("valid selector");
            let content = result.select(&content_selector).next()
                .map(|c| c.text().collect::<String>().trim().to_string())
                .unwrap_or_default();

            // Extract duration
            let duration = result.select(&Selector::parse("span.mc_vtvc_duration span.dur").expect("valid selector")).next()
                .map(|d| d.text().collect::<String>().trim().to_string())
                .filter(|d| !d.is_empty());

            // Extract source/publisher info
            let source = result.select(&Selector::parse("span.mc_vtvc_meta_author")
                .or_else(|_| Selector::parse("span.mc_vtvc_meta span"))
                .expect("valid selector")).next()
                .map(|s| s.text().collect::<String>().trim().to_string())
                .unwrap_or_default();

            // Extract view count if available
            let views = result.select(&Selector::parse("span.mc_vtvc_meta_views")
                .or_else(|_| Selector::parse("span.views"))
                .expect("valid selector")).next()
                .and_then(|v| {
                    let view_str = v.text().collect::<String>();
                    parse_view_count(&view_str)
                });

            let mut metadata = HashMap::new();
            if !source.is_empty() {
                metadata.insert("source".to_string(), source);
            }
            if let Some(dur) = &duration {
                metadata.insert("duration".to_string(), dur.clone());
            }
            if let Some(view_count) = views {
                metadata.insert("views".to_string(), view_count.to_string());
            }

            items.push(SearchResultItem {
                title,
                url: video_url.clone(),
                content,
                display_url: Some(video_url),
                site_name: None,
                score: 1.0,
                result_type: ResultType::Video,
                thumbnail,
                published_date: None,
                template: Some("videos.html".to_string()),
                metadata,
            });
        }

        Ok(items)
    }
}

impl Default for BingVideosEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SearchEngine for BingVideosEngine {
    fn info(&self) -> &EngineInfo {
        &self.info
    }

    async fn search(&self, query: &SearchQuery) -> Result<SearchResult, Box<dyn Error + Send + Sync>> {
        <Self as RequestResponseEngine>::search(self, query).await
    }

    async fn is_available(&self) -> bool {
        self.client.get("https://www.bing.com/videos", None).await.is_ok()
    }
}

#[async_trait]
impl RequestResponseEngine for BingVideosEngine {
    type Response = String;

    fn request(&self, query: &str, params: &mut RequestParams) -> Result<(), Box<dyn Error + Send + Sync>> {
        let base_url = "https://www.bing.com/videos/asyncv2";

        let time_map = HashMap::from([
            ("day", 60 * 24),
            ("week", 60 * 24 * 7),
            ("month", 60 * 24 * 31),
            ("year", 60 * 24 * 365),
        ]);

        let mut query_params = vec![
            ("q", query.to_string()),
            ("async", "content".to_string()),
            ("first", ((params.pageno - 1) * 35 + 1).to_string()),
            ("count", "35".to_string()),
        ];

        if let Some(ref tr) = params.time_range {
            if let Some(minutes) = time_map.get(tr.as_str()) {
                query_params.push(("qft", format!("filterui:age-lt{}", minutes)));
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

// Helper function to parse view count from strings like "1.2K views", "3.4M views"
fn parse_view_count(s: &str) -> Option<u64> {
    let lower = s.to_lowercase();
    if !lower.contains("view") {
        return None;
    }

    let clean_num = lower.replace("views", "").replace("view", "").trim().to_string();

    if clean_num.ends_with('k') {
        let base: f64 = clean_num.trim_end_matches('k').parse().ok()?;
        Some((base * 1000.0) as u64)
    } else if clean_num.ends_with('m') {
        let base: f64 = clean_num.trim_end_matches('m').parse().ok()?;
        Some((base * 1000000.0) as u64)
    } else if clean_num.ends_with('b') {
        let base: f64 = clean_num.trim_end_matches('b').parse().ok()?;
        Some((base * 1000000000.0) as u64)
    } else {
        clean_num.parse().ok()
    }
}