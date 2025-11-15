use async_trait::async_trait;
use std::collections::HashMap;
use std::error::Error;


use crate::derive::{
    EngineCapabilities, EngineInfo, EngineStatus, EngineType, 
    ResultType, SearchEngine, SearchQuery, SearchResult, 
    SearchResultItem, AboutInfo, RequestResponseEngine, RequestParams,
};

static VQD_CACHE: once_cell::sync::Lazy<std::sync::Mutex<HashMap<String, (String, std::time::SystemTime)>>> = 
    once_cell::sync::Lazy::new(|| std::sync::Mutex::new(HashMap::new()));

pub struct DuckDuckGoEngine {
    info: EngineInfo,
    client: reqwest::Client,
}

impl DuckDuckGoEngine {
    pub fn new() -> Self {
        Self {
            info: EngineInfo {
                name: "DuckDuckGo".to_string(),
                engine_type: EngineType::General,
                description: "Privacy-focused search engine".to_string(),
                status: EngineStatus::Active,
                categories: vec!["general".to_string(), "web".to_string()],
                capabilities: EngineCapabilities {
                    result_types: vec![ResultType::Web],
                    supported_params: vec!["region".to_string(), "time_range".to_string()],
                    max_page_size: 30,
                    supports_pagination: true,
                    supports_time_range: true,
                    supports_language_filter: false,
                    supports_region_filter: true,
                    supports_safe_search: true,
                    rate_limit: Some(60),
                },
                about: AboutInfo {
                    website: Some("https://duckduckgo.com".to_string()),
                    wikidata_id: Some("Q12805".to_string()),
                    official_api_documentation: None,
                    use_official_api: false,
                    require_api_key: false,
                    results: "HTML".to_string(),
                },
                shortcut: Some("ddg".to_string()),
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
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(10))
                .build().unwrap_or(reqwest::Client::new()),
        }
    }

    async fn get_vqd(&self, query: &str, region: &str) -> Result<String, Box<dyn Error + Send + Sync>> {
        let cache_key = format!("{}//{}", query, region);
        
        {
            let cache = VQD_CACHE.lock().unwrap();
            if let Some((vqd, time)) = cache.get(&cache_key) {
                if time.elapsed().unwrap_or(std::time::Duration::from_secs(3601)) < std::time::Duration::from_secs(3600) {
                    return Ok(vqd.clone());
                }
            }
        }

        let url = format!("https://duckduckgo.com/?q={}", urlencoding::encode(query));
        let resp = self.client.get(&url).send().await?;
        let text = resp.text().await?;
        
        if let Some(start) = text.find("vqd=\"") {
            let text_from_vqd = &text[start + 5..];
            if let Some(end) = text_from_vqd.find('"') {
                let vqd = text_from_vqd[..end].to_string();
                
                let mut cache = VQD_CACHE.lock().unwrap();
                cache.insert(cache_key, (vqd.clone(), std::time::SystemTime::now()));
                
                return Ok(vqd);
            }
        }

        Err("Failed to extract VQD token".into())
    }

    fn parse_html_results(html: &str) -> Result<Vec<SearchResultItem>, Box<dyn Error + Send + Sync>> {
        use scraper::{Html, Selector};

        if html.is_empty() {
            return Ok(Vec::new());
        }

        let document = Html::parse_document(html);
        let mut items = Vec::new();

        let result_selector = Selector::parse("div.result, article").ok();
        let title_selector = Selector::parse("h2 a, h3 a, a.result__a").ok();
        let url_selector = Selector::parse("a[href]").ok();
        let content_selector = Selector::parse("div.result__snippet, p").ok();

        if let Some(ref sel) = result_selector {
            for result in document.select(sel) {
                let title = if let Some(ref ts) = title_selector {
                    result.select(ts).next()
                        .map(|t| t.text().collect::<String>().trim().to_string())
                        .unwrap_or_default()
                } else {
                    String::new()
                };

                let url = if let Some(ref us) = url_selector {
                    result.select(us).next()
                        .and_then(|a| a.value().attr("href"))
                        .unwrap_or("")
                } else {
                    ""
                };

                let content = if let Some(ref cs) = content_selector {
                    result.select(cs).next()
                        .map(|c| c.text().collect::<String>().trim().to_string())
                        .unwrap_or_default()
                } else {
                    String::new()
                };

                if !title.is_empty() && !url.is_empty() && url.starts_with("http") {
                    items.push(SearchResultItem {
                        title,
                        url: url.to_string(),
                        content,
                        display_url: Some(url.to_string()),
                        site_name: None,
                        score: 1.0,
                        result_type: ResultType::Web,
                        thumbnail: None,
                        published_date: None,
                        template: None,
                        metadata: HashMap::new(),
                    });
                }
            }
        }

        Ok(items)
    }
}

impl Default for DuckDuckGoEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SearchEngine for DuckDuckGoEngine {
    fn info(&self) -> &EngineInfo {
        &self.info
    }

    async fn search(&self, query: &SearchQuery) -> Result<SearchResult, Box<dyn Error + Send + Sync>> {
        <Self as RequestResponseEngine>::search(self, query).await
    }

    async fn is_available(&self) -> bool {
        matches!(self.client.get("https://duckduckgo.com").send().await, Ok(resp) if resp.status().is_success())
    }
}

#[async_trait]
impl RequestResponseEngine for DuckDuckGoEngine {
    type Response = String;

    fn request(&self, query: &str, params: &mut RequestParams) -> Result<(), Box<dyn Error + Send + Sync>> {
        if query.len() >= 500 {
            params.url = None;
            return Err("Query too long (max 499 chars)".into());
        }

        let region = params.custom.get("region").map(|s| s.as_str()).unwrap_or("wt-wt");
        
        if region.starts_with("zh") && params.pageno > 1 {
            params.url = None;
            return Err("Chinese locale does not support pagination".into());
        }

        let mut form_data = vec![("q", query.to_string())];

        if params.pageno == 1 {
            form_data.push(("b", String::new()));
        } else {
            let offset = 10 + (params.pageno - 2) * 15;
            form_data.push(("s", offset.to_string()));
            form_data.push(("nextParams", String::new()));
            form_data.push(("v", "l".to_string()));
            form_data.push(("o", "json".to_string()));
            form_data.push(("dc", (offset + 1).to_string()));
            form_data.push(("api", "d.js".to_string()));
        }

        form_data.push(("kl", if region == "wt-wt" { String::new() } else { region.to_string() }));

        if let Some(ref tr) = params.time_range {
            let df = match tr.as_str() {
                "day" => "d",
                "week" => "w",
                "month" => "m",
                "year" => "y",
                _ => "",
            };
            form_data.push(("df", df.to_string()));
            params.cookies.insert("df".to_string(), df.to_string());
        } else {
            form_data.push(("df", String::new()));
        }

        params.cookies.insert("kl".to_string(), region.to_string());
        params.url = Some("https://html.duckduckgo.com/html/".to_string());
        params.method = "POST".to_string();
        params.data = Some(form_data.into_iter().map(|(k, v)| (k.to_string(), v)).collect());

        params.headers.insert("Content-Type".to_string(), "application/x-www-form-urlencoded".to_string());
        params.headers.insert("Referer".to_string(), "https://html.duckduckgo.com/html/".to_string());
        params.headers.insert("Sec-Fetch-Dest".to_string(), "document".to_string());
        params.headers.insert("Sec-Fetch-Mode".to_string(), "navigate".to_string());
        params.headers.insert("Sec-Fetch-Site".to_string(), "same-origin".to_string());
        params.headers.insert("Sec-Fetch-User".to_string(), "?1".to_string());

        Ok(())
    }

    async fn fetch(&self, params: &RequestParams) -> Result<Self::Response, Box<dyn Error + Send + Sync>> {
        let url = params.url.as_ref().ok_or("URL not set")?;
        
        let mut request = if params.method == "POST" {
            let form_data = params.data.as_ref().ok_or("POST data not set")?;
            let body = form_data.iter()
                .map(|(k, v)| format!("{}={}", k, urlencoding::encode(v)))
                .collect::<Vec<_>>()
                .join("&");
            self.client.post(url).body(body)
        } else {
            self.client.get(url)
        };

        for (key, value) in &params.headers {
            request = request.header(key, value);
        }

        for (key, value) in &params.cookies {
            request = request.header("Cookie", format!("{}={}", key, value));
        }

        let response = request.send().await?;
        Ok(response.text().await?)
    }

    fn response(&self, resp: Self::Response) -> Result<Vec<SearchResultItem>, Box<dyn Error + Send + Sync>> {
        Self::parse_html_results(&resp)
    }
}
