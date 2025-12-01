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
