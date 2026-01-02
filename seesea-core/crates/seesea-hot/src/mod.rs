// Copyright (C) 2025 SeeSea Team
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

//! 热点数据获取模块
//!
//! 该模块用于从newsnow.busiyi.world API获取热点数据，并进行解析和处理。
//! 支持多平台并发获取，使用项目的HTTP客户端请求数据。
//! 包含5分钟缓存机制以减少重复请求。

use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::Semaphore;
use tracing::{debug, error, info};

use seesea_errors::{ErrorInfo, ErrorCategory, ErrorKind, ErrorSeverity, Result};
use seesea_net::client::HttpClient;

pub mod cache;
pub mod client;
pub mod types;

pub use seesea_cache::{HotTrendCache, HotTrendCacheStats, get_hot_trend_cache};

use types::{HotTrendResponse, HotTrendResult};

/// 支持的平台列表
pub const SUPPORTED_PLATFORMS: &[(&str, &str)] = &[
    ("zhihu", "知乎"),
    ("weibo", "微博"),
    ("toutiao", "今日头条"),
    ("baidu", "百度热搜"),
    ("bilibili-hot-search", "B站热搜"),
    ("douyin", "抖音"),
    ("tieba", "百度贴吧"),
    ("wallstreetcn-hot", "华尔街见闻"),
    ("cls-hot", "财联社热门"),
    ("thepaper", "澎湃新闻"),
    ("ifeng", "凤凰网"),
    ("36kr-renqi", "36氪人气榜"),
    ("hupu", "虎扑"),
    ("github-trending-today", "GitHub今日趋势"),
    ("hackernews", "Hacker News"),
    ("producthunt", "Product Hunt"),
    ("juejin", "稀土掘金"),
    ("sspai", "少数派"),
    ("ithome", "IT之家"),
    ("solidot", "Solidot"),
    ("coolapk", "酷安"),
    ("nowcoder", "牛客"),
    ("kuaishou", "快手"),
    ("jintou", "金投网"),
    ("jin10", "金十数据"),
    ("gelonghui", "格隆汇"),
    ("xueqiu-hotstock", "雪球热门股票"),
    ("fastbull-express", "法布财经快讯"),
    ("cankaoxiaoxi", "参考消息"),
    ("zaobao", "联合早报"),
    ("sputniknewscn", "卫星通讯社"),
    ("chongbuluo-hot", "虫部落热门"),
    ("pcbeta-windows11", "远景论坛Win11"),
    ("freebuf", "Freebuf网络安全"),
    ("douban", "豆瓣热门电影"),
    ("steam", "Steam在线人数"),
    ("tencent-hot", "腾讯新闻综合早报"),
    ("v2ex-share", "V2EX最新分享"),
];

/// 热点数据获取器
pub struct HotTrendFetcher {
    /// HTTP客户端
    http_client: Arc<HttpClient>,
    /// 最大并发数
    max_concurrency: usize,
}

impl HotTrendFetcher {
    /// 创建新的热点数据获取器
    pub async fn new(max_concurrency: usize) -> Result<Self> {
        // 获取全局HTTP客户端实例
        let http_client = HttpClient::instance().await?;

        Ok(Self {
            http_client,
            max_concurrency,
        })
    }

    /// 获取单个平台的热点数据
    pub async fn fetch_platform(&self, platform_id: &str) -> Result<HotTrendResult> {
        // 构建API请求URL
        let url = format!("https://newsnow.busiyi.world/api/s?id={platform_id}&latest");
        info!("Fetching hot trends for platform: {platform_id} from {url}");

        // 创建重试执行器
        let retry_config = self.http_client.retry_config().clone();
        let retry_executor = crate::retry::RetryExecutor::new(retry_config);

        // 使用重试执行器包装HTTP请求
        let response_text = retry_executor
            .execute(|| {
                let http_client = self.http_client.clone();
                let url = url.clone();

                Box::pin(async move {
                    // 发送GET请求
                    let response = match http_client.get(&url, None).await {
                        Ok(resp) => resp,
                        Err(e) => {
                            return Err(std::io::Error::other(format!("HTTP request failed: {e}")));
                        }
                    };

                    // 检查HTTP状态码
                    let status = response.status();
                    if !status.is_success() {
                        return Err(std::io::Error::other(format!("HTTP error: {status}")));
                    }

                    // 读取响应文本
                    let text = match response.text().await {
                        Ok(t) => t,
                        Err(e) => {
                            return Err(std::io::Error::other(format!(
                                "Failed to read response text: {e}"
                            )));
                        }
                    };

                    Ok(text)
                })
            })
            .await?;

        debug!("Response for {platform_id}: {response_text}");

        // 解析JSON响应
        let hot_trend_response: HotTrendResponse = serde_json::from_str(&response_text)
            .map_err(|e| ErrorInfo::new(
                "500".to_string(),
                format!("Failed to parse JSON response: {e}"),
                ErrorCategory::System,
                ErrorKind::ParseError,
                ErrorSeverity::Error
            ))?;

        // 获取平台名称
        let platform_name = SUPPORTED_PLATFORMS
            .iter()
            .find(|(id, _)| id == &platform_id)
            .map(|(_, name)| name.to_string())
            .unwrap_or_else(|| platform_id.to_string());

        // 添加排名信息
        let items = hot_trend_response
            .items
            .into_iter()
            .enumerate()
            .map(|(index, mut item)| {
                item.rank = Some((index + 1) as u32);
                item
            })
            .collect();

        Ok(HotTrendResult {
            platform_id: platform_id.to_string(),
            platform_name,
            status: hot_trend_response.status,
            items,
        })
    }

    /// 批量获取多个平台的热点数据
    pub async fn fetch_multiple_platforms(
        &self,
        platform_ids: &[String],
    ) -> Vec<Result<HotTrendResult>> {
        let mut results = Vec::with_capacity(platform_ids.len());
        let semaphore = Arc::new(Semaphore::new(self.max_concurrency));
        let mut tasks = Vec::new();

        for platform_id in platform_ids {
            let permit = semaphore.clone().acquire_owned().await.unwrap();
            let fetcher_clone = self.clone();
            let platform_id_clone = platform_id.clone();

            let task = tokio::spawn(async move {
                let _permit = permit;
                fetcher_clone.fetch_platform(&platform_id_clone).await
            });

            tasks.push(task);
        }

        for task in tasks {
            match task.await {
                Ok(result) => results.push(result),
                Err(e) => {
                    error!("Task failed: {e}");
                    results.push(Err(ErrorInfo::new(
                        "500".to_string(),
                        "Task execution failed".to_string(),
                        ErrorCategory::System,
                        ErrorKind::InternalError,
                        ErrorSeverity::Error,
                    )));
                }
            }
        }

        results
    }

    /// 获取所有平台的热点数据
    pub async fn fetch_all_platforms(&self) -> Vec<Result<HotTrendResult>> {
        let platform_ids: Vec<String> = SUPPORTED_PLATFORMS
            .iter()
            .map(|(id, _)| id.to_string())
            .collect();

        self.fetch_multiple_platforms(&platform_ids).await
    }

    /// 获取所有支持的平台列表
    pub fn list_platforms() -> HashMap<String, String> {
        SUPPORTED_PLATFORMS
            .iter()
            .map(|(id, name)| (id.to_string(), name.to_string()))
            .collect()
    }
}

// 实现Clone trait
impl Clone for HotTrendFetcher {
    fn clone(&self) -> Self {
        Self {
            http_client: self.http_client.clone(),
            max_concurrency: self.max_concurrency,
        }
    }
}
