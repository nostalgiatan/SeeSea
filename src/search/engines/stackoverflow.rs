use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use std::error::Error;
use std::time::Duration;
use serde_json::Value;

use crate::derive::{
    EngineCapabilities, EngineInfo, EngineStatus, EngineType, 
    ResultType, SearchEngine, SearchQuery, SearchResult, 
    SearchResultItem, AboutInfo, RequestResponseEngine, RequestParams,
};
use crate::net::client::HttpClient;
use crate::net::types::{NetworkConfig, RequestOptions};

pub struct StackOverflowEngine {
    info: EngineInfo,
    client: Arc<HttpClient>,
}

impl StackOverflowEngine {
    pub fn new() -> Self {
        let client = HttpClient::new(NetworkConfig::default())
            .unwrap_or_else(|_| panic!("Failed to create HTTP client"));
        Self::with_client(Arc::new(client))
    }

    pub fn with_client(client: Arc<HttpClient>) -> Self {
        Self {
            info: EngineInfo {
                name: "Stack Overflow".to_string(),
                engine_type: EngineType::General,
                description: "Stack Overflow - Q&A for developers".to_string(),
                status: EngineStatus::Active,
                categories: vec!["it".to_string(), "qa".to_string()],
                capabilities: EngineCapabilities {
                    result_types: vec![ResultType::Web],
                    supported_params: vec![],
                    max_page_size: 10,
                    supports_pagination: true,
                    supports_time_range: false,
                    supports_language_filter: false,
                    supports_region_filter: false,
                    supports_safe_search: false,
                    rate_limit: Some(300),
                },
                about: AboutInfo {
                    website: Some("https://stackoverflow.com".to_string()),
                    wikidata_id: Some("Q3495447".to_string()),
                    official_api_documentation: Some("https://api.stackexchange.com/docs".to_string()),
                    use_official_api: true,
                    require_api_key: false,
                    results: "JSON".to_string(),
                },
                shortcut: Some("st".to_string()),
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

    /// HTML unescape helper function (Python: html.unescape)
    /// Complete HTML entity decoding matching Python's html.unescape
    fn html_unescape(text: &str) -> String {
        text.replace("&lt;", "<")
            .replace("&gt;", ">")
            .replace("&amp;", "&")
            .replace("&quot;", "\"")
            .replace("&#39;", "'")
            .replace("&#x27;", "'")
            .replace("&#x2F;", "/")
            .replace("&nbsp;", " ")
            .replace("&apos;", "'")
            .replace("&copy;", "©")
            .replace("&reg;", "®")
            .replace("&trade;", "™")
            .replace("&euro;", "€")
            .replace("&pound;", "£")
            .replace("&yen;", "¥")
            .replace("&cent;", "¢")
            .replace("&sect;", "§")
            .replace("&para;", "¶")
            .replace("&mdash;", "\u{2014}")
            .replace("&ndash;", "\u{2013}")
            .replace("&hellip;", "\u{2026}")
            .replace("&laquo;", "«")
            .replace("&raquo;", "»")
            .replace("&lsquo;", "\u{2018}")
            .replace("&rsquo;", "\u{2019}")
            .replace("&ldquo;", "\u{201C}")
            .replace("&rdquo;", "\u{201D}")
            .replace("&bull;", "•")
            .replace("&middot;", "·")
            .replace("&deg;", "°")
            .replace("&plusmn;", "±")
            .replace("&times;", "×")
            .replace("&divide;", "÷")
            .replace("&frac14;", "¼")
            .replace("&frac12;", "½")
            .replace("&frac34;", "¾")
    }

    fn parse_json_result(json_str: &str) -> Result<Vec<SearchResultItem>, Box<dyn Error + Send + Sync>> {
        let api_result: Value = serde_json::from_str(json_str)?;
        let mut items = Vec::new();

        // Python: for result in json_data['items']:
        if let Some(items_array) = api_result.get("items").and_then(|i| i.as_array()) {
            for result in items_array {
                // Python: 'title': html.unescape(result['title'])
                let title_raw = result.get("title")
                    .and_then(|t| t.as_str())
                    .unwrap_or("")
                    .to_string();
                
                if title_raw.is_empty() {
                    continue;
                }
                
                let title = Self::html_unescape(&title_raw);

                // Python: 'url': "https://%s.com/q/%s" % (api_site, result['question_id'])
                // api_site = 'stackoverflow'
                let question_id = result.get("question_id")
                    .and_then(|q| q.as_i64())
                    .unwrap_or(0);
                
                if question_id == 0 {
                    continue;
                }

                let url = format!("https://stackoverflow.com/q/{}", question_id);

                // Python: content = "[%s]" % ", ".join(result['tags'])
                // content += " %s" % result['owner']['display_name']
                // if result['is_answered']: content += ' // is answered'
                // content += " // score: %s" % result['score']
                let mut content_parts = Vec::new();
                
                // Tags
                if let Some(tags) = result.get("tags").and_then(|t| t.as_array()) {
                    let tag_str = tags.iter()
                        .filter_map(|t| t.as_str())
                        .collect::<Vec<_>>()
                        .join(", ");
                    if !tag_str.is_empty() {
                        content_parts.push(format!("[{}]", tag_str));
                    }
                }
                
                // Owner display name
                if let Some(owner) = result.get("owner").and_then(|o| o.get("display_name")).and_then(|d| d.as_str()) {
                    content_parts.push(owner.to_string());
                }
                
                // Is answered
                if let Some(is_answered) = result.get("is_answered").and_then(|a| a.as_bool()) {
                    if is_answered {
                        content_parts.push("is answered".to_string());
                    }
                }
                
                // Score
                if let Some(score) = result.get("score").and_then(|s| s.as_i64()) {
                    content_parts.push(format!("score: {}", score));
                }
                
                // Python: html.unescape(content)
                let content = Self::html_unescape(&content_parts.join(" // "));

                // Additional metadata
                let mut metadata = HashMap::new();
                if let Some(score) = result.get("score").and_then(|s| s.as_i64()) {
                    metadata.insert("score".to_string(), score.to_string());
                }
                if let Some(answer_count) = result.get("answer_count").and_then(|a| a.as_i64()) {
                    metadata.insert("answer_count".to_string(), answer_count.to_string());
                }
                if let Some(view_count) = result.get("view_count").and_then(|v| v.as_i64()) {
                    metadata.insert("view_count".to_string(), view_count.to_string());
                }
                if let Some(is_answered) = result.get("is_answered").and_then(|a| a.as_bool()) {
                    metadata.insert("is_answered".to_string(), is_answered.to_string());
                }

                items.push(SearchResultItem {
                    title,
                    url: url.clone(),
                    content,
                    display_url: Some(url),
                    site_name: Some("Stack Overflow".to_string()),
                    score: 1.0,
                    result_type: ResultType::Web,
                    thumbnail: None,
                    published_date: None,
                    template: None,
                    metadata,
                });
            }
        }

        Ok(items)
    }
}

impl Default for StackOverflowEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SearchEngine for StackOverflowEngine {
    fn info(&self) -> &EngineInfo {
        &self.info
    }

    async fn search(&self, query: &SearchQuery) -> Result<SearchResult, Box<dyn Error + Send + Sync>> {
        <Self as RequestResponseEngine>::search(self, query).await
    }

    async fn is_available(&self) -> bool {
        self.client.get("https://api.stackexchange.com", None).await.is_ok()
    }
}

#[async_trait]
impl RequestResponseEngine for StackOverflowEngine {
    type Response = String;

    fn request(&self, query: &str, params: &mut RequestParams) -> Result<(), Box<dyn Error + Send + Sync>> {
        // Python: search_api = 'https://api.stackexchange.com/2.3/search/advanced?'
        // args = urlencode({'q': query, 'page': params['pageno'], 'pagesize': pagesize, 
        //                   'site': api_site, 'sort': api_sort, 'order': 'desc'})
        // pagesize = 10
        // api_site = 'stackoverflow'
        // api_sort = 'activity'
        let query_params = vec![
            ("q", query.to_string()),
            ("page", params.pageno.to_string()),
            ("pagesize", "10".to_string()),
            ("site", "stackoverflow".to_string()),
            ("sort", "activity".to_string()),
            ("order", "desc".to_string()),
        ];

        let query_string = query_params.iter()
            .map(|(k, v)| format!("{}={}", k, urlencoding::encode(v)))
            .collect::<Vec<_>>()
            .join("&");
        
        params.url = Some(format!("https://api.stackexchange.com/2.3/search/advanced?{}", query_string));
        params.method = "GET".to_string();

        Ok(())
    }

    async fn fetch(&self, params: &RequestParams) -> Result<Self::Response, Box<dyn Error + Send + Sync>> {
        let url = params.url.as_ref().ok_or("URL not set")?;
        
        let mut options = RequestOptions::default();
        options.timeout = Duration::from_secs(10);
        
        for (key, value) in &params.headers {
            options.headers.push((key.clone(), value.clone()));
        }

        let response = self.client.get(url, Some(options)).await
            .map_err(|e| format!("Request failed: {}", e))?;

        response.text().await.map_err(|e| format!("Failed to read response: {}", e).into())
    }

    fn response(&self, resp: Self::Response) -> Result<Vec<SearchResultItem>, Box<dyn Error + Send + Sync>> {
        Self::parse_json_result(&resp)
    }
}
