//! Mojeek 搜索引擎实现
//!
//! 这是一个基于 Mojeek API 的搜索引擎实现。
//! 参考了 Python SearXNG 的 Mojeek 引擎实现。
//!
//! ## 功能特性
//!
//! - 支持基本的网页搜索
//! - 支持分页（最多10页）
//! - 支持时间范围过滤
//! - 支持安全搜索
//! - 独立索引（不依赖其他搜索引擎）
//!
//! ## API 说明
//!
//! Mojeek 是一个拥有独立索引的搜索引擎：
//! - q: 查询关键词
//! - s: 分页偏移量
//! - since: 时间范围
//!
//! ## 安全性
//!
//! - 避免使用 unwrap()，使用 ? 操作符处理错误
//! - 所有网络请求都有超时设置
//!
//! ## 示例
//!
//! ```no_run
//! use SeeSea::search::engines::mojeek::MojeekEngine;
//! use SeeSea::derive::{SearchEngine, SearchQuery};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let engine = MojeekEngine::new();
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
    SearchResultItem, TimeRange, AboutInfo, RequestResponseEngine, RequestParams,
};
use crate::net::client::HttpClient;
use crate::net::types::{NetworkConfig, RequestOptions};
use super::utils::build_query_string_owned;

/// Mojeek 搜索引擎
///
/// 使用 Mojeek API 进行搜索的引擎实现
pub struct MojeekEngine {
    /// 引擎信息
    info: EngineInfo,
    /// HTTP 客户端
    client: Arc<HttpClient>,
}

impl MojeekEngine {
    /// 创建新的 Mojeek 引擎实例
    ///
    /// # 示例
    ///
    /// ```
    /// use SeeSea::search::engines::mojeek::MojeekEngine;
    ///
    /// let engine = MojeekEngine::new();
    /// ```
    pub fn new() -> Self {
        let client = HttpClient::new(NetworkConfig::default())
            .unwrap_or_else(|_| panic!("Failed to create HTTP client"));
        Self::with_client(Arc::new(client))
    }

    pub fn with_client(client: Arc<HttpClient>) -> Self {
        Self {
            info: EngineInfo {
                name: "Mojeek".to_string(),
                engine_type: EngineType::General,
                description: "Mojeek 是一个拥有独立索引的搜索引擎".to_string(),
                status: EngineStatus::Active,
                categories: vec!["general".to_string(), "web".to_string()],
                capabilities: EngineCapabilities {
                    result_types: vec![ResultType::Web],
                    supported_params: vec![
                        "time_range".to_string(),
                        "language".to_string(),
                    ],
                    max_page_size: 10,
                    supports_pagination: true,
                    supports_time_range: true,
                    supports_language_filter: true,
                    supports_region_filter: true,
                    supports_safe_search: true,
                    rate_limit: Some(60),
                },
                about: AboutInfo {
                    website: Some("https://www.mojeek.com".to_string()),
                    wikidata_id: Some("Q60747299".to_string()),
                    official_api_documentation: Some("https://www.mojeek.com/support/api/".to_string()),
                    use_official_api: false,
                    require_api_key: false,
                    results: "HTML".to_string(),
                },
                shortcut: Some("mojeek".to_string()),
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

    /// 将时间范围转换为 Mojeek 的日期格式
    ///
    /// # 参数
    ///
    /// * `time_range` - 时间范围枚举值
    ///
    /// # 返回
    ///
    /// Mojeek API 的日期字符串（YYYYMMDD 格式）
    fn time_range_to_mojeek(time_range: TimeRange) -> String {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        
        let days_ago = match time_range {
            TimeRange::Day => 1,
            TimeRange::Week => 7,
            TimeRange::Month => 30,
            TimeRange::Year => 365,
            _ => return String::new(),
        };
        
        let past = now - (days_ago * 86400);
        let dt = chrono::DateTime::<chrono::Utc>::from_timestamp(past as i64, 0);
        
        if let Some(datetime) = dt {
            datetime.format("%Y%m%d").to_string()
        } else {
            String::new()
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

        if html.is_empty() {
            return Ok(Vec::new());
        }

        let document = Html::parse_document(html);
        let mut items = Vec::new();

        // results_xpath = '//ul[@class="results-standard"]/li/a[@class="ob"]'
        let results_selector = Selector::parse("ul.results-standard li").expect("valid selector");
        let url_selector = Selector::parse("a.ob").expect("valid selector");
        let title_selector = Selector::parse("h2 a").expect("valid selector");
        let content_selector = Selector::parse("p.s").expect("valid selector");

        for result in document.select(&results_selector) {
            // url_xpath = './@href'
            let url = result.select(&url_selector).next()
                .and_then(|a| a.value().attr("href"))
                .map(|s| s.to_string())
                .unwrap_or_default();

            // title_xpath = '../h2/a'
            let title = result.select(&title_selector).next()
                .map(|t| t.text().collect::<String>().trim().to_string())
                .unwrap_or_default();

            // content_xpath = '..//p[@class="s"]'
            let content = result.select(&content_selector).next()
                .map(|c| c.text().collect::<String>().trim().to_string())
                .unwrap_or_default();

            if !url.is_empty() && !title.is_empty() {
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

        if items.is_empty() {
            for result in document.select(&Selector::parse("a[href]").expect("valid selector")) {
                let url = result.value().attr("href").unwrap_or_default();
                let title = result.text().collect::<String>().trim().to_string();

                if !title.is_empty() && !url.is_empty() && url.starts_with("http") {
                    items.push(SearchResultItem {
                        title,
                        url: url.to_string(),
                        content: String::new(),
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
        }

  
        Ok(items)
    }
}

impl Default for MojeekEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SearchEngine for MojeekEngine {
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
        self.client.get("https://www.mojeek.com", None).await.is_ok()
    }
}

#[async_trait]
impl RequestResponseEngine for MojeekEngine {
    type Response = String;

    /// 准备请求参数
    fn request(&self, query: &str, params: &mut RequestParams) -> Result<(), Box<dyn Error + Send + Sync>> {
        let mut query_params = vec![
            ("q", query.to_string()),
            ("safe", std::cmp::min(params.safesearch, 1).to_string()),
            ("lb", params.language.clone().unwrap_or_else(|| "".to_string())), // 与 Python SearXNG 一致，默认为空
            ("arc", "none".to_string()), // 与 Python SearXNG 的 engine_traits.json 一致，默认为 "none"
        ];

        // s: pagination offset - 避免在第一页添加s参数，防止触发反爬虫
        if params.pageno > 1 {
            query_params.push(("s", (10 * (params.pageno - 1)).to_string()));
        }

        // time range: since parameter (YYYYMMDD format) - 与 Python SearXNG 一致
        if let Some(ref time_range) = params.time_range {
            let since = match time_range.as_str() {
                "day" => Self::time_range_to_mojeek(TimeRange::Day),
                "week" => Self::time_range_to_mojeek(TimeRange::Week),
                "month" => Self::time_range_to_mojeek(TimeRange::Month),
                "year" => Self::time_range_to_mojeek(TimeRange::Year),
                _ => String::new(),
            };
            if !since.is_empty() {
                query_params.push(("since", since));
            }
        }

        let query_string = build_query_string_owned(query_params.into_iter());

        params.url = Some(format!("https://www.mojeek.com/search?{}", query_string));
        params.method = "GET".to_string();

        // Use simple headers, consistent with Python SearXNG
        // 不使用复杂的反爬虫头部，避免触发检测
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

        // 检查状态码 - HttpClient 已经在底层处理了状态码检查
        // 这里我们只需要检查是否成功
        let status = response.status();
        match status.as_u16() {
            403 => return Err("Mojeek 访问被拒绝，可能触发了反爬虫机制。请稍后重试或使用其他搜索引擎。".into()),
            429 => return Err("Mojeek 请求过于频繁，请稍后重试。".into()),
            503 => return Err("Mojeek 服务暂时不可用，请稍后重试。".into()),
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
        let engine = MojeekEngine::new();
        assert_eq!(engine.info().name, "Mojeek");
        assert_eq!(engine.info().engine_type, EngineType::General);
    }

    #[test]
    fn test_engine_info() {
        let engine = MojeekEngine::new();
        let info = engine.info();
        
        assert!(info.capabilities.supports_pagination);
        assert!(info.capabilities.supports_time_range);
        assert!(info.capabilities.supports_safe_search);
        assert_eq!(info.max_page, 10);
    }

    #[test]
    fn test_request_preparation() {
        let engine = MojeekEngine::new();
        let mut params = RequestParams::default();
        
        let result = engine.request("test query", &mut params);
        assert!(result.is_ok());
        assert!(params.url.is_some());
        
        let url = params.url.expect("Expected valid value");
        assert!(url.contains("mojeek.com"));
        assert!(url.contains("q=test%20query"));
    }

    #[test]
    fn test_request_with_pagination() {
        let engine = MojeekEngine::new();
        let mut params = RequestParams::default();
        params.pageno = 2;
        
        let result = engine.request("test", &mut params);
        assert!(result.is_ok());
        
        let url = params.url.expect("Expected valid value");
        assert!(url.contains("s=10")); // (2-1) * 10 = 10
    }

    #[test]
    fn test_default() {
        let engine = MojeekEngine::default();
        assert_eq!(engine.info().name, "Mojeek");
    }

    #[tokio::test]
    async fn test_is_available() {
        let engine = MojeekEngine::new();
        let _ = engine.is_available().await;
    }

    #[test]
    fn test_parse_empty_html() {
        let result = MojeekEngine::parse_html_results("");
        assert!(result.is_ok());
        assert_eq!(result.expect("Expected valid value").len(), 0);
    }
}
