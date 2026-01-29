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

//! API 类型定义模块
//!
//! 定义所有 API 相关的数据结构和类型

use seesea_derive::SearchQuery;
use seesea_search::search::EngineListConfig;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// API 搜索请求
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ApiSearchRequest {
    /// 搜索查询字符串（主要字段）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub query: Option<String>,

    /// 搜索查询字符串（短参数名，等价于 query）
    #[serde(alias = "q", skip_serializing_if = "Option::is_none")]
    pub _q: Option<String>,

    /// 搜索引擎类型
    /// 可选值: "general", "image", "video", "news", "social"
    /// 如果不指定，默认为 "general"（文本搜索）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub engine_type: Option<String>,

    /// 引擎数量（可选）- 根据引擎延迟选择低延迟的引擎
    /// 如果不提供，默认使用全部引擎
    #[serde(alias = "n", skip_serializing_if = "Option::is_none")]
    pub engine_count: Option<u32>,

    /// 页码（从1开始）
    #[serde(default = "default_page")]
    pub page: u32,

    /// 每页结果数
    #[serde(default = "default_page_size")]
    pub page_size: u32,

    /// 语言过滤（可选）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,

    /// 地区过滤（可选）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub region: Option<String>,

    /// 安全搜索级别（可选）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub safe_search: Option<String>,

    /// 时间范围（可选）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time_range: Option<String>,

    /// 指定搜索引擎（可选，逗号分隔）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub engines: Option<String>,

    /// 是否包含深网搜索（默认false）
    #[serde(default = "default_include_deepweb")]
    pub include_deepweb: bool,
}

fn default_page() -> u32 {
    1
}

fn default_page_size() -> u32 {
    10
}

fn default_include_deepweb() -> bool {
    false
}

impl ApiSearchRequest {
    /// 获取查询字符串（支持 q 和 query 参数）
    pub fn get_query(&self) -> Result<String, String> {
        match (self.query.as_ref(), self._q.as_ref()) {
            (Some(query), _) => Ok(query.clone()),
            (_, Some(q)) => Ok(q.clone()),
            (None, None) => Err("查询参数 'query' 或 'q' 是必需的".to_string()),
        }
    }

    /// 转换为内部 SearchQuery
    pub fn to_search_query(&self) -> Result<SearchQuery, String> {
        let query_text = self.get_query()?;

        let mut query = SearchQuery {
            query: query_text,
            page: self.page as usize,
            page_size: self.page_size as usize,
            ..Default::default()
        };

        if let Some(ref lang) = self.language {
            query.language = Some(lang.clone());
        }

        if let Some(ref region) = self.region {
            query.region = Some(region.clone());
        }

        Ok(query)
    }

    /// 获取搜索引擎列表
    ///
    /// 根据以下优先级返回引擎列表:
    /// 1. 如果指定了 engines 参数，使用自定义引擎列表
    /// 2. 如果指定了 engine_type 参数，根据引擎类型选择对应的引擎
    /// 3. 如果指定了 engine_count 参数，根据引擎延迟选择低延迟引擎
    /// 4. 根据 include_deepweb 参数选择引擎列表
    pub fn get_engines(&self) -> Vec<String> {
        if let Some(ref engines_str) = self.engines {
            // 自定义引擎列表
            engines_str
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect()
        } else {
            // 使用统一的引擎配置模块获取引擎
            let config = EngineListConfig::default();

            // 根据引擎类型获取引擎列表
            let base_engines = if let Some(ref engine_type) = self.engine_type {
                config.get_engines_for_type(engine_type)
            } else if self.include_deepweb {
                config.global_engines
            } else {
                config.get_engines_for_type("general") // 默认使用文本引擎
            };

            if let Some(count) = self.engine_count {
                // 根据引擎数量限制引擎列表
                let count = count as usize;
                if count > 0 && count < base_engines.len() {
                    base_engines.into_iter().take(count).collect()
                } else {
                    base_engines
                }
            } else {
                // 默认使用基础引擎列表
                base_engines
            }
        }
    }
}

/// API 搜索响应
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ApiSearchResponse {
    /// 查询字符串
    pub query: String,

    /// 搜索结果列表
    pub results: Vec<ApiSearchResultItem>,

    /// 结果总数
    pub total_count: usize,

    /// 当前页码
    pub page: u32,

    /// 每页结果数
    pub page_size: u32,

    /// 使用的搜索引擎列表
    pub engines_used: Vec<String>,

    /// 查询耗时（毫秒）
    pub query_time_ms: u64,

    /// 是否来自缓存
    pub cached: bool,
}

/// API 搜索结果项
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ApiSearchResultItem {
    /// 结果标题
    pub title: String,

    /// 结果URL
    pub url: String,

    /// 结果描述/摘要
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// 来源引擎
    pub engine: String,

    /// 评分（用于排序）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub score: Option<f64>,

    /// 发布时间
    #[serde(skip_serializing_if = "Option::is_none")]
    pub published_date: Option<String>,
}

/// API 错误响应
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ApiErrorResponse {
    /// 错误代码
    pub code: String,

    /// 错误消息
    pub message: String,

    /// 详细错误信息（可选）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,
}

/// API 健康检查响应
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ApiHealthResponse {
    /// 服务状态
    pub status: String,

    /// 版本号
    pub version: String,

    /// 可用引擎数量
    pub available_engines: usize,

    /// 总引擎数量
    pub total_engines: usize,
}

/// API 引擎信息
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ApiEngineInfo {
    /// 引擎名称
    pub name: String,

    /// 引擎描述
    pub description: String,

    /// 引擎类型
    pub engine_type: String,

    /// 是否可用
    pub enabled: bool,

    /// 支持的功能
    pub capabilities: Vec<String>,
}

/// API 统计信息响应
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ApiStatsResponse {
    /// 总搜索次数
    pub total_searches: u64,

    /// 缓存命中次数
    pub cache_hits: u64,

    /// 缓存未命中次数
    pub cache_misses: u64,

    /// 缓存命中率
    pub cache_hit_rate: f64,

    /// 引擎失败次数
    pub engine_failures: u64,

    /// 超时次数
    pub timeouts: u64,

    /// 搜索历史数据（最近24小时）
    pub search_history: Vec<SearchHistoryEntry>,
}

/// 搜索历史记录条目
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SearchHistoryEntry {
    /// 时间戳（小时）
    pub hour: u32,
    /// 该小时的搜索次数
    pub count: u64,
}

impl ApiStatsResponse {
    /// 从搜索统计信息创建
    pub fn from_search_stats(stats: &seesea_search::search::on::SearchStatsResult) -> Self {
        let total = stats.cache_hits + stats.cache_misses;
        let hit_rate = if total > 0 {
            stats.cache_hits as f64 / total as f64
        } else {
            0.0
        };

        Self {
            total_searches: stats.total_searches,
            cache_hits: stats.cache_hits,
            cache_misses: stats.cache_misses,
            cache_hit_rate: hit_rate,
            engine_failures: stats.engine_failures,
            timeouts: stats.timeouts,
            search_history: stats
                .search_history
                .iter()
                .map(|h| SearchHistoryEntry {
                    hour: h.hour,
                    count: h.count,
                })
                .collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_search_request_defaults() {
        let json = r#"{"query": "test"}"#;
        let request: ApiSearchRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.query, Some("test".to_string()));
        assert_eq!(request.page, 1);
        assert_eq!(request.page_size, 10);
        assert_eq!(request.engine_count, None);
    }

    #[test]
    fn test_api_search_request_q_parameter() {
        let json = r#"{"q": "test"}"#;
        let request: ApiSearchRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.get_query().unwrap(), "test");
        assert_eq!(request.page, 1);
        assert_eq!(request.page_size, 10);
    }

    #[test]
    fn test_api_search_request_engine_count() {
        let json = r#"{"q": "test", "engine_count": 3}"#;
        let request: ApiSearchRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.get_query().unwrap(), "test");
        assert_eq!(request.engine_count, Some(3));

        let engines = request.get_engines();
        assert_eq!(engines.len(), 3);
    }

    #[test]
    fn test_api_search_request_engine_count_short() {
        let json = r#"{"q": "test", "n": 5}"#;
        let request: ApiSearchRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.get_query().unwrap(), "test");
        assert_eq!(request.engine_count, Some(5));

        let engines = request.get_engines();
        assert_eq!(engines.len(), 5);
    }

    #[test]
    fn test_api_search_request_default_all_engines() {
        let json = r#"{"q": "test"}"#;
        let request: ApiSearchRequest = serde_json::from_str(json).unwrap();

        let engines = request.get_engines();
        // 默认情况下应返回快速引擎
        let config = EngineListConfig::default();
        assert_eq!(engines.len(), config.fast_engines.len());
        assert_eq!(engines, config.fast_engines);
    }

    #[test]
    fn test_api_search_request_to_search_query() {
        let request = ApiSearchRequest {
            query: Some("rust programming".to_string()),
            _q: None,
            engine_count: None,
            page: 2,
            page_size: 20,
            language: Some("en".to_string()),
            region: Some("us".to_string()),
            safe_search: None,
            time_range: None,
            engines: None,
            include_deepweb: false,
        };

        let query = request.to_search_query().unwrap();
        assert_eq!(query.query, "rust programming");
        assert_eq!(query.page, 2);
        assert_eq!(query.page_size, 20);
        assert_eq!(query.language, Some("en".to_string()));
    }

    #[test]
    fn test_api_stats_response_cache_hit_rate() {
        use seesea_search::search::SearchStatsResult;

        let stats = SearchStatsResult {
            total_searches: 100,
            cache_hits: 60,
            cache_misses: 40,
            engine_failures: 5,
            timeouts: 2,
        };

        let api_stats = ApiStatsResponse::from_search_stats(&stats);
        assert_eq!(api_stats.cache_hit_rate, 0.6);
    }

    #[test]
    fn test_api_search_request_include_deepweb() {
        // 测试默认情况（不包含深网搜索）
        let json = r#"{"q": "test"}"#;
        let request: ApiSearchRequest = serde_json::from_str(json).unwrap();
        assert!(!request.include_deepweb);

        let config = EngineListConfig::default();
        let engines = request.get_engines();
        assert_eq!(engines, config.fast_engines);

        // 测试包含深网搜索
        let json = r#"{"q": "test", "include_deepweb": true}"#;
        let request: ApiSearchRequest = serde_json::from_str(json).unwrap();
        assert!(request.include_deepweb);

        let engines = request.get_engines();
        assert_eq!(engines, config.global_engines);
    }
}
