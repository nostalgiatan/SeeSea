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

//! # 搜索类型定义
//!
//! 定义搜索模块使用的核心类型和数据结构

use crate::derive::{SearchQuery, SearchResult};
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
}

impl Default for SearchRequest {
    fn default() -> Self {
        Self {
            query: SearchQuery::default(),
            engines: Vec::new(),
            timeout: Some(Duration::from_secs(30)),
            max_results: Some(100),
            force: false,
            cache_timeline: Some(3600), // 默认1小时刷新
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

/// 搜索配置
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
            default_timeout: Duration::from_secs(60),  // 增加到60秒
            enable_cache: true,
            max_concurrent_engines: 20,          // 拉满并发数
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
