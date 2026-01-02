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

//! 热点数据客户端
//!
//! 提供更高层次的热点数据获取客户端接口，支持同步和异步调用。

use std::sync::Arc;

use tokio::runtime::Runtime;

use crate::HotTrendFetcher;
use crate::types::HotTrendResult;
use seesea_errors::{ErrorInfo, Result};

/// 热点数据同步客户端
///
/// 提供同步API，内部使用tokio运行时处理异步操作
pub struct HotTrendClient {
    /// 异步热点数据获取器
    fetcher: Arc<HotTrendFetcher>,
    /// Tokio运行时，使用Arc包裹以允许共享
    runtime: Arc<Runtime>,
}

impl HotTrendClient {
    /// 创建新的热点数据同步客户端
    pub fn new(max_concurrency: usize) -> Result<Self> {
        // 创建Tokio运行时
        let runtime = Runtime::new()
            .map_err(|e| ErrorInfo::new(500, format!("Failed to create Tokio runtime: {e}")))?;

        // 在运行时中创建异步热点数据获取器
        let fetcher = runtime.block_on(async { HotTrendFetcher::new(max_concurrency).await })?;

        Ok(Self {
            fetcher: Arc::new(fetcher),
            runtime: Arc::new(runtime),
        })
    }

    /// 获取单个平台的热点数据
    pub fn fetch_platform(&self, platform_id: &str) -> Result<HotTrendResult> {
        let fetcher = self.fetcher.clone();
        let platform_id = platform_id.to_string();

        self.runtime
            .block_on(async move { fetcher.fetch_platform(&platform_id).await })
    }

    /// 批量获取多个平台的热点数据
    pub fn fetch_multiple_platforms(&self, platform_ids: &[String]) -> Vec<Result<HotTrendResult>> {
        let fetcher = self.fetcher.clone();
        let platform_ids = platform_ids.to_vec();

        self.runtime
            .block_on(async move { fetcher.fetch_multiple_platforms(&platform_ids).await })
    }

    /// 获取所有平台的热点数据
    pub fn fetch_all_platforms(&self) -> Vec<Result<HotTrendResult>> {
        let fetcher = self.fetcher.clone();

        self.runtime
            .block_on(async move { fetcher.fetch_all_platforms().await })
    }

    /// 获取所有支持的平台列表
    pub fn list_platforms(&self) -> std::collections::HashMap<String, String> {
        HotTrendFetcher::list_platforms()
    }
}

/// 热点数据异步客户端
///
/// 提供异步API，直接使用tokio异步运行时
pub struct AsyncHotTrendClient {
    /// 异步热点数据获取器
    fetcher: Arc<HotTrendFetcher>,
}

impl AsyncHotTrendClient {
    /// 创建新的热点数据异步客户端
    pub async fn new(max_concurrency: usize) -> Result<Self> {
        let fetcher = HotTrendFetcher::new(max_concurrency).await?;

        Ok(Self {
            fetcher: Arc::new(fetcher),
        })
    }

    /// 获取单个平台的热点数据
    pub async fn fetch_platform(&self, platform_id: &str) -> Result<HotTrendResult> {
        self.fetcher.fetch_platform(platform_id).await
    }

    /// 批量获取多个平台的热点数据
    pub async fn fetch_multiple_platforms(
        &self,
        platform_ids: &[String],
    ) -> Vec<Result<HotTrendResult>> {
        self.fetcher.fetch_multiple_platforms(platform_ids).await
    }

    /// 获取所有平台的热点数据
    pub async fn fetch_all_platforms(&self) -> Vec<Result<HotTrendResult>> {
        self.fetcher.fetch_all_platforms().await
    }

    /// 获取所有支持的平台列表
    pub fn list_platforms(&self) -> std::collections::HashMap<String, String> {
        HotTrendFetcher::list_platforms()
    }
}

impl Clone for AsyncHotTrendClient {
    fn clone(&self) -> Self {
        Self {
            fetcher: self.fetcher.clone(),
        }
    }
}

impl Clone for HotTrendClient {
    fn clone(&self) -> Self {
        Self {
            fetcher: self.fetcher.clone(),
            runtime: self.runtime.clone(), // Arc的clone是廉价的，只增加引用计数
        }
    }
}
