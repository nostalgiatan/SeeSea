//! 搜索引擎便利宏定义
//!
//! 提供简化搜索引擎实现的宏（暂时简化版本）

/// 创建简单搜索引擎的宏
#[macro_export]
macro_rules! simple_engine {
    (
        $(#[$struct_meta:meta])*
        pub struct $name:ident {
            client: reqwest::Client,
            $($field:ident: $field_type:ty),* $(,)?
        }
    ) => {
        $(#[$struct_meta])*
        pub struct $name {
            client: reqwest::Client,
            $($field: $field_type),*
        }

        impl $name {
            /// 创建新的搜索引擎实例
            pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
                let client = reqwest::Client::builder()
                    .user_agent(concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION")))
                    .timeout(std::time::Duration::from_secs(30))
                    .build()?;

                Ok(Self {
                    client,
                    $($field: Default::default()),*
                })
            }

            /// 带自定义客户端的构造函数
            pub fn with_client(client: reqwest::Client) -> Self {
                Self {
                    client,
                    $($field: Default::default()),*
                }
            }

            /// 带超时时间的构造函数
            pub fn with_timeout(timeout_secs: u64) -> Result<Self, Box<dyn std::error::Error>> {
                let client = reqwest::Client::builder()
                    .user_agent(concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION")))
                    .timeout(std::time::Duration::from_secs(timeout_secs))
                    .build()?;

                Ok(Self {
                    client,
                    $($field: Default::default()),*
                })
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self {
                    client: reqwest::Client::new(),
                    $($field: Default::default()),*
                }
            }
        }
    };
}

/// 为结构体添加基础查询处理方法的宏
#[macro_export]
macro_rules! query_processor_impl {
    ($struct_name:ident) => {
        impl $struct_name {
            /// 清理查询字符串
            pub fn clean_query(&self, query: &str) -> String {
                query.trim()
                    .chars()
                    .filter(|c| c.is_alphanumeric() || c.is_whitespace() || "-+\"".contains(*c))
                    .collect::<String>()
                    .split_whitespace()
                    .collect::<Vec<_>>()
                    .join(" ")
            }

            /// 优化页面大小
            pub fn optimize_page_size(&self, query: &mut $crate::derive::SearchQuery, default_size: usize) {
                if query.page_size == 0 {
                    query.page_size = default_size;
                } else if query.page_size > 100 {
                    query.page_size = 100;
                }
            }

            /// 设置默认语言
            pub fn set_default_language(&self, query: &mut $crate::derive::SearchQuery, lang: &str) {
                if query.language.is_none() {
                    query.language = Some(lang.to_string());
                }
            }

            /// 设置默认地区
            pub fn set_default_region(&self, query: &mut $crate::derive::SearchQuery, region: &str) {
                if query.region.is_none() {
                    query.region = Some(region.to_string());
                }
            }
        }
    };
}

/// 为结构体添加结果处理方法的宏
#[macro_export]
macro_rules! result_processor_impl {
    ($struct_name:ident) => {
        impl $struct_name {
            /// 去重结果
            pub fn deduplicate(&self, results: &mut Vec<$crate::derive::SearchResultItem>) {
                let mut seen = std::collections::HashSet::new();
                results.retain(|item| seen.insert(item.url.clone()));
            }

            /// 过滤低质量结果
            pub fn filter_low_quality(&self, results: &mut Vec<$crate::derive::SearchResultItem>, min_score: f64) {
                results.retain(|item| item.score >= min_score);
            }

            /// 按评分排序
            pub fn sort_by_score(&self, results: &mut Vec<$crate::derive::SearchResultItem>) {
                results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
            }

            /// 限制结果数量
            pub fn limit_results(&self, results: &mut Vec<$crate::derive::SearchResultItem>, limit: usize) {
                results.truncate(limit);
            }

            /// 格式化为 JSON
            pub fn to_json(&self, results: &[$crate::derive::SearchResultItem]) -> Result<String, Box<dyn std::error::Error>> {
                Ok(serde_json::to_string_pretty(results)?)
            }

            /// 格式化为 URL 参数
            pub fn to_url_params(&self, query: &$crate::derive::SearchQuery) -> String {
                let mut params = Vec::new();
                params.push(format!("q={}", urlencoding::encode(&query.query)));

                if let Some(lang) = &query.language {
                    params.push(format!("lang={}", urlencoding::encode(lang)));
                }

                if let Some(region) = &query.region {
                    params.push(format!("region={}", urlencoding::encode(region)));
                }

                params.push(format!("page={}", query.page));
                params.push(format!("page_size={}", query.page_size));

                params.join("&")
            }
        }
    };
}

/// 创建引擎信息的宏
#[macro_export]
macro_rules! engine_info {
    (
        name: $name:expr,
        engine_type: $engine_type:expr,
        website: $website:expr,
        categories: [$($category:expr),* $(,)?],
        max_page_size: $max_page_size:expr,
        supports_pagination: $supports_pagination:expr,
        supports_time_range: $supports_time_range:expr,
        supports_language_filter: $supports_language_filter:expr,
        supports_region_filter: $supports_region_filter:expr,
        supports_safe_search: $supports_safe_search:expr $(,)?
    ) => {
        $crate::derive::EngineInfo {
            name: $name.to_string(),
            engine_type: $crate::derive::EngineType::$engine_type,
            description: concat!("Search engine: ", $name).to_string(),
            website: Some($website.to_string()),
            status: $crate::derive::EngineStatus::Active,
            categories: vec![$($category.to_string()),*],
            capabilities: $crate::derive::EngineCapabilities {
                result_types: vec![$crate::derive::ResultType::Web],
                supported_params: vec![
                    "q".to_string(), "lang".to_string(), "region".to_string(),
                    "page".to_string(), "page_size".to_string(), "time_range".to_string(),
                    "safesearch".to_string()
                ],
                max_page_size: $max_page_size,
                supports_pagination: $supports_pagination,
                supports_time_range: $supports_time_range,
                supports_language_filter: $supports_language_filter,
                supports_region_filter: $supports_region_filter,
                supports_safe_search: $supports_safe_search,
                rate_limit: Some(60),
            },
            timeout: Some(30),
            version: Some("1.0.0".to_string()),
            last_checked: Some(chrono::Utc::now()),
        }
    };
}