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

//! # RSS 模块
//!
//! RSS 模块是 SeeSea 的核心组件之一，提供完整的 RSS 订阅、获取、解析、排序和模板化功能，支持多种 RSS 格式和灵活的配置选项。
//!
//! ## 模块架构
//!
//! RSS 模块采用分层设计，主要包含以下核心组件：
//!
//! - **types**：核心类型定义，包括 RSS 源、条目和查询等
//! - **parser**：RSS 解析器，支持多种 RSS 格式（RSS 2.0、Atom 等）
//! - **fetcher**：RSS 获取器，负责从网络获取 RSS 内容
//! - **template**：RSS 模板系统，支持自定义 RSS 输出格式
//! - **ranking**：RSS 排序系统，支持基于多种因素的结果排序
//! - **on**：主要接口定义，提供统一的 RSS 服务访问接口
//!
//! ## 核心功能
//!
//! ### RSS 获取与解析
//! - 支持多种 RSS 格式（RSS 2.0、Atom、RDF）
//! - 自动检测和解析不同格式的 RSS 内容
//! - 支持 HTTPS 和代理访问
//! - 智能重试机制，提高获取成功率
//!
//! ### RSS 源管理
//! - 支持添加、删除和更新 RSS 源
//! - 支持按类别和标签管理 RSS 源
//! - 支持 RSS 源的状态监控
//! - 支持自动发现网页中的 RSS 源
//!
//! ### RSS 内容处理
//! - 支持内容过滤和去重
//! - 支持基于关键词的内容匹配
//! - 支持内容摘要生成
//! - 支持图片和媒体内容处理
//!
//! ### RSS 排序与推荐
//! - 基于发布时间的排序
//! - 基于相关性的排序
//! - 基于阅读习惯的推荐
//! - 支持自定义排序规则
//!
//! ### RSS 模板系统
//! - 支持多种输出格式（HTML、JSON、XML）
//! - 支持自定义模板
//! - 支持模板变量和条件渲染
//! - 支持响应式设计
//!
//! ## 使用示例
//!
//! ```rust,no_run
//! use seesea::rss::{RssFetcher, RssParser, RssFeedQuery};
//!
//! // 创建 RSS 获取器
//! let fetcher = RssFetcher::new();
//!
//! // 获取 RSS 内容
//! let rss_url = "https://example.com/feed.xml";
//! let rss_content = fetcher.fetch(rss_url).await?;
//!
//! // 解析 RSS 内容
//! let parser = RssParser::new();
//! let feed = parser.parse(&rss_content).await?;
//!
//! // 打印 RSS 信息
//! println!("Feed 标题: {}", feed.title);
//! println!("Feed 链接: {}", feed.link);
//! println!("共有 {} 条条目", feed.items.len());
//!
//! // 使用查询获取特定条目
//! let query = RssFeedQuery {
//!     keywords: vec!["rust", "programming"],
//!     limit: Some(10),
//!     ..Default::default()
//! };
//!
//! let filtered_items = feed.query(&query);
//! println!("匹配到 {} 条条目", filtered_items.len());
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

/// 核心类型定义模块，包括 RSS 源、条目和查询等
pub mod types;

/// RSS 解析器模块，支持多种 RSS 格式（RSS 2.0、Atom 等）
pub mod parser;

/// RSS 获取器模块，负责从网络获取 RSS 内容
pub mod fetcher;

/// RSS 模板系统模块，支持自定义 RSS 输出格式
pub mod template;

/// RSS 排序系统模块，支持基于多种因素的结果排序
pub mod ranking;

/// 主要接口定义模块，提供统一的 RSS 服务访问接口
pub mod on;

// 导出所有核心类型和功能，方便外部使用

/// RSS 核心类型，包括源、条目、查询等
pub use types::*;

/// RSS 解析器功能，用于解析不同格式的 RSS 内容
pub use parser::*;

/// RSS 获取器功能，用于从网络获取 RSS 内容
pub use fetcher::*;

/// RSS 模板系统功能，用于自定义 RSS 输出格式
pub use template::*;

/// RSS 排序系统功能，用于对 RSS 条目进行排序
pub use ranking::*;

/// RSS 主要接口，提供统一的 RSS 服务访问
pub use on::*;
