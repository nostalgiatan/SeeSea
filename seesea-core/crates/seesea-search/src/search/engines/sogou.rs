// Copyright (C) 2025 nostalgiatan
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

use async_trait::async_trait;
use std::collections::HashMap;
use std::error::Error;
use std::sync::Arc;

use super::utils::{build_query_string_owned, time_extractor};
use seesea_derive::{
    AboutInfo, EngineCapabilities, EngineInfo, EngineStatus, EngineType, RequestParams,
    RequestResponseEngine, ResultType, SearchResultItem,
};

// 使用宏定义引擎结构体和基本方法
define_engine! {
    SogouEngine,
    EngineInfo {
        name: "Sogou".to_string(),
        engine_type: EngineType::General,
        description: "Sogou - Chinese search engine".to_string(),
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
            website: Some("https://www.sogou.com/".to_string()),
            wikidata_id: Some("Q7554565".to_string()),
            official_api_documentation: None,
            use_official_api: false,
            require_api_key: false,
            results: "HTML".to_string(),
        },
        shortcut: Some("sogou".to_string()),
        timeout: Some(10),
        disabled: false,
        inactive: false,
        version: Some("1.0.0".to_string()),
        last_checked: None,
        using_tor_proxy: false,
        display_error_messages: true,
        tokens: Vec::new(),
        max_page: 10,
    }
}

impl SogouEngine {
    fn parse_html_results(
        html: &str,
    ) -> Result<Vec<SearchResultItem>, Box<dyn Error + Send + Sync>> {
        use scraper::{Html, Selector};

        if html.is_empty() {
            return Ok(Vec::new());
        }

        let document = Html::parse_document(html);
        let mut items = Vec::with_capacity(10);

        let result_selector = Selector::parse("div.vrwrap")
            .or_else(|_| Selector::parse("div[class*=\"vrwrap\"]"))
            .expect("valid selector");

        for result in document.select(&result_selector) {
            let title_selector = Selector::parse("h3.vr-title a")
                .or_else(|_| Selector::parse("h3[class*=\"vr-title\"] a"))
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

            let url_elem = title_elem;
            let url = url_elem.value().attr("href").unwrap_or("").to_string();

            // Handle redirect URLs: if url.startswith("/link?url="):
            if url.starts_with("/link?url=") {
                // In real implementation, we might need to resolve this redirect
                // For now, construct the full URL
                continue; // Skip redirects for now as they require special handling
            }

            if url.is_empty() {
                continue;
            }

            // if not content: content = extract_text(item.xpath('.//div[contains(@class, "fz-mid space-txt")]'))
            let content = result
                .select(&Selector::parse("div.text-layout p.star-wiki").expect("valid selector"))
                .next()
                .map(|c| c.text().collect::<String>().trim().to_string())
                .or_else(|| {
                    result
                        .select(&Selector::parse("div.fz-mid.space-txt").expect("valid selector"))
                        .next()
                        .map(|c| c.text().collect::<String>().trim().to_string())
                })
                .unwrap_or_default();

            // 提取时间
            let mut published_date = None;

            // 尝试从结果卡片中提取时间
            let time_selectors = [
                "span.f13",
                "div.f13",
                "span[class*=\"time\"]",
                "span[class*=\"date\"]",
                "div[class*=\"info\"]",
            ];

            for selector_str in time_selectors {
                if let Ok(selector) = Selector::parse(selector_str) {
                    for elem in result.select(&selector) {
                        let text = elem.text().collect::<String>().trim().to_string();
                        if !text.is_empty() {
                            // 使用时间提取器提取时间
                            let time_result = time_extractor::extract_time(
                                &text,
                                time_extractor::TimeSource::ResultCard,
                            );
                            if time_result.datetime.is_some() {
                                published_date = time_result.datetime;
                                break;
                            }
                        }
                    }
                    if published_date.is_some() {
                        break;
                    }
                }
            }

            items.push(SearchResultItem {
                title,
                url: url.clone(),
                content,
                display_url: Some(url),
                site_name: None,
                score: 1.0,
                result_type: ResultType::Web,
                thumbnail: None,
                published_date,
                template: None,
                metadata: HashMap::new(),
            });
        }

        Ok(items)
    }
}

#[async_trait]
impl RequestResponseEngine for SogouEngine {
    type Response = String;

    fn request(
        &self,
        query: &str,
        params: &mut RequestParams,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        // query_params = {"query": query, "page": params["pageno"]}
        let mut query_params = vec![
            ("query", query.to_string()),
            ("page", params.page.to_string()),
        ];

        // Add time range filter if specified
        if let Some(ref tr) = params.time_range {
            let s_from = match tr.as_str() {
                "day" => "inttime_day",
                "week" => "inttime_week",
                "month" => "inttime_month",
                "year" => "inttime_year",
                _ => "",
            };
            if !s_from.is_empty() {
                query_params.push(("s_from", s_from.to_string()));
                query_params.push(("tsn", "1".to_string()));
            }
        }

        // Build URL with optimized query string
        let query_string = build_query_string_owned(query_params);

        params.url = Some(format!("https://www.sogou.com/web?{query_string}"));
        params.method = "GET".to_string();

        Ok(())
    }

    async fn fetch(
        &self,
        params: &RequestParams,
    ) -> Result<Self::Response, Box<dyn Error + Send + Sync>> {
        // 使用通用引擎的fetch方法
        self.generic.fetch(params).await
    }

    fn response(
        &self,
        resp: Self::Response,
    ) -> Result<Vec<SearchResultItem>, Box<dyn Error + Send + Sync>> {
        Self::parse_html_results(&resp)
    }
}
