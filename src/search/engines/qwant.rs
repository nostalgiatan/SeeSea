//! Qwant 搜索引擎实现
//!
//! 这是一个基于 Qwant API 的搜索引擎实现。
//! 参考了 Python SearXNG 的 Qwant 引擎实现。
//!
//! ## 功能特性
//!
//! - 支持基本的网页搜索
//! - 支持分页（最多5页）
//! - 使用 JSON API
//! - 注重隐私保护（欧洲搜索引擎）
//!
//! ## API 说明
//!
//! Qwant 使用 JSON API：
//! - q: 查询关键词
//! - offset: 分页偏移量
//! - locale: 语言设置
//!
//! ## 安全性
//!
//! - 避免使用 unwrap()，使用 ? 操作符处理错误
//! - 所有网络请求都有超时设置
//! - 处理 CAPTCHA 和速率限制
//!
//! ## 示例
//!
//! ```no_run
//! use SeeSea::search::engines::qwant::QwantEngine;
//! use SeeSea::derive::{SearchEngine, SearchQuery};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let engine = QwantEngine::new();
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

/// Qwant 搜索引擎
///
/// 使用 Qwant JSON API 进行搜索的引擎实现
pub struct QwantEngine {
    /// 引擎信息
    info: EngineInfo,
    /// HTTP 客户端
    client: reqwest::Client,
}

impl QwantEngine {
    /// 创建新的 Qwant 引擎实例
    ///
    /// # 示例
    ///
    /// ```
    /// use SeeSea::search::engines::qwant::QwantEngine;
    ///
    /// let engine = QwantEngine::new();
    /// ```
    pub fn new() -> Self {
        Self {
            info: EngineInfo {
                name: "Qwant".to_string(),
                engine_type: EngineType::General,
                description: "Qwant 是一个注重隐私保护的欧洲搜索引擎".to_string(),
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
                    supports_safe_search: true,
                    rate_limit: Some(60),
                },
                about: AboutInfo {
                    website: Some("https://www.qwant.com".to_string()),
                    wikidata_id: Some("Q14657870".to_string()),
                    official_api_documentation: None,
                    use_official_api: true,
                    require_api_key: false,
                    results: "JSON".to_string(),
                },
                shortcut: Some("qwant".to_string()),
                timeout: Some(10),
                disabled: false,
                inactive: false,
                version: Some("1.0.0".to_string()),
                last_checked: None,
                using_tor_proxy: false,
                display_error_messages: true,
                tokens: Vec::new(),
                max_page: 5, // Qwant 最多支持 5 页
            },
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(10))
                .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
                .redirect(reqwest::redirect::Policy::limited(3)) // 允许重定向
                .build()
                .unwrap_or_else(|_| reqwest::Client::new()),
        }
    }

    /// 解析 JSON 响应为搜索结果项列表
    ///
    /// # 参数
    ///
    /// * `json_str` - JSON 响应字符串
    ///
    /// # 返回
    ///
    /// 解析出的搜索结果项列表
    ///
    /// # 错误
    ///
    /// 如果 JSON 解析失败返回错误
    #[allow(dead_code)]
    fn parse_json_results(json_str: &str) -> Result<Vec<SearchResultItem>, Box<dyn Error + Send + Sync>> {
        use serde_json::Value;
        
        // 检查是否有有效的 JSON 数据
        if json_str.is_empty() {
            return Ok(Vec::new());
        }
        
        let json: Value = serde_json::from_str(json_str)?;
        let mut items = Vec::with_capacity(10);  // Pre-allocate for typical result count
        
        // Qwant JSON API 通常返回的结构
        if let Some(data) = json.get("data") {
            if let Some(result) = data.get("result") {
                if let Some(items_array) = result.get("items").and_then(|i| i.as_array()) {
                    for item in items_array {
                        let title = item.get("title")
                            .and_then(|t| t.as_str())
                            .unwrap_or("")
                            .to_string();
                        
                        let url = item.get("url")
                            .and_then(|u| u.as_str())
                            .unwrap_or("")
                            .to_string();
                        
                        let content = item.get("desc")
                            .or_else(|| item.get("description"))
                            .and_then(|c| c.as_str())
                            .unwrap_or("")
                            .to_string();
                        
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
                }
            }
        }
        
        Ok(items)
    }

    /// 解析 HTML 响应为搜索结果项列表 (基于 searxng web-lite 实现)
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

        if html.is_empty() {
            return Ok(Vec::new());
        }

        let document = Html::parse_document(html);
        let mut items = Vec::with_capacity(10);  // Pre-allocate for typical result count

        // Python SearXNG uses: '//section/article'
        // This matches Qwant Lite's structure
        let article_selector = Selector::parse("section article").expect("valid selector");
        
        for item in document.select(&article_selector) {
            // Python: if eval_xpath(item, "./span[contains(@class, 'tooltip')]"):
            // Skip advertising (items with tooltip class)
            let tooltip_selector = Selector::parse("span[class*=\"tooltip\"]").expect("valid selector");
            if item.select(&tooltip_selector).next().is_some() {
                continue;
            }

            // Python: 'url': extract_text(eval_xpath(item, "./span[contains(@class, 'url partner')]"))
            let url_selector = Selector::parse("span[class*=\"url\"][class*=\"partner\"]").expect("valid selector");
            let url = item.select(&url_selector).next()
                .map(|span| span.text().collect::<String>().trim().to_string())
                .unwrap_or_default();

            // Python: 'title': extract_text(eval_xpath(item, './h2/a'))
            let title_selector = Selector::parse("h2 a").expect("valid selector");
            let title = item.select(&title_selector).next()
                .map(|a| a.text().collect::<String>().trim().to_string())
                .unwrap_or_default();

            // Python: 'content': extract_text(eval_xpath(item, './p'))
            let content_selector = Selector::parse("p").expect("valid selector");
            let content = item.select(&content_selector).next()
                .map(|p| p.text().collect::<String>().trim().to_string())
                .unwrap_or_default();

            // Only add if we have both title and URL
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

impl Default for QwantEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SearchEngine for QwantEngine {
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
        match self.client.get("https://www.qwant.com").send().await {
            Ok(resp) => resp.status().is_success(),
            Err(_) => false,
        }
    }
}

#[async_trait]
impl RequestResponseEngine for QwantEngine {
    type Response = String;

    /// 准备请求参数
    fn request(&self, query: &str, params: &mut RequestParams) -> Result<(), Box<dyn Error + Send + Sync>> {
        // Python SearXNG uses web-lite: https://lite.qwant.com/
        let locale = params.language.as_deref().unwrap_or("en_US");

        // Python SearXNG web-lite query params:
        // - q: query
        // - locale: locale (e.g., en_us)
        // - l: language part (e.g., en)
        // - s: safesearch (0, 1, 2)
        // - p: page number
        let lang_part = locale.split('_').next().unwrap_or("en");
        
        let query_params = vec![
            ("q", query.to_string()),
            ("locale", locale.to_lowercase()),
            ("l", lang_part.to_string()),
            ("s", params.safesearch.to_string()),
            ("p", params.pageno.to_string()),
        ];

        // 构建查询字符串
        let query_string = query_params
            .iter()
            .map(|(k, v)| format!("{}={}", k, urlencoding::encode(v)))
            .collect::<Vec<_>>()
            .join("&");

        // Use Qwant Lite like Python SearXNG
        params.url = Some(format!("https://lite.qwant.com/?{}", query_string));
        params.method = "GET".to_string();

        // 设置必要的头部 - Python SearXNG sets Accept-Language
        params.headers.insert("Accept-Language".to_string(), format!("{},en;q=0.9", locale.replace('_', "-")));
        params.headers.insert("Accept".to_string(), "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8".to_string());

        Ok(())
    }

    /// 发送请求并获取响应
    async fn fetch(&self, params: &RequestParams) -> Result<Self::Response, Box<dyn Error + Send + Sync>> {
        let url = params.url.as_ref()
            .ok_or("请求 URL 未设置")?;
        
        let mut request = self.client.get(url);
        
        // 添加自定义头
        for (key, value) in &params.headers {
            request = request.header(key, value);
        }
        
        // 发送请求
        let response = request.send().await?;
        
        // 检查状态码
        let status = response.status();
        if status.as_u16() == 429 {
            return Err("Qwant 速率限制，请稍后重试".into());
        }
        if status.as_u16() == 403 {
            return Err("Qwant 访问被拒绝，可能需要 CAPTCHA".into());
        }
        if !status.is_success() {
            return Err(format!("HTTP 错误: {}", status).into());
        }
        
        // 获取响应文本
        let text = response.text().await?;
        
        Ok(text)
    }

    /// 解析响应为结果列表
    fn response(&self, resp: Self::Response) -> Result<Vec<SearchResultItem>, Box<dyn Error + Send + Sync>> {
        // 由于我们现在使用 web-lite，需要解析 HTML
        Self::parse_html_results(&resp)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_engine_creation() {
        let engine = QwantEngine::new();
        assert_eq!(engine.info().name, "Qwant");
        assert_eq!(engine.info().engine_type, EngineType::General);
    }

    #[test]
    fn test_engine_info() {
        let engine = QwantEngine::new();
        let info = engine.info();
        
        assert!(info.capabilities.supports_pagination);
        assert!(!info.capabilities.supports_time_range);
        assert!(info.capabilities.supports_safe_search);
        assert_eq!(info.max_page, 5);
    }

    #[test]
    fn test_request_preparation() {
        let engine = QwantEngine::new();
        let mut params = RequestParams::default();
        
        let result = engine.request("test query", &mut params);
        assert!(result.is_ok());
        assert!(params.url.is_some());
        
        let url = params.url.expect("Expected valid value");
        assert!(url.contains("lite.qwant.com"));
        assert!(url.contains("q=test%20query"));
    }

    #[test]
    fn test_request_with_pagination() {
        let engine = QwantEngine::new();
        let mut params = RequestParams::default();
        params.pageno = 3;
        
        let result = engine.request("test", &mut params);
        assert!(result.is_ok());
        
        let url = params.url.expect("Expected valid value");
        assert!(url.contains("p=3"));
    }

    #[test]
    fn test_default() {
        let engine = QwantEngine::default();
        assert_eq!(engine.info().name, "Qwant");
    }

    #[tokio::test]
    async fn test_is_available() {
        let engine = QwantEngine::new();
        let _ = engine.is_available().await;
    }

    #[test]
    fn test_parse_empty_json() {
        let result = QwantEngine::parse_json_results("");
        assert!(result.is_ok());
        assert_eq!(result.expect("Expected valid value").len(), 0);
    }
}
