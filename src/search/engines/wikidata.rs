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
use super::utils::build_query_string_owned;

pub struct WikidataEngine {
    info: EngineInfo,
    client: Arc<HttpClient>,
}

impl WikidataEngine {
    pub fn new() -> Self {
        let client = HttpClient::new(NetworkConfig::default())
            .unwrap_or_else(|_| panic!("Failed to create HTTP client"));
        Self::with_client(Arc::new(client))
    }

    pub fn with_client(client: Arc<HttpClient>) -> Self {
        Self {
            info: EngineInfo {
                name: "Wikidata".to_string(),
                engine_type: EngineType::General,
                description: "Wikidata - Free knowledge base".to_string(),
                status: EngineStatus::Active,
                categories: vec!["general".to_string(), "knowledge".to_string()],
                capabilities: EngineCapabilities {
                    result_types: vec![ResultType::Web],
                    supported_params: vec!["language".to_string()],
                    max_page_size: 7,
                    supports_pagination: false,
                    supports_time_range: false,
                    supports_language_filter: true,
                    supports_region_filter: false,
                    supports_safe_search: false,
                    rate_limit: Some(200),
                },
                about: AboutInfo {
                    website: Some("https://wikidata.org/".to_string()),
                    wikidata_id: Some("Q2013".to_string()),
                    official_api_documentation: Some("https://query.wikidata.org/".to_string()),
                    use_official_api: true,
                    require_api_key: false,
                    results: "JSON".to_string(),
                },
                shortcut: Some("wd".to_string()),
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

        // Simplified: use wbsearchentities API
        if let Some(search_array) = api_result.get("search").and_then(|s| s.as_array()) {
            for result in search_array {
                // Get label (title)
                let title = result.get("label")
                    .and_then(|l| l.as_str())
                    .unwrap_or("")
                    .to_string();
                
                if title.is_empty() {
                    continue;
                }

                // Get concept URI (URL)
                let url = result.get("concepturi")
                    .and_then(|u| u.as_str())
                    .unwrap_or("")
                    .to_string();
                
                if url.is_empty() {
                    continue;
                }

                // Get description (content)
                let content = result.get("description")
                    .and_then(|d| d.as_str())
                    .unwrap_or("")
                    .to_string();

                items.push(SearchResultItem {
                    title,
                    url: url.clone(),
                    content,
                    display_url: Some(url),
                    site_name: Some("Wikidata".to_string()),
                    score: 1.0,
                    result_type: ResultType::Web,
                    thumbnail: None,
                    published_date: None,
                    template: None,
                    metadata: HashMap::new(),
                });
            }
        }

        Ok(items)
    }
}

impl Default for WikidataEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SearchEngine for WikidataEngine {
    fn info(&self) -> &EngineInfo {
        &self.info
    }

    async fn search(&self, query: &SearchQuery) -> Result<SearchResult, Box<dyn Error + Send + Sync>> {
        <Self as RequestResponseEngine>::search(self, query).await
    }

    async fn is_available(&self) -> bool {
        self.client.get("https://www.wikidata.org", None).await.is_ok()
    }
}

#[async_trait]
impl RequestResponseEngine for WikidataEngine {
    type Response = String;

    fn request(&self, query: &str, params: &mut RequestParams) -> Result<(), Box<dyn Error + Send + Sync>> {
        // Use simpler wbsearchentities API instead of complex SPARQL
        let lang = params.language.as_deref().unwrap_or("en");
        
        let query_params = vec![
            ("action", "wbsearchentities".to_string()),
            ("search", query.to_string()),
            ("language", lang.to_string()),
            ("limit", "7".to_string()),
            ("format", "json".to_string()),
        ];

        let query_string = build_query_string_owned(query_params.into_iter());
        
        params.url = Some(format!("https://www.wikidata.org/w/api.php?{}", query_string));
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
