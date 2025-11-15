use async_trait::async_trait;
use std::collections::HashMap;
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

pub struct UnsplashEngine {
    info: EngineInfo,
    client: HttpClient,
}

impl UnsplashEngine {
    pub fn new() -> Self {
        let net_config = NetworkConfig::default();
        let client = HttpClient::new(net_config).unwrap_or_else(|_| {
            panic!("Failed to create HTTP client for Unsplash")
        });
        
        Self {
            info: EngineInfo {
                name: "Unsplash".to_string(),
                engine_type: EngineType::Image,
                description: "Unsplash - Free high-quality images".to_string(),
                status: EngineStatus::Active,
                categories: vec!["images".to_string()],
                capabilities: EngineCapabilities {
                    result_types: vec![ResultType::Image],
                    supported_params: vec![],
                    max_page_size: 20,
                    supports_pagination: true,
                    supports_time_range: false,
                    supports_language_filter: false,
                    supports_region_filter: false,
                    supports_safe_search: false,
                    rate_limit: Some(50),
                },
                about: AboutInfo {
                    website: Some("https://unsplash.com".to_string()),
                    wikidata_id: Some("Q28233552".to_string()),
                    official_api_documentation: Some("https://unsplash.com/developers".to_string()),
                    use_official_api: false,
                    require_api_key: false,
                    results: "JSON".to_string(),
                },
                shortcut: Some("us".to_string()),
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

    /// Clean URL by removing ixid parameter
    /// Python: def clean_url(url):
    ///     parsed = urlparse(url)
    ///     query = [(k, v) for (k, v) in parse_qsl(parsed.query) if k != 'ixid']
    ///     return urlunparse((parsed.scheme, parsed.netloc, parsed.path, parsed.params, urlencode(query), parsed.fragment))
    fn clean_url(url: &str) -> String {
        // Simple implementation: just remove ixid parameter if present
        if let Some(question_mark_pos) = url.find('?') {
            let base = &url[..question_mark_pos];
            let query_string = &url[question_mark_pos + 1..];
            
            let cleaned_params: Vec<String> = query_string
                .split('&')
                .filter(|param| !param.starts_with("ixid="))
                .map(|s| s.to_string())
                .collect();
            
            if cleaned_params.is_empty() {
                base.to_string()
            } else {
                format!("{}?{}", base, cleaned_params.join("&"))
            }
        } else {
            url.to_string()
        }
    }

    fn parse_json_result(json_str: &str) -> Result<Vec<SearchResultItem>, Box<dyn Error + Send + Sync>> {
        let api_result: Value = serde_json::from_str(json_str)?;
        let mut items = Vec::new();

        // Python: for result in json_data['results']:
        if let Some(results_array) = api_result.get("results").and_then(|r| r.as_array()) {
            for result in results_array {
                // Python: 'url': clean_url(result['links']['html'])
                let url_raw = result.get("links")
                    .and_then(|l| l.get("html"))
                    .and_then(|h| h.as_str())
                    .unwrap_or("");
                
                if url_raw.is_empty() {
                    continue;
                }
                
                let url = Self::clean_url(url_raw);

                // Python: 'title': result.get('alt_description') or 'unknown'
                let title = result.get("alt_description")
                    .and_then(|a| a.as_str())
                    .or_else(|| result.get("description").and_then(|d| d.as_str()))
                    .unwrap_or("unknown")
                    .to_string();

                // Python: 'content': result.get('description') or ''
                let content = result.get("description")
                    .and_then(|d| d.as_str())
                    .unwrap_or("")
                    .to_string();

                // Python: 'thumbnail_src': clean_url(result['urls']['thumb'])
                let thumbnail = result.get("urls")
                    .and_then(|u| u.get("thumb"))
                    .and_then(|t| t.as_str())
                    .map(|s| Self::clean_url(s));

                // Python: 'img_src': clean_url(result['urls']['regular'])
                // 'template': 'images.html'
                let mut metadata = HashMap::new();
                if let Some(img_src) = result.get("urls").and_then(|u| u.get("regular")).and_then(|r| r.as_str()) {
                    metadata.insert("img_src".to_string(), Self::clean_url(img_src));
                }
                
                // Additional metadata from Unsplash
                if let Some(user) = result.get("user") {
                    if let Some(username) = user.get("name").and_then(|n| n.as_str()) {
                        metadata.insert("photographer".to_string(), format!("by {}", username));
                    }
                    if let Some(profile_url) = user.get("links").and_then(|l| l.get("html")).and_then(|h| h.as_str()) {
                        metadata.insert("photographer_url".to_string(), profile_url.to_string());
                    }
                }
                
                if let Some(width) = result.get("width").and_then(|w| w.as_i64()) {
                    metadata.insert("width".to_string(), width.to_string());
                }
                if let Some(height) = result.get("height").and_then(|h| h.as_i64()) {
                    metadata.insert("height".to_string(), height.to_string());
                }
                if let Some(color) = result.get("color").and_then(|c| c.as_str()) {
                    metadata.insert("color".to_string(), color.to_string());
                }

                items.push(SearchResultItem {
                    title,
                    url: url.clone(),
                    content,
                    display_url: Some(url),
                    site_name: Some("Unsplash".to_string()),
                    score: 1.0,
                    result_type: ResultType::Image,
                    thumbnail,
                    published_date: None,
                    template: Some("images.html".to_string()), // Python: 'template': 'images.html'
                    metadata,
                });
            }
        }

        Ok(items)
    }
}

impl Default for UnsplashEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SearchEngine for UnsplashEngine {
    fn info(&self) -> &EngineInfo {
        &self.info
    }

    async fn search(&self, query: &SearchQuery) -> Result<SearchResult, Box<dyn Error + Send + Sync>> {
        <Self as RequestResponseEngine>::search(self, query).await
    }

    async fn is_available(&self) -> bool {
        self.client.get("https://unsplash.com", None).await.is_ok()
    }
}

#[async_trait]
impl RequestResponseEngine for UnsplashEngine {
    type Response = String;

    fn request(&self, query: &str, params: &mut RequestParams) -> Result<(), Box<dyn Error + Send + Sync>> {
        // Python: params['url'] = search_url + urlencode({'query': query, 'page': params['pageno'], 'per_page': page_size})
        // search_url = base_url + 'napi/search/photos?'
        // base_url = 'https://unsplash.com/'
        // page_size = 20
        let query_params = vec![
            ("query", query.to_string()),
            ("page", params.pageno.to_string()),
            ("per_page", "20".to_string()),
        ];

        let query_string = query_params.iter()
            .map(|(k, v)| format!("{}={}", k, urlencoding::encode(v)))
            .collect::<Vec<_>>()
            .join("&");
        
        params.url = Some(format!("https://unsplash.com/napi/search/photos?{}", query_string));
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
