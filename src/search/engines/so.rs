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

pub struct SoEngine {
    info: EngineInfo,
    client: Arc<HttpClient>,
}

impl SoEngine {
    pub fn new() -> Self {
        let client = HttpClient::new(NetworkConfig::default())
            .unwrap_or_else(|_| panic!("Failed to create HTTP client"));
        Self::with_client(Arc::new(client))
    }

    pub fn with_client(client: Arc<HttpClient>) -> Self {
        Self {
            info: EngineInfo {
                name: "360 Search".to_string(),
                engine_type: EngineType::General,
                description: "360 Search - Chinese search engine".to_string(),
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
                    wikidata_id: Some("Q337939".to_string()),
                    official_api_documentation: None,
                    use_official_api: false,
                    require_api_key: false,
                    results: "HTML".to_string(),
                },
                shortcut: Some("so".to_string()),
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

        let res_list_selector = Selector::parse("li.res-list")
            .or_else(|_| Selector::parse("li[class*=\"res-list\"]"))
            .expect("valid selector");

        for result in document.select(&res_list_selector) {
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

            let url = title_elem.value().attr("href")
                .unwrap_or("")
                .to_string();

            let real_url = title_elem.value().attr("data-mdurl")
                .unwrap_or("")
                .to_string();

            let final_url = if !real_url.is_empty() { real_url } else { url.clone() };

            if final_url.is_empty() {
                continue;
            }

            let content = result.select(&Selector::parse("p").expect("valid selector")).next()
                .map(|c| c.text().collect::<String>().trim().to_string())
                .or_else(|| {
                    result.select(&Selector::parse("div[class*=\"desc\"]").expect("valid selector")).next()
                        .map(|c| c.text().collect::<String>().trim().to_string())
                })
                .or_else(|| {
                    result.select(&Selector::parse("div[class*=\"content\"]").expect("valid selector")).next()
                        .map(|c| c.text().collect::<String>().trim().to_string())
                })
                .unwrap_or_default();

            let display_url = result.select(&Selector::parse("cite").expect("valid selector")).next()
                .map(|c| c.text().collect::<String>().trim().to_string())
                .or_else(|| Some(final_url.clone()));

            items.push(SearchResultItem {
                title,
                url: final_url.clone(),
                content,
                display_url,
                site_name: None,
                score: 1.0,
                result_type: ResultType::Web,
                thumbnail: None,
                published_date: None,
                template: None,
                metadata: HashMap::new(),
            });
        }

        if items.is_empty() {
            let res_rich_selector = Selector::parse("div.res-rich")
                .or_else(|_| Selector::parse("div[class*=\"res-rich\"]"))
                .expect("valid selector");

            for result in document.select(&res_rich_selector) {
                let link_selector = Selector::parse("a").expect("valid selector");
                let links = result.select(&link_selector);

                for link in links {
                    let text = link.text().collect::<String>().trim().to_string();

                    if text.len() < 5 || text == "360软件宝库" {
                        continue;
                    }

                    let url = link.value().attr("href")
                        .unwrap_or("")
                        .to_string();

                    let real_url = link.value().attr("data-mdurl")
                        .unwrap_or("")
                        .to_string();

                    let final_url = if !real_url.is_empty() { real_url } else { url.clone() };

                    if final_url.is_empty() {
                        continue;
                    }

                    let text_content = result.text().collect::<String>().trim().to_string();
                    let lines: Vec<&str> = text_content.split('\n').collect();
                    let mut content = String::new();

                    for line in lines {
                        if line.len() > 30 && line != text {
                            content = line[..200.min(line.len())].to_string();
                            break;
                        }
                    }

                    items.push(SearchResultItem {
                        title: text,
                        url: final_url.clone(),
                        content,
                        display_url: Some(final_url),
                        site_name: None,
                        score: 1.0,
                        result_type: ResultType::Web,
                        thumbnail: None,
                        published_date: None,
                        template: None,
                        metadata: HashMap::new(),
                    });
                    break;
                }
            }
        }

        Ok(items)
    }
}

impl Default for SoEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SearchEngine for SoEngine {
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
impl RequestResponseEngine for SoEngine {
    type Response = String;

    fn request(&self, query: &str, params: &mut RequestParams) -> Result<(), Box<dyn Error + Send + Sync>> {
        let mut query_params = vec![
            ("q", query.to_string()),
            ("ie", "utf-8".to_string()),
            ("src", "srp".to_string()),
        ];

        if params.pageno > 1 {
            query_params.push(("pn", ((params.pageno - 1) * 10).to_string()));
        }

        if let Some(ref tr) = params.time_range {
            let time_filter = match tr.as_str() {
                "day" => "d",
                "week" => "w",
                "month" => "m",
                "year" => "y",
                _ => "",
            };
            if !time_filter.is_empty() {
                query_params.push(("adv", time_filter.to_string()));
            }
        }

        let query_string = build_query_string_owned(query_params.into_iter());

        params.url = Some(format!("https://www.so.com/s?{}", query_string));
        params.method = "GET".to_string();

        Ok(())
    }

    async fn fetch(&self, params: &RequestParams) -> Result<Self::Response, Box<dyn Error + Send + Sync>> {
        let url = params.url.as_ref().ok_or("URL not set")?;

        let mut options = RequestOptions::default();

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