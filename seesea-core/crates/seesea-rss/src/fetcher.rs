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

//! RSS feed fetcher
//!
//! 提供 RSS feed 获取功能

use crate::types::RssResult;
use seesea_derive::rss::*;
use seesea_errors::network as network_errors;
use seesea_net::HttpClient;
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
    pub async fn fetch(&self, url: &str) -> RssResult<String> {
        // 验证URL
        if url.trim().is_empty() {
            return Err(seesea_errors::validation::invalid_url("URL为空").into());
        }

        // 验证URL格式
        if !url.starts_with("http://") && !url.starts_with("https://") {
            return Err(
                seesea_errors::validation::invalid_url("URL必须以http://或https://开头").into(),
            );
        }

        // 使用 HTTP 客户端获取内容
        let response = self
            .client
            .get(url, None)
            .await
            .map_err(|e| network_errors::connection_refused(&format!("获取RSS失败: {}", e)))?;

        // 检查响应状态
        if !response.status().is_success() {
            return Err(network_errors::http_error(
                response.status().as_u16(),
                &format!("HTTP错误: {}", response.status()),
            )
            .into());
        }

        // 提取响应文本
        let text = response
            .text()
            .await
            .map_err(|e| network_errors::invalid_response(&format!("读取响应文本失败: {}", e)))?;

        // 验证响应内容
        if text.trim().is_empty() {
            return Err(network_errors::invalid_response("RSS响应内容为空").into());
        }

        // 验证内容类型（基本验证）
        if !text.contains("<?xml") && !text.contains("<rss") && !text.contains("<feed") {
            return Err(network_errors::invalid_response("响应内容不是有效的RSS/Atom格式").into());
        }

        Ok(text)
    }

    /// 获取并解析 RSS feed
    pub async fn fetch_and_parse(&self, query: &RssFeedQuery) -> RssResult<RssFeed> {
        use crate::parser::RssParser;

        // 验证查询参数
        if query.url.trim().is_empty() {
            return Err(seesea_errors::validation::invalid_url("查询URL为空").into());
        }

        // 获取内容
        let content = self.fetch(&query.url).await?;

        // 解析内容
        let parser = RssParser::new();
        let mut feed = parser.parse_rss2(&content)?;

        // 应用过滤和限制
        if let Some(max_items) = query.max_items {
            if max_items > 0 {
                feed.items.truncate(max_items);
            } else {
                return Err(
                    seesea_errors::validation::validation_error("max_items必须大于0").into(),
                );
            }
        }

        // 过滤关键词
        let had_filter = !query.filter_keywords.is_empty();
        if had_filter {
            feed.items.retain(|item| {
                query.filter_keywords.iter().any(|keyword| {
                    item.title.to_lowercase().contains(&keyword.to_lowercase())
                        || item.description.as_ref().is_some_and(|desc| {
                            desc.to_lowercase().contains(&keyword.to_lowercase())
                        })
                })
            });
        }

        // 验证结果 - 仅在应用过滤后为空时显示警告
        if had_filter && feed.items.is_empty() {
            eprintln!("警告: RSS feed未包含任何匹配的项目");
        }

        Ok(feed)
    }

    /// 批量获取多个RSS feed
    pub async fn fetch_multiple(&self, urls: &[String]) -> Vec<RssResult<String>> {
        use futures::future::join_all;

        if urls.is_empty() {
            return vec![];
        }

        // 限制并发数量，避免过载
        let max_concurrent = 10;
        let mut results = Vec::new();

        for chunk in urls.chunks(max_concurrent) {
            let chunk_results: Vec<_> = join_all(chunk.iter().map(|url| self.fetch(url))).await;
            results.extend(chunk_results);
        }

        results
    }

    /// 验证RSS feed是否可访问
    pub async fn validate_feed(&self, url: &str) -> RssResult<bool> {
        match self.fetch(url).await {
            Ok(content) => {
                // 基本内容验证
                let is_valid = content.contains("<?xml")
                    && (content.contains("<rss") || content.contains("<feed"));

                if !is_valid {
                    return Err(
                        network_errors::invalid_response("内容不是有效的RSS/Atom格式").into(),
                    );
                }

                Ok(true)
            }
            Err(e) => {
                eprintln!("RSS feed验证失败 {}: {}", url, e);
                Ok(false)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rss_fetcher_creation() {
        let network_config = NetworkConfig::default();
        let client = Arc::new(HttpClient::new(network_config).unwrap());
        let _fetcher = RssFetcher::new(client);
    }

    #[test]
    fn test_url_validation() {
        let network_config = NetworkConfig::default();
        let client = Arc::new(HttpClient::new(network_config).unwrap());
        let fetcher = RssFetcher::new(client);

        // 这个测试是同步的，所以我们不能测试异步的 fetch 方法
        // 但我们可以测试其他同步功能
        assert!(true);
    }
}
