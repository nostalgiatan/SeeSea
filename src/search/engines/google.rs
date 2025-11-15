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
use std::sync::Arc;
use std::error::Error;

use crate::derive::{
    EngineCapabilities, EngineInfo, EngineStatus, EngineType,
    ResultType, SearchEngine, SearchQuery, SearchResult,
    SearchResultItem, TimeRange, AboutInfo, RequestResponseEngine, RequestParams,
};
use crate::net::client::HttpClient;
use crate::net::types::NetworkConfig;
use super::utils::build_query_string_owned;

/// Google 搜索引擎
///
/// 使用 Google HTML API 进行搜索的引擎实现
pub struct GoogleEngine {
    /// 引擎信息
    info: EngineInfo,
    /// HTTP 客户端
    client: Arc<HttpClient>,
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
        let client = HttpClient::new(NetworkConfig::default())
            .unwrap_or_else(|_| panic!("Failed to create HTTP client"));
        Self::with_client(Arc::new(client))
    }

    pub fn with_client(client: Arc<HttpClient>) -> Self {
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
            client,
        }
    }

    /// Generate Google's ARC async parameter for AJAX API requests
    ///
    /// Google's ARC (Asynchronous Result Component) uses a specific format to track
    /// pagination state and enable progressive loading. This implementation follows
    /// SearXNG's ui_async pattern.
    ///
    /// Format: `arc_id:srp_{23_random_chars}_{page_marker},use_ac:true,_fmt:prog`
    ///
    /// The random component helps prevent cache collisions, while the page marker
    /// ensures results are fetched for the correct page.
    fn generate_async_param(start: u32) -> String {
        let page_num = start / 10;

        // Use nanosecond timestamp for pseudo-random ID (deterministic but unique enough)
        let random_part = format!("{:023}", std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or_else(|_| 0) % 1000000000000000000000);
        let arc_id = format!("srp_{random_part}_1{:02}", page_num);

        format!("arc_id:{},use_ac:true,_fmt:prog", arc_id)
    }

    /// Map internal TimeRange enum to Google's time filter codes
    ///
    /// Google uses single-letter codes: h(hour), d(day), w(week), m(month), y(year)
    #[allow(dead_code)]
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
    fn parse_html_results(response: &str) -> Result<Vec<SearchResultItem>, Box<dyn Error + Send + Sync>> {
        // 检查响应是否为空
        if response.trim().is_empty() {
            return Ok(Vec::new());
        }

        // 检查响应类型
        let trimmed = response.trim_start();
        if trimmed.starts_with(")]}") || trimmed.starts_with("[") {
            // 这是 Google AJAX API 响应
            return Self::parse_ajax_response(response);
        } else if trimmed.starts_with("<") || trimmed.starts_with("<!") {
            // 这是 HTML 响应（回退）
            return Self::parse_html_response(response);
        } else {
            // 未知格式，尝试HTML解析
            return Self::parse_html_response(response);
        }
    }

    /// 解析 Google AJAX API 响应
    fn parse_ajax_response(response: &str) -> Result<Vec<SearchResultItem>, Box<dyn Error + Send + Sync>> {

        // 移除响应前缀，找到JSON部分
        // Google响应格式: )]}'
        // 寻找实际的JSON数组开始
        let mut json_start = None;
        let mut bracket_count = 0;
        let mut in_string = false;
        let mut escape_next = false;
        
        // 首先找到第一个 '['
        for (i, ch) in response.char_indices() {
            if ch == '[' && json_start.is_none() {
                json_start = Some(i);
                bracket_count = 1;
                break;
            }
        }

        // 如果没有找到JSON开始标记，回退到HTML解析
        let start = match json_start {
            Some(s) => s,
            None => return Self::parse_html_response(response),
        };
        let mut end = start;
        
        // 正确地找到匹配的右括号，考虑字符串中的括号
        for (i, ch) in response[start + 1..].char_indices() {
            let actual_i = start + 1 + i;
            
            if escape_next {
                escape_next = false;
                continue;
            }
            
            match ch {
                '\\' if in_string => {
                    escape_next = true;
                }
                '"' => {
                    in_string = !in_string;
                }
                '[' if !in_string => {
                    bracket_count += 1;
                }
                ']' if !in_string => {
                    bracket_count -= 1;
                    if bracket_count == 0 {
                        end = actual_i;
                        break;
                    }
                }
                _ => {}
            }
        }

        if bracket_count != 0 {
            // JSON is incomplete/truncated
            println!("JSON解析失败: EOF while parsing a list at line 1 column {}", end);
            println!("原始JSON: {}", &response[start..response.len().min(start + 100)]);
            // 回退到HTML解析
            return Self::parse_html_response(response);
        }

        let json_str = &response[start..=end];

        // 尝试解析为JSON
        match serde_json::from_str::<serde_json::Value>(json_str) {
            Ok(data) => {
                // 解析Google的AJAX响应格式
                Self::parse_google_ajax_data(data)
            }
            Err(e) => {
                println!("JSON解析失败: {}", e);
                // 输出原始JSON用于调试（限制长度）
                let preview_len = json_str.len().min(200);
                println!("原始JSON: {}", &json_str[..preview_len]);
                // 回退到HTML解析
                Self::parse_html_response(response)
            }
        }
    }

    /// 解析Google的AJAX数据结构
    fn parse_google_ajax_data(data: serde_json::Value) -> Result<Vec<SearchResultItem>, Box<dyn Error + Send + Sync>> {
        let mut items = Vec::with_capacity(10);  // Pre-allocate for typical result count

        // Google AJAX响应的结构比较复杂，这里使用简化的解析方法
        // 实际实现可能需要根据具体的数据结构调整

        if let Some(array) = data.as_array() {
            for (i, item) in array.iter().enumerate() {
                if let Some(obj) = item.as_array() {
                    if obj.len() >= 3 {
                        // 尝试提取结果信息
                        let title = obj.get(0).and_then(|v| v.as_str()).unwrap_or("").to_string();
                        let url = obj.get(1).and_then(|v| v.as_str()).unwrap_or("").to_string();

                        if !title.is_empty() && !url.is_empty() && url.starts_with("http") {
                            items.push(SearchResultItem {
                                title,
                                url: url.clone(),
                                content: format!("搜索结果 #{}", i + 1), // 简化的内容
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

        println!("从AJAX响应解析出 {} 个结果", items.len());
        Ok(items)
    }

    /// 解析HTML响应（使用 Python SearxNG 的选择器）
    fn parse_html_response(html: &str) -> Result<Vec<SearchResultItem>, Box<dyn Error + Send + Sync>> {
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
        let mut items = Vec::with_capacity(10);  // Pre-allocate for typical result count

        // 使用 Python SearxNG 的选择器: div[contains(@jscontroller, "SC7lYd")]
        // 在 CSS 选择器中，我们使用属性包含选择器
        let result_selector = match Selector::parse("div[jscontroller*=\"SC7lYd\"]") {
            Ok(sel) => sel,
            Err(_) => {
                // 回退到通用选择器
                match Selector::parse("div.g") {
                    Ok(sel) => sel,
                    Err(_) => return Ok(Vec::new()),
                }
            }
        };

        for result in document.select(&result_selector) {
            // Python: title_tag = eval_xpath_getindex(result, './/a/h3[1]', 0, default=None)
            // CSS: a > h3 或 a h3
            let title_selector = Selector::parse("a > h3, a h3").expect("valid selector");
            let title_elem = match result.select(&title_selector).next() {
                Some(elem) => elem,
                None => continue,
            };
            
            let title = title_elem.text().collect::<String>().trim().to_string();
            
            // Python: url = eval_xpath_getindex(result, './/a[h3]/@href', 0, None)
            // 找到包含 h3 的 a 标签
            let link_selector = Selector::parse("a").expect("valid selector");
            let h3_selector = Selector::parse("h3").expect("valid selector");
            
            let mut url = String::new();
            for link in result.select(&link_selector) {
                // 检查这个 a 标签是否包含 h3
                if link.select(&h3_selector).next().is_some() {
                    if let Some(href) = link.value().attr("href") {
                        url = href.to_string();
                        break;
                    }
                }
            }
            
            if url.is_empty() {
                continue;
            }
            
            // Python: content_nodes = eval_xpath(result, './/div[contains(@data-sncf, "1")]')
            let content_selector = Selector::parse("div[data-sncf*=\"1\"]").expect("valid selector");
            let mut content = String::new();
            
            for content_node in result.select(&content_selector) {
                // 移除 script 标签的文本（Python 版本中的逻辑）
                let text = content_node.text()
                    .filter(|t| !t.trim().is_empty())
                    .collect::<Vec<_>>()
                    .join(" ")
                    .trim()
                    .to_string();
                
                if !text.is_empty() {
                    content = text;
                    break;
                }
            }
            
            // Python 中如果没有 content，尝试其他选择器作为后备
            if content.is_empty() {
                // 尝试查找任何包含文本的 div
                let fallback_selectors = vec!["div[data-content-feature]", "div.VwiC3b", "div span"];
                for fallback_sel_str in fallback_selectors {
                    if let Ok(fallback_sel) = Selector::parse(fallback_sel_str) {
                        for node in result.select(&fallback_sel) {
                            let text = node.text()
                                .filter(|t| !t.trim().is_empty())
                                .collect::<Vec<_>>()
                                .join(" ")
                                .trim()
                                .to_string();
                            if !text.is_empty() && text.len() > 10 {
                                content = text;
                                break;
                            }
                        }
                        if !content.is_empty() {
                            break;
                        }
                    }
                }
            }
            
            // 如果仍然没有 content，使用标题作为内容
            if content.is_empty() {
                content = format!("来自 {}", title);
            }
            
            // 提取缩略图 (thumbnail)
            let thumbnail = if let Some(content_node) = result.select(&content_selector).next() {
                let img_selector = Selector::parse("img").expect("valid selector");
                content_node.select(&img_selector)
                    .next()
                    .and_then(|img| img.value().attr("src"))
                    .map(|src| src.to_string())
            } else {
                None
            };

            if !title.is_empty() && !url.is_empty() && url.starts_with("http") {
                items.push(SearchResultItem {
                    title,
                    url: url.clone(),
                    content,
                    display_url: Some(url.clone()),
                    site_name: None,
                    score: 1.0,
                    result_type: ResultType::Web,
                    thumbnail,
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
        match self.client.get("https://www.google.com", None).await {
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
        
        // 生成 ARC async 参数 (基于 SearXNG 实现)
        let async_param = Self::generate_async_param(start as u32);

        // 构建 ARC AJAX API 查询参数 (基于 SearXNG 的实现)
        // Construct query parameters with optimal capacity
        let mut query_params = Vec::with_capacity(15);
        query_params.push(("q", query.to_string()));
        query_params.push(("start", start.to_string()));
        query_params.push(("ie", "utf8".to_string()));
        query_params.push(("oe", "utf8".to_string()));
        query_params.push(("filter", "0".to_string()));
        query_params.push(("hl", "en-US".to_string()));
        query_params.push(("lr", "lang_en".to_string()));
        query_params.push(("cr", "countryUS".to_string()));
        query_params.push(("asearch", "arc".to_string()));
        query_params.push(("async", async_param));
        
        // Add optional language parameter
        if let Some(ref lang) = params.language {
            if !lang.is_empty() && lang != "all" {
                query_params.push(("lr", format!("lang_{}", lang)));
            }
        }
        
        // Add time range filter if specified
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
        
        // Add safe search level
        if params.safesearch > 0 {
            query_params.push(("safe", Self::safesearch_to_google(params.safesearch).to_string()));
        }
        
        // Build query string using optimized utility function
        let query_string = build_query_string_owned(query_params.into_iter());
        
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
        
        // 创建请求选项
        let mut options = crate::net::types::RequestOptions::default();
        options.timeout = std::time::Duration::from_secs(10);
        
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
        
        // 获取最终 URL（可能有重定向）
        let final_url = response.url().to_string();
        
        // 检查状态码
        let status = response.status();
        match status.as_u16() {
            429 => return Err(format!("Google 请求过于频繁 (429)，URL: {}", final_url).into()),
            403 => return Err(format!("Google 访问被拒绝 (403)，可能是 CAPTCHA。URL: {}", final_url).into()),
            302 | 301 => return Err(format!("Google 重定向到: {}", final_url).into()),
            _ if !status.is_success() => {
                // 尝试获取响应内容来诊断问题
                let error_text = response.text().await.unwrap_or_else(|_| "无法读取错误响应".to_string());
                let preview = if error_text.len() > 200 {
                    &error_text[..200]
                } else {
                    &error_text
                };
                return Err(format!("HTTP 错误: {}, URL: {}, 响应预览: {}", status, final_url, preview).into());
            },
            _ => {} // 继续处理
        }
        
        // 获取响应文本
        let text = response.text().await
            .map_err(|e| format!("Failed to read response: {}", e))?;

        
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
        
        let url = params.url.expect("Expected valid value");
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
        
        let url = params.url.expect("Expected valid value");
        assert!(url.contains("lr=lang_en"));
    }

    #[test]
    fn test_request_with_pagination() {
        let engine = GoogleEngine::new();
        let mut params = RequestParams::default();
        params.pageno = 3;
        
        let result = engine.request("test", &mut params);
        assert!(result.is_ok());
        
        let url = params.url.expect("Expected valid value");
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
        assert_eq!(result.expect("Expected valid value").len(), 0);
    }

    #[test]
    fn test_parse_no_results_html() {
        let html = "<html><body>did not match any documents</body></html>";
        let result = GoogleEngine::parse_html_results(html);
        assert!(result.is_ok());
        assert_eq!(result.expect("Expected valid value").len(), 0);
    }

    #[test]
    fn test_parse_captcha_html() {
        let html = "<html><body>sorry.google.com</body></html>";
        let result = GoogleEngine::parse_html_results(html);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_html_with_results() {
        // 测试包含搜索结果的 HTML
        let html = "<html><body><div jscontroller=\"SC7lYd\"><a href=\"https://example.com\"><h3>Example Title</h3></a><div data-sncf=\"1\"><span>This is example content for testing.</span></div></div></body></html>";
        let result = GoogleEngine::parse_html_results(html);
        assert!(result.is_ok());
        let items = result.unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].title, "Example Title");
        assert_eq!(items[0].url, "https://example.com");
        assert!(items[0].content.contains("example content"));
    }

    #[test]
    fn test_parse_html_with_missing_content() {
        // 测试缺少内容字段的 HTML，应该使用标题作为内容
        let html = "<html><body><div jscontroller=\"SC7lYd\"><a href=\"https://example.com\"><h3>Example Title</h3></a></div></body></html>";
        let result = GoogleEngine::parse_html_results(html);
        assert!(result.is_ok());
        let items = result.unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].title, "Example Title");
        assert!(items[0].content.contains("Example Title"));
    }

    #[test]
    fn test_parse_ajax_response_invalid() {
        // 测试无效的 AJAX 响应应该回退到 HTML 解析
        let invalid_ajax = ")]}'{invalid json";
        let result = GoogleEngine::parse_ajax_response(invalid_ajax);
        assert!(result.is_ok());
        // 应该返回空结果，因为没有有效的 HTML 结构
        let items = result.unwrap();
        assert_eq!(items.len(), 0);
    }

    #[test]
    fn test_detect_empty_response() {
        // 测试空响应
        let result = GoogleEngine::parse_html_results("");
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 0);
    }
}
