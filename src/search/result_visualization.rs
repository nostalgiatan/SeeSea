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

//! 搜索结果可视化模块
//!
//! 负责将搜索结果以二维方式排列，以时间线为横轴，相关性为竖轴
//!
//! 实现高效高质量的结果排列算法，确保结果在时间线上均匀分布
//! 并根据相关性进行排序

use crate::derive::{SearchQuery, SearchResult, SearchResultItem};
use chrono::{DateTime, Datelike, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 时间粒度
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum TimeGranularity {
    /// 小时
    Hour,
    /// 天
    Day,
    /// 周
    Week,
    /// 月
    Month,
    /// 年
    Year,
}

impl TimeGranularity {
    /// 根据时间范围自动选择合适的时间粒度
    pub fn from_time_range(start: &DateTime<Utc>, end: &DateTime<Utc>) -> Self {
        let duration = *end - *start;
        match duration.num_days() {
            0 => TimeGranularity::Hour,        // 0天（不到24小时）使用小时粒度
            1 => TimeGranularity::Day,         // 1天使用天粒度
            2..=7 => TimeGranularity::Week,    // 2-7天使用周粒度
            8..=30 => TimeGranularity::Month,  // 8-30天使用月粒度
            31..=365 => TimeGranularity::Year, // 31-365天使用年粒度
            _ => TimeGranularity::Year,        // 超过1年使用年粒度
        }
    }

    /// 将时间转换为对应粒度的键
    pub fn to_key(&self, date: &DateTime<Utc>) -> String {
        match self {
            TimeGranularity::Hour => date.format("%Y-%m-%d %H:00").to_string(),
            TimeGranularity::Day => date.format("%Y-%m-%d").to_string(),
            TimeGranularity::Week => {
                // 计算周数
                let week_num = date.iso_week().week();
                format!("{}-W{:02}", date.year(), week_num)
            }
            TimeGranularity::Month => date.format("%Y-%m").to_string(),
            TimeGranularity::Year => date.format("%Y").to_string(),
        }
    }
}

impl Default for ResultVisualizer {
    /// 创建默认配置的可视化器
    fn default() -> Self {
        Self {
            config: TwoDimensionalConfig::default(),
        }
    }
}

/// 二维排列结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TwoDimensionalResult {
    /// 时间粒度
    pub time_granularity: TimeGranularity,
    /// 时间线结果映射
    pub timeline_results: HashMap<String, Vec<SearchResultItem>>,
    /// 所有结果（按最优顺序）
    pub ordered_results: Vec<SearchResultItem>,
    /// 统计信息
    pub stats: TwoDimensionalStats,
}

/// 二维排列统计信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TwoDimensionalStats {
    /// 总结果数
    pub total_results: usize,
    /// 时间范围（开始时间）
    pub start_time: Option<DateTime<Utc>>,
    /// 时间范围（结束时间）
    pub end_time: Option<DateTime<Utc>>,
    /// 时间区间数量
    pub time_intervals: usize,
    /// 平均每个时间区间的结果数
    pub avg_results_per_interval: f64,
}

/// 结果分布策略
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DistributionStrategy {
    /// 均匀分布
    Even,
    /// 优先显示最新结果
    RecentFirst,
    /// 优先显示最相关结果
    RelevantFirst,
    /// 混合策略（平衡时间和相关性）
    Hybrid,
}

/// 二维排列配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TwoDimensionalConfig {
    /// 时间粒度（可选，自动选择）
    pub time_granularity: Option<TimeGranularity>,
    /// 每个时间区间的最大结果数
    pub max_results_per_interval: usize,
    /// 是否按时间线顺序返回
    pub return_timeline_order: bool,
    /// 结果分布策略
    pub distribution_strategy: DistributionStrategy,
    /// 是否对结果进行聚类
    pub enable_clustering: bool,
    /// 聚类阈值（0.0-1.0，值越高聚类越严格）
    pub clustering_threshold: f64,
    /// 无时间结果的权重（0.0-1.0，值越高无时间结果排名越靠前）
    pub untimed_results_weight: f64,
    /// 相关性权重（0.0-1.0，值越高相关性影响越大）
    pub relevance_weight: f64,
    /// 时间权重（0.0-1.0，值越高时间影响越大）
    pub time_weight: f64,
}

impl Default for TwoDimensionalConfig {
    fn default() -> Self {
        Self {
            time_granularity: None,
            max_results_per_interval: 5,
            return_timeline_order: true,
            distribution_strategy: DistributionStrategy::Hybrid,
            enable_clustering: false,
            clustering_threshold: 0.7,
            untimed_results_weight: 0.3,
            relevance_weight: 0.7,
            time_weight: 0.3,
        }
    }
}

/// 搜索结果二维可视化器
#[derive(Debug, Clone)]
pub struct ResultVisualizer {
    /// 配置
    config: TwoDimensionalConfig,
}

impl ResultVisualizer {
    /// 创建新的可视化器
    pub fn new(config: TwoDimensionalConfig) -> Self {
        Self { config }
    }

    /// 将搜索结果转换为二维排列
    pub fn visualize(&self, results: &SearchResult, _query: &SearchQuery) -> TwoDimensionalResult {
        // 1. 直接使用results.items，避免不必要的克隆
        let items = &results.items;

        // 2. 预分配内存，减少内存分配次数
        let total_items = items.len();
        let mut timed_items = Vec::with_capacity(total_items);
        let mut untimed_items = Vec::with_capacity(total_items);

        // 3. 一次遍历完成时间和非时间结果的分离
        for item in items {
            if let Some(date) = item.published_date {
                timed_items.push((item, date));
            } else {
                untimed_items.push(item);
            }
        }

        // 4. 按相关性排序（仅用于无时间结果）
        let mut sorted_untimed_items: Vec<&SearchResultItem> = untimed_items;
        sorted_untimed_items.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // 5. 确定时间范围
        let (start_time, end_time) = if timed_items.is_empty() {
            (None, None)
        } else {
            let min_time = timed_items.iter().map(|(_, date)| *date).min().unwrap();
            let max_time = timed_items.iter().map(|(_, date)| *date).max().unwrap();
            (Some(min_time), Some(max_time))
        };

        // 6. 选择时间粒度
        let time_granularity = match (self.config.time_granularity, &start_time, &end_time) {
            (Some(granularity), _, _) => granularity,
            (None, Some(start), Some(end)) => TimeGranularity::from_time_range(start, end),
            _ => TimeGranularity::Day, // 默认使用天粒度
        };

        // 7. 将结果按时间粒度分组
        // 使用HashMap::with_capacity预分配空间，减少内存分配
        let mut timeline_map = HashMap::with_capacity(total_items / 2);

        // 处理有时间的结果
        for (item, date) in timed_items {
            let key = time_granularity.to_key(&date);
            // 使用or_insert_with和Vec::with_capacity预分配空间
            let entry = timeline_map
                .entry(key)
                .or_insert_with(|| Vec::with_capacity(5));
            entry.push(item.clone());
        }

        // 8. 对每个时间区间的结果按相关性排序
        // 同时限制每个时间区间的结果数
        for items in timeline_map.values_mut() {
            // 按相关性排序
            items.sort_by(|a, b| {
                b.score
                    .partial_cmp(&a.score)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });

            // 限制每个时间区间的结果数
            if items.len() > self.config.max_results_per_interval {
                items.truncate(self.config.max_results_per_interval);
            }
        }

        // 9. 处理无时间的结果
        // 预分配ordered_results的容量，减少内存分配
        let estimated_ordered_len = if timeline_map.is_empty() {
            sorted_untimed_items.len()
        } else {
            // 估计时间结果和无时间结果的总和
            let timeline_results_len: usize = timeline_map.values().map(|v| v.len()).sum();
            timeline_results_len + sorted_untimed_items.len()
        };
        let mut ordered_results = Vec::with_capacity(estimated_ordered_len);

        if self.config.return_timeline_order {
            // 按时间线顺序返回结果
            let mut sorted_keys: Vec<String> = timeline_map.keys().cloned().collect();
            sorted_keys.sort();

            for key in sorted_keys {
                if let Some(items) = timeline_map.get(&key) {
                    ordered_results.extend(items.clone());
                }
            }
        } else {
            // 按最优顺序返回结果
            ordered_results = self.optimize_result_order(&timeline_map);
        }

        // 10. 添加无时间的结果到末尾
        // 使用extend_from_slice避免逐个克隆
        ordered_results.reserve(sorted_untimed_items.len());
        for item in sorted_untimed_items {
            ordered_results.push(item.clone());
        }

        // 11. 计算统计信息
        let stats = TwoDimensionalStats {
            total_results: ordered_results.len(),
            start_time,
            end_time,
            time_intervals: timeline_map.len(),
            avg_results_per_interval: if timeline_map.is_empty() {
                0.0
            } else {
                ordered_results.len() as f64 / timeline_map.len() as f64
            },
        };

        TwoDimensionalResult {
            time_granularity,
            timeline_results: timeline_map,
            ordered_results,
            stats,
        }
    }

    /// 计算结果的综合得分
    fn calculate_composite_score(&self, item: &SearchResultItem, date: &DateTime<Utc>, now: &DateTime<Utc>) -> f64 {
        // 计算时间衰减因子（越新的结果得分越高）
        let time_diff = now.signed_duration_since(*date);
        let days_diff = time_diff.num_days() as f64;
        let time_score = if days_diff < 1.0 {
            1.0
        } else {
            1.0 / (1.0 + days_diff.log10())
        };

        // 综合得分 = 相关性得分 * 相关性权重 + 时间得分 * 时间权重
        (item.score * self.config.relevance_weight) + (time_score * self.config.time_weight)
    }

    /// 优化结果顺序，实现不同的分布策略
    fn optimize_result_order(
        &self,
        timeline_map: &HashMap<String, Vec<SearchResultItem>>,
    ) -> Vec<SearchResultItem> {
        // 按时间排序键
        let mut sorted_keys: Vec<String> = timeline_map.keys().cloned().collect();
        sorted_keys.sort();
        let now = Utc::now();

        match self.config.distribution_strategy {
            DistributionStrategy::Even => {
                // 均匀分布策略
                // 计算每个时间区间的结果数
                let mut interval_sizes: Vec<(String, usize, usize)> = Vec::with_capacity(sorted_keys.len());

                for (idx, key) in sorted_keys.iter().enumerate() {
                    let size = timeline_map.get(key).map(|v| v.len()).unwrap_or(0);
                    if size > 0 {
                        interval_sizes.push((key.clone(), size, idx));
                    }
                }

                if interval_sizes.is_empty() {
                    return Vec::new();
                }

                // 计算总结果数
                let total_results: usize = interval_sizes.iter().map(|(_, size, _)| *size).sum();

                // 实现轮询选择算法
                let mut indices: Vec<usize> = vec![0; interval_sizes.len()];
                let remaining: Vec<usize> = interval_sizes.iter().map(|(_, size, _)| *size).collect();

                // 预分配结果向量的容量
                let mut result = Vec::with_capacity(total_results);

                while result.len() < total_results {
                    for i in 0..interval_sizes.len() {
                        if indices[i] < remaining[i] {
                            let key = &interval_sizes[i].0;
                            if let Some(items) = timeline_map.get(key) {
                                if let Some(item) = items.get(indices[i]) {
                                    result.push(item.clone());
                                    indices[i] += 1;
                                }
                            }
                        }
                    }
                }

                result
            }
            DistributionStrategy::RecentFirst => {
                // 优先显示最新结果
                let mut all_items = Vec::new();
                
                // 收集所有结果并计算综合得分
                for key in sorted_keys.iter().rev() { // 从最新的时间区间开始
                    if let Some(items) = timeline_map.get(key) {
                        for item in items {
                            if let Some(date) = item.published_date {
                                let score = self.calculate_composite_score(item, &date, &now);
                                all_items.push((score, item.clone()));
                            } else {
                                // 无时间信息的结果得分较低
                                all_items.push((item.score * self.config.untimed_results_weight, item.clone()));
                            }
                        }
                    }
                }
                
                // 按得分排序
                all_items.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
                
                // 提取结果
                all_items.into_iter().map(|(_, item)| item).collect()
            }
            DistributionStrategy::RelevantFirst => {
                // 优先显示最相关结果
                let mut all_items = Vec::new();
                
                // 收集所有结果
                for key in &sorted_keys {
                    if let Some(items) = timeline_map.get(key) {
                        all_items.extend(items.clone());
                    }
                }
                
                // 按相关性排序
                all_items.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
                
                all_items
            }
            DistributionStrategy::Hybrid => {
                // 混合策略（平衡时间和相关性）
                let mut all_items = Vec::new();
                
                // 收集所有结果并计算综合得分
                for key in &sorted_keys {
                    if let Some(items) = timeline_map.get(key) {
                        for item in items {
                            if let Some(date) = item.published_date {
                                let score = self.calculate_composite_score(item, &date, &now);
                                all_items.push((score, item.clone()));
                            } else {
                                // 无时间信息的结果得分较低
                                all_items.push((item.score * self.config.untimed_results_weight, item.clone()));
                            }
                        }
                    }
                }
                
                // 按综合得分排序
                all_items.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
                
                // 提取结果
                all_items.into_iter().map(|(_, item)| item).collect()
            }
        }
    }

    /// 将二维结果转换为标准搜索结果
    pub fn to_search_result(
        &self,
        two_d_result: &TwoDimensionalResult,
        original_result: &SearchResult,
    ) -> SearchResult {
        SearchResult {
            engine_name: format!("{}-visualized", original_result.engine_name),
            total_results: Some(two_d_result.ordered_results.len()),
            elapsed_ms: original_result.elapsed_ms,
            items: two_d_result.ordered_results.clone(),
            pagination: original_result.pagination.clone(),
            suggestions: original_result.suggestions.clone(),
            metadata: {
                let mut metadata = original_result.metadata.clone();
                metadata.insert(
                    "visualization_time_granularity".to_string(),
                    format!("{:?}", two_d_result.time_granularity),
                );
                metadata.insert(
                    "visualization_time_intervals".to_string(),
                    two_d_result.stats.time_intervals.to_string(),
                );
                metadata
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::derive::{ResultType, SearchQuery};
    use chrono::{DateTime, Duration, Utc};
    use std::collections::HashMap;

    fn create_test_item(
        url: &str,
        title: &str,
        score: f64,
        published_date: Option<DateTime<Utc>>,
    ) -> SearchResultItem {
        SearchResultItem {
            title: title.to_string(),
            url: url.to_string(),
            content: "test content".to_string(),
            display_url: Some(url.to_string()),
            site_name: None,
            score,
            result_type: ResultType::Web,
            thumbnail: None,
            published_date,
            template: None,
            metadata: HashMap::new(),
        }
    }

    #[test]
    fn test_time_granularity_selection() {
        let now = Utc::now();
        let one_hour_ago = now - Duration::hours(1);
        let one_day_ago = now - Duration::days(1);
        let one_week_ago = now - Duration::weeks(1);
        let one_month_ago = now - Duration::days(30);
        let one_year_ago = now - Duration::days(365);

        assert_eq!(
            TimeGranularity::from_time_range(&one_hour_ago, &now),
            TimeGranularity::Hour
        );
        assert_eq!(
            TimeGranularity::from_time_range(&one_day_ago, &now),
            TimeGranularity::Day
        );
        assert_eq!(
            TimeGranularity::from_time_range(&one_week_ago, &now),
            TimeGranularity::Week
        );
        assert_eq!(
            TimeGranularity::from_time_range(&one_month_ago, &now),
            TimeGranularity::Month
        );
        assert_eq!(
            TimeGranularity::from_time_range(&one_year_ago, &now),
            TimeGranularity::Year
        );
    }

    #[test]
    fn test_two_dimensional_visualization() {
        let now = Utc::now();
        let one_hour_ago = now - Duration::hours(1);
        let two_hours_ago = now - Duration::hours(2);

        // 创建测试结果
        let search_result = SearchResult {
            engine_name: "test_engine".to_string(),
            total_results: Some(3),
            elapsed_ms: 100,
            items: vec![
                create_test_item("https://example.com/1", "Title 1", 0.9, Some(now)),
                create_test_item("https://example.com/2", "Title 2", 0.8, Some(one_hour_ago)),
                create_test_item("https://example.com/3", "Title 3", 0.7, Some(two_hours_ago)),
            ],
            pagination: None,
            suggestions: Vec::new(),
            metadata: HashMap::new(),
        };

        let query = SearchQuery {
            query: "test".to_string(),
            ..Default::default()
        };

        let visualizer = ResultVisualizer::default();
        let result = visualizer.visualize(&search_result, &query);

        // 验证结果
        assert_eq!(result.ordered_results.len(), 3);
        assert!(!result.timeline_results.is_empty());
        assert_eq!(result.stats.total_results, 3);
    }

    #[test]
    fn test_optimize_result_order() {
        let now = Utc::now();
        let one_day_ago = now - Duration::days(1);
        let two_days_ago = now - Duration::days(2);

        // 创建测试数据
        let mut timeline_map: HashMap<String, Vec<SearchResultItem>> = HashMap::new();

        timeline_map.insert(
            "2023-01-01".to_string(),
            vec![
                create_test_item("https://example.com/1", "Title 1", 0.9, Some(now)),
                create_test_item("https://example.com/2", "Title 2", 0.8, Some(now)),
            ],
        );

        timeline_map.insert(
            "2023-01-02".to_string(),
            vec![create_test_item(
                "https://example.com/3",
                "Title 3",
                0.7,
                Some(one_day_ago),
            )],
        );

        timeline_map.insert(
            "2023-01-03".to_string(),
            vec![
                create_test_item("https://example.com/4", "Title 4", 0.6, Some(two_days_ago)),
                create_test_item("https://example.com/5", "Title 5", 0.5, Some(two_days_ago)),
                create_test_item("https://example.com/6", "Title 6", 0.4, Some(two_days_ago)),
            ],
        );

        let visualizer = ResultVisualizer::default();
        let result = visualizer.optimize_result_order(&timeline_map);

        // 验证结果数量
        assert_eq!(result.len(), 6);

        // 验证结果顺序（应该是轮询选择）
        // 期望顺序：1, 3, 4, 2, 5, 6 或类似的轮询顺序
        assert!(
            result[0].title == "Title 1"
                || result[0].title == "Title 3"
                || result[0].title == "Title 4"
        );
    }

    #[test]
    fn test_visualize_no_time_info() {
        // 创建没有时间信息的测试结果
        let search_result = SearchResult {
            engine_name: "test_engine".to_string(),
            total_results: Some(3),
            elapsed_ms: 100,
            items: vec![
                create_test_item("https://example.com/1", "Title 1", 0.9, None),
                create_test_item("https://example.com/2", "Title 2", 0.8, None),
                create_test_item("https://example.com/3", "Title 3", 0.7, None),
            ],
            pagination: None,
            suggestions: Vec::new(),
            metadata: HashMap::new(),
        };

        let query = SearchQuery {
            query: "test".to_string(),
            ..Default::default()
        };

        let visualizer = ResultVisualizer::default();
        let result = visualizer.visualize(&search_result, &query);

        // 验证结果
        assert_eq!(result.ordered_results.len(), 3);
        assert_eq!(result.stats.total_results, 3);
    }

    #[test]
    fn test_visualize_single_result() {
        let now = Utc::now();

        // 创建只有一个结果的测试数据
        let search_result = SearchResult {
            engine_name: "test_engine".to_string(),
            total_results: Some(1),
            elapsed_ms: 100,
            items: vec![create_test_item(
                "https://example.com/1",
                "Title 1",
                0.9,
                Some(now),
            )],
            pagination: None,
            suggestions: Vec::new(),
            metadata: HashMap::new(),
        };

        let query = SearchQuery {
            query: "test".to_string(),
            ..Default::default()
        };

        let visualizer = ResultVisualizer::default();
        let result = visualizer.visualize(&search_result, &query);

        // 验证结果
        assert_eq!(result.ordered_results.len(), 1);
        assert_eq!(result.stats.total_results, 1);
        assert!(!result.timeline_results.is_empty());
    }

    #[test]
    fn test_to_search_result() {
        let now = Utc::now();
        let one_hour_ago = now - Duration::hours(1);

        // 创建测试数据
        let original_result = SearchResult {
            engine_name: "test_engine".to_string(),
            total_results: Some(2),
            elapsed_ms: 100,
            items: vec![
                create_test_item("https://example.com/1", "Title 1", 0.9, Some(now)),
                create_test_item("https://example.com/2", "Title 2", 0.8, Some(one_hour_ago)),
            ],
            pagination: None,
            suggestions: Vec::new(),
            metadata: HashMap::new(),
        };

        let query = SearchQuery {
            query: "test".to_string(),
            ..Default::default()
        };

        let visualizer = ResultVisualizer::default();
        let two_d_result = visualizer.visualize(&original_result, &query);
        let converted_result = visualizer.to_search_result(&two_d_result, &original_result);

        // 验证结果
        assert_eq!(converted_result.engine_name, "test_engine-visualized");
        assert_eq!(converted_result.items.len(), 2);
        assert_eq!(converted_result.total_results, Some(2));
    }

    #[test]
    fn test_visualize_large_results() {
        let now = Utc::now();

        // 创建大量测试结果
        let mut items = Vec::new();
        for i in 0..20 {
            let score = 1.0 - (i as f64 * 0.05);
            let date = now - Duration::hours(i as i64);
            items.push(create_test_item(
                &format!("https://example.com/{}", i),
                &format!("Title {}", i),
                score,
                Some(date),
            ));
        }

        let search_result = SearchResult {
            engine_name: "test_engine".to_string(),
            total_results: Some(20),
            elapsed_ms: 100,
            items,
            pagination: None,
            suggestions: Vec::new(),
            metadata: HashMap::new(),
        };

        let query = SearchQuery {
            query: "test".to_string(),
            ..Default::default()
        };

        let visualizer = ResultVisualizer::default();
        let result = visualizer.visualize(&search_result, &query);

        // 验证结果
        assert_eq!(result.ordered_results.len(), 20);
        assert_eq!(result.stats.total_results, 20);
        assert!(!result.timeline_results.is_empty());
    }
}
