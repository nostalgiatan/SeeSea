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

//! RSS feed types and abstractions
//!
//! 为 RSS 源提供基础类型定义和抽象接口

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// RSS Feed 项目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RssFeedItem {
    /// 标题
    pub title: String,
    /// 链接
    pub link: String,
    /// 描述/摘要
    pub description: Option<String>,
    /// 作者
    pub author: Option<String>,
    /// 发布时间
    pub pub_date: Option<String>,
    /// 内容
    pub content: Option<String>,
    /// 分类/标签
    pub categories: Vec<String>,
    /// GUID (唯一标识符)
    pub guid: Option<String>,
    /// 附件 (例如图片、音频)
    pub enclosures: Vec<RssEnclosure>,
    /// 自定义字段
    pub custom_fields: HashMap<String, String>,
}

/// RSS 附件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RssEnclosure {
    /// URL
    pub url: String,
    /// MIME 类型
    pub mime_type: Option<String>,
    /// 大小（字节）
    pub length: Option<u64>,
}

/// RSS Feed 元数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RssFeedMeta {
    /// Feed 标题
    pub title: String,
    /// Feed 链接
    pub link: String,
    /// Feed 描述
    pub description: Option<String>,
    /// 语言
    pub language: Option<String>,
    /// 版权信息
    pub copyright: Option<String>,
    /// 最后构建日期
    pub last_build_date: Option<String>,
    /// 发布日期
    pub pub_date: Option<String>,
    /// 图片
    pub image: Option<RssFeedImage>,
}

/// RSS Feed 图片
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RssFeedImage {
    /// URL
    pub url: String,
    /// 标题
    pub title: String,
    /// 链接
    pub link: String,
    /// 宽度
    pub width: Option<u32>,
    /// 高度
    pub height: Option<u32>,
}

/// 完整的 RSS Feed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RssFeed {
    /// Feed 元数据
    pub meta: RssFeedMeta,
    /// Feed 项目列表
    pub items: Vec<RssFeedItem>,
}

/// RSS Feed 查询
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RssFeedQuery {
    /// Feed URL
    pub url: String,
    /// 最大项目数
    pub max_items: Option<usize>,
    /// 过滤关键词
    pub filter_keywords: Vec<String>,
    /// 只获取特定日期之后的项目
    pub after_date: Option<String>,
}

impl Default for RssFeedQuery {
    fn default() -> Self {
        Self {
            url: String::new(),
            max_items: Some(10),
            filter_keywords: vec![],
            after_date: None,
        }
    }
}

/// RSS Feed 源 trait
///
/// 定义 RSS Feed 源的抽象接口
#[async_trait::async_trait]
pub trait RssFeedSource: Send + Sync {
    /// 获取 Feed 名称
    fn name(&self) -> &str;

    /// 获取 Feed URL
    fn url(&self) -> &str;

    /// 获取 Feed 内容
    async fn fetch(&self, query: &RssFeedQuery) -> Result<RssFeed, Box<dyn std::error::Error + Send + Sync>>;

    /// 解析 Feed 内容
    fn parse(&self, content: &str) -> Result<RssFeed, Box<dyn std::error::Error + Send + Sync>>;
}
