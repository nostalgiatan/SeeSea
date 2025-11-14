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
}

impl Default for SearchRequest {
    fn default() -> Self {
        Self {
            query: SearchQuery::default(),
            engines: Vec::new(),
            timeout: Some(Duration::from_secs(30)),
            max_results: Some(100),
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
            default_timeout: Duration::from_secs(30),
            enable_cache: true,
            max_concurrent_engines: 10,
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
        assert_eq!(config.default_timeout, Duration::from_secs(30));
        assert!(config.enable_cache);
    }

    #[test]
    fn test_search_response_creation() {
        let response = SearchResponse {
            results: Vec::new(),
            engines_used: vec!["google".to_string()],
            total_count: 0,
            query_time_ms: 100,
        };
        assert_eq!(response.engines_used.len(), 1);
    }
}
