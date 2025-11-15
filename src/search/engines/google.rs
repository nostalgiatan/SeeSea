//! Google 搜索引擎实现
//!
//! 这是一个基于 Google HTML API 的搜索引擎实现。
//! 参考了 Python SearXNG 的 Google 引擎实现。
//!
//! ## 功能特性
//!
//! - 支持基本的网页搜索
//! - 支持分页（最多50页）
//! - 支持时间范围过滤
//! - 支持安全搜索级别
//! - 支持语言和地区过滤
//!
//! ## API 说明
//!
//! Google 使用标准的 URL 参数进行搜索：
//! - q: 查询关键词
//! - start: 分页偏移量
//! - num: 每页结果数（最多20）
//! - lr: 语言限制
//! - cr: 国家/地区限制
//! - safe: 安全搜索级别
//! - tbs: 时间范围过滤
//!
//! ## 安全性
//!
//! - 避免使用 unwrap()，使用 ? 操作符处理错误
//! - 所有网络请求都有超时设置
//! - 处理 Google 的 CAPTCHA 检测
//! - 设置适当的 cookies 以避免同意页面
//!
//! ## 示例
//!
//! ```no_run
//! use SeeSea::search::engines::google::GoogleEngine;
//! use SeeSea::derive::{SearchEngine, SearchQuery};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let engine = GoogleEngine::new();
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

/// Google 搜索引擎
///
/// 使用 Google HTML API 进行搜索的引擎实现
pub struct GoogleEngine {
    /// 引擎信息
    info: EngineInfo,
    /// HTTP 客户端
    client: reqwest::Client,
}

impl GoogleEngine {
    /// 创建新的 Google 引擎实例
    ///
    /// # 示例
    ///
    /// ```
    /// use SeeSea::search::engines::google::GoogleEngine;
    ///
    /// let engine = GoogleEngine::new();
    /// ```
    pub fn new() -> Self {
        Self {
            info: EngineInfo {
                name: "Google".to_string(),
                engine_type: EngineType::General,
                description: "Google 是世界上最流行的搜索引擎".to_string(),
                status: EngineStatus::Active,
                categories: vec!["general".to_string(), "web".to_string()],
                capabilities: EngineCapabilities {
                    result_types: vec![ResultType::Web],
                    supported_params: vec![
                        "language".to_string(),
                        "region".to_string(),
                        "time_range".to_string(),
                        "num".to_string(),
                    ],
                    max_page_size: 20, // Google 最多返回 20 个结果
                    supports_pagination: true,
                    supports_time_range: true,
                    supports_language_filter: true,
                    supports_region_filter: true,
                    supports_safe_search: true,
                    rate_limit: Some(100), // 每分钟 100 次请求
                },
                about: AboutInfo {
                    website: Some("https://www.google.com".to_string()),
                    wikidata_id: Some("Q9366".to_string()),
                    official_api_documentation: Some("https://developers.google.com/custom-search/".to_string()),
                    use_official_api: false,
                    require_api_key: false,
                    results: "HTML".to_string(),
                },
                shortcut: Some("google".to_string()),
                timeout: Some(10),
                disabled: false,
                inactive: false,
                version: Some("1.0.0".to_string()),
                last_checked: None,
                using_tor_proxy: false,
                display_error_messages: true,
                tokens: Vec::new(),
                max_page: 50, // Google 最多支持 50 页
            },
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(10))
                .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
                .build()
                .expect("无法创建 HTTP 客户端"),
        }
    }

    /// 将时间范围转换为 Google 的时间过滤参数
    ///
    /// # 参数
    ///
    /// * `time_range` - 时间范围枚举值
    ///
    /// # 返回
    ///
    /// Google API 的时间过滤字符串
    fn time_range_to_google(time_range: TimeRange) -> &'static str {
        match time_range {
            TimeRange::Hour => "h",
            TimeRange::Day => "d",
            TimeRange::Week => "w",
            TimeRange::Month => "m",
            TimeRange::Year => "y",
            TimeRange::Any => "",
        }
    }

    /// 将安全搜索级别转换为 Google 的安全搜索参数
    ///
    /// # 参数
    ///
    /// * `level` - 安全搜索级别（0: 关闭, 1: 中等, 2: 严格）
    ///
    /// # 返回
    ///
    /// Google API 的安全搜索字符串
    fn safesearch_to_google(level: i32) -> &'static str {
        match level {
            0 => "off",
            1 => "medium",
            2 => "high",
            _ => "medium",
        }
    }

    /// 获取 Google 子域名
    ///
    /// 根据语言/地区返回合适的 Google 子域名
    ///
    /// # 参数
    ///
    /// * `region` - 地区代码
    ///
    /// # 返回
    ///
    /// Google 子域名字符串
    fn get_subdomain(region: Option<&str>) -> &'static str {
        // 简化版本：默认使用 www.google.com
        // 实际实现中应该根据地区代码返回不同的子域名
        // 例如：google.co.uk, google.de, google.fr 等
        match region {
            Some("uk") => "www.google.co.uk",
            Some("de") => "www.google.de",
            Some("fr") => "www.google.fr",
            Some("jp") => "www.google.co.jp",
            Some("cn") => "www.google.cn",
            _ => "www.google.com",
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
        
        // 检查是否有 CAPTCHA
        if html.contains("sorry.google.com") || html.contains("/sorry") {
            return Err("检测到 Google CAPTCHA，请稍后重试".into());
        }
        
        // 检查是否有结果
        if html.contains("did not match any documents") || html.is_empty() {
            return Ok(Vec::new());
        }
        
        let document = Html::parse_document(html);
        let mut items = Vec::new();
        
        // Google 的搜索结果通常在 div.g 或 div[data-hveid] 元素中
        // 尝试多个选择器以适应不同的 Google 页面版本
        let selectors = vec![
            "div.g",
            "div[data-hveid]",
            "div.Gx5Zad",
        ];
        
        let mut result_selector = None;
        for sel_str in selectors {
            if let Ok(sel) = Selector::parse(sel_str) {
                if document.select(&sel).count() > 0 {
                    result_selector = Some(sel);
                    break;
                }
            }
        }
        
        let result_sel = match result_selector {
            Some(sel) => sel,
            None => {
                // 如果找不到标准的结果容器，尝试基本的链接提取
                let link_selector = Selector::parse("a").unwrap();
                let h3_selector = Selector::parse("h3").unwrap();
                
                for element in document.select(&link_selector) {
                    if let Some(href) = element.value().attr("href") {
                        // 过滤掉内部链接和无效链接
                        if href.starts_with("http") && !href.contains("google.com") {
                            let title = element.select(&h3_selector).next()
                                .map(|h3| h3.text().collect::<String>())
                                .unwrap_or_else(|| element.text().collect::<String>());
                            
                            if !title.is_empty() && title.len() > 3 {
                                items.push(SearchResultItem {
                                    title: title.trim().to_string(),
                                    url: href.to_string(),
                                    content: String::new(),
                                    display_url: Some(href.to_string()),
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
                return Ok(items);
            }
        };
        
        // 提取每个搜索结果
        for result in document.select(&result_sel) {
            // 提取标题和 URL
            let title_selector = Selector::parse("h3").unwrap();
            let link_selector = Selector::parse("a").unwrap();
            let snippet_selectors = vec![
                Selector::parse("div[data-sncf]").ok(),
                Selector::parse("div.VwiC3b").ok(),
                Selector::parse("span.aCOpRe").ok(),
                Selector::parse("div.s").ok(),
            ];
            
            let title = result.select(&title_selector).next()
                .map(|t| t.text().collect::<String>().trim().to_string())
                .unwrap_or_default();
            
            let url = result.select(&link_selector).next()
                .and_then(|a| a.value().attr("href"))
                .unwrap_or_default();
            
            // 提取摘要
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
        
        Ok(items)
    }

    /// 检测是否遇到 Google CAPTCHA
    ///
    /// # 参数
    ///
    /// * `url` - 响应的 URL
    ///
    /// # 返回
    ///
    /// 如果检测到 CAPTCHA 返回 true
    fn detect_google_sorry(url: &str) -> bool {
        url.contains("sorry.google.com") || url.contains("/sorry")
    }
}

impl Default for GoogleEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SearchEngine for GoogleEngine {
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
        // 尝试访问 Google 主页检查可用性
        match self.client.get("https://www.google.com").send().await {
            Ok(resp) => resp.status().is_success() && !Self::detect_google_sorry(&resp.url().to_string()),
            Err(_) => false,
        }
    }
}

#[async_trait]
impl RequestResponseEngine for GoogleEngine {
    type Response = (String, String); // (HTML, URL)

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
        // 获取子域名
        let subdomain = Self::get_subdomain(params.language.as_deref());
        
        // 计算分页偏移量
        let start = (params.pageno - 1) * 10;
        
        // 构建查询参数
        let mut query_params = vec![
            ("q", query.to_string()),
            ("start", start.to_string()),
            ("ie", "utf8".to_string()),
            ("oe", "utf8".to_string()),
            ("filter", "0".to_string()),
        ];
        
        // 添加语言参数
        if let Some(ref lang) = params.language {
            if !lang.is_empty() && lang != "all" {
                query_params.push(("lr", format!("lang_{}", lang)));
            }
        }
        
        // 添加时间范围
        if let Some(ref time_range) = params.time_range {
            let tr = match time_range.as_str() {
                "hour" => "h",
                "day" => "d",
                "week" => "w",
                "month" => "m",
                "year" => "y",
                _ => "",
            };
            if !tr.is_empty() {
                query_params.push(("tbs", format!("qdr:{}", tr)));
            }
        }
        
        // 添加安全搜索
        if params.safesearch > 0 {
            query_params.push(("safe", Self::safesearch_to_google(params.safesearch).to_string()));
        }
        
        // 构建完整 URL
        let query_string = query_params
            .iter()
            .map(|(k, v)| format!("{}={}", k, urlencoding::encode(v)))
            .collect::<Vec<_>>()
            .join("&");
        
        let url = format!("https://{}/search?{}", subdomain, query_string);
        params.url = Some(url);
        params.method = "GET".to_string();
        
        // 设置请求头
        params.headers.insert("Accept".to_string(), "*/*".to_string());
        
        // 设置 cookies 以避免同意页面
        params.cookies.insert("CONSENT".to_string(), "YES+".to_string());
        
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
    /// (HTML 响应字符串, URL) 或错误
    async fn fetch(&self, params: &RequestParams) -> Result<Self::Response, Box<dyn Error + Send + Sync>> {
        let url = params.url.as_ref()
            .ok_or("请求 URL 未设置")?;
        
        let mut request = self.client.get(url);
        
        // 添加自定义头
        for (key, value) in &params.headers {
            request = request.header(key, value);
        }
        
        // 添加 cookies
        for (key, value) in &params.cookies {
            request = request.header("Cookie", format!("{}={}", key, value));
        }
        
        // 发送请求
        let response = request.send().await?;
        
        // 获取最终 URL（可能有重定向）
        let final_url = response.url().to_string();
        
        // 检查状态码
        if !response.status().is_success() {
            return Err(format!("HTTP 错误: {}", response.status()).into());
        }
        
        // 获取响应文本
        let text = response.text().await?;
        
        Ok((text, final_url))
    }

    /// 解析响应为结果列表
    ///
    /// # 参数
    ///
    /// * `resp` - (HTML 响应字符串, URL)
    ///
    /// # 返回
    ///
    /// 搜索结果项列表或错误
    fn response(&self, resp: Self::Response) -> Result<Vec<SearchResultItem>, Box<dyn Error + Send + Sync>> {
        let (html, url) = resp;
        
        // 检查是否遇到 CAPTCHA
        if Self::detect_google_sorry(&url) {
            return Err("检测到 Google CAPTCHA，请稍后重试".into());
        }
        
        Self::parse_html_results(&html)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_engine_creation() {
        let engine = GoogleEngine::new();
        assert_eq!(engine.info().name, "Google");
        assert_eq!(engine.info().engine_type, EngineType::General);
    }

    #[test]
    fn test_time_range_conversion() {
        assert_eq!(GoogleEngine::time_range_to_google(TimeRange::Hour), "h");
        assert_eq!(GoogleEngine::time_range_to_google(TimeRange::Day), "d");
        assert_eq!(GoogleEngine::time_range_to_google(TimeRange::Week), "w");
        assert_eq!(GoogleEngine::time_range_to_google(TimeRange::Month), "m");
        assert_eq!(GoogleEngine::time_range_to_google(TimeRange::Year), "y");
        assert_eq!(GoogleEngine::time_range_to_google(TimeRange::Any), "");
    }

    #[test]
    fn test_safesearch_conversion() {
        assert_eq!(GoogleEngine::safesearch_to_google(0), "off");
        assert_eq!(GoogleEngine::safesearch_to_google(1), "medium");
        assert_eq!(GoogleEngine::safesearch_to_google(2), "high");
        assert_eq!(GoogleEngine::safesearch_to_google(99), "medium");
    }

    #[test]
    fn test_subdomain() {
        assert_eq!(GoogleEngine::get_subdomain(None), "www.google.com");
        assert_eq!(GoogleEngine::get_subdomain(Some("uk")), "www.google.co.uk");
        assert_eq!(GoogleEngine::get_subdomain(Some("de")), "www.google.de");
        assert_eq!(GoogleEngine::get_subdomain(Some("fr")), "www.google.fr");
        assert_eq!(GoogleEngine::get_subdomain(Some("unknown")), "www.google.com");
    }

    #[test]
    fn test_detect_sorry() {
        assert!(GoogleEngine::detect_google_sorry("https://sorry.google.com/"));
        assert!(GoogleEngine::detect_google_sorry("https://www.google.com/sorry"));
        assert!(!GoogleEngine::detect_google_sorry("https://www.google.com/search"));
    }

    #[test]
    fn test_engine_info() {
        let engine = GoogleEngine::new();
        let info = engine.info();
        
        assert!(info.capabilities.supports_pagination);
        assert!(info.capabilities.supports_time_range);
        assert!(info.capabilities.supports_safe_search);
        assert_eq!(info.capabilities.max_page_size, 20);
        assert_eq!(info.max_page, 50);
    }

    #[test]
    fn test_request_preparation() {
        let engine = GoogleEngine::new();
        let mut params = RequestParams::default();
        
        let result = engine.request("test query", &mut params);
        assert!(result.is_ok());
        assert!(params.url.is_some());
        assert_eq!(params.method, "GET");
        
        let url = params.url.unwrap();
        assert!(url.contains("www.google.com"));
        assert!(url.contains("q=test%20query"));
    }

    #[test]
    fn test_request_with_language() {
        let engine = GoogleEngine::new();
        let mut params = RequestParams::default();
        params.language = Some("en".to_string());
        
        let result = engine.request("test", &mut params);
        assert!(result.is_ok());
        
        let url = params.url.unwrap();
        assert!(url.contains("lr=lang_en"));
    }

    #[test]
    fn test_request_with_pagination() {
        let engine = GoogleEngine::new();
        let mut params = RequestParams::default();
        params.pageno = 3;
        
        let result = engine.request("test", &mut params);
        assert!(result.is_ok());
        
        let url = params.url.unwrap();
        assert!(url.contains("start=20")); // (3-1) * 10 = 20
    }

    #[test]
    fn test_default() {
        let engine = GoogleEngine::default();
        assert_eq!(engine.info().name, "Google");
    }

    #[tokio::test]
    async fn test_is_available() {
        let engine = GoogleEngine::new();
        // 注意：这个测试需要网络连接
        // 在 CI 环境中可能会失败
        let _ = engine.is_available().await;
        // 不断言结果，因为可能没有网络连接或遇到 CAPTCHA
    }

    #[test]
    fn test_parse_empty_html() {
        let result = GoogleEngine::parse_html_results("");
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 0);
    }

    #[test]
    fn test_parse_no_results_html() {
        let html = "<html><body>did not match any documents</body></html>";
        let result = GoogleEngine::parse_html_results(html);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 0);
    }

    #[test]
    fn test_parse_captcha_html() {
        let html = "<html><body>sorry.google.com</body></html>";
        let result = GoogleEngine::parse_html_results(html);
        assert!(result.is_err());
    }
}
