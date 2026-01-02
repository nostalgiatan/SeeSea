// Copyright (C) 2025 nostalgiatan
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

//! 搜索引擎便利宏定义
//!
//! 提供简化搜索引擎实现的宏

/// 定义引擎元数据的宏
///
/// 用于声明引擎的基本元数据信息，包括名称、分类、分页支持、时间范围支持和安全搜索支持等。
///
/// # 示例
///
/// ```ignore
/// engine_metadata! {
///     name: "MyEngine",
///     categories: ["general"],
///     paging: true,
///     time_range_support: false,
///     safesearch: true,
///     about: {
///         website: "https://example.com",
///         wikidata_id: "Q12345",
///         use_official_api: false,
///         require_api_key: false,
///         results: "HTML"
///     }
/// }
/// ```
#[macro_export]
macro_rules! engine_metadata {
    (
        name: $name:expr,
        categories: [$($category:expr),* $(,)?],
        paging: $paging:expr,
        time_range_support: $time_range:expr,
        safesearch: $safesearch:expr
        $(, about: {
            website: $website:expr,
            wikidata_id: $wikidata:expr,
            use_official_api: $use_api:expr,
            require_api_key: $require_key:expr,
            results: $results:expr
        })?
        $(,)?
    ) => {
        $crate::EngineMetadata {
            name: $name.to_string(),
            categories: vec![$($category.to_string()),*],
            paging: $paging,
            time_range_support: $time_range,
            safesearch: $safesearch,
            about: None $(.or_else(|| Some($crate::AboutInfo {
                website: Some($website.to_string()),
                wikidata_id: Some($wikidata.to_string()),
                official_api_documentation: None,
                use_official_api: $use_api,
                require_api_key: $require_key,
                results: $results.to_string(),
            })))?,
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
                query
                    .trim()
                    .chars()
                    .filter(|c| c.is_alphanumeric() || c.is_whitespace() || "-+\"".contains(*c))
                    .collect::<String>()
                    .split_whitespace()
                    .collect::<Vec<_>>()
                    .join(" ")
            }

            /// 优化页面大小
            pub fn optimize_page_size(&self, query: &mut $crate::SearchQuery, default_size: usize) {
                if query.page_size == 0 {
                    query.page_size = default_size;
                } else if query.page_size > 100 {
                    query.page_size = 100;
                }
            }

            /// 设置默认语言
            pub fn set_default_language(&self, query: &mut $crate::SearchQuery, lang: &str) {
                if query.language.is_none() {
                    query.language = Some(lang.to_string());
                }
            }

            /// 设置默认地区
            pub fn set_default_region(&self, query: &mut $crate::SearchQuery, region: &str) {
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
            pub fn deduplicate(&self, results: &mut Vec<$crate::SearchResultItem>) {
                let mut seen = std::collections::HashSet::new();
                results.retain(|item| seen.insert(item.url.clone()));
            }

            /// 过滤低质量结果
            pub fn filter_low_quality(
                &self,
                results: &mut Vec<$crate::SearchResultItem>,
                min_score: f64,
            ) {
                results.retain(|item| item.score >= min_score);
            }

            /// 按评分排序
            pub fn sort_by_score(&self, results: &mut Vec<$crate::SearchResultItem>) {
                results.sort_by(|a, b| {
                    b.score
                        .partial_cmp(&a.score)
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
            }

            /// 限制结果数量
            pub fn limit_results(&self, results: &mut Vec<$crate::SearchResultItem>, limit: usize) {
                results.truncate(limit);
            }

            /// 格式化为 JSON
            pub fn to_json(
                &self,
                results: &[$crate::SearchResultItem],
            ) -> Result<String, Box<dyn std::error::Error>> {
                Ok(serde_json::to_string_pretty(results)?)
            }

            /// 格式化为 URL 参数
            pub fn to_url_params(&self, query: &$crate::SearchQuery) -> String {
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
        supports_safe_search: $supports_safe_search:expr,
        shortcut: $shortcut:expr $(,)?
    ) => {
        $crate::EngineInfo {
            name: $name.to_string(),
            engine_type: $engine_type,
            description: concat!("Search engine: ", $name).to_string(),
            status: $crate::EngineStatus::Active,
            categories: vec![$($category.to_string()),*],
            capabilities: $crate::EngineCapabilities {
                result_types: vec![$crate::ResultType::Web],
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
            about: $crate::AboutInfo {
                website: Some($website.to_string()),
                wikidata_id: None,
                official_api_documentation: None,
                use_official_api: false,
                require_api_key: false,
                results: "HTML".to_string(),
            },
            shortcut: $shortcut,
            timeout: Some(30),
            version: Some("1.0.0".to_string()),
            last_checked: Some(chrono::Utc::now()),
            disabled: false,
            inactive: false,
            using_tor_proxy: false,
            display_error_messages: true,
            tokens: Vec::new(),
            max_page: 0,
        }
    };
}
