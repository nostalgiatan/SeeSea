//! Brave 搜索引擎实现
//!
//! 这是一个基于 Brave Search API 的搜索引擎实现。
//! 参考了 Python SearXNG 的 Brave 引擎实现。
//!
//! ## 功能特性
//!
//! - 支持基本的网页搜索
//! - 支持分页
//! - 支持时间范围过滤
//! - 注重隐私保护
//!
//! ## API 说明
//!
//! Brave Search 是一个注重隐私的搜索引擎：
//! - q: 查询关键词
//! - offset: 分页偏移量
//! - tf: 时间过滤
//!
//! ## 安全性
//!
//! - 避免使用 unwrap()，使用 ? 操作符处理错误
//! - 所有网络请求都有超时设置
//!
//! ## 示例
//!
//! ```no_run
//! use SeeSea::search::engines::brave::BraveEngine;
//! use SeeSea::derive::{SearchEngine, SearchQuery};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let engine = BraveEngine::new();
//!     let query = SearchQuery::default();
//!     let results = engine.search(&query).await?;
//!     println!("找到 {} 个结果", results.items.len());
//!     Ok(())
//! }
//! ```

use async_trait::async_trait;
use std::collections::HashMap;
use std::error::Error;

use crate::derive::{
    EngineCapabilities, EngineInfo, EngineStatus, EngineType,
    ResultType, SearchEngine, SearchQuery, SearchResult,
    SearchResultItem, TimeRange, AboutInfo, RequestResponseEngine, RequestParams,
};
use crate::net::client::HttpClient;
use crate::net::types::{NetworkConfig, RequestOptions};

/// Brave 搜索引擎
///
/// 使用 Brave Search API 进行搜索的引擎实现
pub struct BraveEngine {
    /// 引擎信息
    info: EngineInfo,
    /// HTTP 客户端
    client: HttpClient,
}

impl BraveEngine {
    /// 创建新的 Brave 引擎实例
    ///
    /// # 示例
    ///
    /// ```
    /// use SeeSea::search::engines::brave::BraveEngine;
    ///
    /// let engine = BraveEngine::new();
    /// ```
    pub fn new() -> Self {
        Self {
            info: EngineInfo {
                name: "Brave".to_string(),
                engine_type: EngineType::General,
                description: "Brave 是一个注重隐私保护的搜索引擎".to_string(),
                status: EngineStatus::Active,
                categories: vec!["general".to_string(), "web".to_string()],
                capabilities: EngineCapabilities {
                    result_types: vec![ResultType::Web],
                    supported_params: vec![
                        "time_range".to_string(),
                    ],
                    max_page_size: 20,
                    supports_pagination: true,
                    supports_time_range: true,
                    supports_language_filter: true,
                    supports_region_filter: true,
                    supports_safe_search: true,
                    rate_limit: Some(60),
                },
                about: AboutInfo {
                    website: Some("https://search.brave.com".to_string()),
                    wikidata_id: Some("Q22906900".to_string()),
                    official_api_documentation: None,
                    use_official_api: false,
                    require_api_key: false,
                    results: "HTML".to_string(),
                },
                shortcut: Some("brave".to_string()),
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
            client: HttpClient::new(NetworkConfig::default()).unwrap_or_else(|_| {
                    panic!("Failed to create HTTP client for Brave")
                }),
        }
    }

    /// 将时间范围转换为 Brave 的时间过滤参数
    ///
    /// # 参数
    ///
    /// * `time_range` - 时间范围枚举值
    ///
    /// # 返回
    ///
    /// Brave API 的时间过滤字符串
    #[allow(dead_code)]
    fn time_range_to_brave(time_range: TimeRange) -> &'static str {
        match time_range {
            TimeRange::Day => "pd",    // past day
            TimeRange::Week => "pw",   // past week
            TimeRange::Month => "pm",  // past month
            TimeRange::Year => "py",   // past year
            _ => "",
        }
    }

    /// 解析 HTML 响应为搜索结果项列表 (对齐 Python SearxNG)
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
        
        // Python XPath: '//div[contains(@class, "snippet ")]'
        // CSS equivalent: div.snippet, div[class*="snippet "]
        let results_selector = match Selector::parse("div[class*=\"snippet \"]") {
            Ok(sel) => sel,
            Err(_) => return Ok(Vec::new()),
        };

        // 预解析所有 selectors 以避免在循环中重复解析
        let link_selector = match Selector::parse("a[class*=\"h\"]") {
            Ok(sel) => sel,
            Err(e) => return Err(format!("Failed to parse link selector: {}", e).into()),
        };
        let title_selector = match Selector::parse("a[class*=\"h\"] div[class*=\"title\"]") {
            Ok(sel) => sel,
            Err(e) => return Err(format!("Failed to parse title selector: {}", e).into()),
        };
        let content_selector = match Selector::parse("div[class*=\"snippet-description\"]") {
            Ok(sel) => sel,
            Err(e) => return Err(format!("Failed to parse content selector: {}", e).into()),
        };
        let thumbnail_selector = match Selector::parse("img[class*=\"thumb\"]") {
            Ok(sel) => sel,
            Err(e) => return Err(format!("Failed to parse thumbnail selector: {}", e).into()),
        };

        for result in document.select(&results_selector) {
            // Python XPath: './/a[contains(@class, "h")]/@href'
            // CSS: a.h or a[class*="h"]
            let url = match result.select(&link_selector).next() {
                Some(link) => {
                    match link.value().attr("href") {
                        Some(href) => {
                            // 检查是否是完整URL (Python: if not urlparse(url).netloc: continue)
                            if href.starts_with("http://") || href.starts_with("https://") {
                                href.to_string()
                            } else {
                                continue; // 部分 URL 可能是广告
                            }
                        }
                        None => continue,
                    }
                }
                None => continue,
            };
            
            // Python XPath: './/a[contains(@class, "h")]//div[contains(@class, "title")]'
            // CSS: a[class*="h"] div[class*="title"]
            let title = match result.select(&title_selector).next() {
                Some(title_elem) => title_elem.text().collect::<String>().trim().to_string(),
                None => continue,
            };
            
            // Python XPath: './/div[contains(@class, "snippet-description")]'
            // CSS: div[class*="snippet-description"]
            let content = result.select(&content_selector).next()
                .map(|c| c.text().collect::<String>().trim().to_string())
                .unwrap_or_else(|| String::new());
            
            // Python: pub_date extraction from content
            // 暂时跳过发布日期解析，因为需要更复杂的逻辑
            
            // Python XPath: './/img[contains(@class, "thumb")]/@src'
            // CSS: img[class*="thumb"]
            let thumbnail = result.select(&thumbnail_selector).next()
                .and_then(|img| img.value().attr("src"))
                .map(|src| src.to_string());
            
            items.push(SearchResultItem {
                title,
                url: url.clone(),
                content,
                display_url: Some(url),
                site_name: None,
                score: 1.0,
                result_type: ResultType::Web,
                thumbnail,
                published_date: None,
                template: None,
                metadata: HashMap::new(),
            });
        }
        
        Ok(items)
    }
}

impl Default for BraveEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SearchEngine for BraveEngine {
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
        self.client.get("https://search.brave.com", None).await.is_ok()
    }
}

#[async_trait]
impl RequestResponseEngine for BraveEngine {
    type Response = String;

    /// 准备请求参数
    fn request(&self, query: &str, params: &mut RequestParams) -> Result<(), Box<dyn Error + Send + Sync>> {
        // 使用与 searxng 相同的参数结构
        let mut query_params = vec![
            ("q", query.to_string()),
            ("source", "web".to_string()),
        ];

        // 添加偏移量（searxng 使用页码-1，而不是结果数）
        let page_offset = params.pageno - 1;
        if page_offset > 0 {
            query_params.push(("offset", page_offset.to_string()));
        }

        // 添加时间范围（使用 searxng 的时间映射）
        if let Some(ref time_range) = params.time_range {
            let tf = match time_range.as_str() {
                "day" => "pd",
                "week" => "pw",
                "month" => "pm",
                "year" => "py",
                _ => "",
            };
            if !tf.is_empty() {
                query_params.push(("tf", tf.to_string()));
            }
        }

        let query_string = query_params
            .iter()
            .map(|(k, v)| format!("{}={}", k, urlencoding::encode(v)))
            .collect::<Vec<_>>()
            .join("&");

        params.url = Some(format!("https://search.brave.com/search?{}", query_string));
        params.method = "GET".to_string();

        // 添加与 searxng 相同的 cookies
        params.cookies.insert("safesearch".to_string(), "moderate".to_string());
        params.cookies.insert("useLocation".to_string(), "0".to_string());
        params.cookies.insert("summarizer".to_string(), "0".to_string());
        params.cookies.insert("country".to_string(), "us".to_string());
        params.cookies.insert("ui_lang".to_string(), "en-us".to_string());

        // 添加与 searxng 相同的关键头部
        params.headers.insert("Sec-Fetch-Dest".to_string(), "document".to_string());
        params.headers.insert("Sec-Fetch-Mode".to_string(), "navigate".to_string());
        params.headers.insert("Sec-Fetch-Site".to_string(), "same-origin".to_string());
        params.headers.insert("Sec-Fetch-User".to_string(), "?1".to_string());
        params.headers.insert("Accept".to_string(), "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8".to_string());
        params.headers.insert("Accept-Language".to_string(), "en-US,en;q=0.5".to_string());

        Ok(())
    }

    /// 发送请求并获取响应
    async fn fetch(&self, params: &RequestParams) -> Result<Self::Response, Box<dyn Error + Send + Sync>> {
        let url = params.url.as_ref()
            .ok_or("请求 URL 未设置")?;

        // 创建请求选项
        let mut options = RequestOptions::default();
        options.timeout = std::time::Duration::from_secs(10);

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
            403 => return Err("Brave 访问被拒绝，可能触发了反爬虫机制".into()),
            429 => return Err("Brave 请求过于频繁，请稍后重试".into()),
            503 => return Err("Brave 服务暂时不可用，请稍后重试".into()),
            _ if !status.is_success() => return Err(format!("HTTP 错误: {}", status).into()),
            _ => {} // 继续处理
        }

        // 获取响应文本
        let text = response.text().await
            .map_err(|e| format!("Failed to read response: {}", e))?;
        
        Ok(text)
    }

    /// 解析响应为结果列表
    fn response(&self, resp: Self::Response) -> Result<Vec<SearchResultItem>, Box<dyn Error + Send + Sync>> {
        Self::parse_html_results(&resp)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_engine_creation() {
        let engine = BraveEngine::new();
        assert_eq!(engine.info().name, "Brave");
        assert_eq!(engine.info().engine_type, EngineType::General);
    }

    #[test]
    fn test_time_range_conversion() {
        assert_eq!(BraveEngine::time_range_to_brave(TimeRange::Day), "pd");
        assert_eq!(BraveEngine::time_range_to_brave(TimeRange::Week), "pw");
        assert_eq!(BraveEngine::time_range_to_brave(TimeRange::Month), "pm");
        assert_eq!(BraveEngine::time_range_to_brave(TimeRange::Year), "py");
        assert_eq!(BraveEngine::time_range_to_brave(TimeRange::Any), "");
    }

    #[test]
    fn test_engine_info() {
        let engine = BraveEngine::new();
        let info = engine.info();
        
        assert!(info.capabilities.supports_pagination);
        assert!(info.capabilities.supports_time_range);
        assert!(info.capabilities.supports_safe_search);
        assert_eq!(info.capabilities.max_page_size, 20);
    }

    #[test]
    fn test_request_preparation() {
        let engine = BraveEngine::new();
        let mut params = RequestParams::default();
        
        let result = engine.request("test query", &mut params);
        assert!(result.is_ok());
        assert!(params.url.is_some());
        
        let url = params.url.expect("Expected valid value");
        assert!(url.contains("search.brave.com"));
        assert!(url.contains("q=test%20query"));
    }

    #[test]
    fn test_request_with_pagination() {
        let engine = BraveEngine::new();
        let mut params = RequestParams::default();
        params.pageno = 2;
        
        let result = engine.request("test", &mut params);
        assert!(result.is_ok());
        
        let url = params.url.expect("Expected valid value");
        assert!(url.contains("offset=20")); // (2-1) * 20 = 20
    }

    #[test]
    fn test_request_with_time_range() {
        let engine = BraveEngine::new();
        let mut params = RequestParams::default();
        params.time_range = Some("week".to_string());
        
        let result = engine.request("test", &mut params);
        assert!(result.is_ok());
        
        let url = params.url.expect("Expected valid value");
        assert!(url.contains("tf=pw"));
    }

    #[test]
    fn test_default() {
        let engine = BraveEngine::default();
        assert_eq!(engine.info().name, "Brave");
    }

    #[tokio::test]
    async fn test_is_available() {
        let engine = BraveEngine::new();
        let _ = engine.is_available().await;
    }

    #[test]
    fn test_parse_empty_html() {
        let result = BraveEngine::parse_html_results("");
        assert!(result.is_ok());
        assert_eq!(result.expect("Expected valid value").len(), 0);
    }
}
