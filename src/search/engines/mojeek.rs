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
use std::error::Error;

use crate::derive::{
    EngineCapabilities, EngineInfo, EngineStatus, EngineType,
    ResultType, SearchEngine, SearchQuery, SearchResult,
    SearchResultItem, TimeRange, AboutInfo, RequestResponseEngine, RequestParams,
};

/// Mojeek 搜索引擎
///
/// 使用 Mojeek API 进行搜索的引擎实现
pub struct MojeekEngine {
    /// 引擎信息
    info: EngineInfo,
    /// HTTP 客户端
    client: reqwest::Client,
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
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(10))
                .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
                .build()
                .expect("无法创建 HTTP 客户端"),
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
        
        // 检查是否有结果
        if html.is_empty() {
            return Ok(Vec::new());
        }
        
        let document = Html::parse_document(html);
        let mut items = Vec::new();
        
        // Mojeek 的搜索结果通常在 li.result 或类似元素中
        let result_selectors = vec![
            "li.result",
            "div.result",
            "article.result",
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
                let title_selector = Selector::parse("h2, h3, a.title").unwrap();
                let link_selector = Selector::parse("a").unwrap();
                let snippet_selector = Selector::parse("p.s, p.snippet, div.snippet").unwrap();
                
                let title = result.select(&title_selector).next()
                    .map(|t| t.text().collect::<String>().trim().to_string())
                    .unwrap_or_default();
                
                let url = result.select(&link_selector).next()
                    .and_then(|a| a.value().attr("href"))
                    .unwrap_or_default();
                
                let content = result.select(&snippet_selector).next()
                    .map(|s| s.text().collect::<String>().trim().to_string())
                    .unwrap_or_default();
                
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
        match self.client.get("https://www.mojeek.com").send().await {
            Ok(resp) => resp.status().is_success(),
            Err(_) => false,
        }
    }
}

#[async_trait]
impl RequestResponseEngine for MojeekEngine {
    type Response = String;

    /// 准备请求参数
    fn request(&self, query: &str, params: &mut RequestParams) -> Result<(), Box<dyn Error + Send + Sync>> {
        let results_per_page = 10;
        let offset = (params.pageno - 1) * results_per_page;
        
        // 构建查询参数
        let mut query_params = vec![
            ("q", query.to_string()),
            ("safe", if params.safesearch > 0 { "1" } else { "0" }.to_string()),
        ];
        
        // 添加偏移量
        if offset > 0 {
            query_params.push(("s", offset.to_string()));
        }
        
        // 添加时间范围
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
        
        // 构建 URL
        let query_string = query_params
            .iter()
            .map(|(k, v)| format!("{}={}", k, urlencoding::encode(v)))
            .collect::<Vec<_>>()
            .join("&");
        
        params.url = Some(format!("https://www.mojeek.com/search?{}", query_string));
        params.method = "GET".to_string();
        
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
        if !response.status().is_success() {
            return Err(format!("HTTP 错误: {}", response.status()).into());
        }
        
        // 获取响应文本
        let text = response.text().await?;
        
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
        
        let url = params.url.unwrap();
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
        
        let url = params.url.unwrap();
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
        assert_eq!(result.unwrap().len(), 0);
    }
}
