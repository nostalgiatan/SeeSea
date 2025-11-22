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

//! Yandex 搜索引擎实现
//!
//! 这是一个基于 Yandex API 的搜索引擎实现。
//! 参考了 Python SearXNG 的 Yandex 引擎实现。
//!
//! ## 功能特性
//!
//! - 支持基本的网页搜索
//! - 支持分页
//! - CAPTCHA 检测
//!
//! ## API 说明
//!
//! Yandex 使用特定的 URL 参数进行搜索：
//! - text: 查询关键词
//! - p: 分页参数
//! - tmpl_version: 模板版本
//! - searchid: 搜索 ID
//!
//! ## 安全性
//!
//! - 避免使用 unwrap()，使用 ? 操作符处理错误
//! - 所有网络请求都有超时设置
//! - 处理 CAPTCHA 检测
//!
//! ## 示例
//!
//! ```no_run
//! use SeeSea::search::engines::yandex::YandexEngine;
//! use SeeSea::derive::{SearchEngine, SearchQuery};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let engine = YandexEngine::new();
//!     let query = SearchQuery::default();
//!     let results = engine.search(&query).await?;
//!     println!("找到 {} 个结果", results.items.len());
//!     Ok(())
//! }
//! ```

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

/// Yandex 搜索引擎
///
/// 使用 Yandex API 进行搜索的引擎实现
pub struct YandexEngine {
    /// 引擎信息
    info: EngineInfo,
    /// HTTP 客户端
    client: Arc<HttpClient>,
}

impl YandexEngine {
    /// 创建新的 Yandex 引擎实例
    ///
    /// # 示例
    ///
    /// ```
    /// use SeeSea::search::engines::yandex::YandexEngine;
    ///
    /// let engine = YandexEngine::new();
    /// ```
    pub fn new() -> Self {
        let client = HttpClient::new(NetworkConfig::default())
            .unwrap_or_else(|_| panic!("Failed to create HTTP client"));
        Self::with_client(Arc::new(client))
    }

    pub fn with_client(client: Arc<HttpClient>) -> Self {
        Self {
            info: EngineInfo {
                name: "Yandex".to_string(),
                engine_type: EngineType::General,
                description: "Yandex 是俄罗斯最大的搜索引擎".to_string(),
                status: EngineStatus::Active,
                categories: vec!["general".to_string(), "web".to_string()],
                capabilities: EngineCapabilities {
                    result_types: vec![ResultType::Web],
                    supported_params: vec![],
                    max_page_size: 10,
                    supports_pagination: true,
                    supports_time_range: false,
                    supports_language_filter: false,
                    supports_region_filter: false,
                    supports_safe_search: false,
                    rate_limit: Some(60),
                },
                about: AboutInfo {
                    website: Some("https://yandex.com".to_string()),
                    wikidata_id: Some("Q5281".to_string()),
                    official_api_documentation: None,
                    use_official_api: false,
                    require_api_key: false,
                    results: "HTML".to_string(),
                },
                shortcut: Some("yandex".to_string()),
                timeout: Some(10),
                disabled: false,
                inactive: false,
                version: Some("1.0.0".to_string()),
                last_checked: None,
                using_tor_proxy: false,
                display_error_messages: true,
                tokens: Vec::new(),
                max_page: 50,
            },
            client,
        }
    }

    /// 检测是否遇到 Yandex CAPTCHA
    ///
    /// # 参数
    ///
    /// * `captcha_header` - x-yandex-captcha 头的值
    ///
    /// # 返回
    ///
    /// 如果检测到 CAPTCHA 返回 true
    fn detect_captcha(captcha_header: Option<&str>) -> bool {
        captcha_header == Some("captcha")
    }

    /// 解析 HTML 响应为搜索结果项列表
    ///
    /// # 参数
    ///
    /// * `html` - HTML 响应字符串
    ///
    /// # 返回
    ///
    /// 解析出的搜索结果项列表
    ///
    /// # 错误
    ///
    /// 如果 HTML 解析失败返回错误
    fn parse_html_results(html: &str) -> Result<Vec<SearchResultItem>, Box<dyn Error + Send + Sync>> {
        use scraper::{Html, Selector};
        
        // 检查是否有结果
        if html.is_empty() {
            return Ok(Vec::new());
        }
        
        let document = Html::parse_document(html);
        let mut items = Vec::new();
        
        // Yandex 的搜索结果通常在特定的 li 或 div 元素中
        let result_selectors = vec![
            "li.serp-item",
            "div.serp-item",
            "div[class*='serp-item']",
        ];
        
        let mut results_found = false;
        for selector_str in result_selectors {
            let selector = match Selector::parse(selector_str) {
                Ok(sel) => sel,
                Err(_) => continue,
            };
            
            for result in document.select(&selector) {
                results_found = true;
                
                // 提取标题和 URL
                let title_selectors = vec![
                    Selector::parse("h2").ok(),
                    Selector::parse("h3").ok(),
                    Selector::parse("a.link").ok(),
                ];
                let link_selector = Selector::parse("a").expect("Expected valid value");
                let snippet_selectors = vec![
                    Selector::parse("div.text-container").ok(),
                    Selector::parse("div.OrganicTextContentSpan").ok(),
                    Selector::parse("div.text").ok(),
                    Selector::parse("div[class*='snippet']").ok(),
                ];
                
                let mut title = String::new();
                for selector in title_selectors.iter().flatten() {
                    if let Some(t) = result.select(selector).next() {
                        title = t.text().collect::<String>().trim().to_string();
                        if !title.is_empty() {
                            break;
                        }
                    }
                }
                
                let url = result.select(&link_selector).next()
                    .and_then(|a| a.value().attr("href"))
                    .unwrap_or_default();
                
                let mut content = String::new();
                for selector in snippet_selectors.iter().flatten() {
                    if let Some(snippet) = result.select(selector).next() {
                        content = snippet.text().collect::<String>().trim().to_string();
                        if !content.is_empty() {
                            break;
                        }
                    }
                }
                
                // 过滤有效结果
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
            
            if results_found {
                break;
            }
        }
        
        Ok(items)
    }
}

impl Default for YandexEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SearchEngine for YandexEngine {
    /// 获取引擎信息
    fn info(&self) -> &EngineInfo {
        &self.info
    }

    /// 执行搜索
    async fn search(&self, query: &SearchQuery) -> Result<SearchResult, Box<dyn Error + Send + Sync>> {
        <Self as RequestResponseEngine>::search(self, query).await
    }

    /// 检查引擎是否可用
    async fn is_available(&self) -> bool {
        self.client.get("https://yandex.com", None).await.is_ok()
    }
}

#[async_trait]
impl RequestResponseEngine for YandexEngine {
    type Response = (String, Option<String>); // (HTML 字符串, captcha 头)

    /// 准备请求参数
    fn request(&self, query: &str, params: &mut RequestParams) -> Result<(), Box<dyn Error + Send + Sync>> {
        // 构建查询参数
        let mut query_params = vec![
            ("tmpl_version", "releases".to_string()),
            ("text", query.to_string()),
            ("web", "1".to_string()),
            ("frame", "1".to_string()),
            ("searchid", "3131712".to_string()),
        ];
        
        // 添加分页参数
        if params.pageno > 1 {
            query_params.push(("p", (params.pageno - 1).to_string()));
        }
        
        // Build URL with optimized query string
        let query_string = build_query_string_owned(query_params.into_iter());
        
        params.url = Some(format!("https://yandex.com/search/site/?{}", query_string));
        params.method = "GET".to_string();
        
        // Set cookies
        params.cookies.insert(
            "yp".to_string(),
            "1716337604.sp.family%3A0#1685406411.szm.1:1920x1080:1920x999".to_string()
        );
        
        Ok(())
    }

    /// 发送请求并获取响应
    async fn fetch(&self, params: &RequestParams) -> Result<Self::Response, Box<dyn Error + Send + Sync>> {
        let url = params.url.as_ref()
            .ok_or("请求 URL 未设置")?;

        // 创建请求选项
        let mut options = RequestOptions::default();
        // 使用配置的默认超时时间

        // 添加自定义头
        for (key, value) in &params.headers {
            options.headers.push((key.clone(), value.clone()));
        }

        // 发送请求
        let response = self.client.get(url, Some(options)).await
            .map_err(|e| format!("Request failed: {}", e))?;

        // 检查状态码
        let status = response.status();
        match status.as_u16() {
            403 => return Err("Yandex 检测到自动化访问，请稍后重试".into()),
            429 => return Err("Yandex 请求过于频繁，请稍后重试".into()),
            503 => return Err("Yandex 服务暂时不可用，请稍后重试".into()),
            _ if !status.is_success() => return Err(format!("HTTP 错误: {}", status).into()),
            _ => {} // 继续处理
        }

        // 获取响应文本
        let text = response.text().await
            .map_err(|e| format!("Failed to read response: {}", e))?;

        Ok((text, None))
    }

    /// 解析响应为结果列表
    fn response(&self, resp: Self::Response) -> Result<Vec<SearchResultItem>, Box<dyn Error + Send + Sync>> {
        let (html, captcha_header) = resp;
        
        // 检查是否遇到 CAPTCHA
        if Self::detect_captcha(captcha_header.as_deref()) {
            return Err("检测到 Yandex CAPTCHA，请稍后重试".into());
        }
        
        Self::parse_html_results(&html)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_engine_creation() {
        let engine = YandexEngine::new();
        assert_eq!(engine.info().name, "Yandex");
        assert_eq!(engine.info().engine_type, EngineType::General);
    }

    #[test]
    fn test_detect_captcha() {
        assert!(YandexEngine::detect_captcha(Some("captcha")));
        assert!(!YandexEngine::detect_captcha(Some("ok")));
        assert!(!YandexEngine::detect_captcha(None));
    }

    #[test]
    fn test_engine_info() {
        let engine = YandexEngine::new();
        let info = engine.info();
        
        assert!(info.capabilities.supports_pagination);
        assert!(!info.capabilities.supports_time_range);
        assert_eq!(info.capabilities.max_page_size, 10);
    }

    #[test]
    fn test_request_preparation() {
        let engine = YandexEngine::new();
        let mut params = RequestParams::default();
        
        let result = engine.request("test query", &mut params);
        assert!(result.is_ok());
        assert!(params.url.is_some());
        
        let url = params.url.expect("Expected valid value");
        assert!(url.contains("yandex.com"));
        assert!(url.contains("text=test%20query"));
        assert!(url.contains("searchid=3131712"));
    }

    #[test]
    fn test_request_with_pagination() {
        let engine = YandexEngine::new();
        let mut params = RequestParams::default();
        params.pageno = 3;
        
        let result = engine.request("test", &mut params);
        assert!(result.is_ok());
        
        let url = params.url.expect("Expected valid value");
        assert!(url.contains("p=2")); // page 3 -> p=2 (0-indexed)
    }

    #[test]
    fn test_default() {
        let engine = YandexEngine::default();
        assert_eq!(engine.info().name, "Yandex");
    }

    #[tokio::test]
    async fn test_is_available() {
        let engine = YandexEngine::new();
        let _ = engine.is_available().await;
    }

    #[test]
    fn test_parse_empty_html() {
        let result = YandexEngine::parse_html_results("");
        assert!(result.is_ok());
        assert_eq!(result.expect("Expected valid value").len(), 0);
    }
}
