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

//! 搜索结果时间排序模块
//!
//! 负责将搜索结果按时间线排序，优先显示最新结果。

use chrono::{DateTime, Utc};
use seesea_derive::{SearchResult, SearchResultItem};
use serde::{Deserialize, Serialize};
#[allow(unused_imports)]
use std::collections::HashMap;

/// 时间排序结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeSortedResult {
    /// 有时间的结果（按时间降序）
    pub timed_items: Vec<SearchResultItem>,
    /// 无时间的结果（按评分降序）
    pub untimed_items: Vec<SearchResultItem>,
    /// 统计信息
    pub stats: TimeSortStats,
}

/// 时间排序统计信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeSortStats {
    /// 总结果数
    pub total_results: usize,
    /// 有时间的结果数
    pub timed_count: usize,
    /// 无时间的结果数
    pub untimed_count: usize,
    /// 时间范围（开始时间）
    pub start_time: Option<DateTime<Utc>>,
    /// 时间范围（结束时间）
    pub end_time: Option<DateTime<Utc>>,
}

/// 时间排序器
pub struct TimeSorter {
    /// 是否优先显示最新结果
    pub prioritize_recent: bool,
}

impl Default for TimeSorter {
    fn default() -> Self {
        Self {
            prioritize_recent: true,
        }
    }
}

impl TimeSorter {
    pub fn new(prioritize_recent: bool) -> Self {
        Self { prioritize_recent }
    }

    pub fn sort_by_time(&self, items: Vec<SearchResultItem>) -> TimeSortedResult {
        let total_results = items.len();
        let mut timed_items = Vec::new();
        let mut untimed_items = Vec::new();

        for item in items {
            if let Some(date) = item.published_date {
                timed_items.push((item, date));
            } else {
                untimed_items.push(item);
            }
        }

        timed_items.sort_by(|a, b| {
            if self.prioritize_recent {
                b.1.cmp(&a.1)
            } else {
                a.1.cmp(&b.1)
            }
        });

        let timed_count = timed_items.len();
        let timed_items: Vec<SearchResultItem> =
            timed_items.into_iter().map(|(item, _)| item).collect();

        untimed_items.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let untimed_count = untimed_items.len();

        let (start_time, end_time) = if timed_items.is_empty() {
            (None, None)
        } else {
            let dates: Vec<_> = timed_items
                .iter()
                .filter_map(|item| item.published_date)
                .collect();
            let min_time = dates.iter().min().copied();
            let max_time = dates.iter().max().copied();
            (min_time, max_time)
        };

        TimeSortedResult {
            timed_items,
            untimed_items,
            stats: TimeSortStats {
                total_results,
                timed_count,
                untimed_count,
                start_time,
                end_time,
            },
        }
    }

    pub fn to_search_result(
        &self,
        time_sorted: &TimeSortedResult,
        original_result: &SearchResult,
    ) -> SearchResult {
        let mut ordered_results =
            Vec::with_capacity(time_sorted.timed_items.len() + time_sorted.untimed_items.len());

        ordered_results.extend(time_sorted.timed_items.clone());
        ordered_results.extend(time_sorted.untimed_items.clone());

        SearchResult {
            engine_name: format!("{}-time-sorted", original_result.engine_name),
            total_results: Some(ordered_results.len()),
            elapsed_ms: original_result.elapsed_ms,
            items: ordered_results,
            pagination: original_result.pagination.clone(),
            suggestions: original_result.suggestions.clone(),
            metadata: {
                let mut metadata = original_result.metadata.clone();
                metadata.insert("time_sort_enabled".to_string(), "true".to_string());
                metadata.insert(
                    "timed_count".to_string(),
                    time_sorted.stats.timed_count.to_string(),
                );
                metadata.insert(
                    "untimed_count".to_string(),
                    time_sorted.stats.untimed_count.to_string(),
                );
                if let Some(start) = time_sorted.stats.start_time {
                    metadata.insert("time_range_start".to_string(), start.to_rfc3339());
                }
                if let Some(end) = time_sorted.stats.end_time {
                    metadata.insert("time_range_end".to_string(), end.to_rfc3339());
                }
                metadata
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn create_test_item(title: &str, days_old: Option<i64>) -> SearchResultItem {
        SearchResultItem {
            title: title.to_string(),
            url: format!("https://example.com/{}", title),
            content: "Test content".to_string(),
            display_url: Some(format!("example.com/{}", title)),
            site_name: Some("Example".to_string()),
            score: 0.5,
            result_type: seesea_derive::ResultType::Web,
            thumbnail: None,
            published_date: days_old.map(|days| Utc::now() - chrono::Duration::days(days)),
            template: None,
            metadata: HashMap::new(),
        }
    }

    #[test]
    fn test_time_sorting() {
        let sorter = TimeSorter::default();

        let items = vec![
            create_test_item("Old", Some(100)),
            create_test_item("New", Some(1)),
            create_test_item("Medium", Some(30)),
            create_test_item("NoTime", None),
        ];

        let result = sorter.sort_by_time(items);

        assert_eq!(result.stats.total_results, 4);
        assert_eq!(result.stats.timed_count, 3);
        assert_eq!(result.stats.untimed_count, 1);

        assert_eq!(result.timed_items[0].title, "New");
        assert_eq!(result.timed_items[1].title, "Medium");
        assert_eq!(result.timed_items[2].title, "Old");
        assert_eq!(result.untimed_items[0].title, "NoTime");
    }

    #[test]
    fn test_oldest_first() {
        let sorter = TimeSorter::new(false);

        let items = vec![
            create_test_item("Old", Some(100)),
            create_test_item("New", Some(1)),
        ];

        let result = sorter.sort_by_time(items);

        assert_eq!(result.timed_items[0].title, "Old");
        assert_eq!(result.timed_items[1].title, "New");
    }

    #[test]
    fn test_all_untimed() {
        let sorter = TimeSorter::default();

        let items = vec![create_test_item("A", None), create_test_item("B", None)];

        let result = sorter.sort_by_time(items);

        assert_eq!(result.stats.timed_count, 0);
        assert_eq!(result.stats.untimed_count, 2);
        assert!(result.timed_items.is_empty());
    }

    #[test]
    fn test_all_timed() {
        let sorter = TimeSorter::default();

        let items = vec![
            create_test_item("A", Some(10)),
            create_test_item("B", Some(5)),
        ];

        let result = sorter.sort_by_time(items);

        assert_eq!(result.stats.timed_count, 2);
        assert_eq!(result.stats.untimed_count, 0);
        assert!(result.untimed_items.is_empty());
    }
}
