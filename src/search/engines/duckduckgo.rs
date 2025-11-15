use async_trait::async_trait;
use std::collections::HashMap;
use std::error::Error;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};

use crate::derive::{
    EngineCapabilities, EngineInfo, EngineStatus, EngineType, 
    ResultType, SearchEngine, SearchQuery, SearchResult, 
    SearchResultItem, AboutInfo, RequestResponseEngine, RequestParams,
};
use crate::net::client::HttpClient;
use crate::net::types::{NetworkConfig, RequestOptions};

// VQD token cache entry
#[allow(dead_code)]
struct VqdCacheEntry {
    token: String,
    expires_at: SystemTime,
}

// Simple in-memory VQD cache
lazy_static::lazy_static! {
    static ref VQD_CACHE: Arc<Mutex<HashMap<String, VqdCacheEntry>>> = 
        Arc::new(Mutex::new(HashMap::new()));
}

pub struct DuckDuckGoEngine {
    info: EngineInfo,
    client: HttpClient,
}

impl DuckDuckGoEngine {
    pub fn new() -> Self {
        let net_config = NetworkConfig::default();
        let client = HttpClient::new(net_config).unwrap_or_else(|_| {
            panic!("Failed to create HTTP client for DuckDuckGo")
        });
        
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
            client,
        }
    }

    /// Quote DDG bangs - Python duckduckgo.py lines 226-237
    /// quote ddg bangs to avoid being parsed as DDG special commands
    fn quote_ddg_bangs(query: &str) -> String {
        // Python: for val in re.split(r'(\s+)', query):
        //     if not val.strip(): continue
        //     if val.startswith('!') and external_bang.get_node(external_bang.EXTERNAL_BANGS, val[1:]):
        //         val = f"'{val}'"
        //     query_parts.append(val)
        // return ' '.join(query_parts)
        
        // Simplified implementation: quote any word starting with !
        let parts: Vec<&str> = query.split_whitespace().collect();
        let quoted_parts: Vec<String> = parts.iter().map(|part| {
            if part.starts_with('!') && part.len() > 1 {
                format!("'{}'", part)
            } else {
                part.to_string()
            }
        }).collect();
        quoted_parts.join(" ")
    }

    #[allow(dead_code)]
    async fn get_vqd(&self, query: &str, region: &str) -> Result<String, Box<dyn Error + Send + Sync>> {
        let cache_key = format!("{}_{}", query, region);
        
        // Check cache
        {
            let mut cache = VQD_CACHE.lock().unwrap();
            if let Some(entry) = cache.get(&cache_key) {
                if entry.expires_at > SystemTime::now() {
                    return Ok(entry.token.clone());
                } else {
                    cache.remove(&cache_key);
                }
            }
        }

        // Fetch new VQD
        let url = format!("https://duckduckgo.com/?q={}", urlencoding::encode(query));
        let response = self.client.get(&url, None).await
            .map_err(|e| format!("Failed to fetch VQD: {}", e))?;
        let text = response.text().await
            .map_err(|e| format!("Failed to read response: {}", e))?;
        
        if let Some(start) = text.find("vqd=\"") {
            let text_from_vqd = &text[start + 5..];
            if let Some(end) = text_from_vqd.find('"') {
                let vqd = text_from_vqd[..end].to_string();
                
                // Cache for 1 hour
                let mut cache = VQD_CACHE.lock().unwrap();
                cache.insert(cache_key, VqdCacheEntry {
                    token: vqd.clone(),
                    expires_at: SystemTime::now() + Duration::from_secs(3600),
                });
                
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
        
        // Check for CAPTCHA - Python: if is_ddg_captcha(doc): raise SearxEngineCaptchaException
        let captcha_selector = Selector::parse("form#challenge-form").expect("valid selector");
        if document.select(&captcha_selector).next().is_some() {
            return Err("DDG CAPTCHA detected".into());
        }

        let mut items = Vec::with_capacity(10);

        // Python SearXNG: for div_result in eval_xpath(doc, '//div[@id="links"]/div[contains(@class, "web-result")]')
        // IMPORTANT: Only select .web-result, NOT .result--ad (ads)
        let result_selector = Selector::parse("div#links > div.web-result")
            .expect("valid selector");
        
        for result in document.select(&result_selector) {
            // Python: title = eval_xpath(div_result, './/h2/a')
            let title_selector = Selector::parse("h2 a").expect("valid selector");
            let title_elem = result.select(&title_selector).next();
            
            if title_elem.is_none() {
                // Python: if not title: continue (this is the "No results." item)
                continue;
            }
            
            let title_elem = title_elem.unwrap();
            // Python: item["title"] = extract_text(title)
            let title = title_elem.text().collect::<String>().trim().to_string();
            
            if title.is_empty() {
                continue;
            }
            
            // Python: item["url"] = eval_xpath(div_result, './/h2/a/@href')[0]
            let url = title_elem.value().attr("href")
                .unwrap_or("")
                .to_string();
            
            if url.is_empty() || !url.starts_with("http") {
                continue;
            }
            
            // Python: item["content"] = extract_text(eval_xpath_getindex(div_result, './/a[contains(@class, "result__snippet")]', 0, []))
            let content_selector = Selector::parse("a.result__snippet")
                .or_else(|_| Selector::parse("a[class*=\"result__snippet\"]"))
                .expect("valid selector");
            let content = result.select(&content_selector).next()
                .map(|c| c.text().collect::<String>().trim().to_string())
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
        self.client.get("https://duckduckgo.com", None).await.is_ok()
    }
}

#[async_trait]
impl RequestResponseEngine for DuckDuckGoEngine {
    type Response = String;

    fn request(&self, query: &str, params: &mut RequestParams) -> Result<(), Box<dyn Error + Send + Sync>> {
        // Python: query = quote_ddg_bangs(query) - line 241
        let query = Self::quote_ddg_bangs(query);
        
        if query.len() >= 500 {
            // Python: if len(query) >= 500:
            //     params["url"] = None
            //     return
            params.url = None;
            return Err("Query too long (max 499 chars)".into());
        }

        let region = params.custom.get("region").map(|s| s.as_str()).unwrap_or("wt-wt");
        
        // Python: Some locales (at least China) does not support pagination
        // if params['searxng_locale'].startswith("zh"):
        //     params["url"] = None
        //     return
        if region.starts_with("zh") && params.pageno > 1 {
            params.url = None;
            return Err("Chinese locale does not support pagination".into());
        }

        // Python SearXNG: The order of params['data'] dictionary matters for DDG bot detection!
        // Python code lines 267-308 show the exact order:
        // params['data']['q'] = query
        // if params['pageno'] == 1:
        //     params['data']['b'] = ""
        // elif params['pageno'] >= 2:
        //     params['data']['s'] = offset
        //     params['data']['nextParams'] = ''
        //     params['data']['v'] = 'l'
        //     params['data']['o'] = 'json'
        //     params['data']['dc'] = offset + 1
        //     params['data']['api'] = 'd.js'
        //     params['data']['vqd'] = vqd
        // params['data']['kl'] = eng_region or ""
        // params['data']['df'] = ''
        
        let mut form_data: Vec<(String, String)> = vec![("q".to_string(), query.to_string())];

        if params.pageno == 1 {
            form_data.push(("b".to_string(), String::new()));
        } else if params.pageno >= 2 {
            let offset = 10 + (params.pageno - 2) * 15;
            form_data.push(("s".to_string(), offset.to_string()));
            form_data.push(("nextParams".to_string(), String::new()));
            form_data.push(("v".to_string(), "l".to_string()));
            form_data.push(("o".to_string(), "json".to_string()));
            form_data.push(("dc".to_string(), (offset + 1).to_string()));
            form_data.push(("api".to_string(), "d.js".to_string()));
            
            // Note: vqd is required for page 2+
            // Python: vqd = get_vqd(query, eng_region, force_request=False)
            // if vqd: params['data']['vqd'] = vqd
            // else: params["url"] = None; return  # Don't try without vqd - DDG detects bots
            // For now we skip pagination > 1 to avoid bot detection
            // TODO: Implement proper VQD caching
        }

        // Python: Put empty kl in form data if language/region set to all
        // if eng_region == "wt-wt":
        //     params['data']['kl'] = ""
        // else:
        //     params['data']['kl'] = eng_region
        form_data.push(("kl".to_string(), if region == "wt-wt" { String::new() } else { region.to_string() }));

        // Python: params['data']['df'] = ''
        // if params['time_range'] in time_range_dict:
        //     params['data']['df'] = time_range_dict[params['time_range']]
        //     params['cookies']['df'] = time_range_dict[params['time_range']]
        let df_value = if let Some(ref tr) = params.time_range {
            match tr.as_str() {
                "day" => "d",
                "week" => "w",
                "month" => "m",
                "year" => "y",
                _ => "",
            }
        } else {
            ""
        };
        form_data.push(("df".to_string(), df_value.to_string()));
        
        if !df_value.is_empty() {
            params.cookies.insert("df".to_string(), df_value.to_string());
        }

        // Python: params['cookies']['kl'] = eng_region
        params.cookies.insert("kl".to_string(), region.to_string());
        
        params.url = Some("https://html.duckduckgo.com/html/".to_string());
        params.method = "POST".to_string();
        
        // 将 Vec 转换为 HashMap
        // 注意：HashMap 的迭代顺序不确定，但我们会在 fetch() 中按正确顺序重建
        params.data = Some(form_data.into_iter().collect());

        // Python SearXNG headers - critical for bot detection (lines 313-318)
        // params['headers']['Content-Type'] = 'application/x-www-form-urlencoded'
        // params['headers']['Referer'] = url
        // params['headers']['Sec-Fetch-Dest'] = "document"
        // params['headers']['Sec-Fetch-Mode'] = "navigate"  # at least this one is used by ddg's bot detection
        // params['headers']['Sec-Fetch-Site'] = "same-origin"
        // params['headers']['Sec-Fetch-User'] = "?1"
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
        
        let mut options = RequestOptions::default();
        options.timeout = Duration::from_secs(10);
        
        for (key, value) in &params.headers {
            options.headers.push((key.clone(), value.clone()));
        }

        let response = if params.method == "POST" {
            let form_data_map = params.data.as_ref().ok_or("POST data not set")?;
            
            // IMPORTANT: 必须按照特定顺序构建表单数据，因为 DDG 的机器人检测会检查顺序
            // 顺序: q, b/s+nextParams+v+o+dc+api, kl, df
            let mut form_parts = Vec::new();
            
            // 1. q (query) - 必须第一个
            if let Some(q) = form_data_map.get("q") {
                form_parts.push(format!("q={}", urlencoding::encode(q)));
            }
            
            // 2. b (第一页) 或 s, nextParams, v, o, dc, api (后续页)
            if let Some(b) = form_data_map.get("b") {
                form_parts.push(format!("b={}", urlencoding::encode(b)));
            } else {
                // 分页参数，按顺序添加
                if let Some(s) = form_data_map.get("s") {
                    form_parts.push(format!("s={}", urlencoding::encode(s)));
                }
                if let Some(np) = form_data_map.get("nextParams") {
                    form_parts.push(format!("nextParams={}", urlencoding::encode(np)));
                }
                if let Some(v) = form_data_map.get("v") {
                    form_parts.push(format!("v={}", urlencoding::encode(v)));
                }
                if let Some(o) = form_data_map.get("o") {
                    form_parts.push(format!("o={}", urlencoding::encode(o)));
                }
                if let Some(dc) = form_data_map.get("dc") {
                    form_parts.push(format!("dc={}", urlencoding::encode(dc)));
                }
                if let Some(api) = form_data_map.get("api") {
                    form_parts.push(format!("api={}", urlencoding::encode(api)));
                }
                if let Some(vqd) = form_data_map.get("vqd") {
                    form_parts.push(format!("vqd={}", urlencoding::encode(vqd)));
                }
            }
            
            // 3. kl (region/language)
            if let Some(kl) = form_data_map.get("kl") {
                form_parts.push(format!("kl={}", urlencoding::encode(kl)));
            }
            
            // 4. df (time filter)
            if let Some(df) = form_data_map.get("df") {
                form_parts.push(format!("df={}", urlencoding::encode(df)));
            }
            
            let body = form_parts.join("&");
            self.client.post(url, body.into_bytes(), Some(options)).await
        } else {
            self.client.get(url, Some(options)).await
        }.map_err(|e| format!("Request failed: {}", e))?;

        response.text().await.map_err(|e| format!("Failed to read response: {}", e).into())
    }

    fn response(&self, resp: Self::Response) -> Result<Vec<SearchResultItem>, Box<dyn Error + Send + Sync>> {
        Self::parse_html_results(&resp)
    }
}
