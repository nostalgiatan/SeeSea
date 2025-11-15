//! Yahoo 搜索引擎实现
//!
//! 这是一个基于 Yahoo HTML API 的搜索引擎实现。
//! 参考了 Python SearXNG 的 Yahoo 引擎实现。
//!
//! ## 功能特性
//!
//! - 支持基本的网页搜索
//! - 支持分页
//! - 支持时间范围过滤
//! - 支持安全搜索
//! - 支持多语言和地区
//!
//! ## API 说明
//!
//! Yahoo 使用标准的 URL 参数进行搜索：
//! - p: 查询关键词
//! - btf: 时间过滤（d=天, w=周, m=月）
//! - b: 分页偏移量
//! - pz: 每页结果数量
//! - sB cookie: 包含安全搜索和语言设置
//!
//! ## 安全性
//!
//! - 避免使用 unwrap()，使用 ? 操作符处理错误
//! - 所有网络请求都有超时设置
//! - 完整的错误处理
//!
//! ## 示例
//!
//! ```no_run
//! use SeeSea::search::engines::yahoo::YahooEngine;
//! use SeeSea::derive::{SearchEngine, SearchQuery};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let engine = YahooEngine::new();
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

/// Yahoo 搜索引擎
///
/// 使用 Yahoo HTML API 进行搜索的引擎实现
pub struct YahooEngine {
    /// 引擎信息
    info: EngineInfo,
    /// HTTP 客户端
    client: reqwest::Client,
}

impl YahooEngine {
    /// 创建新的 Yahoo 引擎实例
    ///
    /// # 示例
    ///
    /// ```
    /// use SeeSea::search::engines::yahoo::YahooEngine;
    ///
    /// let engine = YahooEngine::new();
    /// ```
    pub fn new() -> Self {
        Self {
            info: EngineInfo {
                name: "Yahoo".to_string(),
                engine_type: EngineType::General,
                description: "Yahoo 是一个流行的搜索引擎".to_string(),
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
                    rate_limit: Some(60),
                },
                about: AboutInfo {
                    website: Some("https://search.yahoo.com".to_string()),
                    wikidata_id: None,
                    official_api_documentation: Some("https://developer.yahoo.com/api/".to_string()),
                    use_official_api: false,
                    require_api_key: false,
                    results: "HTML".to_string(),
                },
                shortcut: Some("yahoo".to_string()),
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
                .build().unwrap_or(reqwest::Client::new())
                ,
        }
    }

    /// 获取 Yahoo 域名
    ///
    /// 根据地区代码返回合适的 Yahoo 域名
    ///
    /// # 参数
    ///
    /// * `region` - 地区代码
    ///
    /// # 返回
    ///
    /// Yahoo 域名字符串
    fn get_domain(region: Option<&str>) -> &'static str {
        match region {
            Some("CO") => "co.search.yahoo.com",
            Some("TH") => "th.search.yahoo.com",
            Some("VE") => "ve.search.yahoo.com",
            Some("CL") => "cl.search.yahoo.com",
            Some("HK") => "hk.search.yahoo.com",
            Some("PE") => "pe.search.yahoo.com",
            Some("CA") => "ca.search.yahoo.com",
            Some("DE") => "de.search.yahoo.com",
            Some("FR") => "fr.search.yahoo.com",
            Some("TW") => "tw.search.yahoo.com",
            Some("GB") | Some("UK") => "uk.search.yahoo.com",
            Some("BR") => "br.search.yahoo.com",
            Some("IN") => "in.search.yahoo.com",
            Some("ES") => "espanol.search.yahoo.com",
            Some("PH") => "ph.search.yahoo.com",
            Some("AR") => "ar.search.yahoo.com",
            Some("MX") => "mx.search.yahoo.com",
            Some("SG") => "sg.search.yahoo.com",
            _ => "search.yahoo.com",
        }
    }

    /// 将时间范围转换为 Yahoo 的时间过滤参数
    ///
    /// # 参数
    ///
    /// * `time_range` - 时间范围枚举值
    ///
    /// # 返回
    ///
    /// Yahoo API 的时间过滤字符串
    fn time_range_to_yahoo(time_range: TimeRange) -> &'static str {
        match time_range {
            TimeRange::Day => "d",
            TimeRange::Week => "w",
            TimeRange::Month => "m",
            _ => "",
        }
    }

    /// 将安全搜索级别转换为 Yahoo 的安全搜索参数
    ///
    /// # 参数
    ///
    /// * `level` - 安全搜索级别（0: 关闭, 1: 中等, 2: 严格）
    ///
    /// # 返回
    ///
    /// Yahoo API 的安全搜索字符串
    fn safesearch_to_yahoo(level: i32) -> &'static str {
        match level {
            0 => "p",
            1 => "i",
            2 => "r",
            _ => "i",
        }
    }

    /// 构建 sB cookie
    ///
    /// # 参数
    ///
    /// * `safesearch` - 安全搜索级别
    /// * `language` - 语言代码
    ///
    /// # 返回
    ///
    /// sB cookie 字符串
    fn build_sb_cookie(safesearch: i32, language: &str) -> String {
        let lang_value = format!("lang_{}", language);
        let params = vec![
            ("v", "1".to_string()),
            ("vm", Self::safesearch_to_yahoo(safesearch).to_string()),
            ("fl", "1".to_string()),
            ("vl", lang_value),
            ("pn", "10".to_string()),
            ("rw", "new".to_string()),
            ("userset", "1".to_string()),
        ];
        
        params
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect::<Vec<_>>()
            .join("&")
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
        
        if html.contains("No results") || html.is_empty() {
            return Ok(Vec::new());
        }
        
        let document = Html::parse_document(html);
        let mut items = Vec::new();
        
        // Python SearXNG uses: '//div[contains(@class,"algo-sr")]'
        // This is the correct selector for Yahoo search results
        let result_selector = Selector::parse("div[class*=\"algo-sr\"]")
            .or_else(|_| Selector::parse("div[class*=\"algo\"]"))
            .expect("valid selector");
        
        for result in document.select(&result_selector) {
            // Python: url_xpath = './/div[contains(@class,"compTitle")]/h3/a/@href'
            // or for search.yahoo.com: './/div[contains(@class,"compTitle")]/a/@href'
            let url_selector = Selector::parse("div[class*=\"compTitle\"] a").expect("valid selector");
            let url_elem = result.select(&url_selector).next();
            
            if url_elem.is_none() {
                continue;
            }
            
            let url_elem = url_elem.unwrap();
            let mut url = url_elem.value().attr("href").unwrap_or("").to_string();
            
            // Parse Yahoo tracking URL: remove /RU= and /RK= wrappers
            if url.contains("/RU=") {
                if let Some(start_pos) = url.find("/RU=") {
                    let after_ru = &url[start_pos + 4..];
                    if let Some(http_start) = after_ru.find("http") {
                        let url_part = &after_ru[http_start..];
                        // Find end marker /RS or /RK
                        let mut end_pos = url_part.len();
                        for marker in &["/RS", "/RK"] {
                            if let Some(pos) = url_part.rfind(marker) {
                                if pos < end_pos {
                                    end_pos = pos;
                                }
                            }
                        }
                        url = urlencoding::decode(&url_part[..end_pos])
                            .unwrap_or_else(|_| url_part[..end_pos].into())
                            .to_string();
                    }
                }
            }
            
            if url.is_empty() || !url.starts_with("http") {
                continue;
            }
            
            // Python: title_xpath = './/h3//a/@aria-label'
            // or for search.yahoo.com: './/div[contains(@class,"compTitle")]/a/h3/span'
            let title = url_elem.value().attr("aria-label")
                .map(|s| s.to_string())
                .or_else(|| {
                    // Try h3 text content
                    let h3_selector = Selector::parse("h3").expect("valid selector");
                    url_elem.select(&h3_selector).next()
                        .map(|h3| h3.text().collect::<String>().trim().to_string())
                })
                .or_else(|| {
                    // Fallback: use link text
                    Some(url_elem.text().collect::<String>().trim().to_string())
                })
                .unwrap_or_default();
            
            if title.is_empty() {
                continue;
            }
            
            // Python: content = eval_xpath_getindex(result, './/div[contains(@class, "compText")]', 0, default='')
            let content_selector = Selector::parse("div[class*=\"compText\"]").expect("valid selector");
            let content = result.select(&content_selector).next()
                .map(|c| c.text().collect::<String>().trim().to_string())
                .unwrap_or_default();
            
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
        
        Ok(items)
    }
}

impl Default for YahooEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SearchEngine for YahooEngine {
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
        match self.client.get("https://search.yahoo.com").send().await {
            Ok(resp) => resp.status().is_success(),
            Err(_) => false,
        }
    }
}

#[async_trait]
impl RequestResponseEngine for YahooEngine {
    type Response = String;

    /// 准备请求参数
    fn request(&self, query: &str, params: &mut RequestParams) -> Result<(), Box<dyn Error + Send + Sync>> {
        let language = params.language.as_deref().unwrap_or("en");
        let region = params.language.as_deref();
        
        // 构建查询参数
        let mut query_params = vec![("p", query.to_string())];
        
        // 添加时间范围
        if let Some(ref time_range) = params.time_range {
            let btf = match time_range.as_str() {
                "day" => "d",
                "week" => "w",
                "month" => "m",
                _ => "",
            };
            if !btf.is_empty() {
                query_params.push(("btf", btf.to_string()));
            }
        }
        
        // 添加分页参数
        if params.pageno == 1 {
            query_params.push(("iscqry", String::new()));
        } else if params.pageno >= 2 {
            query_params.push(("b", (params.pageno * 7 + 1).to_string()));
            query_params.push(("pz", "7".to_string()));
            query_params.push(("bct", "0".to_string()));
            query_params.push(("xargs", "0".to_string()));
        }
        
        // 构建 URL
        let domain = Self::get_domain(region);
        let query_string = query_params
            .iter()
            .map(|(k, v)| format!("{}={}", k, urlencoding::encode(v)))
            .collect::<Vec<_>>()
            .join("&");
        
        params.url = Some(format!("https://{}/search?{}", domain, query_string));
        params.method = "GET".to_string();
        
        // 设置 sB cookie
        let sb_cookie = Self::build_sb_cookie(params.safesearch, language);
        params.cookies.insert("sB".to_string(), sb_cookie);
        
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
        
        // 添加 cookies
        for (key, value) in &params.cookies {
            request = request.header("Cookie", format!("{}={}", key, value));
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
        let engine = YahooEngine::new();
        assert_eq!(engine.info().name, "Yahoo");
        assert_eq!(engine.info().engine_type, EngineType::General);
    }

    #[test]
    fn test_domain_selection() {
        assert_eq!(YahooEngine::get_domain(None), "search.yahoo.com");
        assert_eq!(YahooEngine::get_domain(Some("UK")), "uk.search.yahoo.com");
        assert_eq!(YahooEngine::get_domain(Some("FR")), "fr.search.yahoo.com");
        assert_eq!(YahooEngine::get_domain(Some("DE")), "de.search.yahoo.com");
        assert_eq!(YahooEngine::get_domain(Some("JP")), "search.yahoo.com");
    }

    #[test]
    fn test_time_range_conversion() {
        assert_eq!(YahooEngine::time_range_to_yahoo(TimeRange::Day), "d");
        assert_eq!(YahooEngine::time_range_to_yahoo(TimeRange::Week), "w");
        assert_eq!(YahooEngine::time_range_to_yahoo(TimeRange::Month), "m");
        assert_eq!(YahooEngine::time_range_to_yahoo(TimeRange::Year), "");
    }

    #[test]
    fn test_safesearch_conversion() {
        assert_eq!(YahooEngine::safesearch_to_yahoo(0), "p");
        assert_eq!(YahooEngine::safesearch_to_yahoo(1), "i");
        assert_eq!(YahooEngine::safesearch_to_yahoo(2), "r");
    }

    #[test]
    fn test_build_sb_cookie() {
        let cookie = YahooEngine::build_sb_cookie(1, "en");
        assert!(cookie.contains("v=1"));
        assert!(cookie.contains("vm=i"));
        assert!(cookie.contains("vl=lang_en"));
    }

    #[test]
    fn test_engine_info() {
        let engine = YahooEngine::new();
        let info = engine.info();
        
        assert!(info.capabilities.supports_pagination);
        assert!(info.capabilities.supports_time_range);
        assert!(info.capabilities.supports_safe_search);
        assert_eq!(info.capabilities.max_page_size, 10);
    }

    #[test]
    fn test_request_preparation() {
        let engine = YahooEngine::new();
        let mut params = RequestParams::default();
        
        let result = engine.request("test query", &mut params);
        assert!(result.is_ok());
        assert!(params.url.is_some());
        
        let url = params.url.expect("Expected valid value");
        assert!(url.contains("search.yahoo.com"));
        assert!(url.contains("p=test%20query"));
    }

    #[test]
    fn test_request_with_pagination() {
        let engine = YahooEngine::new();
        let mut params = RequestParams::default();
        params.pageno = 2;
        
        let result = engine.request("test", &mut params);
        assert!(result.is_ok());
        
        let url = params.url.expect("Expected valid value");
        assert!(url.contains("b=15")); // page 2: 2 * 7 + 1 = 15
        assert!(url.contains("pz=7"));
    }

    #[test]
    fn test_default() {
        let engine = YahooEngine::default();
        assert_eq!(engine.info().name, "Yahoo");
    }

    #[tokio::test]
    async fn test_is_available() {
        let engine = YahooEngine::new();
        let _ = engine.is_available().await;
    }

    #[test]
    fn test_parse_empty_html() {
        let result = YahooEngine::parse_html_results("");
        assert!(result.is_ok());
        assert_eq!(result.expect("Expected valid value").len(), 0);
    }
}
