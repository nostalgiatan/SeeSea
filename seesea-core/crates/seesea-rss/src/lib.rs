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

//! RSS 模块
//!
//! 提供 RSS feed 的获取、解析、缓存和管理功能

// 导出类型定义
pub mod types;
pub use types::{RssError, RssResult};

// 导出解析器
pub mod parser;
pub use parser::RssParser;

// 导出获取器
pub mod fetcher;
pub use fetcher::RssFetcher;

// 导出模板管理器
pub mod template;
pub use template::RssTemplateManager;

// 导出排名系统
pub mod ranking;
pub use ranking::{RankingConfig, RankingKeyword, RssRanking, RssRankingEngine, ScoredRssItem};

// 导出主接口
pub mod on;
pub use on::RssInterface;
