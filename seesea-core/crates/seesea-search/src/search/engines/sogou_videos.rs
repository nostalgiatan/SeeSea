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

use super::utils::build_query_string_owned;
use super::utils::time_extractor::{TimeSource, extract_time, extract_time_from_url};
use seesea_derive::{
    AboutInfo, EngineCapabilities, EngineInfo, EngineStatus, EngineType, RequestParams,
    RequestResponseEngine, ResultType, SearchResultItem,
};

// 使用宏定义引擎结构体和基本方法
define_engine! {
    SogouVideosEngine,
    EngineInfo {
        name: "Sogou Videos".to_string(),
        engine_type: EngineType::Video,
        description: "Sogou Videos - Chinese video search engine".to_string(),
        status: EngineStatus::Active,
        categories: vec!["videos".to_string()],
        capabilities: EngineCapabilities {
            result_types: vec![ResultType::Video],
            supported_params: vec!["page".to_string()],
            max_page_size: 10,
            supports_pagination: true,
            supports_time_range: false,
            supports_language_filter: false,
            supports_region_filter: false,
            supports_safe_search: false,
            rate_limit: Some(30),
        },
        about: AboutInfo {
            website: Some("https://v.sogou.com/".to_string()),
            wikidata_id: Some("Q7554565".to_string()), // Same as Sogou main
            official_api_documentation: None,
            use_official_api: false,
            require_api_key: false,
            results: "HTML".to_string(),
        },
        shortcut: Some("sogou vid".to_string()),
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

impl SogouVideosEngine {
    fn parse_html_results(
        html: &str,
    ) -> Result<Vec<SearchResultItem>, Box<dyn Error + Send + Sync>> {
        use scraper::{Html, Selector};

        if html.is_empty() {
            return Ok(Vec::new());
        }

        let document = Html::parse_document(html);
        let mut items = Vec::with_capacity(10);

        // Sogou video results - typical pattern for video listings
        let result_selector = Selector::parse("div.video-box")
            .or_else(|_| Selector::parse("div[class*=\"video\"]"))
            .or_else(|_| Selector::parse("li.vr-item"))
            .or_else(|_| Selector::parse("div.video-item"))
            .expect("valid selector");

        for result in document.select(&result_selector) {
            // Extract title from video title element
            let title_selector = Selector::parse("h3 a")
                .or_else(|_| Selector::parse("h4 a"))
                .or_else(|_| Selector::parse("a.video-title"))
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

            // Extract video URL
            let video_url = title_elem.value().attr("href").unwrap_or("").to_string();

            if video_url.is_empty() {
                continue;
            }

            // Extract thumbnail image
            let img_selector = Selector::parse("img").expect("valid selector");
            let thumbnail_url = result
                .select(&img_selector)
                .next()
                .and_then(|img| {
                    img.value()
                        .attr("src")
                        .or_else(|| img.value().attr("data-src"))
                        .or_else(|| img.value().attr("data-original"))
                })
                .map(|s| {
                    if s.starts_with("//") {
                        format!("https:{s}")
                    } else if !s.starts_with("http") {
                        format!("https://v.sogou.com{s}")
                    } else {
                        s.to_string()
                    }
                });

            // Extract description/content
            let content_selector = Selector::parse("p.desc")
                .or_else(|_| Selector::parse("p.video-desc"))
                .or_else(|_| Selector::parse("span.txt"))
                .expect("valid selector");
            let content = result
                .select(&content_selector)
                .next()
                .map(|c| c.text().collect::<String>().trim().to_string())
                .unwrap_or_default();

            // Extract duration if available
            let duration_selector = Selector::parse("span.duration")
                .or_else(|_| Selector::parse("span.time"))
                .expect("valid selector");
            let duration = result
                .select(&duration_selector)
                .next()
                .map(|d| d.text().collect::<String>().trim().to_string())
                .filter(|d| !d.is_empty());

            // Extract published date from result card
            let published_date = {
                // Try to extract from various date selectors
                let date_selector = Selector::parse("span.date")
                    .or_else(|_| Selector::parse("span.time"))
                    .or_else(|_| Selector::parse("div.info span"))
                    .or_else(|_| Selector::parse("span.publish-time"))
                    .expect("valid selector");

                let mut best_date = None;
                let mut best_confidence = 0.0;

                // Try from URL first - high confidence
                if let Some(dt) = extract_time_from_url(&video_url) {
                    best_date = Some(dt);
                    best_confidence = 0.9;
                }

                // Try from result card - check if confidence is higher than current best
                if let Some(date_elem) = result.select(&date_selector).next() {
                    let date_text = date_elem.text().collect::<String>().trim().to_string();
                    if !date_text.is_empty() {
                        let extract_result = extract_time(&date_text, TimeSource::ResultCard);
                        if let Some(dt) = extract_result.datetime
                            && extract_result.confidence > best_confidence
                        {
                            best_date = Some(dt);
                            // 提取置信度，但暂时不使用
                            let _ = extract_result.confidence;
                        }
                    }
                }

                // Try from content - only if current confidence is low
                if best_confidence < 0.8 {
                    let extract_result = extract_time(&content, TimeSource::Content);
                    if let Some(dt) = extract_result.datetime
                        && extract_result.confidence > best_confidence
                    {
                        best_date = Some(dt);
                        best_confidence = extract_result.confidence;
                    }
                }

                // 使用best_confidence变量，避免未使用警告
                if best_confidence > 0.5 {
                    // 如果置信度较高，将其存储在metadata中
                    let mut metadata = HashMap::new();
                    metadata.insert("date_confidence".to_string(), best_confidence.to_string());
                }

                best_date
            };

            let mut metadata = HashMap::new();
            if let Some(dur) = duration {
                metadata.insert("duration".to_string(), dur);
            }

            items.push(SearchResultItem {
                title,
                url: video_url.clone(),
                content,
                display_url: Some(video_url),
                site_name: None,
                score: 1.0,
                result_type: ResultType::Video,
                thumbnail: thumbnail_url,
                published_date,
                template: None,
                metadata,
            });
        }

        Ok(items)
    }
}

#[async_trait]
impl RequestResponseEngine for SogouVideosEngine {
    type Response = String;

    fn request(
        &self,
        query: &str,
        params: &mut RequestParams,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        // Sogou video search URL
        let query_params = vec![
            ("query", query.to_string()),
            ("page", params.page.to_string()),
        ];

        // Build URL with optimized query string
        let query_string = build_query_string_owned(query_params);

        params.url = Some(format!("https://v.sogou.com/v?{query_string}"));
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
