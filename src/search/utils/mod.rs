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

//! 搜索工具模块
//!
//! 提供搜索相关的工具函数和组件

/// 时间提取器模块
///
/// 负责从HTML、URL、内容中提取时间信息，并进行标准化处理
pub mod time_extractor;

// 统一导出时间提取器的核心功能
pub use time_extractor::{
    TimeExtractResult, TimeSource, extract_time, extract_time_from_url, parse_relative_time,
    parse_time, standardize_time,
};
