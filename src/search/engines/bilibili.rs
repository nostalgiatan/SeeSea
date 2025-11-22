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
use rand::Rng;

use crate::derive::{
    EngineCapabilities, EngineInfo, EngineStatus, EngineType,
    ResultType, SearchEngine, SearchQuery, SearchResult,
    SearchResultItem, AboutInfo, RequestResponseEngine, RequestParams,
};
use crate::net::client::HttpClient;
use crate::net::types::{NetworkConfig, RequestOptions};
use super::utils::build_query_string_owned;

pub struct BilibiliEngine {
    info: EngineInfo,
    client: Arc<HttpClient>,
}

impl BilibiliEngine {
    pub fn new() -> Self {
        let client = HttpClient::new(NetworkConfig::default())
            .unwrap_or_else(|_| panic!("Failed to create HTTP client"));
        Self::with_client(Arc::new(client))
    }

    pub fn with_client(client: Arc<HttpClient>) -> Self {
        Self {
            info: EngineInfo {
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
            },
            client,
        }
    }

    fn parse_json_results(json_str: &str) -> Result<Vec<SearchResultItem>, Box<dyn Error + Send + Sync>> {
        use serde_json::Value;

        let json: Value = serde_json::from_str(json_str)?;
        let mut items = Vec::with_capacity(20);

        // Python: for item in search_res.get("data", {}).get("result", []):
        if let Some(data) = json.get("data") {
            if let Some(results) = data.get("result") {
                if let Some(result_array) = results.as_array() {
                    for item in result_array {
                        // Python: title = utils.html_to_text(item["title"])
                        let raw_title = item.get("title")
                            .and_then(|v| v.as_str())
                            .unwrap_or_default();

                        // 提取keywords并清理HTML标签
                        let (title, keywords) = extract_keywords_and_clean_html(raw_title);

                        if title.is_empty() {
                            continue;
                        }

                        // Python: url = item["arcurl"]
                        let url = item.get("arcurl")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();

                        if url.is_empty() {
                            continue;
                        }

                        // Python: thumbnail = item["pic"]
                        let thumbnail = item.get("pic")
                            .and_then(|v| v.as_str())
                            .map(|s| {
                                if s.starts_with("//") {
                                    format!("https:{}", s)
                                } else if !s.starts_with("http") {
                                    format!("https:{}", s)
                                } else {
                                    s.to_string()
                                }
                            });

                        // Python: description = item["description"]
                        let content = item.get("description")
                            .and_then(|v| v.as_str())
                            .map(|s| strip_html_entities(s))
                            .unwrap_or_default();

                        // Python: author = item["author"]
                        let author = item.get("author")
                            .and_then(|v| v.as_str())
                            .unwrap_or("");

                        // Python: video_id = item["aid"]
                        let video_id = item.get("aid")
                            .and_then(|v| v.as_i64())
                            .unwrap_or(0);

                        // Python: unix_date = item["pubdate"]
                        let published_date = item.get("pubdate")
                            .and_then(|v| v.as_i64())
                            .and_then(|ts| chrono::DateTime::from_timestamp(ts, 0));

                        // Python: duration = utils.parse_duration_string(item["duration"])
                        let duration_str = item.get("duration")
                            .and_then(|v| v.as_str())
                            .unwrap_or("");

                        // Python: iframe_url = f"https://player.bilibili.com/player.html?aid={video_id}&high_quality=1&autoplay=false&danmaku=0"
                        let iframe_url = format!("https://player.bilibili.com/player.html?aid={}&high_quality=1&autoplay=false&danmaku=0", video_id);

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
            }
        }

        Ok(items)
    }

    fn generate_bilibili_cookies() -> HashMap<String, String> {
        let mut rng = rand::rng();

        // Python: buvid3 = "".join(random.choice(string.hexdigits) for _ in range(16)) + "infoc"
        let buvid3: String = (0..16)
            .map(|_| {
                let chars = b"0123456789abcdef";
                chars[rng.random_range(0..16)] as char
            })
            .collect::<String>() + "infoc";

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

impl Default for BilibiliEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SearchEngine for BilibiliEngine {
    fn info(&self) -> &EngineInfo {
        &self.info
    }

    async fn search(&self, query: &SearchQuery) -> Result<SearchResult, Box<dyn Error + Send + Sync>> {
        <Self as RequestResponseEngine>::search(self, query).await
    }

    async fn is_available(&self) -> bool {
        self.client.get("https://www.bilibili.com", None).await.is_ok()
    }
}

#[async_trait]
impl RequestResponseEngine for BilibiliEngine {
    type Response = String;

    fn request(&self, query: &str, params: &mut RequestParams) -> Result<(), Box<dyn Error + Send + Sync>> {
        // Python: base_url = "https://api.bilibili.com/x/web-interface/search/type"
        let base_url = "https://api.bilibili.com/x/web-interface/search/type";

        // Python: query_params = {
        //     "__refresh__": "true",
        //     "page": params["pageno"],
        //     "page_size": results_per_page,
        //     "single_column": "0",
        //     "keyword": query,
        //     "search_type": "video",
        // }
        let query_params = vec![
            ("__refresh__", "true".to_string()),
            ("page", params.pageno.to_string()),
            ("page_size", "20".to_string()),
            ("single_column", "0".to_string()),
            ("keyword", query.to_string()),
            ("search_type", "video".to_string()),
        ];

        // Build URL with optimized query string
        let query_string = build_query_string_owned(query_params.into_iter());

        params.url = Some(format!("{}?{}", base_url, query_string));
        params.method = "GET".to_string();

        // Set cookies
        params.cookies = Self::generate_bilibili_cookies();

        Ok(())
    }

    async fn fetch(&self, params: &RequestParams) -> Result<Self::Response, Box<dyn Error + Send + Sync>> {
        let url = params.url.as_ref().ok_or("URL not set")?;

        let mut options = RequestOptions::default();
        // 使用配置的默认超时时间

        for (key, value) in &params.headers {
            options.headers.push((key.clone(), value.clone()));
        }

        // Add cookies
        for (key, value) in &params.cookies {
            options.headers.push(("Cookie".to_string(), format!("{}={}", key, value)));
        }

        let response = self.client.get(url, Some(options)).await
            .map_err(|e| format!("Request failed: {}", e))?;

        response.text().await.map_err(|e| format!("Failed to read response: {}", e).into())
    }

    fn response(&self, resp: Self::Response) -> Result<Vec<SearchResultItem>, Box<dyn Error + Send + Sync>> {
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
            let keyword = strip_html_entities(keyword_match.as_str()).trim().to_string();
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
    cleaned_html = cleaned_html.split_whitespace().collect::<Vec<_>>().join(" ");

    (cleaned_html, keywords)
}