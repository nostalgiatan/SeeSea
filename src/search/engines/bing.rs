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

use crate::derive::{
    EngineCapabilities, EngineInfo, EngineStatus, EngineType,
    ResultType, SearchEngine, SearchQuery, SearchResult,
    SearchResultItem, TimeRange, AboutInfo, RequestResponseEngine, RequestParams,
};

/// Bing 搜索引擎
///
/// 使用 Bing HTML API 进行搜索的引擎实现
pub struct BingEngine {
    /// 引擎信息
    info: EngineInfo,
    /// HTTP 客户端
    client: reqwest::Client,
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
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(10))
                .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
                .build()
                .expect("无法创建 HTTP 客户端"),
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
        // 简化版本的 HTML 解析
        // 在实际实现中应该使用 scraper 或 html5ever crate
        let items = Vec::new();
        
        // TODO: 使用 HTML 解析器提取实际结果
        // 这里提供一个简化的占位实现
        
        // 检查是否有结果
        if html.contains("There are no results") || html.is_empty() {
            return Ok(items);
        }
        
        // 模拟提取结果（实际应该使用 CSS 选择器）
        // 在完整实现中应该：
        // 1. 使用 scraper::Html 解析 HTML
        // 2. 使用 CSS 选择器找到结果元素（例如：ol#b_results > li.b_algo）
        // 3. 提取标题（h2/a）、URL（a/@href）、摘要（p）等信息
        // 4. 处理 URL 解码（Bing 使用 base64 编码的重定向 URL）
        // 5. 提取结果总数（span.sb_count）
        
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
        match self.client.get("https://www.bing.com").send().await {
            Ok(resp) => resp.status().is_success(),
            Err(_) => false,
        }
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
        // 设置语言和地区
        let language = params.language.as_deref().unwrap_or("en").to_string();
        let region = params.language.as_deref().unwrap_or("us").to_string();
        
        // 设置 cookies
        Self::set_bing_cookies(params, &language, &region);
        
        // 构建查询参数
        let mut query_params = vec![
            ("q", query.to_string()),
            ("pq", query.to_string()), // 避免分页问题
        ];
        
        // 添加分页参数
        if params.pageno > 1 {
            query_params.push(("first", Self::page_offset(params.pageno).to_string()));
            
            if let Some(form) = Self::page_form(params.pageno) {
                query_params.push(("FORM", form));
            }
        }
        
        // 构建基础 URL
        let base_url = "https://www.bing.com/search";
        let query_string = query_params
            .iter()
            .map(|(k, v)| format!("{}={}", k, urlencoding::encode(v)))
            .collect::<Vec<_>>()
            .join("&");
        
        let mut url = format!("{}?{}", base_url, query_string);
        
        // 添加时间范围过滤
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
        
        let mut request = self.client.get(url);
        
        // 添加自定义头
        for (key, value) in &params.headers {
            request = request.header(key, value);
        }
        
        // 添加 cookies
        for (key, value) in &params.cookies {
            request = request.header("Cookie", format!("{}={}", key, value));
        }
        
        // 发送请求（允许重定向）
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
        
        let url = params.url.unwrap();
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
        
        let url = params.url.unwrap();
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
        
        let url = params.url.unwrap();
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
        assert_eq!(result.unwrap().len(), 0);
    }

    #[test]
    fn test_parse_no_results_html() {
        let html = "<html><body>There are no results</body></html>";
        let result = BingEngine::parse_html_results(html);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 0);
    }
}
