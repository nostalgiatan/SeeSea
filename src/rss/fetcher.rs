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

//! RSS feed fetcher
//!
//! 提供 RSS feed 获取功能

use crate::derive::rss::*;
use crate::net::client::HttpClient;
use std::sync::Arc;

/// RSS Feed 获取器
pub struct RssFetcher {
    /// HTTP 客户端
    client: Arc<HttpClient>,
}

impl RssFetcher {
    /// 创建新的获取器
    pub fn new(client: Arc<HttpClient>) -> Self {
        Self { client }
    }

    /// 获取 RSS feed 内容
    pub async fn fetch(&self, url: &str) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        // 使用 HTTP 客户端获取内容
        let response = self.client.get(url, None).await
            .map_err(|e| format!("Failed to fetch RSS feed: {}", e))?;

        // 提取响应文本
        let text = response.text().await
            .map_err(|e| format!("Failed to read response text: {}", e))?;

        Ok(text)
    }

    /// 获取并解析 RSS feed
    pub async fn fetch_and_parse(&self, query: &RssFeedQuery) -> Result<RssFeed, Box<dyn std::error::Error + Send + Sync>> {
        use crate::rss::parser::RssParser;

        // 获取内容
        let content = self.fetch(&query.url).await?;

        // 解析内容
        let parser = RssParser::new();
        let mut feed = parser.parse(&content)?;

        // 应用过滤和限制
        if let Some(max_items) = query.max_items {
            feed.items.truncate(max_items);
        }

        // 过滤关键词
        if !query.filter_keywords.is_empty() {
            feed.items.retain(|item| {
                query.filter_keywords.iter().any(|keyword| {
                    item.title.to_lowercase().contains(&keyword.to_lowercase()) ||
                    item.description.as_ref().map_or(false, |desc| desc.to_lowercase().contains(&keyword.to_lowercase()))
                })
            });
        }

        Ok(feed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rss_fetcher_creation() {
        let network_config = crate::net::types::NetworkConfig::default();
        let client = Arc::new(HttpClient::new(network_config).unwrap());
        let fetcher = RssFetcher::new(client);
        assert!(true);
    }
}
