//! Baidu 搜索引擎实现
//!
//! 这是一个基于 Baidu API 的搜索引擎实现。
//! 参考了 Python SearXNG 的 Baidu 引擎实现。
//!
//! ## 功能特性
//!
//! - 支持基本的网页搜索
//! - 支持分页
//! - 支持时间范围过滤
//! - 使用 JSON API
//!
//! ## API 说明
//!
//! Baidu 使用 JSON API 进行搜索：
//! - wd: 查询关键词
//! - rn: 每页结果数量
//! - pn: 分页偏移量
//! - tn: 响应格式（json）
//! - gpc: 时间范围过滤
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
//! use SeeSea::search::engines::baidu::BaiduEngine;
//! use SeeSea::derive::{SearchEngine, SearchQuery};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let engine = BaiduEngine::new();
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

/// Baidu 搜索引擎
///
/// 使用 Baidu JSON API 进行搜索的引擎实现
pub struct BaiduEngine {
    /// 引擎信息
    info: EngineInfo,
    /// HTTP 客户端
    client: reqwest::Client,
}

impl BaiduEngine {
    /// 创建新的 Baidu 引擎实例
    ///
    /// # 示例
    ///
    /// ```
    /// use SeeSea::search::engines::baidu::BaiduEngine;
    ///
    /// let engine = BaiduEngine::new();
    /// ```
    pub fn new() -> Self {
        Self {
            info: EngineInfo {
                name: "Baidu".to_string(),
                engine_type: EngineType::General,
                description: "百度是中国最大的搜索引擎".to_string(),
                status: EngineStatus::Active,
                categories: vec!["general".to_string(), "web".to_string()],
                capabilities: EngineCapabilities {
                    result_types: vec![ResultType::Web],
                    supported_params: vec![
                        "time_range".to_string(),
                    ],
                    max_page_size: 10,
                    supports_pagination: true,
                    supports_time_range: true,
                    supports_language_filter: false,
                    supports_region_filter: false,
                    supports_safe_search: false,
                    rate_limit: Some(60),
                },
                about: AboutInfo {
                    website: Some("https://www.baidu.com".to_string()),
                    wikidata_id: Some("Q14772".to_string()),
                    official_api_documentation: None,
                    use_official_api: false,
                    require_api_key: false,
                    results: "JSON".to_string(),
                },
                shortcut: Some("baidu".to_string()),
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
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(10))
                .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
                .redirect(reqwest::redirect::Policy::none()) // 禁用自动重定向以检测 CAPTCHA
                .build()
                .expect("无法创建 HTTP 客户端"),
        }
    }

    /// 将时间范围转换为秒数
    ///
    /// # 参数
    ///
    /// * `time_range` - 时间范围枚举值
    ///
    /// # 返回
    ///
    /// 时间范围的秒数
    fn time_range_to_seconds(time_range: TimeRange) -> u64 {
        match time_range {
            TimeRange::Day => 86400,      // 1 天
            TimeRange::Week => 604800,    // 7 天
            TimeRange::Month => 2592000,  // 30 天
            TimeRange::Year => 31536000,  // 365 天
            _ => 0,
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
    fn parse_json_results(json_str: &str) -> Result<Vec<SearchResultItem>, Box<dyn Error + Send + Sync>> {
        // 简化版本的 JSON 解析
        // 在实际实现中应该使用 serde_json 完整解析
        let items = Vec::new();
        
        // TODO: 使用 serde_json 解析实际结果
        // 检查是否有有效的 JSON 数据
        if json_str.is_empty() || !json_str.contains("feed") {
            return Ok(items);
        }
        
        Ok(items)
    }

    /// 检测是否遇到 Baidu CAPTCHA
    ///
    /// # 参数
    ///
    /// * `location` - 重定向的 Location 头
    ///
    /// # 返回
    ///
    /// 如果检测到 CAPTCHA 返回 true
    fn detect_captcha(location: Option<&str>) -> bool {
        if let Some(loc) = location {
            loc.contains("wappass.baidu.com/static/captcha")
        } else {
            false
        }
    }
}

impl Default for BaiduEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SearchEngine for BaiduEngine {
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
        match self.client.get("https://www.baidu.com").send().await {
            Ok(resp) => resp.status().is_success(),
            Err(_) => false,
        }
    }
}

#[async_trait]
impl RequestResponseEngine for BaiduEngine {
    type Response = (String, Option<String>); // (JSON 字符串, Location 头)

    /// 准备请求参数
    fn request(&self, query: &str, params: &mut RequestParams) -> Result<(), Box<dyn Error + Send + Sync>> {
        let results_per_page = 10;
        let page_offset = (params.pageno - 1) * results_per_page;
        
        // 构建查询参数
        let mut query_params = vec![
            ("wd", query.to_string()),
            ("rn", results_per_page.to_string()),
            ("pn", page_offset.to_string()),
            ("tn", "json".to_string()),
        ];
        
        // 添加时间范围过滤
        if let Some(ref time_range) = params.time_range {
            let seconds = match time_range.as_str() {
                "day" => 86400,
                "week" => 604800,
                "month" => 2592000,
                "year" => 31536000,
                _ => 0,
            };
            
            if seconds > 0 {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0);
                let past = now.saturating_sub(seconds);
                query_params.push(("gpc", format!("stf={},{}", past, now) + "|stftype=1"));
            }
        }
        
        // 构建 URL
        let query_string = query_params
            .iter()
            .map(|(k, v)| format!("{}={}", k, urlencoding::encode(v)))
            .collect::<Vec<_>>()
            .join("&");
        
        params.url = Some(format!("https://www.baidu.com/s?{}", query_string));
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
        
        // 检查重定向（可能是 CAPTCHA）
        let location = response.headers()
            .get("location")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());
        
        // 检查状态码
        let status = response.status();
        if status.is_redirection() {
            // 可能是 CAPTCHA 重定向
            return Ok((String::new(), location));
        }
        
        if !status.is_success() {
            return Err(format!("HTTP 错误: {}", status).into());
        }
        
        // 获取响应文本
        let text = response.text().await?;
        
        Ok((text, location))
    }

    /// 解析响应为结果列表
    fn response(&self, resp: Self::Response) -> Result<Vec<SearchResultItem>, Box<dyn Error + Send + Sync>> {
        let (json_str, location) = resp;
        
        // 检查是否遇到 CAPTCHA
        if Self::detect_captcha(location.as_deref()) {
            return Err("检测到 Baidu CAPTCHA，请稍后重试".into());
        }
        
        Self::parse_json_results(&json_str)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_engine_creation() {
        let engine = BaiduEngine::new();
        assert_eq!(engine.info().name, "Baidu");
        assert_eq!(engine.info().engine_type, EngineType::General);
    }

    #[test]
    fn test_time_range_conversion() {
        assert_eq!(BaiduEngine::time_range_to_seconds(TimeRange::Day), 86400);
        assert_eq!(BaiduEngine::time_range_to_seconds(TimeRange::Week), 604800);
        assert_eq!(BaiduEngine::time_range_to_seconds(TimeRange::Month), 2592000);
        assert_eq!(BaiduEngine::time_range_to_seconds(TimeRange::Year), 31536000);
    }

    #[test]
    fn test_detect_captcha() {
        assert!(BaiduEngine::detect_captcha(Some("https://wappass.baidu.com/static/captcha")));
        assert!(!BaiduEngine::detect_captcha(Some("https://www.baidu.com")));
        assert!(!BaiduEngine::detect_captcha(None));
    }

    #[test]
    fn test_engine_info() {
        let engine = BaiduEngine::new();
        let info = engine.info();
        
        assert!(info.capabilities.supports_pagination);
        assert!(info.capabilities.supports_time_range);
        assert!(!info.capabilities.supports_safe_search);
        assert_eq!(info.capabilities.max_page_size, 10);
    }

    #[test]
    fn test_request_preparation() {
        let engine = BaiduEngine::new();
        let mut params = RequestParams::default();
        
        let result = engine.request("测试查询", &mut params);
        assert!(result.is_ok());
        assert!(params.url.is_some());
        
        let url = params.url.unwrap();
        assert!(url.contains("www.baidu.com"));
        assert!(url.contains("wd="));
        assert!(url.contains("tn=json"));
    }

    #[test]
    fn test_request_with_pagination() {
        let engine = BaiduEngine::new();
        let mut params = RequestParams::default();
        params.pageno = 2;
        
        let result = engine.request("test", &mut params);
        assert!(result.is_ok());
        
        let url = params.url.unwrap();
        assert!(url.contains("pn=10")); // (2-1) * 10 = 10
    }

    #[test]
    fn test_default() {
        let engine = BaiduEngine::default();
        assert_eq!(engine.info().name, "Baidu");
    }

    #[tokio::test]
    async fn test_is_available() {
        let engine = BaiduEngine::new();
        let _ = engine.is_available().await;
    }

    #[test]
    fn test_parse_empty_json() {
        let result = BaiduEngine::parse_json_results("");
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 0);
    }

    #[test]
    fn test_parse_invalid_json() {
        let result = BaiduEngine::parse_json_results("{}");
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 0);
    }
}
