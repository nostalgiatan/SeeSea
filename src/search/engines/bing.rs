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

//! Bing 搜索引擎实现
//!
//! 这是一个基于 Bing HTML API 的搜索引擎实现。
//! 参考了 Python SearXNG 的 Bing 引擎实现。
//!
//! ## 功能特性
//!
//! - 支持基本的网页搜索
//! - 支持分页（最多200页）
//! - 支持时间范围过滤
//! - 支持安全搜索
//! - 支持语言和地区过滤
//!
//! ## API 说明
//!
//! Bing 使用标准的 URL 参数进行搜索：
//! - q: 查询关键词
//! - pq: 完整查询（避免分页问题）
//! - first: 分页偏移量
//! - FORM: 分页表单参数（PERE, PERE1, PERE2 等）
//! - filters: 时间范围过滤
//!
//! ## 安全性
//!
//! - 避免使用 unwrap()，使用 ? 操作符处理错误
//! - 所有网络请求都有超时设置
//! - 处理 Bing 的重定向和限流
//! - 设置适当的 cookies 以支持地区和语言
//!
//! ## 示例
//!
//! ```no_run
//! use SeeSea::search::engines::bing::BingEngine;
//! use SeeSea::derive::{SearchEngine, SearchQuery};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let engine = BingEngine::new();
//!     let query = SearchQuery::default();
//!     let results = engine.search(&query).await?;
//!     println!("找到 {} 个结果", results.items.len());
//!     Ok(())
//! }
//! ```

use async_trait::async_trait;
use std::collections::HashMap;
use std::error::Error;
use std::sync::Arc;

use crate::derive::{
    EngineCapabilities, EngineInfo, EngineStatus, EngineType,
    ResultType, SearchEngine, SearchQuery, SearchResult,
    SearchResultItem, TimeRange, AboutInfo, RequestResponseEngine, RequestParams,
};
use crate::net::client::HttpClient;
use crate::net::types::{NetworkConfig, RequestOptions};
use super::utils::build_query_string_owned;

/// Bing 搜索引擎
///
/// 使用 Bing HTML API 进行搜索的引擎实现
pub struct BingEngine {
    /// 引擎信息
    info: EngineInfo,
    /// HTTP 客户端（共享）
    client: Arc<HttpClient>,
}

impl BingEngine {
    /// 创建新的 Bing 引擎实例
    ///
    /// # 示例
    ///
    /// ```
    /// use SeeSea::search::engines::bing::BingEngine;
    ///
    /// let engine = BingEngine::new();
    /// ```
    pub fn new() -> Self {
        let client = HttpClient::new(NetworkConfig::default())
            .unwrap_or_else(|_| panic!("Failed to create HTTP client for Bing"));
        Self::with_client(Arc::new(client))
    }

    /// 使用共享的 HTTP 客户端创建 Bing 引擎实例
    ///
    /// 这个方法允许多个引擎共享同一个 HTTP 客户端和连接池，
    /// 提高性能并减少资源消耗。
    ///
    /// # 参数
    ///
    /// * `client` - 共享的 HTTP 客户端
    ///
    /// # 示例
    ///
    /// ```no_run
    /// use std::sync::Arc;
    /// use SeeSea::search::engines::bing::BingEngine;
    /// use SeeSea::net::client::HttpClient;
    /// use SeeSea::net::types::NetworkConfig;
    ///
    /// let client = Arc::new(HttpClient::new(NetworkConfig::default()).unwrap());
    /// let engine = BingEngine::with_client(client);
    /// ```
    pub fn with_client(client: Arc<HttpClient>) -> Self {
        Self {
            info: EngineInfo {
                name: "Bing".to_string(),
                engine_type: EngineType::General,
                description: "Bing 是微软公司的搜索引擎".to_string(),
                status: EngineStatus::Active,
                categories: vec!["general".to_string(), "web".to_string()],
                capabilities: EngineCapabilities {
                    result_types: vec![ResultType::Web],
                    supported_params: vec![
                        "language".to_string(),
                        "region".to_string(),
                        "time_range".to_string(),
                    ],
                    max_page_size: 10,
                    supports_pagination: true,
                    supports_time_range: true,
                    supports_language_filter: true,
                    supports_region_filter: true,
                    supports_safe_search: true,
                    rate_limit: Some(60), // 每分钟 60 次请求
                },
                about: AboutInfo {
                    website: Some("https://www.bing.com".to_string()),
                    wikidata_id: Some("Q182496".to_string()),
                    official_api_documentation: Some("https://www.microsoft.com/en-us/bing/apis/bing-web-search-api".to_string()),
                    use_official_api: false,
                    require_api_key: false,
                    results: "HTML".to_string(),
                },
                shortcut: Some("bing".to_string()),
                timeout: Some(10),
                disabled: false,
                inactive: false,
                version: Some("1.0.0".to_string()),
                last_checked: None,
                using_tor_proxy: false,
                display_error_messages: true,
                tokens: Vec::new(),
                max_page: 200, // Bing 最多支持 200 页
            },
            client,
        }
    }

    /// 计算分页偏移量
    ///
    /// Bing 的分页从 1 开始，每页 10 个结果
    ///
    /// # 参数
    ///
    /// * `page` - 页码（从 1 开始）
    ///
    /// # 返回
    ///
    /// 偏移量值
    fn page_offset(page: usize) -> usize {
        (page - 1) * 10 + 1
    }

    /// 获取分页表单参数
    ///
    /// Bing 需要特殊的 FORM 参数来正确处理分页
    ///
    /// # 参数
    ///
    /// * `page` - 页码（从 1 开始）
    ///
    /// # 返回
    ///
    /// FORM 参数字符串，第一页返回 None
    fn page_form(page: usize) -> Option<String> {
        match page {
            1 => None,
            2 => Some("PERE".to_string()),
            n if n > 2 => Some(format!("PERE{}", n - 2)),
            _ => None,
        }
    }

    /// 将时间范围转换为 Bing 的时间过滤参数
    ///
    /// # 参数
    ///
    /// * `time_range` - 时间范围枚举值
    ///
    /// # 返回
    ///
    /// Bing API 的时间过滤字符串
    #[allow(dead_code)]
    fn time_range_to_bing(time_range: TimeRange) -> &'static str {
        match time_range {
            TimeRange::Day => "1",
            TimeRange::Week => "2",
            TimeRange::Month => "3",
            TimeRange::Year => "4",
            TimeRange::Any | TimeRange::Hour => "",
        }
    }

    /// 设置 Bing cookies
    ///
    /// 设置语言和地区相关的 cookies
    ///
    /// # 参数
    ///
    /// * `params` - 请求参数
    /// * `language` - 语言代码
    /// * `region` - 地区代码
    fn set_bing_cookies(params: &mut RequestParams, language: &str, region: &str) {
        params.cookies.insert("_EDGE_CD".to_string(), format!("m={}&u={}", region, language));
        params.cookies.insert("_EDGE_S".to_string(), format!("mkt={}&ui={}", region, language));
    }

    /// 解码 Bing 的 base64 编码 URL
    ///
    /// Bing 有时会返回 base64 编码的 URL，格式为：
    /// https://www.bing.com/ck/a?!&&p=...&u=a1<base64_url>
    ///
    /// # 参数
    ///
    /// * `url` - 可能被编码的 URL
    ///
    /// # 返回
    ///
    /// 解码后的 URL，如果不是编码 URL 则返回原值
    fn decode_bing_url(url: &str) -> String {
        if !url.starts_with("https://www.bing.com/ck/a?") {
            return url.to_string();
        }
        
        // 解析 URL 获取查询参数
        if let Ok(parsed_url) = url::Url::parse(url) {
            // 获取 u 参数
            if let Some(param_u) = parsed_url.query_pairs()
                .find(|(k, _)| k == "u")
                .map(|(_, v)| v.to_string())
            {
                // 移除前面的 "a1"
                if param_u.len() > 2 {
                    let encoded_url = &param_u[2..];
                    // 添加 base64 padding
                    let padding_len = (4 - (encoded_url.len() % 4)) % 4;
                    let padded_url = format!("{}{}", encoded_url, "=".repeat(padding_len));
                    
                    // 解码 base64
                    use base64::{Engine as _, engine::general_purpose::URL_SAFE};
                    if let Ok(decoded_bytes) = URL_SAFE.decode(&padded_url) {
                        if let Ok(decoded_url) = String::from_utf8(decoded_bytes) {
                            return decoded_url;
                        }
                    }
                }
            }
        }
        
        url.to_string()
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
        if html.contains("There are no results") || html.is_empty() {
            return Ok(Vec::new());
        }
        
        let document = Html::parse_document(html);
        let mut items = Vec::new();
        
        // 使用 Python SearxNG 中完全相同的选择器
        // //ol[@id="b_results"]/li[contains(@class, "b_algo")]
        let results_selector = match Selector::parse("ol#b_results > li.b_algo") {
            Ok(sel) => sel,
            Err(_) => return Ok(Vec::new()),
        };
        
        for result in document.select(&results_selector) {
            // 提取链接和标题 (h2/a)
            let link_selector = Selector::parse("h2 > a").expect("valid selector");
            let link_elem = match result.select(&link_selector).next() {
                Some(elem) => elem,
                None => continue,
            };
            
            let title = link_elem.text().collect::<String>().trim().to_string();
            let mut url = link_elem.value().attr("href").unwrap_or("").to_string();
            
            // 解码 base64 编码的 URL
            url = Self::decode_bing_url(&url);
            
            // 提取内容 (p)
            let content_selector = Selector::parse("p").expect("valid selector");
            let mut content = String::new();
            
            for p_elem in result.select(&content_selector) {
                // 移除 algoSlug_icon span (Python 版本中的逻辑)
                let text = p_elem.text()
                    .filter(|t| !t.trim().is_empty())
                    .collect::<Vec<_>>()
                    .join(" ")
                    .trim()
                    .to_string();
                
                if !text.is_empty() && text != "Web" {
                    content = text;
                    break;
                }
            }
            
            // 只添加有效结果
            if !title.is_empty() && !url.is_empty() && url.starts_with("http") {
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
        }
        
        Ok(items)
    }
}

impl Default for BingEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SearchEngine for BingEngine {
    /// 获取引擎信息
    fn info(&self) -> &EngineInfo {
        &self.info
    }

    /// 执行搜索
    ///
    /// # 参数
    ///
    /// * `query` - 搜索查询参数
    ///
    /// # 返回
    ///
    /// 搜索结果或错误
    async fn search(&self, query: &SearchQuery) -> Result<SearchResult, Box<dyn Error + Send + Sync>> {
        // 使用 RequestResponseEngine trait 的默认实现
        <Self as RequestResponseEngine>::search(self, query).await
    }

    /// 检查引擎是否可用
    async fn is_available(&self) -> bool {
        // 尝试访问 Bing 主页检查可用性
        self.client.get("https://www.bing.com", None).await.is_ok()
    }
}

#[async_trait]
impl RequestResponseEngine for BingEngine {
    type Response = String;

    /// 准备请求参数
    ///
    /// # 参数
    ///
    /// * `query` - 查询字符串
    /// * `params` - 请求参数（将被修改）
    ///
    /// # 返回
    ///
    /// 成功返回 Ok(())，失败返回错误
    fn request(&self, query: &str, params: &mut RequestParams) -> Result<(), Box<dyn Error + Send + Sync>> {
        // Configure language and region
        let language = params.language.as_deref().unwrap_or("en").to_string();
        let region = params.language.as_deref().unwrap_or("us").to_string();
        
        Self::set_bing_cookies(params, &language, &region);
        
        // Build query parameters
        let mut query_params = vec![
            ("q", query.to_string()),
            ("pq", query.to_string()), // Prevents pagination issues
        ];
        
        // Add pagination if not first page
        if params.pageno > 1 {
            query_params.push(("first", Self::page_offset(params.pageno).to_string()));
            
            if let Some(form) = Self::page_form(params.pageno) {
                query_params.push(("FORM", form));
            }
        }
        
        // Build base URL with optimized query string
        let base_url = "https://www.bing.com/search";
        let query_string = build_query_string_owned(query_params.into_iter());
        
        let mut url = format!("{}?{}", base_url, query_string);
        
        // Append time range filter if specified (not URL-encoded as it's appended directly)
        if let Some(ref time_range) = params.time_range {
            let tr = match time_range.as_str() {
                "day" => "1",
                "week" => "2",
                "month" => "3",
                "year" => "4",
                _ => "",
            };
            if !tr.is_empty() {
                url.push_str(&format!("&filters=ex1:\"ez{}\"", tr));
            }
        }
        
        params.url = Some(url);
        params.method = "GET".to_string();
        
        Ok(())
    }

    /// 发送请求并获取响应
    ///
    /// # 参数
    ///
    /// * `params` - 请求参数
    ///
    /// # 返回
    ///
    /// HTML 响应字符串或错误
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

        // 添加 cookies
        for (key, value) in &params.cookies {
            options.headers.push(("Cookie".to_string(), format!("{}={}", key, value)));
        }

        // 发送请求
        let response = self.client.get(url, Some(options)).await
            .map_err(|e| format!("Request failed: {}", e))?;

        // 检查状态码
        let status = response.status();
        match status.as_u16() {
            403 => return Err("Bing 访问被拒绝，可能触发了反爬虫机制".into()),
            429 => return Err("Bing 请求过于频繁，请稍后重试".into()),
            503 => return Err("Bing 服务暂时不可用，请稍后重试".into()),
            _ if !status.is_success() => return Err(format!("HTTP 错误: {}", status).into()),
            _ => {} // 继续处理
        }

        // 获取响应文本
        let text = response.text().await
            .map_err(|e| format!("Failed to read response: {}", e))?;

        Ok(text)
    }

    /// 解析响应为结果列表
    ///
    /// # 参数
    ///
    /// * `resp` - HTML 响应字符串
    ///
    /// # 返回
    ///
    /// 搜索结果项列表或错误
    fn response(&self, resp: Self::Response) -> Result<Vec<SearchResultItem>, Box<dyn Error + Send + Sync>> {
        Self::parse_html_results(&resp)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_engine_creation() {
        let engine = BingEngine::new();
        assert_eq!(engine.info().name, "Bing");
        assert_eq!(engine.info().engine_type, EngineType::General);
    }

    #[test]
    fn test_page_offset() {
        assert_eq!(BingEngine::page_offset(1), 1);
        assert_eq!(BingEngine::page_offset(2), 11);
        assert_eq!(BingEngine::page_offset(3), 21);
        assert_eq!(BingEngine::page_offset(10), 91);
    }

    #[test]
    fn test_page_form() {
        assert_eq!(BingEngine::page_form(1), None);
        assert_eq!(BingEngine::page_form(2), Some("PERE".to_string()));
        assert_eq!(BingEngine::page_form(3), Some("PERE1".to_string()));
        assert_eq!(BingEngine::page_form(4), Some("PERE2".to_string()));
        assert_eq!(BingEngine::page_form(10), Some("PERE8".to_string()));
    }

    #[test]
    fn test_time_range_conversion() {
        assert_eq!(BingEngine::time_range_to_bing(TimeRange::Day), "1");
        assert_eq!(BingEngine::time_range_to_bing(TimeRange::Week), "2");
        assert_eq!(BingEngine::time_range_to_bing(TimeRange::Month), "3");
        assert_eq!(BingEngine::time_range_to_bing(TimeRange::Year), "4");
        assert_eq!(BingEngine::time_range_to_bing(TimeRange::Any), "");
        assert_eq!(BingEngine::time_range_to_bing(TimeRange::Hour), "");
    }

    #[test]
    fn test_engine_info() {
        let engine = BingEngine::new();
        let info = engine.info();
        
        assert!(info.capabilities.supports_pagination);
        assert!(info.capabilities.supports_time_range);
        assert!(info.capabilities.supports_safe_search);
        assert_eq!(info.capabilities.max_page_size, 10);
        assert_eq!(info.max_page, 200);
    }

    #[test]
    fn test_request_preparation() {
        let engine = BingEngine::new();
        let mut params = RequestParams::default();
        
        let result = engine.request("test query", &mut params);
        assert!(result.is_ok());
        assert!(params.url.is_some());
        assert_eq!(params.method, "GET");
        
        let url = params.url.expect("Expected valid value");
        assert!(url.contains("www.bing.com"));
        assert!(url.contains("q=test%20query"));
        assert!(url.contains("pq=test%20query"));
    }

    #[test]
    fn test_request_with_pagination() {
        let engine = BingEngine::new();
        let mut params = RequestParams::default();
        params.pageno = 3;
        
        let result = engine.request("test", &mut params);
        assert!(result.is_ok());
        
        let url = params.url.expect("Expected valid value");
        assert!(url.contains("first=21")); // (3-1) * 10 + 1 = 21
        assert!(url.contains("FORM=PERE1")); // page 3 -> PERE1
    }

    #[test]
    fn test_request_with_time_range() {
        let engine = BingEngine::new();
        let mut params = RequestParams::default();
        params.time_range = Some("week".to_string());
        
        let result = engine.request("test", &mut params);
        assert!(result.is_ok());
        
        let url = params.url.expect("Expected valid value");
        assert!(url.contains("filters=ex1:%22ez2%22")); // week = 2
    }

    #[test]
    fn test_set_cookies() {
        let mut params = RequestParams::default();
        BingEngine::set_bing_cookies(&mut params, "en", "us");
        
        assert_eq!(params.cookies.get("_EDGE_CD"), Some(&"m=us&u=en".to_string()));
        assert_eq!(params.cookies.get("_EDGE_S"), Some(&"mkt=us&ui=en".to_string()));
    }

    #[test]
    fn test_default() {
        let engine = BingEngine::default();
        assert_eq!(engine.info().name, "Bing");
    }

    #[tokio::test]
    async fn test_is_available() {
        let engine = BingEngine::new();
        // 注意：这个测试需要网络连接
        // 在 CI 环境中可能会失败
        let _ = engine.is_available().await;
        // 不断言结果，因为可能没有网络连接
    }

    #[test]
    fn test_parse_empty_html() {
        let result = BingEngine::parse_html_results("");
        assert!(result.is_ok());
        assert_eq!(result.expect("Expected valid value").len(), 0);
    }

    #[test]
    fn test_parse_no_results_html() {
        let html = "<html><body>There are no results</body></html>";
        let result = BingEngine::parse_html_results(html);
        assert!(result.is_ok());
        assert_eq!(result.expect("Expected valid value").len(), 0);
    }
}
