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

/// Baidu 搜索引擎
///
/// 使用 Baidu JSON API 进行搜索的引擎实现
pub struct BaiduEngine {
    /// 引擎信息
    info: EngineInfo,
    /// HTTP 客户端
    client: Arc<HttpClient>,
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
        let client = HttpClient::new(NetworkConfig::default())
            .unwrap_or_else(|_| panic!("Failed to create HTTP client"));
        Self::with_client(Arc::new(client))
    }

    pub fn with_client(client: Arc<HttpClient>) -> Self {
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
            client,
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
    #[allow(dead_code)]
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
        use serde_json::Value;

        // 检查是否有有效的 JSON 数据
        if json_str.is_empty() {
            return Ok(Vec::new());
        }

        // 检查是否收到了HTML/CAPTCHA而不是JSON
        let trimmed = json_str.trim();
        if trimmed.starts_with('<') ||
           trimmed.starts_with("Found") ||
           trimmed.contains("wappass.baidu.com") ||
           trimmed.contains("captcha") ||
           trimmed.to_lowercase().contains("please verify") {
            return Err("Baidu返回HTML/CAPTTCHA页面而不是JSON，可能触发了反爬虫机制".into());
        }

        // 尝试解析JSON，如果失败提供更详细的错误信息
        let json: Value = match serde_json::from_str(json_str) {
            Ok(json) => json,
            Err(e) => {
                return Err(format!("Baidu JSON解析失败: {}。响应内容前100字符: {}",
                    e, &json_str[..json_str.len().min(100)]).into());
            }
        };
        let mut items = Vec::new();
        
        if let Some(feed) = json.get("feed") {
            if let Some(entries) = feed.get("entry").and_then(|e| e.as_array()) {
                for entry in entries {
                    let title = entry.get("title")
                        .and_then(|t| t.as_str())
                        .unwrap_or("")
                        .to_string();
                    
                    let url = entry.get("url")
                        .or_else(|| entry.get("link"))
                        .and_then(|u| u.as_str())
                        .unwrap_or("")
                        .to_string();
                    
                    let content = entry.get("content")
                        .or_else(|| entry.get("abstract"))
                        .or_else(|| entry.get("summary"))
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
        } else if let Some(results) = json.get("results").and_then(|r| r.as_array()) {
            for result in results {
                let title = result.get("title")
                    .and_then(|t| t.as_str())
                    .unwrap_or("")
                    .to_string();
                
                let url = result.get("url")
                    .or_else(|| result.get("link"))
                    .and_then(|u| u.as_str())
                    .unwrap_or("")
                    .to_string();
                
                let content = result.get("content")
                    .or_else(|| result.get("abstract"))
                    .or_else(|| result.get("summary"))
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
        self.client.get("https://www.baidu.com", None).await.is_ok()
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
        
        // Build URL with optimized query string
        let query_string = build_query_string_owned(query_params.into_iter());
        
        params.url = Some(format!("https://www.baidu.com/s?{}", query_string));
        params.method = "GET".to_string();
        
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

        // 检查重定向（可能是 CAPTCHA）
        let location = response.headers()
            .get("location")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());

        if status.is_redirection() {
            // 可能是 CAPTCHA 重定向
            return Ok((String::new(), location));
        }

        if !status.is_success() {
            return Err(format!("HTTP 错误: {}", status).into());
        }

        // 获取响应文本
        let text = response.text().await
            .map_err(|e| format!("Failed to read response: {}", e))?;

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
        
        let url = params.url.expect("URL should be set after request preparation");
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
        
        let url = params.url.expect("URL should be set after request preparation");
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
        assert_eq!(result.expect("Valid result expected").len(), 0);
    }

    #[test]
    fn test_parse_invalid_json() {
        let result = BaiduEngine::parse_json_results("{}");
        assert!(result.is_ok());
        assert_eq!(result.expect("Valid result expected").len(), 0);
    }
}
