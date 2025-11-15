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

pub struct WikipediaEngine {
    info: EngineInfo,
    client: Arc<HttpClient>,
    display_type: Vec<String>, // Python: display_type = ["infobox"]
}

impl WikipediaEngine {
    pub fn new() -> Self {
        let client = HttpClient::new(NetworkConfig::default())
            .unwrap_or_else(|_| panic!("Failed to create HTTP client"));
        Self::with_client(Arc::new(client))
    }

    pub fn with_client(client: Arc<HttpClient>) -> Self {
        Self {
            info: EngineInfo {
                name: "Wikipedia".to_string(),
                engine_type: EngineType::General,
                description: "Wikipedia - Free encyclopedia".to_string(),
                status: EngineStatus::Active,
                categories: vec!["general".to_string(), "knowledge".to_string()],
                capabilities: EngineCapabilities {
                    result_types: vec![ResultType::Web],
                    supported_params: vec!["language".to_string()],
                    max_page_size: 1,
                    supports_pagination: false,
                    supports_time_range: false,
                    supports_language_filter: true,
                    supports_region_filter: false,
                    supports_safe_search: false,
                    rate_limit: Some(200),
                },
                about: AboutInfo {
                    website: Some("https://www.wikipedia.org/".to_string()),
                    wikidata_id: Some("Q52".to_string()),
                    official_api_documentation: Some("https://en.wikipedia.org/api/".to_string()),
                    use_official_api: true,
                    require_api_key: false,
                    results: "JSON".to_string(),
                },
                shortcut: Some("wp".to_string()),
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
            // Python line 77: display_type = ["infobox"]
            display_type: vec!["infobox".to_string()],
        }
    }

    /// Python: utils.html_to_text() - decodes HTML entities and strips tags
    fn html_to_text(html: &str) -> String {
        // Basic HTML entity decoding
        let text = html
            .replace("&lt;", "<")
            .replace("&gt;", ">")
            .replace("&amp;", "&")
            .replace("&quot;", "\"")
            .replace("&#39;", "'")
            .replace("&#x27;", "'")
            .replace("&#x2F;", "/")
            .replace("&nbsp;", " ");
        
        // Remove HTML tags (simplified)
        let re = regex::Regex::new(r"<[^>]*>").unwrap();
        let text = re.replace_all(&text, "");
        
        // Normalize whitespace
        let text = text.trim().to_string();
        text
    }

    fn parse_json_result(&self, json_str: &str) -> Result<Vec<SearchResultItem>, Box<dyn Error + Send + Sync>> {
        let api_result: Value = serde_json::from_str(json_str)?;
        
        let mut items = Vec::new();

        // Python line 188: title = utils.html_to_text(api_result.get('titles', {}).get('display') or api_result.get('title'))
        let title_raw = api_result.get("titles")
            .and_then(|t| t.get("display"))
            .and_then(|d| d.as_str())
            .or_else(|| api_result.get("title").and_then(|t| t.as_str()))
            .unwrap_or("")
            .to_string();
        
        if title_raw.is_empty() {
            return Ok(items);
        }
        
        let title = Self::html_to_text(&title_raw);

        // Python line 189: wikipedia_link = api_result['content_urls']['desktop']['page']
        let wikipedia_link = api_result.get("content_urls")
            .and_then(|c| c.get("desktop"))
            .and_then(|d| d.get("page"))
            .and_then(|p| p.as_str())
            .unwrap_or("")
            .to_string();
        
        if wikipedia_link.is_empty() {
            return Ok(items);
        }

        // Python lines 191-194: if "list" in display_type or api_result.get('type') != 'standard':
        //     results.append({'url': wikipedia_link, 'title': title, 'content': api_result.get('description', '')})
        let api_type = api_result.get("type").and_then(|t| t.as_str()).unwrap_or("");
        
        if self.display_type.contains(&"list".to_string()) || api_type != "standard" {
            // show item in the result list if 'list' is in the display options or it
            // is a item that can't be displayed in a infobox.
            let description = api_result.get("description")
                .and_then(|d| d.as_str())
                .unwrap_or("")
                .to_string();
            
            items.push(SearchResultItem {
                title: title.clone(),
                url: wikipedia_link.clone(),
                content: description,
                display_url: Some(wikipedia_link.clone()),
                site_name: Some("Wikipedia".to_string()),
                score: 1.0,
                result_type: ResultType::Web,
                thumbnail: None,
                published_date: None,
                template: None,
                metadata: HashMap::new(),
            });
        }

        // Python lines 196-206: if "infobox" in display_type:
        //     if api_result.get('type') == 'standard':
        //         results.append({ 'infobox': title, ... })
        if self.display_type.contains(&"infobox".to_string()) {
            if api_type == "standard" {
                let extract = api_result.get("extract")
                    .and_then(|e| e.as_str())
                    .unwrap_or("")
                    .to_string();
                
                let thumbnail = api_result.get("thumbnail")
                    .and_then(|t| t.get("source"))
                    .and_then(|s| s.as_str())
                    .map(|s| s.to_string());
                
                // Build infobox result
                let mut metadata = HashMap::new();
                metadata.insert("infobox".to_string(), title.clone());
                metadata.insert("id".to_string(), wikipedia_link.clone());
                metadata.insert("content".to_string(), extract.clone());
                if let Some(ref thumb) = thumbnail {
                    metadata.insert("img_src".to_string(), thumb.clone());
                }
                metadata.insert("urls".to_string(), format!("[{{\"title\":\"Wikipedia\",\"url\":\"{}\"}}]", wikipedia_link));
                
                items.push(SearchResultItem {
                    title: title.clone(),
                    url: wikipedia_link.clone(),
                    content: extract,
                    display_url: Some(wikipedia_link),
                    site_name: Some("Wikipedia".to_string()),
                    score: 1.0,
                    result_type: ResultType::Web,
                    thumbnail,
                    published_date: None,
                    template: Some("infobox".to_string()),
                    metadata,
                });
            }
        }

        Ok(items)
    }
}

impl Default for WikipediaEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SearchEngine for WikipediaEngine {
    fn info(&self) -> &EngineInfo {
        &self.info
    }

    async fn search(&self, query: &SearchQuery) -> Result<SearchResult, Box<dyn Error + Send + Sync>> {
        <Self as RequestResponseEngine>::search(self, query).await
    }

    async fn is_available(&self) -> bool {
        self.client.get("https://en.wikipedia.org", None).await.is_ok()
    }
}

#[async_trait]
impl RequestResponseEngine for WikipediaEngine {
    type Response = String;

    fn request(&self, query: &str, params: &mut RequestParams) -> Result<(), Box<dyn Error + Send + Sync>> {
        // Python line 154: if query.islower(): query = query.title()
        let query_title = if query.chars().all(|c| !c.is_uppercase()) {
            query.split_whitespace()
                .map(|word| {
                    let mut chars = word.chars();
                    match chars.next() {
                        None => String::new(),
                        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                    }
                })
                .collect::<Vec<_>>()
                .join(" ")
        } else {
            query.to_string()
        };

        // Determine language (default to English)
        let lang = params.language.as_deref().unwrap_or("en");
        let wiki_netloc = format!("{}.wikipedia.org", lang);
        
        // Python line 159: params['url'] = rest_v1_summary_url.format(wiki_netloc=wiki_netloc, title=title)
        let title = urlencoding::encode(&query_title);
        params.url = Some(format!("https://{}/api/rest_v1/page/summary/{}", wiki_netloc, title));
        params.method = "GET".to_string();
        
        // Python lines 161-162: 
        // params['raise_for_httperror'] = False
        // params['soft_max_redirects'] = 2
        params.custom.insert("raise_for_httperror".to_string(), "false".to_string());
        params.custom.insert("soft_max_redirects".to_string(), "2".to_string());

        Ok(())
    }

    async fn fetch(&self, params: &RequestParams) -> Result<Self::Response, Box<dyn Error + Send + Sync>> {
        let url = params.url.as_ref().ok_or("URL not set")?;
        
        let mut options = RequestOptions::default();
        options.timeout = Duration::from_secs(10);
        
        // Add Accept-Language header for language variants
        if let Some(lang) = &params.language {
            options.headers.push(("Accept-Language".to_string(), lang.clone()));
        }
        
        for (key, value) in &params.headers {
            options.headers.push((key.clone(), value.clone()));
        }

        let response = self.client.get(url, Some(options)).await
            .map_err(|e| format!("Request failed: {}", e))?;

        // Python handles 404 and 400 gracefully
        response.text().await.map_err(|e| format!("Failed to read response: {}", e).into())
    }

    fn response(&self, resp: Self::Response) -> Result<Vec<SearchResultItem>, Box<dyn Error + Send + Sync>> {
        // Handle error cases - Python returns empty array for 404
        if resp.contains("\"type\":\"https://mediawiki.org/wiki/HyperSwitch/errors/not_found\"") {
            return Ok(Vec::new());
        }
        if resp.contains("title-invalid-characters") {
            return Ok(Vec::new());
        }
        
        self.parse_json_result(&resp)
    }
}
