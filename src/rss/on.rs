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

//! RSS feed external interface
//!
//! 提供统一的 RSS feed 外部接口

use super::fetcher::RssFetcher;
use super::parser::RssParser;
use super::template::RssTemplateManager;
use crate::cache::rss::RssCache;
use crate::derive::rss::*;
use crate::net::client::HttpClient;
use std::sync::Arc;
use tokio::sync::RwLock;

/// RSS Feed 接口
///
/// 统一的 RSS feed 外部接口，封装获取、解析和缓存功能
pub struct RssInterface {
    /// Feed 获取器
    fetcher: RssFetcher,
    /// Feed 解析器
    parser: RssParser,
    /// RSS 缓存
    cache: Option<Arc<RwLock<RssCache>>>,
    /// 模板管理器
    template_manager: Option<RssTemplateManager>,
}

impl RssInterface {
    /// 创建新的 RSS 接口
    pub fn new(client: Arc<HttpClient>) -> Self {
        Self {
            fetcher: RssFetcher::new(client),
            parser: RssParser::new(),
            cache: None,
            template_manager: None,
        }
    }

    /// 创建带缓存的 RSS 接口
    pub fn with_cache(client: Arc<HttpClient>, cache: Arc<RwLock<RssCache>>) -> Self {
        Self {
            fetcher: RssFetcher::new(client),
            parser: RssParser::new(),
            cache: Some(cache),
            template_manager: None,
        }
    }

    /// 设置模板目录
    pub fn set_template_dir(&mut self, template_dir: &str) {
        self.template_manager = Some(RssTemplateManager::new(template_dir));
    }

    /// 获取 RSS feed（支持缓存）
    pub async fn fetch(
        &self,
        query: &RssFeedQuery,
    ) -> Result<RssFeed, Box<dyn std::error::Error + Send + Sync>> {
        // 检查缓存
        if let Some(ref cache) = self.cache {
            let cache_guard = cache.read().await;

            // 检查是否需要更新
            let needs_update = cache_guard.needs_update(&query.url).unwrap_or(true);

            if !needs_update {
                // 从缓存获取
                if let Ok(Some(feed)) = cache_guard.get(&query.url) {
                    return Ok(feed);
                }
            }
        }

        // 获取新数据
        let feed = self.fetcher.fetch_and_parse(query).await?;

        // 存入缓存（临时 RSS，默认TTL）
        if let Some(ref cache) = self.cache {
            let cache_guard = cache.write().await;
            let _ = cache_guard.set(
                &query.url,
                &feed,
                false,                                      // 临时
                None,                                       // 无自动更新间隔
                Some(std::time::Duration::from_secs(3600)), // 1小时TTL
            );
        }

        Ok(feed)
    }

    /// 获取持久化 RSS feed
    pub async fn fetch_persistent(
        &self,
        url: &str,
        update_interval: u64,
    ) -> Result<RssFeed, Box<dyn std::error::Error + Send + Sync>> {
        // 检查缓存
        if let Some(ref cache) = self.cache {
            let cache_guard = cache.read().await;

            // 检查是否需要更新
            let needs_update = cache_guard.needs_update(url).unwrap_or(true);

            if !needs_update {
                // 从缓存获取
                if let Ok(Some(feed)) = cache_guard.get(url) {
                    return Ok(feed);
                }
            }
        }

        // 获取新数据
        let query = RssFeedQuery {
            url: url.to_string(),
            max_items: None,
            filter_keywords: vec![],
            after_date: None,
        };
        let feed = self.fetcher.fetch_and_parse(&query).await?;

        // 存入缓存（持久化 RSS）
        if let Some(ref cache) = self.cache {
            let cache_guard = cache.write().await;
            let _ = cache_guard.set(
                url,
                &feed,
                true, // 持久化
                Some(update_interval),
                None, // 不设置TTL
            );
        }

        Ok(feed)
    }

    /// 解析 RSS feed 内容
    pub fn parse(
        &self,
        content: &str,
    ) -> Result<RssFeed, Box<dyn std::error::Error + Send + Sync>> {
        self.parser.parse(content)
    }

    /// 获取多个 RSS feeds
    pub async fn fetch_multiple(
        &self,
        queries: Vec<RssFeedQuery>,
    ) -> Vec<Result<RssFeed, Box<dyn std::error::Error + Send + Sync>>> {
        let mut results = Vec::new();
        for query in queries {
            results.push(self.fetch(&query).await);
        }
        results
    }

    /// 列出可用的模板
    pub fn list_templates(&self) -> Result<Vec<String>, Box<dyn std::error::Error + Send + Sync>> {
        if let Some(ref manager) = self.template_manager {
            manager.list_templates()
        } else {
            Ok(vec![])
        }
    }

    /// 从模板添加 RSS feeds
    pub async fn add_from_template(
        &self,
        template_name: &str,
        categories: Option<Vec<String>>,
    ) -> Result<usize, Box<dyn std::error::Error + Send + Sync>> {
        let manager = self
            .template_manager
            .as_ref()
            .ok_or("Template manager not initialized")?;

        let template = manager.load_template(template_name)?;
        let mut added_count = 0;

        for (category, url) in template.feeds {
            // 如果指定了分类，只添加指定的分类
            if let Some(ref cats) = categories
                && !cats.contains(&category)
            {
                continue;
            }

            // 获取并缓存
            let _ = self
                .fetch_persistent(&url, template.meta.update_interval)
                .await;
            added_count += 1;
        }

        Ok(added_count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rss_interface_creation() {
        let network_config = crate::net::config::NetworkConfig::default();
        let client = Arc::new(HttpClient::new(network_config).unwrap());
        let _interface = RssInterface::new(client);
        assert!(true);
    }
}
