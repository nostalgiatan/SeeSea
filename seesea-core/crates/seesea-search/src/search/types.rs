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

//! # 搜索类型定义
//!
//! 定义搜索模块使用的核心类型和数据结构

use seesea_config::SearchConfig as CentralizedSearchConfig;
use seesea_derive::{SearchQuery, SearchResult};
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// 搜索请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchRequest {
    /// 搜索查询
    pub query: SearchQuery,
    /// 指定使用的引擎列表（为空则使用所有引擎）
    pub engines: Vec<String>,
    /// 超时时间
    pub timeout: Option<Duration>,
    /// 最大结果数
    pub max_results: Option<usize>,
    /// 强制搜索（绕过缓存）
    pub force: bool,
    /// 缓存刷新时间线（秒），超过此时间强制刷新
    pub cache_timeline: Option<u64>,
    /// 是否包含深网搜索（默认false）
    pub include_deepweb: bool,
}

impl Default for SearchRequest {
    fn default() -> Self {
        Self {
            query: SearchQuery::default(),
            engines: Vec::new(),
            timeout: Some(Duration::from_secs(30)),
            max_results: Some(100),
            force: false,
            cache_timeline: Some(3600),
            include_deepweb: false,
        }
    }
}

/// 搜索响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResponse {
    /// 搜索结果
    pub results: Vec<SearchResult>,
    /// 使用的引擎列表
    pub engines_used: Vec<String>,
    /// 总结果数
    pub total_count: usize,
    /// 查询时间（毫秒）
    pub query_time_ms: u64,
    /// 原始查询
    pub query: SearchQuery,
    /// 是否从缓存获取
    pub cached: bool,
}

/// 搜索配置（兼容层，使用集中配置）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchConfig {
    /// 默认超时时间
    pub default_timeout: Duration,
    /// 启用缓存
    pub enable_cache: bool,
    /// 最大并发引擎数
    pub max_concurrent_engines: usize,
}

impl Default for SearchConfig {
    fn default() -> Self {
        Self {
            default_timeout: Duration::from_secs(60),
            enable_cache: true,
            max_concurrent_engines: 20,
        }
    }
}

impl From<CentralizedSearchConfig> for SearchConfig {
    fn from(config: CentralizedSearchConfig) -> Self {
        Self {
            default_timeout: Duration::from_secs(config.search_timeout),
            enable_cache: true,
            max_concurrent_engines: config.max_concurrent_engines,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_request_default() {
        let req = SearchRequest::default();
        assert!(req.engines.is_empty());
        assert_eq!(req.timeout, Some(Duration::from_secs(30)));
    }

    #[test]
    fn test_search_config_default() {
        let config = SearchConfig::default();
        assert_eq!(config.default_timeout, Duration::from_secs(60));
        assert!(config.enable_cache);
    }

    #[test]
    fn test_search_response_creation() {
        let response = SearchResponse {
            results: Vec::new(),
            engines_used: vec!["google".to_string()],
            total_count: 0,
            query_time_ms: 100,
            query: SearchQuery::default(),
            cached: false,
        };
        assert_eq!(response.engines_used.len(), 1);
    }
}
