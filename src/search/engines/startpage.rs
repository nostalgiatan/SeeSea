//! Startpage 搜索引擎实现
//!
//! 这是一个基于 Startpage API 的搜索引擎实现。
//! 参考了 Python SearXNG 的 Startpage 引擎实现。
//!
//! ## 功能特性
//!
//! - 支持基本的网页搜索
//! - 支持分页
//! - 注重隐私保护（使用 Google 结果但不跟踪）
//!
//! ## API 说明
//!
//! Startpage 是一个隐私搜索引擎，使用 Google 的搜索结果：
//! - query: 查询关键词
//! - page: 页码
//! - language: 语言设置
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
//! use SeeSea::search::engines::startpage::StartpageEngine;
//! use SeeSea::derive::{SearchEngine, SearchQuery};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let engine = StartpageEngine::new();
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
    SearchResultItem, AboutInfo, RequestResponseEngine, RequestParams,
};
use crate::net::client::HttpClient;
use crate::net::types::NetworkConfig;

/// Startpage 搜索引擎
///
/// 使用 Startpage API 进行搜索的引擎实现
pub struct StartpageEngine {
    /// 引擎信息
    info: EngineInfo,
    /// HTTP 客户端
    client: HttpClient,
}

impl StartpageEngine {
    /// 创建新的 Startpage 引擎实例
    ///
    /// # 示例
    ///
    /// ```
    /// use SeeSea::search::engines::startpage::StartpageEngine;
    ///
    /// let engine = StartpageEngine::new();
    /// ```
    pub fn new() -> Self {
        Self {
            info: EngineInfo {
                name: "Startpage".to_string(),
                engine_type: EngineType::General,
                description: "Startpage 是一个注重隐私的搜索引擎，使用 Google 的搜索结果".to_string(),
                status: EngineStatus::Active,
                categories: vec!["general".to_string(), "web".to_string()],
                capabilities: EngineCapabilities {
                    result_types: vec![ResultType::Web],
                    supported_params: vec!["language".to_string()],
                    max_page_size: 10,
                    supports_pagination: true,
                    supports_time_range: false,
                    supports_language_filter: true,
                    supports_region_filter: true,
                    supports_safe_search: false,
                    rate_limit: Some(60),
                },
                about: AboutInfo {
                    website: Some("https://www.startpage.com".to_string()),
                    wikidata_id: Some("Q2333294".to_string()),
                    official_api_documentation: None,
                    use_official_api: false,
                    require_api_key: false,
                    results: "HTML".to_string(),
                },
                shortcut: Some("startpage".to_string()),
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
            client: HttpClient::new(NetworkConfig::default())
                .unwrap_or_else(|_| panic!("Failed to create HTTP client for Startpage")),
        }
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
        
        // 检查是否被重定向到 CAPTCHA 页面（基于 SearXNG 的检查）
        // SearXNG: if str(resp.url).startswith('https://www.startpage.com/sp/captcha'):
        if html.contains("www.startpage.com/sp/captcha") || html.contains("/sp/captcha") {
            return Err("检测到 Startpage CAPTCHA，请稍后重试".into());
        }
        
        let document = Html::parse_document(html);
        let mut items = Vec::new();
        
        // Startpage 的搜索结果选择器（多个后备选项）
        // 基于 SearXNG 的实现，Startpage 使用 React 和 JSON 嵌入在 HTML 中
        // 但我们也提供基于 HTML 结构的解析作为后备
        let result_selectors = vec![
            "div.w-gl__result",
            "div.result",
            "article.result",
            "div[class*=\"result\"]",
        ];
        
        let mut results_found = false;
        for selector_str in result_selectors {
            let selector = match Selector::parse(selector_str) {
                Ok(sel) => sel,
                Err(_) => continue,
            };
            
            for result in document.select(&selector) {
                results_found = true;
                
                // 提取标题和 URL - 尝试多个选择器
                let title_selectors = vec!["h2", "h3", "a.w-gl__result-title", "a[class*=\"title\"]"];
                let mut title = String::new();
                for title_sel_str in title_selectors {
                    if let Ok(title_selector) = Selector::parse(title_sel_str) {
                        if let Some(t) = result.select(&title_selector).next() {
                            title = t.text().collect::<String>().trim().to_string();
                            if !title.is_empty() {
                                break;
                            }
                        }
                    }
                }
                
                // 提取 URL
                let link_selector = Selector::parse("a").expect("Expected valid value");
                let mut url = String::new();
                for link in result.select(&link_selector) {
                    if let Some(href) = link.value().attr("href") {
                        // 确保是有效的 URL
                        if href.starts_with("http") {
                            url = href.to_string();
                            break;
                        }
                    }
                }
                
                // 提取内容片段
                let snippet_selectors = vec![
                    "p.w-gl__description",
                    "p.result-snippet",
                    "div.result-snippet",
                    "p[class*=\"description\"]",
                    "div[class*=\"description\"]",
                    "p",
                ];
                let mut content = String::new();
                for snippet_sel_str in snippet_selectors {
                    if let Ok(snippet_selector) = Selector::parse(snippet_sel_str) {
                        if let Some(s) = result.select(&snippet_selector).next() {
                            content = s.text().collect::<String>().trim().to_string();
                            if !content.is_empty() {
                                break;
                            }
                        }
                    }
                }
                
                // 过滤有效结果 - 必须有标题和有效的 URL
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
            
            if results_found && !items.is_empty() {
                break;
            }
        }
        
        Ok(items)
    }
}

impl Default for StartpageEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SearchEngine for StartpageEngine {
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
        match self.client.get("https://www.startpage.com", None).await {
            Ok(resp) => resp.status().is_success(),
            Err(_) => false,
        }
    }
}

#[async_trait]
impl RequestResponseEngine for StartpageEngine {
    type Response = String;

    /// 准备请求参数
    fn request(&self, query: &str, params: &mut RequestParams) -> Result<(), Box<dyn Error + Send + Sync>> {
        // 构建查询参数
        let mut query_params = vec![
            ("query", query.to_string()),
            ("page", params.pageno.to_string()),
        ];
        
        // 添加语言
        if let Some(ref lang) = params.language {
            query_params.push(("language", lang.clone()));
        }
        
        // 构建 URL - pre-allocate with estimated size
        let estimated_size: usize = query_params.iter()
            .map(|(k, v)| k.len() + v.len() + 2)
            .sum();
        let mut query_string = String::with_capacity(estimated_size);
        
        for (i, (k, v)) in query_params.iter().enumerate() {
            if i > 0 {
                query_string.push('&');
            }
            query_string.push_str(k);
            query_string.push('=');
            query_string.push_str(&urlencoding::encode(v));
        }
        
        params.url = Some(format!("https://www.startpage.com/sp/search?{}", query_string));
        params.method = "GET".to_string();
        
        Ok(())
    }

    /// 发送请求并获取响应
    async fn fetch(&self, params: &RequestParams) -> Result<Self::Response, Box<dyn Error + Send + Sync>> {
        let url = params.url.as_ref()
            .ok_or("请求 URL 未设置")?;
        
        // 创建请求选项
        let mut options = crate::net::types::RequestOptions::default();
        options.timeout = std::time::Duration::from_secs(10);
        
        // 添加自定义头
        for (key, value) in &params.headers {
            options.headers.push((key.clone(), value.clone()));
        }
        
        // 发送请求
        let response = self.client.get(url, Some(options)).await
            .map_err(|e| format!("Request failed: {}", e))?;
        
        // 检查响应的最终 URL 是否被重定向到 CAPTCHA 页面
        // SearXNG: if str(resp.url).startswith('https://www.startpage.com/sp/captcha'):
        let final_url = response.url().to_string();
        if final_url.contains("/sp/captcha") {
            return Err("检测到 Startpage CAPTCHA，请稍后重试".into());
        }
        
        // 检查状态码
        if !response.status().is_success() {
            return Err(format!("HTTP 错误: {}", response.status()).into());
        }
        
        // 获取响应文本
        let text = response.text().await
            .map_err(|e| format!("Failed to read response: {}", e))?;
        
        // 再次检查响应内容中是否包含 CAPTCHA 标记
        // 使用更严格的检查，避免误报
        if text.contains("\"captchaUrl\"") || text.contains("sp/captcha") && text.contains("challenge") {
            return Err("检测到 Startpage CAPTCHA，请稍后重试".into());
        }
        
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
        let engine = StartpageEngine::new();
        assert_eq!(engine.info().name, "Startpage");
        assert_eq!(engine.info().engine_type, EngineType::General);
    }

    #[test]
    fn test_engine_info() {
        let engine = StartpageEngine::new();
        let info = engine.info();
        
        assert!(info.capabilities.supports_pagination);
        assert!(!info.capabilities.supports_time_range);
        assert!(!info.capabilities.supports_safe_search);
    }

    #[test]
    fn test_request_preparation() {
        let engine = StartpageEngine::new();
        let mut params = RequestParams::default();
        
        let result = engine.request("test query", &mut params);
        assert!(result.is_ok());
        assert!(params.url.is_some());
        
        let url = params.url.expect("Expected valid value");
        assert!(url.contains("startpage.com"));
        assert!(url.contains("query=test%20query"));
    }

    #[test]
    fn test_request_with_pagination() {
        let engine = StartpageEngine::new();
        let mut params = RequestParams::default();
        params.pageno = 2;
        
        let result = engine.request("test", &mut params);
        assert!(result.is_ok());
        
        let url = params.url.expect("Expected valid value");
        assert!(url.contains("page=2"));
    }

    #[test]
    fn test_default() {
        let engine = StartpageEngine::default();
        assert_eq!(engine.info().name, "Startpage");
    }

    #[tokio::test]
    async fn test_is_available() {
        let engine = StartpageEngine::new();
        let _ = engine.is_available().await;
    }

    #[test]
    fn test_parse_empty_html() {
        let result = StartpageEngine::parse_html_results("");
        assert!(result.is_ok());
        assert_eq!(result.expect("Expected valid value").len(), 0);
    }

    #[test]
    fn test_parse_captcha_detection() {
        // 测试 CAPTCHA 检测
        let html_with_captcha = "<html><body><div>Redirected to www.startpage.com/sp/captcha</div></body></html>";
        let result = StartpageEngine::parse_html_results(html_with_captcha);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("CAPTCHA"));
    }

    #[test]
    fn test_parse_html_with_results() {
        // 测试包含搜索结果的 HTML
        let html = "<html><body><div class=\"w-gl__result\"><h2>Example Title</h2><a href=\"https://example.com\">Link</a><p class=\"w-gl__description\">This is example content.</p></div></body></html>";
        let result = StartpageEngine::parse_html_results(html);
        assert!(result.is_ok());
        let items = result.unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].title, "Example Title");
        assert_eq!(items[0].url, "https://example.com");
        assert!(items[0].content.contains("example content"));
    }

    #[test]
    fn test_parse_html_multiple_selectors() {
        // 测试使用不同选择器的 HTML
        let html = "<html><body><div class=\"result\"><h3>Test Title</h3><a href=\"https://test.com\">Link</a><p class=\"result-snippet\">Test content here.</p></div></body></html>";
        let result = StartpageEngine::parse_html_results(html);
        assert!(result.is_ok());
        let items = result.unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].title, "Test Title");
        assert_eq!(items[0].url, "https://test.com");
    }

    #[test]
    fn test_parse_html_invalid_url() {
        // 测试无效 URL 被过滤
        let html = "<html><body><div class=\"result\"><h2>Title</h2><a href=\"/relative/path\">Relative Link</a><p>Content</p></div></body></html>";
        let result = StartpageEngine::parse_html_results(html);
        assert!(result.is_ok());
        let items = result.unwrap();
        // 相对路径应该被过滤
        assert_eq!(items.len(), 0);
    }
}
