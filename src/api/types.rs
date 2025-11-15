//! API 类型定义模块
//!
//! 定义所有 API 相关的数据结构和类型

use serde::{Deserialize, Serialize};
use crate::derive::{SearchQuery, SearchResult};

/// API 搜索请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiSearchRequest {
    /// 搜索查询字符串
    pub query: String,
    
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
}

fn default_page() -> u32 {
    1
}

fn default_page_size() -> u32 {
    10
}

impl ApiSearchRequest {
    /// 转换为内部 SearchQuery
    pub fn to_search_query(&self) -> SearchQuery {
        let mut query = SearchQuery {
            query: self.query.clone(),
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
        
        query
    }
}

/// API 搜索响应
#[derive(Debug, Clone, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Serialize, Deserialize)]
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
}

/// API 错误响应
#[derive(Debug, Clone, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Serialize, Deserialize)]
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
}

impl ApiStatsResponse {
    /// 从搜索统计信息创建
    pub fn from_search_stats(stats: &crate::search::SearchStats) -> Self {
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
        assert_eq!(request.query, "test");
        assert_eq!(request.page, 1);
        assert_eq!(request.page_size, 10);
    }

    #[test]
    fn test_api_search_request_to_search_query() {
        let request = ApiSearchRequest {
            query: "rust programming".to_string(),
            page: 2,
            page_size: 20,
            language: Some("en".to_string()),
            region: Some("us".to_string()),
            safe_search: None,
            time_range: None,
            engines: None,
        };
        
        let query = request.to_search_query();
        assert_eq!(query.query, "rust programming");
        assert_eq!(query.page, 2);
        assert_eq!(query.page_size, 20);
        assert_eq!(query.language, Some("en".to_string()));
    }

    #[test]
    fn test_api_stats_response_cache_hit_rate() {
        use crate::search::SearchStats;
        
        let stats = SearchStats {
            total_searches: 100,
            cache_hits: 60,
            cache_misses: 40,
            engine_failures: 5,
            timeouts: 2,
        };
        
        let api_stats = ApiStatsResponse::from_search_stats(&stats);
        assert_eq!(api_stats.cache_hit_rate, 0.6);
    }
}
