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

//! 搜索结果聚合器模块
//!
//! 负责合并、去重、排序多个搜索引擎的结果

use std::collections::HashSet;
use crate::derive::{SearchResult, SearchResultItem, SearchQuery};
use super::scoring::{score_and_sort_results, ScoringWeights};
use super::standardization::{standardize_results, deduplicate_by_url};

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
    /// 评分权重（可选）
    scoring_weights: Option<ScoringWeights>,
}

impl SearchAggregator {
    /// 创建新的聚合器
    pub fn new(strategy: AggregationStrategy, sort_by: SortBy) -> Self {
        Self { 
            strategy, 
            sort_by,
            scoring_weights: None,
        }
    }

    /// 设置评分权重
    pub fn with_scoring(mut self, weights: ScoringWeights) -> Self {
        self.scoring_weights = Some(weights);
        self
    }

    /// 聚合多个搜索结果（使用智能评分）
    pub fn aggregate_with_scoring(
        &self, 
        mut results: Vec<SearchResult>,
        query: &SearchQuery,
    ) -> SearchResult {
        use std::collections::HashMap;
        
        if results.is_empty() {
            return SearchResult {
                engine_name: "aggregated".to_string(),
                total_results: Some(0),
                elapsed_ms: 0,
                items: Vec::new(),
                pagination: None,
                suggestions: Vec::new(),
                metadata: HashMap::new(),
            };
        }

        // 1. 标准化每个引擎的结果
        for result in &mut results {
            standardize_results(result);
        }

        // 2. 合并所有结果
        let mut all_items: Vec<SearchResultItem> = results
            .into_iter()
            .flat_map(|r| r.items.into_iter())
            .collect();

        // 3. 去重
        deduplicate_by_url(&mut all_items);

        // 4. 重新评分（基于查询）
        score_and_sort_results(&mut all_items, query, "aggregated", self.scoring_weights.clone());

        let total_results = all_items.len();

        SearchResult {
            engine_name: "aggregated".to_string(),
            total_results: Some(total_results),
            elapsed_ms: 0,
            items: all_items,
            pagination: None,
            suggestions: Vec::new(),
            metadata: HashMap::new(),
        }
    }

    /// 聚合多个搜索结果
    pub fn aggregate(&self, results: Vec<SearchResult>) -> SearchResult {
        use std::collections::HashMap;
        
        if results.is_empty() {
            return SearchResult {
                engine_name: "aggregated".to_string(),
                total_results: Some(0),
                elapsed_ms: 0,
                items: Vec::new(),
                pagination: None,
                suggestions: Vec::new(),
                metadata: HashMap::new(),
            };
        }

        let items = self.deduplicate_and_merge(results);
        let total_results = items.len();

        SearchResult {
            engine_name: "aggregated".to_string(),
            total_results: Some(total_results),
            elapsed_ms: 0,
            items,
            pagination: None,
            suggestions: Vec::new(),
            metadata: HashMap::new(),
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
    use crate::derive::{SearchResultItem, ResultType};

    fn create_test_item(url: &str, title: &str) -> SearchResultItem {
        SearchResultItem {
            title: title.to_string(),
            url: url.to_string(),
            content: "test".to_string(),
            display_url: None,
            site_name: None,
            score: 1.0,
            result_type: ResultType::Web,
            thumbnail: None,
            published_date: None,
            template: None,
            metadata: std::collections::HashMap::new(),
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
        use std::collections::HashMap;
        
        let agg = SearchAggregator::default();
        
        let result1 = SearchResult {
            engine_name: "engine1".to_string(),
            total_results: Some(2),
            elapsed_ms: 100,
            items: vec![
                create_test_item("https://example.com/1", "Title 1"),
                create_test_item("https://example.com/2", "Title 2"),
            ],
            pagination: None,
            suggestions: Vec::new(),
            metadata: HashMap::new(),
        };

        let result2 = SearchResult {
            engine_name: "engine2".to_string(),
            total_results: Some(2),
            elapsed_ms: 150,
            items: vec![
                create_test_item("https://example.com/1", "Title 1"), // 重复
                create_test_item("https://example.com/3", "Title 3"),
            ],
            pagination: None,
            suggestions: Vec::new(),
            metadata: HashMap::new(),
        };

        let aggregated = agg.aggregate(vec![result1, result2]);
        assert_eq!(aggregated.items.len(), 3); // 去重后只有3个
    }

    #[test]
    fn test_round_robin_strategy() {
        use std::collections::HashMap;
        
        let agg = SearchAggregator::new(AggregationStrategy::RoundRobin, SortBy::Relevance);
        
        let result1 = SearchResult {
            engine_name: "engine1".to_string(),
            total_results: Some(2),
            elapsed_ms: 100,
            items: vec![
                create_test_item("https://example.com/1", "A1"),
                create_test_item("https://example.com/2", "A2"),
            ],
            pagination: None,
            suggestions: Vec::new(),
            metadata: HashMap::new(),
        };

        let result2 = SearchResult {
            engine_name: "engine2".to_string(),
            total_results: Some(2),
            elapsed_ms: 150,
            items: vec![
                create_test_item("https://example.com/3", "B1"),
                create_test_item("https://example.com/4", "B2"),
            ],
            pagination: None,
            suggestions: Vec::new(),
            metadata: HashMap::new(),
        };

        let aggregated = agg.aggregate(vec![result1, result2]);
        assert_eq!(aggregated.items.len(), 4);
        // 轮询顺序：A1, B1, A2, B2
        assert_eq!(aggregated.items[0].title, "A1");
        assert_eq!(aggregated.items[1].title, "B1");
    }
}
