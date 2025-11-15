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

pub struct GitHubEngine {
    info: EngineInfo,
    client: HttpClient,
}

impl GitHubEngine {
    pub fn new() -> Self {
        let net_config = NetworkConfig::default();
        let client = HttpClient::new(net_config).unwrap_or_else(|_| {
            panic!("Failed to create HTTP client for GitHub")
        });
        
        Self {
            info: EngineInfo {
                name: "GitHub".to_string(),
                engine_type: EngineType::Code,
                description: "GitHub - Code hosting platform".to_string(),
                status: EngineStatus::Active,
                categories: vec!["it".to_string(), "repos".to_string()],
                capabilities: EngineCapabilities {
                    result_types: vec![ResultType::Web],
                    supported_params: vec![],
                    max_page_size: 30,
                    supports_pagination: false,
                    supports_time_range: false,
                    supports_language_filter: false,
                    supports_region_filter: false,
                    supports_safe_search: false,
                    rate_limit: Some(60),
                },
                about: AboutInfo {
                    website: Some("https://github.com/".to_string()),
                    wikidata_id: Some("Q364".to_string()),
                    official_api_documentation: Some("https://developer.github.com/v3/".to_string()),
                    use_official_api: true,
                    require_api_key: false,
                    results: "JSON".to_string(),
                },
                shortcut: Some("gh".to_string()),
                timeout: Some(10),
                disabled: false,
                inactive: false,
                version: Some("1.0.0".to_string()),
                last_checked: None,
                using_tor_proxy: false,
                display_error_messages: true,
                tokens: Vec::new(),
                max_page: 1,
            },
            client,
        }
    }

    fn parse_json_result(json_str: &str) -> Result<Vec<SearchResultItem>, Box<dyn Error + Send + Sync>> {
        let api_result: Value = serde_json::from_str(json_str)?;
        let mut items = Vec::new();

        // Python: for item in resp.json().get('items', []):
        if let Some(items_array) = api_result.get("items").and_then(|i| i.as_array()) {
            for item in items_array {
                // Python: 'title': item.get('full_name')
                let title = item.get("full_name")
                    .and_then(|t| t.as_str())
                    .unwrap_or("")
                    .to_string();
                
                if title.is_empty() {
                    continue;
                }

                // Python: 'url': item.get('html_url')
                let url = item.get("html_url")
                    .and_then(|u| u.as_str())
                    .unwrap_or("")
                    .to_string();
                
                if url.is_empty() {
                    continue;
                }

                // Python: content = [item.get(i) for i in ['language', 'description'] if item.get(i)]
                // content': ' / '.join(content)
                let mut content_parts = Vec::new();
                if let Some(lang) = item.get("language").and_then(|l| l.as_str()) {
                    if !lang.is_empty() {
                        content_parts.push(lang.to_string());
                    }
                }
                if let Some(desc) = item.get("description").and_then(|d| d.as_str()) {
                    if !desc.is_empty() {
                        content_parts.push(desc.to_string());
                    }
                }
                let content = content_parts.join(" / ");

                // Python: 'thumbnail': item.get('owner', {}).get('avatar_url')
                let thumbnail = item.get("owner")
                    .and_then(|o| o.get("avatar_url"))
                    .and_then(|a| a.as_str())
                    .map(|s| s.to_string());

                // Python: 'popularity': item.get('stargazers_count')
                let mut metadata = HashMap::new();
                if let Some(stars) = item.get("stargazers_count").and_then(|s| s.as_i64()) {
                    metadata.insert("stars".to_string(), stars.to_string());
                }
                if let Some(forks) = item.get("forks_count").and_then(|f| f.as_i64()) {
                    metadata.insert("forks".to_string(), forks.to_string());
                }

                items.push(SearchResultItem {
                    title,
                    url: url.clone(),
                    content,
                    display_url: Some(url),
                    site_name: Some("GitHub".to_string()),
                    score: 1.0,
                    result_type: ResultType::Web,
                    thumbnail,
                    published_date: None,
                    template: None,
                    metadata,
                });
            }
        }

        Ok(items)
    }
}

impl Default for GitHubEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SearchEngine for GitHubEngine {
    fn info(&self) -> &EngineInfo {
        &self.info
    }

    async fn search(&self, query: &SearchQuery) -> Result<SearchResult, Box<dyn Error + Send + Sync>> {
        <Self as RequestResponseEngine>::search(self, query).await
    }

    async fn is_available(&self) -> bool {
        self.client.get("https://api.github.com", None).await.is_ok()
    }
}

#[async_trait]
impl RequestResponseEngine for GitHubEngine {
    type Response = String;

    fn request(&self, query: &str, params: &mut RequestParams) -> Result<(), Box<dyn Error + Send + Sync>> {
        // Python: search_url = 'https://api.github.com/search/repositories?sort=stars&order=desc&{query}'
        // params['url'] = search_url.format(query=urlencode({'q': query}))
        let query_encoded = urlencoding::encode(query);
        params.url = Some(format!(
            "https://api.github.com/search/repositories?sort=stars&order=desc&q={}",
            query_encoded
        ));
        params.method = "GET".to_string();
        
        // Python: params['headers']['Accept'] = accept_header
        // accept_header = 'application/vnd.github.preview.text-match+json'
        params.headers.insert(
            "Accept".to_string(), 
            "application/vnd.github.preview.text-match+json".to_string()
        );

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
