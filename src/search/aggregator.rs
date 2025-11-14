//! 搜索结果聚合器模块
//!
//! 负责合并、去重、排序多个搜索引擎的结果

use std::collections::HashSet;
use crate::derive::{SearchResult, SearchResultItem};

/// 聚合策略
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AggregationStrategy {
    /// 按相关性合并
    Merged,
    /// 轮询各引擎
    RoundRobin,
    /// 加权排序
    Ranked,
    /// 自定义
    Custom,
}

/// 排序方式
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortBy {
    /// 相关性
    Relevance,
    /// 时间
    Time,
    /// 来源
    Source,
}

/// 搜索聚合器
pub struct SearchAggregator {
    /// 聚合策略
    strategy: AggregationStrategy,
    /// 排序方式
    sort_by: SortBy,
}

impl SearchAggregator {
    /// 创建新的聚合器
    pub fn new(strategy: AggregationStrategy, sort_by: SortBy) -> Self {
        Self { strategy, sort_by }
    }

    /// 聚合多个搜索结果
    pub fn aggregate(&self, results: Vec<SearchResult>) -> SearchResult {
        if results.is_empty() {
            return SearchResult {
                query: crate::derive::SearchQuery::default(),
                items: Vec::new(),
                engine: "aggregated".to_string(),
                total_results: 0,
                page: 1,
                has_next_page: false,
            };
        }

        let query = results[0].query.clone();
        let items = self.deduplicate_and_merge(results);

        SearchResult {
            query,
            items,
            engine: "aggregated".to_string(),
            total_results: 0,
            page: 1,
            has_next_page: false,
        }
    }

    /// 去重并合并结果
    fn deduplicate_and_merge(&self, results: Vec<SearchResult>) -> Vec<SearchResultItem> {
        let mut seen_urls = HashSet::new();
        let mut merged_items = Vec::new();

        match self.strategy {
            AggregationStrategy::Merged => {
                for result in results {
                    for item in result.items {
                        if seen_urls.insert(item.url.clone()) {
                            merged_items.push(item);
                        }
                    }
                }
                self.sort_items(&mut merged_items);
            }
            AggregationStrategy::RoundRobin => {
                let max_len = results.iter().map(|r| r.items.len()).max().unwrap_or(0);
                for i in 0..max_len {
                    for result in &results {
                        if let Some(item) = result.items.get(i) {
                            if seen_urls.insert(item.url.clone()) {
                                merged_items.push(item.clone());
                            }
                        }
                    }
                }
            }
            AggregationStrategy::Ranked => {
                for result in results {
                    for item in result.items {
                        if seen_urls.insert(item.url.clone()) {
                            merged_items.push(item);
                        }
                    }
                }
                self.sort_items(&mut merged_items);
            }
            AggregationStrategy::Custom => {
                for result in results {
                    for item in result.items {
                        if seen_urls.insert(item.url.clone()) {
                            merged_items.push(item);
                        }
                    }
                }
            }
        }

        merged_items
    }

    /// 排序结果项
    fn sort_items(&self, items: &mut [SearchResultItem]) {
        match self.sort_by {
            SortBy::Relevance => {
                // 默认顺序即为相关性顺序
            }
            SortBy::Time => {
                // 按时间排序（如果有的话）
            }
            SortBy::Source => {
                items.sort_by(|a, b| a.url.cmp(&b.url));
            }
        }
    }
}

impl Default for SearchAggregator {
    fn default() -> Self {
        Self::new(AggregationStrategy::Merged, SortBy::Relevance)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::derive::{SearchQuery, SearchResultItem, ResultType};

    fn create_test_item(url: &str, title: &str) -> SearchResultItem {
        SearchResultItem {
            title: title.to_string(),
            url: url.to_string(),
            description: Some("test".to_string()),
            result_type: ResultType::Web,
            score: 1.0,
            metadata: std::collections::HashMap::new(),
            template: None,
        }
    }

    #[test]
    fn test_aggregator_creation() {
        let agg = SearchAggregator::default();
        assert_eq!(agg.strategy, AggregationStrategy::Merged);
        assert_eq!(agg.sort_by, SortBy::Relevance);
    }

    #[test]
    fn test_empty_aggregation() {
        let agg = SearchAggregator::default();
        let result = agg.aggregate(vec![]);
        assert_eq!(result.items.len(), 0);
    }

    #[test]
    fn test_deduplication() {
        let agg = SearchAggregator::default();
        let query = SearchQuery::default();
        
        let result1 = SearchResult {
            query: query.clone(),
            items: vec![
                create_test_item("https://example.com/1", "Title 1"),
                create_test_item("https://example.com/2", "Title 2"),
            ],
            engine: "engine1".to_string(),
            total_results: 2,
            page: 1,
            has_next_page: false,
        };

        let result2 = SearchResult {
            query: query.clone(),
            items: vec![
                create_test_item("https://example.com/1", "Title 1"), // 重复
                create_test_item("https://example.com/3", "Title 3"),
            ],
            engine: "engine2".to_string(),
            total_results: 2,
            page: 1,
            has_next_page: false,
        };

        let aggregated = agg.aggregate(vec![result1, result2]);
        assert_eq!(aggregated.items.len(), 3); // 去重后只有3个
    }

    #[test]
    fn test_round_robin_strategy() {
        let agg = SearchAggregator::new(AggregationStrategy::RoundRobin, SortBy::Relevance);
        let query = SearchQuery::default();
        
        let result1 = SearchResult {
            query: query.clone(),
            items: vec![
                create_test_item("https://example.com/1", "A1"),
                create_test_item("https://example.com/2", "A2"),
            ],
            engine: "engine1".to_string(),
            total_results: 2,
            page: 1,
            has_next_page: false,
        };

        let result2 = SearchResult {
            query: query.clone(),
            items: vec![
                create_test_item("https://example.com/3", "B1"),
                create_test_item("https://example.com/4", "B2"),
            ],
            engine: "engine2".to_string(),
            total_results: 2,
            page: 1,
            has_next_page: false,
        };

        let aggregated = agg.aggregate(vec![result1, result2]);
        assert_eq!(aggregated.items.len(), 4);
        // 轮询顺序：A1, B1, A2, B2
        assert_eq!(aggregated.items[0].title, "A1");
        assert_eq!(aggregated.items[1].title, "B1");
    }
}
