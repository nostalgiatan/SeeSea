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

//! 热点数据类型定义
//!
//! 包含热点数据相关的所有类型定义，包括API响应、热点项和结果结构。

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// API返回的热点数据结构
#[derive(Debug, Deserialize, Serialize)]
pub struct HotTrendResponse {
    /// 响应状态，success或cache（可选，某些API响应可能不包含此字段）
    #[serde(default = "default_status")]
    pub status: String,

    /// 热点新闻列表（主要字段名）
    #[serde(alias = "news", alias = "data", default)]
    pub items: Vec<HotTrendItem>,

    /// 响应消息（可选）
    #[serde(default)]
    pub message: String,

    /// 响应代码（可选）
    #[serde(default)]
    pub code: i32,
}

/// 状态字段的默认值
fn default_status() -> String {
    "success".to_string()
}

/// 单个热点新闻项
#[derive(Debug, Deserialize, Serialize, Clone, ToSchema)]
pub struct HotTrendItem {
    /// 新闻标题
    #[serde(default)]
    pub title: String,

    /// 新闻链接
    #[serde(default)]
    pub url: String,

    /// 移动端新闻链接，API返回的字段名是mobileUrl，所以需要使用serde的rename属性
    #[serde(rename = "mobileUrl", alias = "mobile_url", default)]
    pub mobile_url: Option<String>,

    /// 排名，API返回中没有这个字段，由客户端添加
    #[serde(skip_deserializing)]
    pub rank: Option<u32>,

    /// 热度值（可选）
    #[serde(rename = "hotValue", alias = "hot", alias = "heat", default)]
    pub hot_value: Option<String>,

    /// 热度指数（可选）
    #[serde(rename = "hotIndex", alias = "index", default)]
    pub hot_index: Option<u32>,

    /// 新闻来源（可选）
    #[serde(rename = "source", alias = "from", default)]
    pub source: Option<String>,

    /// 发布时间（可选）
    #[serde(rename = "publishTime", alias = "time", default)]
    pub publish_time: Option<String>,
}

/// 处理后的热点数据结果
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct HotTrendResult {
    /// 平台ID
    pub platform_id: String,

    /// 平台名称
    pub platform_name: String,

    /// 响应状态，success或cache
    pub status: String,

    /// 热点新闻列表
    pub items: Vec<HotTrendItem>,
}

/// 热点数据获取请求参数
#[derive(Debug, Serialize)]
pub struct HotTrendRequest {
    /// 平台ID
    pub platform_id: String,

    /// 是否获取最新数据
    pub latest: bool,
}

/// 批量热点数据获取请求参数
#[derive(Debug, Serialize)]
pub struct BatchHotTrendRequest {
    /// 平台ID列表
    pub platform_ids: Vec<String>,

    /// 最大并发数
    pub max_concurrency: usize,

    /// 是否获取最新数据
    pub latest: bool,
}

/// 批量热点数据获取结果
#[derive(Debug, Serialize)]
pub struct BatchHotTrendResult {
    /// 成功获取的平台数量
    pub success_count: usize,

    /// 失败的平台数量
    pub failed_count: usize,

    /// 热点数据结果列表
    pub results: Vec<HotTrendResult>,
}
