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

//! RSS 结果缓存
//!
//! 提供 RSS feed 结果的专门缓存功能，支持持久化和自动更新

use crate::cache::manager::{CacheManager, CacheError};
use crate::derive::rss::RssFeed;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use serde::{Deserialize, Serialize};

type Result<T> = std::result::Result<T, CacheError>;

/// RSS 缓存键前缀
const RSS_KEY_PREFIX: &str = "rss:";
const RSS_META_PREFIX: &str = "rss_meta:";

/// RSS Feed 缓存元数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RssFeedCacheMeta {
    /// Feed URL
    pub url: String,
    /// Feed 名称
    pub name: Option<String>,
    /// 最后更新时间（Unix 时间戳）
    pub last_updated: u64,
    /// 是否为持久化 RSS
    pub persistent: bool,
    /// 自动更新间隔（秒）
    pub update_interval: Option<u64>,
    /// Feed 项目数量
    pub item_count: usize,
}

/// RSS 结果缓存
///
/// 封装 CacheManager，提供 RSS feed 专用的缓存接口
pub struct RssCache {
    manager: Arc<CacheManager>,
}

impl RssCache {
    /// 创建 RSS 缓存实例
    pub fn new(manager: Arc<CacheManager>) -> Self {
        Self { manager }
    }

    /// 生成 RSS feed 缓存键
    pub fn generate_feed_key(url: &str) -> String {
        format!("{}{}", RSS_KEY_PREFIX, url)
    }

    /// 生成 RSS 元数据缓存键
    pub fn generate_meta_key(url: &str) -> String {
        format!("{}{}", RSS_META_PREFIX, url)
    }

    /// 获取当前时间戳
    fn current_timestamp() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or(Duration::from_secs(0))
            .as_secs()
    }

    /// 存储 RSS feed 到缓存
    pub fn set(
        &self,
        url: &str,
        feed: &RssFeed,
        persistent: bool,
        update_interval: Option<u64>,
        ttl: Option<Duration>,
    ) -> Result<()> {
        let key = Self::generate_feed_key(url);
        let meta_key = Self::generate_meta_key(url);

        // 获取现有缓存以进行去重
        let existing_items = if let Ok(Some(existing_feed)) = self.get(url) {
            existing_feed.items
        } else {
            vec![]
        };

        // 去重：基于 link 和 guid
        let mut deduped_items = Vec::new();
        let mut seen_keys = std::collections::HashSet::new();

        for item in &feed.items {
            let key = if let Some(ref guid) = item.guid {
                guid.clone()
            } else {
                item.link.clone()
            };

            if !seen_keys.contains(&key) {
                seen_keys.insert(key);
                deduped_items.push(item.clone());
            }
        }

        // 合并新旧项目（保留旧的，添加新的）
        for item in existing_items {
            let key = if let Some(ref guid) = item.guid {
                guid.clone()
            } else {
                item.link.clone()
            };

            if !seen_keys.contains(&key) {
                seen_keys.insert(key);
                deduped_items.push(item);
            }
        }

        // 创建去重后的 feed
        let deduped_feed = RssFeed {
            meta: feed.meta.clone(),
            items: deduped_items.clone(),
        };

        // 序列化并存储 feed
        let feed_bytes = bincode::serde::encode_to_vec(&deduped_feed, bincode::config::standard())
            .map_err(|e| CacheError::SerializationError(format!("Failed to serialize feed: {}", e)))?;
        self.manager.set(key, feed_bytes, ttl)?;

        // 存储元数据
        let meta = RssFeedCacheMeta {
            url: url.to_string(),
            name: Some(feed.meta.title.clone()),
            last_updated: Self::current_timestamp(),
            persistent,
            update_interval,
            item_count: deduped_items.len(),
        };
        let meta_bytes = bincode::serde::encode_to_vec(&meta, bincode::config::standard())
            .map_err(|e| CacheError::SerializationError(format!("Failed to serialize meta: {}", e)))?;
        self.manager.set(meta_key, meta_bytes, None)?;

        Ok(())
    }

    /// 从缓存获取 RSS feed
    pub fn get(&self, url: &str) -> Result<Option<RssFeed>> {
        let key = Self::generate_feed_key(url);
        if let Some(bytes) = self.manager.get(&key)? {
            let (feed, _) = bincode::serde::decode_from_slice(&bytes, bincode::config::standard())
                .map_err(|e| CacheError::SerializationError(format!("Failed to deserialize feed: {}", e)))?;
            Ok(Some(feed))
        } else {
            Ok(None)
        }
    }

    /// 获取 RSS feed 元数据
    pub fn get_meta(&self, url: &str) -> Result<Option<RssFeedCacheMeta>> {
        let key = Self::generate_meta_key(url);
        if let Some(bytes) = self.manager.get(&key)? {
            let (meta, _) = bincode::serde::decode_from_slice(&bytes, bincode::config::standard())
                .map_err(|e| CacheError::SerializationError(format!("Failed to deserialize meta: {}", e)))?;
            Ok(Some(meta))
        } else {
            Ok(None)
        }
    }

    /// 检查是否需要更新
    pub fn needs_update(&self, url: &str) -> Result<bool> {
        if let Some(meta) = self.get_meta(url)? {
            if let Some(interval) = meta.update_interval {
                let elapsed = Self::current_timestamp() - meta.last_updated;
                Ok(elapsed >= interval)
            } else {
                Ok(false)
            }
        } else {
            Ok(true) // 如果没有元数据，需要更新
        }
    }

    /// 列出所有持久化的 RSS feeds
    pub fn list_persistent_feeds(&self) -> Result<Vec<RssFeedCacheMeta>> {
        // Note: This is a simplified implementation
        // In production, you might want to maintain a separate index
        Ok(vec![])
    }

    /// 删除 RSS feed 缓存
    pub fn delete(&self, url: &str) -> Result<()> {
        let key = Self::generate_feed_key(url);
        let meta_key = Self::generate_meta_key(url);
        
        self.manager.delete(&key)?;
        self.manager.delete(&meta_key)?;
        
        Ok(())
    }

    /// 全文搜索 - 在所有缓存的 RSS items 中查找包含关键词的项目
    ///
    /// # 参数
    ///
    /// * `keywords` - 搜索关键词列表
    /// * `include_stale` - 是否包含过期的缓存结果
    /// * `max_results` - 最大返回结果数（可选）
    ///
    /// # 返回值
    ///
    /// 返回匹配的 RSS 项列表，每项包含 (feed_url, item)
    ///
    /// # 性能说明
    ///
    /// 此方法遍历所有 RSS 缓存条目。对于大量 RSS 订阅，建议：
    /// - 使用 max_results 参数限制返回数量
    /// - 定期清理不再需要的 RSS feeds
    /// - 实现分页或增量加载
    pub fn search_fulltext(
        &self,
        keywords: &[String],
        include_stale: bool,
        max_results: Option<usize>,
    ) -> Result<Vec<(String, crate::derive::rss::RssFeedItem)>> {
        use crate::derive::rss::RssFeed;

        let mut matched_items = Vec::new();
        let max = max_results.unwrap_or(usize::MAX);

        // 遍历所有以 rss: 开头的缓存键
        for item in self.manager.iter() {
            if matched_items.len() >= max {
                break;
            }

            let (key, value) = item.map_err(|e| {
                CacheError::DatabaseError(format!("遍历缓存失败: {}", e))
            })?;

            let key_str = String::from_utf8_lossy(&key);
            
            // 只处理 RSS feed 缓存
            if !key_str.starts_with(RSS_KEY_PREFIX) {
                continue;
            }

            // 提取 feed URL（移除前缀）
            let feed_url = key_str.strip_prefix(RSS_KEY_PREFIX).unwrap_or(&key_str).to_string();

            // 检查是否过期（如果不包含过期结果）
            if !include_stale {
                if let Some(metadata) = self.manager.get_metadata(&key_str)? {
                    if metadata.is_expired() {
                        continue;
                    }
                }
            }

            // 反序列化 RSS feed
            let feed: RssFeed = match bincode::serde::decode_from_slice(&value, bincode::config::standard()) {
                Ok((f, _)) => f,
                Err(_) => continue, // 跳过损坏的数据
            };

            // 在 RSS items 中搜索关键词
            for rss_item in feed.items {
                if matched_items.len() >= max {
                    break;
                }

                // 检查标题、描述和内容是否包含任何关键词
                let matches = keywords.iter().any(|keyword| {
                    let keyword_lower = keyword.to_lowercase();
                    rss_item.title.to_lowercase().contains(&keyword_lower)
                        || rss_item.description.as_ref()
                            .map(|d| d.to_lowercase().contains(&keyword_lower))
                            .unwrap_or(false)
                        || rss_item.content.as_ref()
                            .map(|c| c.to_lowercase().contains(&keyword_lower))
                            .unwrap_or(false)
                        || rss_item.link.to_lowercase().contains(&keyword_lower)
                });

                if matches {
                    matched_items.push((feed_url.clone(), rss_item));
                }
            }
        }

        Ok(matched_items)
    }

    /// 列出所有缓存的 RSS feeds（包括过期的）
    ///
    /// # 返回值
    ///
    /// 返回所有 RSS feed URLs 和元数据
    pub fn list_all_feeds(&self) -> Result<Vec<(String, Option<RssFeedCacheMeta>)>> {
        let mut feeds = Vec::new();

        for item in self.manager.iter() {
            let (key, _value) = item.map_err(|e| {
                CacheError::DatabaseError(format!("遍历缓存失败: {}", e))
            })?;

            let key_str = String::from_utf8_lossy(&key);
            
            // 只处理 RSS feed 缓存
            if !key_str.starts_with(RSS_KEY_PREFIX) {
                continue;
            }

            // 提取 feed URL（移除前缀）
            let feed_url = key_str.strip_prefix(RSS_KEY_PREFIX).unwrap_or(&key_str).to_string();
            
            // 获取元数据
            let meta = self.get_meta(&feed_url).ok().flatten();
            
            feeds.push((feed_url, meta));
        }

        Ok(feeds)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cache::types::CacheImplConfig;

    #[test]
    fn test_rss_cache_keys() {
        let url = "https://example.com/rss";
        let feed_key = RssCache::generate_feed_key(url);
        let meta_key = RssCache::generate_meta_key(url);
        
        assert!(feed_key.starts_with(RSS_KEY_PREFIX));
        assert!(meta_key.starts_with(RSS_META_PREFIX));
    }

    #[test]
    fn test_rss_cache_creation() {
        let config = CacheImplConfig::default();
        let manager = CacheManager::instance(config).unwrap();
        let _cache = RssCache::new(manager);
    }
}
