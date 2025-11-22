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

use crate::derive::{
    EngineCapabilities, EngineInfo, EngineStatus, EngineType,
    ResultType, SearchEngine, SearchQuery, SearchResult,
    SearchResultItem, AboutInfo, RequestResponseEngine, RequestParams,
};
use crate::net::client::HttpClient;
use crate::net::types::{NetworkConfig, RequestOptions};
use super::utils::build_query_string_owned;

pub struct BingImagesEngine {
    info: EngineInfo,
    client: Arc<HttpClient>,
}

impl BingImagesEngine {
    pub fn new() -> Self {
        let client = HttpClient::new(NetworkConfig::default())
            .unwrap_or_else(|_| panic!("Failed to create HTTP client"));
        Self::with_client(Arc::new(client))
    }

    pub fn with_client(client: Arc<HttpClient>) -> Self {
        Self {
            info: EngineInfo {
                name: "Bing Images".to_string(),
                engine_type: EngineType::Image,
                description: "Bing Images - Microsoft's image search engine".to_string(),
                status: EngineStatus::Active,
                categories: vec!["images".to_string(), "web".to_string()],
                capabilities: EngineCapabilities {
                    result_types: vec![ResultType::Image],
                    supported_params: vec!["page".to_string(), "time_range".to_string()],
                    max_page_size: 35,
                    supports_pagination: true,
                    supports_time_range: true,
                    supports_language_filter: false,
                    supports_region_filter: false,
                    supports_safe_search: true,
                    rate_limit: Some(30),
                },
                about: AboutInfo {
                    website: Some("https://www.bing.com/images".to_string()),
                    wikidata_id: Some("Q182496".to_string()),
                    official_api_documentation: Some("https://www.microsoft.com/en-us/bing/apis/bing-image-search-api".to_string()),
                    use_official_api: false,
                    require_api_key: false,
                    results: "HTML".to_string(),
                },
                shortcut: Some("bing img".to_string()),
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
        use serde_json;

        if html.is_empty() {
            return Ok(Vec::new());
        }

        let document = Html::parse_document(html);
        let mut items = Vec::with_capacity(35);

        // Python: for result in dom.xpath('//ul[contains(@class, "dgControl_list")]/li'):
        let result_selector = Selector::parse("ul.dgControl_list li")
            .or_else(|_| Selector::parse("ul[class*=\"dgControl_list\"] li"))
            .expect("valid selector");

        for result in document.select(&result_selector) {
            // Python: metadata = result.xpath('.//a[@class="iusc"]/@m')
            let metadata_elem = result.select(&Selector::parse("a.iusc").expect("valid selector")).next();

            if metadata_elem.is_none() {
                continue;
            }

            let metadata_elem = metadata_elem.unwrap();
            let metadata_str = metadata_elem.value().attr("m").unwrap_or("");

            if metadata_str.is_empty() {
                continue;
            }

            // Parse metadata JSON
            let metadata: HashMap<String, serde_json::Value> = serde_json::from_str(metadata_str)
                .unwrap_or_else(|_| HashMap::new());

            // Python: title = ' '.join(result.xpath('.//div[@class="infnmpt"]//a/text()')).strip()
            let title = result.select(&Selector::parse("div.infnmpt a").expect("valid selector"))
                .map(|a| a.text().collect::<String>().trim().to_string())
                .collect::<Vec<_>>()
                .join(" ")
                .trim()
                .to_string();

            // Python: img_format = ' '.join(result.xpath('.//div[@class="imgpt"]/div/span/text()')).strip().split(" · ")
            let img_format = result.select(&Selector::parse("div.imgpt div span").expect("valid selector"))
                .map(|span| span.text().collect::<String>().trim().to_string())
                .collect::<Vec<_>>()
                .join(" ")
                .trim()
                .to_string();

            // Python: source = ' '.join(result.xpath('.//div[@class="imgpt"]//div[@class="lnkw"]//a/text()')).strip()
            let source = result.select(&Selector::parse("div.imgpt div.lnkw a").expect("valid selector"))
                .map(|a| a.text().collect::<String>().trim().to_string())
                .collect::<Vec<_>>()
                .join(" ")
                .trim()
                .to_string();

            let img_src = metadata.get("murl")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let thumbnail_src = metadata.get("turl")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let page_url = metadata.get("purl")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let content = metadata.get("desc")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            if img_src.is_empty() {
                continue;
            }

            let mut meta = HashMap::new();
            meta.insert("source".to_string(), source);
            meta.insert("format".to_string(), img_format.clone());
            if !img_format.is_empty() {
                let parts: Vec<&str> = img_format.split(" · ").collect();
                if let Some(resolution) = parts.first() {
                    meta.insert("resolution".to_string(), resolution.to_string());
                }
                if let Some(format) = parts.get(1) {
                    meta.insert("img_format".to_string(), format.to_string());
                }
            }

            items.push(SearchResultItem {
                title,
                url: page_url.clone(),
                content,
                display_url: if !page_url.is_empty() { Some(page_url) } else { Some(img_src.clone()) },
                site_name: None,
                score: 1.0,
                result_type: ResultType::Image,
                thumbnail: if !thumbnail_src.is_empty() { Some(thumbnail_src) } else { Some(img_src.clone()) },
                published_date: None,
                template: Some("images.html".to_string()),
                metadata: {
                    let mut final_meta = meta;
                    final_meta.insert("image_url".to_string(), img_src);
                    final_meta
                },
            });
        }

        Ok(items)
    }
}

impl Default for BingImagesEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SearchEngine for BingImagesEngine {
    fn info(&self) -> &EngineInfo {
        &self.info
    }

    async fn search(&self, query: &SearchQuery) -> Result<SearchResult, Box<dyn Error + Send + Sync>> {
        <Self as RequestResponseEngine>::search(self, query).await
    }

    async fn is_available(&self) -> bool {
        self.client.get("https://www.bing.com/images", None).await.is_ok()
    }
}

#[async_trait]
impl RequestResponseEngine for BingImagesEngine {
    type Response = String;

    fn request(&self, query: &str, params: &mut RequestParams) -> Result<(), Box<dyn Error + Send + Sync>> {
        // Python: base_url = 'https://www.bing.com/images/async'
        let base_url = "https://www.bing.com/images/async";

        // Python: time_map = {'day': 60 * 24, 'week': 60 * 24 * 7, 'month': 60 * 24 * 31, 'year': 60 * 24 * 365}
        let time_map = HashMap::from([
            ("day", 60 * 24),
            ("week", 60 * 24 * 7),
            ("month", 60 * 24 * 31),
            ("year", 60 * 24 * 365),
        ]);

        // Python: query_params = {'q': query, 'async': '1', 'first': (int(params.get('pageno', 1)) - 1) * 35 + 1, 'count': 35}
        let mut query_params = vec![
            ("q", query.to_string()),
            ("async", "1".to_string()),
            ("first", ((params.pageno - 1) * 35 + 1).to_string()),
            ("count", "35".to_string()),
        ];

        // Add time range filter if specified
        // Python: if params['time_range']: query_params['qft'] = 'filterui:age-lt%s' % time_map[params['time_range']]
        if let Some(ref tr) = params.time_range {
            if let Some(minutes) = time_map.get(tr.as_str()) {
                query_params.push(("qft", format!("filterui:age-lt{}", minutes)));
            }
        }

        // Build URL with optimized query string
        let query_string = build_query_string_owned(query_params.into_iter());

        params.url = Some(format!("{}?{}", base_url, query_string));
        params.method = "GET".to_string();

        Ok(())
    }

    async fn fetch(&self, params: &RequestParams) -> Result<Self::Response, Box<dyn Error + Send + Sync>> {
        let url = params.url.as_ref().ok_or("URL not set")?;

        let mut options = RequestOptions::default();
        // 使用配置的默认超时时间

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