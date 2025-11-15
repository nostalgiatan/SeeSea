use async_trait::async_trait;
use std::collections::HashMap;
use std::error::Error;
use std::time::Duration;

use crate::derive::{
    EngineCapabilities, EngineInfo, EngineStatus, EngineType, 
    ResultType, SearchEngine, SearchQuery, SearchResult, 
    SearchResultItem, AboutInfo, RequestResponseEngine, RequestParams,
};
use crate::net::client::HttpClient;
use crate::net::types::{NetworkConfig, RequestOptions};
use super::utils::build_query_string_owned;

pub struct Search360Engine {
    info: EngineInfo,
    client: HttpClient,
}

impl Search360Engine {
    pub fn new() -> Self {
        let net_config = NetworkConfig::default();
        let client = HttpClient::new(net_config).unwrap_or_else(|_| {
            panic!("Failed to create HTTP client for 360Search")
        });
        
        Self {
            info: EngineInfo {
                name: "360Search".to_string(),
                engine_type: EngineType::General,
                description: "360Search - Chinese search engine".to_string(),
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
                    website: Some("https://www.so.com/".to_string()),
                    wikidata_id: Some("Q10846064".to_string()),
                    official_api_documentation: None,
                    use_official_api: false,
                    require_api_key: false,
                    results: "HTML".to_string(),
                },
                shortcut: Some("360so".to_string()),
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

        // Python: for item in dom.xpath('//li[contains(@class, "res-list")]'):
        let result_selector = Selector::parse("li.res-list")
            .or_else(|_| Selector::parse("li[class*=\"res-list\"]"))
            .expect("valid selector");
        
        for result in document.select(&result_selector) {
            // Python: title = extract_text(item.xpath('.//h3[contains(@class, "res-title")]/a'))
            let title_selector = Selector::parse("h3.res-title a")
                .or_else(|_| Selector::parse("h3[class*=\"res-title\"] a"))
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
            
            // Python: url = extract_text(item.xpath('.//h3[contains(@class, "res-title")]/a/@data-mdurl'))
            // if not url: url = extract_text(item.xpath('.//h3[contains(@class, "res-title")]/a/@href'))
            let url = title_elem.value().attr("data-mdurl")
                .or_else(|| title_elem.value().attr("href"))
                .unwrap_or("")
                .to_string();
            
            if url.is_empty() {
                continue;
            }
            
            // Python: content = extract_text(item.xpath('.//p[@class="res-desc"]'))
            // if not content: content = extract_text(item.xpath('.//span[@class="res-list-summary"]'))
            let content_selector = Selector::parse("p.res-desc")
                .expect("valid selector");
            let content = result.select(&content_selector).next()
                .map(|c| c.text().collect::<String>().trim().to_string())
                .or_else(|| {
                    let summary_selector = Selector::parse("span.res-list-summary").ok()?;
                    result.select(&summary_selector).next()
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

impl Default for Search360Engine {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SearchEngine for Search360Engine {
    fn info(&self) -> &EngineInfo {
        &self.info
    }

    async fn search(&self, query: &SearchQuery) -> Result<SearchResult, Box<dyn Error + Send + Sync>> {
        <Self as RequestResponseEngine>::search(self, query).await
    }

    async fn is_available(&self) -> bool {
        self.client.get("https://www.so.com", None).await.is_ok()
    }
}

#[async_trait]
impl RequestResponseEngine for Search360Engine {
    type Response = String;

    fn request(&self, query: &str, params: &mut RequestParams) -> Result<(), Box<dyn Error + Send + Sync>> {
        // Python: base_url = "https://www.so.com"
        // query_params = {"pn": params["pageno"], "q": query}
        let mut query_params = vec![
            ("pn", params.pageno.to_string()),
            ("q", query.to_string()),
        ];

        // Add time range filter if specified
        if let Some(ref tr) = params.time_range {
            let adv_t = match tr.as_str() {
                "day" => "d",
                "week" => "w",
                "month" => "m",
                "year" => "y",
                _ => "",
            };
            if !adv_t.is_empty() {
                query_params.push(("adv_t", adv_t.to_string()));
            }
        }

        // Build URL with optimized query string
        let query_string = build_query_string_owned(query_params.into_iter());
        
        params.url = Some(format!("https://www.so.com/s?{}", query_string));
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
        Self::parse_html_results(&resp)
    }
}
