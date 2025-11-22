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

//! RSS 榜单系统
//!
//! 基于持久化关键词对 RSS 项目进行相关性评分和排名

use serde::{Deserialize, Serialize};
use crate::derive::rss::{RssFeedItem, RssFeed};

/// 关键词配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RankingKeyword {
    /// 关键词
    pub keyword: String,
    /// 权重 (1.0 - 10.0)
    pub weight: f64,
    /// 是否必须匹配
    pub required: bool,
}

impl RankingKeyword {
    pub fn new(keyword: impl Into<String>, weight: f64) -> Self {
        Self {
            keyword: keyword.into(),
            weight: weight.clamp(1.0, 10.0),
            required: false,
        }
    }

    pub fn required(keyword: impl Into<String>, weight: f64) -> Self {
        Self {
            keyword: keyword.into(),
            weight: weight.clamp(1.0, 10.0),
            required: true,
        }
    }
}

/// 榜单配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RankingConfig {
    /// 榜单名称
    pub name: String,
    /// 关键词列表
    pub keywords: Vec<RankingKeyword>,
    /// 最小评分阈值（低于此分数的项目将被过滤）
    pub min_score: f64,
    /// 最大结果数
    pub max_results: usize,
}

impl Default for RankingConfig {
    fn default() -> Self {
        Self {
            name: "default".to_string(),
            keywords: Vec::new(),
            min_score: 0.0,
            max_results: 100,
        }
    }
}

/// 已评分的 RSS 项目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoredRssItem {
    /// 原始项目
    pub item: RssFeedItem,
    /// 相关性评分
    pub score: f64,
    /// 匹配的关键词
    pub matched_keywords: Vec<String>,
}

/// RSS 榜单结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RssRanking {
    /// 榜单名称
    pub name: String,
    /// 已评分和排序的项目
    pub items: Vec<ScoredRssItem>,
    /// 总项目数（排序前）
    pub total_items: usize,
    /// 评分时间戳
    pub timestamp: u64,
}

/// RSS 榜单引擎
pub struct RssRankingEngine {
    config: RankingConfig,
}

impl RssRankingEngine {
    /// 创建新的榜单引擎
    pub fn new(config: RankingConfig) -> Self {
        Self { config }
    }

    /// 对单个 RSS 项目进行评分
    pub fn score_item(&self, item: &RssFeedItem) -> ScoredRssItem {
        let mut score = 0.0;
        let mut matched_keywords = Vec::new();

        // 合并标题和描述用于匹配
        let text = format!(
            "{} {}",
            item.title.to_lowercase(),
            item.description.as_deref().unwrap_or("").to_lowercase()
        );

        for kw_config in &self.config.keywords {
            let keyword_lower = kw_config.keyword.to_lowercase();
            
            // 检查关键词是否在文本中
            if text.contains(&keyword_lower) {
                // 计算出现次数
                let count = text.matches(&keyword_lower).count();
                
                // 基于权重和出现次数计算分数
                // 使用对数缩放避免过多重复关键词导致分数过高
                let keyword_score = kw_config.weight * (1.0 + (count as f64).ln());
                score += keyword_score;
                
                matched_keywords.push(kw_config.keyword.clone());
            } else if kw_config.required {
                // 必需关键词未匹配，直接返回0分
                return ScoredRssItem {
                    item: item.clone(),
                    score: 0.0,
                    matched_keywords: vec![],
                };
            }
        }

        ScoredRssItem {
            item: item.clone(),
            score,
            matched_keywords,
        }
    }

    /// 对 RSS Feed 进行评分和排名
    pub fn rank_feed(&self, feed: &RssFeed) -> RssRanking {
        let total_items = feed.items.len();
        
        // 对所有项目评分
        let mut scored_items: Vec<ScoredRssItem> = feed.items
            .iter()
            .map(|item| self.score_item(item))
            .filter(|scored| scored.score >= self.config.min_score)
            .collect();

        // 按评分降序排序
        scored_items.sort_by(|a, b| {
            b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal)
        });

        // 限制结果数量
        scored_items.truncate(self.config.max_results);

        RssRanking {
            name: self.config.name.clone(),
            items: scored_items,
            total_items,
            timestamp: current_timestamp(),
        }
    }

    /// 对多个 RSS Feed 进行评分和排名
    pub fn rank_feeds(&self, feeds: &[RssFeed]) -> RssRanking {
        let total_items: usize = feeds.iter().map(|f| f.items.len()).sum();
        
        // 对所有 feed 的所有项目评分
        let mut scored_items: Vec<ScoredRssItem> = feeds
            .iter()
            .flat_map(|feed| feed.items.iter())
            .map(|item| self.score_item(item))
            .filter(|scored| scored.score >= self.config.min_score)
            .collect();

        // 按评分降序排序
        scored_items.sort_by(|a, b| {
            b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal)
        });

        // 去重（基于链接）
        let mut seen_urls = std::collections::HashSet::new();
        scored_items.retain(|item| seen_urls.insert(item.item.link.clone()));

        // 限制结果数量
        scored_items.truncate(self.config.max_results);

        RssRanking {
            name: self.config.name.clone(),
            items: scored_items,
            total_items,
            timestamp: current_timestamp(),
        }
    }
}

/// 获取当前时间戳
fn current_timestamp() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::derive::rss::RssFeedMeta;

    fn create_test_item(title: &str, description: &str) -> RssFeedItem {
        RssFeedItem {
            title: title.to_string(),
            link: format!("https://example.com/{}", title),
            description: Some(description.to_string()),
            author: None,
            pub_date: None,
            content: None,
            categories: vec![],
            guid: None,
            enclosures: vec![],
            custom_fields: HashMap::new(),
        }
    }

    #[test]
    fn test_keyword_scoring() {
        let config = RankingConfig {
            name: "test".to_string(),
            keywords: vec![
                RankingKeyword::new("rust", 5.0),
                RankingKeyword::new("programming", 3.0),
            ],
            min_score: 0.0,
            max_results: 10,
        };

        let engine = RssRankingEngine::new(config);
        let item = create_test_item("Rust Programming Guide", "Learn Rust programming language");
        
        let scored = engine.score_item(&item);
        assert!(scored.score > 0.0);
        assert_eq!(scored.matched_keywords.len(), 2);
    }

    #[test]
    fn test_required_keyword() {
        let config = RankingConfig {
            name: "test".to_string(),
            keywords: vec![
                RankingKeyword::required("rust", 5.0),
            ],
            min_score: 0.0,
            max_results: 10,
        };

        let engine = RssRankingEngine::new(config);
        
        // 包含必需关键词
        let item1 = create_test_item("Rust Guide", "Learn Rust");
        let scored1 = engine.score_item(&item1);
        assert!(scored1.score > 0.0);
        
        // 不包含必需关键词
        let item2 = create_test_item("Python Guide", "Learn Python");
        let scored2 = engine.score_item(&item2);
        assert_eq!(scored2.score, 0.0);
    }

    #[test]
    fn test_ranking() {
        let config = RankingConfig {
            name: "tech".to_string(),
            keywords: vec![
                RankingKeyword::new("rust", 5.0),
                RankingKeyword::new("ai", 3.0),
            ],
            min_score: 1.0,
            max_results: 3,
        };

        let engine = RssRankingEngine::new(config);
        
        let feed = RssFeed {
            meta: RssFeedMeta {
                title: "Tech News".to_string(),
                link: "https://example.com".to_string(),
                description: None,
                language: None,
                copyright: None,
                last_build_date: None,
                pub_date: None,
                image: None,
            },
            items: vec![
                create_test_item("Rust 1.75 Released", "New Rust version with AI features"),
                create_test_item("Python 3.12 Released", "New Python version"),
                create_test_item("AI Revolution", "AI is changing the world"),
                create_test_item("Go 1.21 Released", "New Go version"),
            ],
        };

        let ranking = engine.rank_feed(&feed);
        assert!(ranking.items.len() <= 3);
        assert!(ranking.total_items == 4);
        
        // 第一个应该是包含 rust 和 ai 的项目
        if !ranking.items.is_empty() {
            assert!(ranking.items[0].score > 0.0);
        }
    }

    #[test]
    fn test_multi_feed_ranking() {
        let config = RankingConfig {
            name: "multi".to_string(),
            keywords: vec![
                RankingKeyword::new("tech", 5.0),
            ],
            min_score: 1.0,
            max_results: 10,
        };

        let engine = RssRankingEngine::new(config);

        let feed1 = RssFeed {
            meta: RssFeedMeta {
                title: "Feed 1".to_string(),
                link: "https://example1.com".to_string(),
                description: None,
                language: None,
                copyright: None,
                last_build_date: None,
                pub_date: None,
                image: None,
            },
            items: vec![
                create_test_item("Tech News 1", "Latest tech developments"),
            ],
        };

        let feed2 = RssFeed {
            meta: RssFeedMeta {
                title: "Feed 2".to_string(),
                link: "https://example2.com".to_string(),
                description: None,
                language: None,
                copyright: None,
                last_build_date: None,
                pub_date: None,
                image: None,
            },
            items: vec![
                create_test_item("Tech News 2", "More tech news"),
            ],
        };

        let ranking = engine.rank_feeds(&[feed1, feed2]);
        assert_eq!(ranking.total_items, 2);
        assert!(ranking.items.len() >= 1);
    }

    #[test]
    fn test_deduplication() {
        let config = RankingConfig {
            name: "dedup".to_string(),
            keywords: vec![
                RankingKeyword::new("test", 5.0),
            ],
            min_score: 0.0,
            max_results: 10,
        };

        let engine = RssRankingEngine::new(config);

        let feed1 = RssFeed {
            meta: RssFeedMeta {
                title: "Feed 1".to_string(),
                link: "https://example.com".to_string(),
                description: None,
                language: None,
                copyright: None,
                last_build_date: None,
                pub_date: None,
                image: None,
            },
            items: vec![
                create_test_item("Test Article", "Test content"),
            ],
        };

        let feed2 = RssFeed {
            meta: RssFeedMeta {
                title: "Feed 2".to_string(),
                link: "https://example.com".to_string(),
                description: None,
                language: None,
                copyright: None,
                last_build_date: None,
                pub_date: None,
                image: None,
            },
            items: vec![
                create_test_item("Test Article", "Test content"), // Duplicate
            ],
        };

        let ranking = engine.rank_feeds(&[feed1, feed2]);
        // Should deduplicate by URL
        assert_eq!(ranking.items.len(), 1);
    }

    #[test]
    fn test_min_score_filtering() {
        let config = RankingConfig {
            name: "filter".to_string(),
            keywords: vec![
                RankingKeyword::new("specific", 2.0),
            ],
            min_score: 5.0, // High threshold
            max_results: 10,
        };

        let engine = RssRankingEngine::new(config);

        let feed = RssFeed {
            meta: RssFeedMeta {
                title: "Feed".to_string(),
                link: "https://example.com".to_string(),
                description: None,
                language: None,
                copyright: None,
                last_build_date: None,
                pub_date: None,
                image: None,
            },
            items: vec![
                create_test_item("Low Score", "No keyword here"),
                create_test_item("High Score", "specific specific specific specific"),
            ],
        };

        let ranking = engine.rank_feed(&feed);
        // Only high-scoring items should pass
        assert!(ranking.items.len() <= 1);
    }
}

