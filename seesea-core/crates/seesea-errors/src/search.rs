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

//! 搜索相关错误定义
//!
//! 包含搜索引擎、搜索请求、搜索结果等相关错误的定义和创建函数。

use crate::{ErrorInfo, ErrorSeverity};
use error::ErrorCategory;

/// 搜索错误码常量
///
/// 搜索错误码范围：2000-2999
pub const SEARCH_ERROR_BASE: u32 = 2000;
pub const ENGINE_UNAVAILABLE: u32 = SEARCH_ERROR_BASE + 1;
pub const SEARCH_TIMEOUT: u32 = SEARCH_ERROR_BASE + 2;
pub const ZERO_RESULTS: u32 = SEARCH_ERROR_BASE + 3;
pub const INVALID_QUERY: u32 = SEARCH_ERROR_BASE + 4;
pub const UNSUPPORTED_SEARCH_TYPE: u32 = SEARCH_ERROR_BASE + 5;
pub const ENGINE_ERROR: u32 = SEARCH_ERROR_BASE + 6;
pub const RESULT_PARSE_FAILED: u32 = SEARCH_ERROR_BASE + 7;
pub const SEARCH_RATE_LIMITED: u32 = SEARCH_ERROR_BASE + 8;
pub const SEARCH_DEPTH_TOO_LARGE: u32 = SEARCH_ERROR_BASE + 9;
pub const INVALID_SEARCH_SCOPE: u32 = SEARCH_ERROR_BASE + 10;

/// 创建引擎不可用错误
///
/// # 参数
/// - `engine_name`: 不可用的引擎名称
///
/// # 返回
/// 包含引擎不可用信息的错误对象
pub fn engine_unavailable(engine_name: &str) -> ErrorInfo {
    ErrorInfo::new(
        ENGINE_UNAVAILABLE,
        format!("搜索引擎 '{engine_name}' 不可用"),
    )
    .with_category(ErrorCategory::Search)
    .with_severity(ErrorSeverity::Error)
}

/// 创建通用搜索错误
///
/// # 参数
/// - `message`: 错误消息
///
/// # 返回
/// 包含搜索错误信息的错误对象
pub fn search_error(message: impl Into<String>) -> ErrorInfo {
    ErrorInfo::new(SEARCH_ERROR_BASE, message.into())
        .with_category(ErrorCategory::Search)
        .with_severity(ErrorSeverity::Error)
}

/// 创建搜索超时错误
///
/// # 参数
/// - `engine_name`: 超时的引擎名称
///
/// # 返回
/// 包含搜索超时信息的错误对象
pub fn search_timeout(engine_name: &str) -> ErrorInfo {
    ErrorInfo::new(SEARCH_TIMEOUT, format!("搜索引擎 '{engine_name}' 搜索超时"))
        .with_category(ErrorCategory::Search)
        .with_severity(ErrorSeverity::Error)
}

/// 创建零结果错误
///
/// # 参数
/// - `engine_name`: 返回零结果的引擎名称
///
/// # 返回
/// 包含零结果信息的错误对象
pub fn zero_results(engine_name: &str) -> ErrorInfo {
    ErrorInfo::new(ZERO_RESULTS, format!("搜索引擎 '{engine_name}' 返回零结果"))
        .with_category(ErrorCategory::Search)
        .with_severity(ErrorSeverity::Warning)
}

/// 创建无效查询错误
///
/// # 参数
/// - `query`: 无效的查询
/// - `reason`: 无效原因
///
/// # 返回
/// 包含无效查询信息的错误对象
pub fn invalid_query(query: &str, reason: &str) -> ErrorInfo {
    ErrorInfo::new(INVALID_QUERY, format!("无效查询 '{query}': {reason}"))
        .with_category(ErrorCategory::Search)
        .with_severity(ErrorSeverity::Error)
}

/// 创建不支持的搜索类型错误
///
/// # 参数
/// - `engine_name`: 搜索引擎名称
/// - `search_type`: 不支持的搜索类型
///
/// # 返回
/// 包含不支持的搜索类型信息的错误对象
pub fn unsupported_search_type(engine_name: &str, search_type: &str) -> ErrorInfo {
    ErrorInfo::new(
        UNSUPPORTED_SEARCH_TYPE,
        format!("搜索引擎 '{engine_name}' 不支持 '{search_type}' 搜索类型"),
    )
    .with_category(ErrorCategory::Search)
    .with_severity(ErrorSeverity::Error)
}

/// 创建引擎错误
///
/// # 参数
/// - `engine_name`: 出错的引擎名称
/// - `message`: 错误消息
///
/// # 返回
/// 包含引擎错误信息的错误对象
pub fn engine_error(engine_name: &str, message: &str) -> ErrorInfo {
    ErrorInfo::new(
        ENGINE_ERROR,
        format!("搜索引擎 '{engine_name}' 错误: {message}"),
    )
    .with_category(ErrorCategory::Search)
    .with_severity(ErrorSeverity::Error)
}

/// 创建结果解析失败错误
///
/// # 参数
/// - `engine_name`: 引擎名称
/// - `reason`: 解析失败原因
///
/// # 返回
/// 包含结果解析失败信息的错误对象
pub fn result_parse_failed(engine_name: &str, reason: &str) -> ErrorInfo {
    ErrorInfo::new(
        RESULT_PARSE_FAILED,
        format!("搜索引擎 '{engine_name}' 结果解析失败: {reason}"),
    )
    .with_category(ErrorCategory::Search)
    .with_severity(ErrorSeverity::Error)
}

/// 创建搜索频率限制错误
///
/// # 参数
/// - `engine_name`: 引擎名称
///
/// # 返回
/// 包含搜索频率限制信息的错误对象
pub fn search_rate_limited(engine_name: &str) -> ErrorInfo {
    ErrorInfo::new(
        SEARCH_RATE_LIMITED,
        format!("搜索引擎 '{engine_name}' 搜索频率受限"),
    )
    .with_category(ErrorCategory::Search)
    .with_severity(ErrorSeverity::Warning)
}

/// 创建搜索深度过大错误
///
/// # 参数
/// - `depth`: 当前搜索深度
/// - `max_depth`: 最大允许深度
///
/// # 返回
/// 包含搜索深度过大信息的错误对象
pub fn search_depth_too_large(depth: u32, max_depth: u32) -> ErrorInfo {
    ErrorInfo::new(
        SEARCH_DEPTH_TOO_LARGE,
        format!("搜索深度过大: 当前深度 {depth}, 最大深度 {max_depth}"),
    )
    .with_category(ErrorCategory::Search)
    .with_severity(ErrorSeverity::Error)
}

/// 创建无效搜索范围错误
///
/// # 参数
/// - `scope`: 无效的范围
///
/// # 返回
/// 包含无效搜索范围信息的错误对象
pub fn invalid_search_scope(scope: &str) -> ErrorInfo {
    ErrorInfo::new(INVALID_SEARCH_SCOPE, format!("无效搜索范围: {scope}"))
        .with_category(ErrorCategory::Search)
        .with_severity(ErrorSeverity::Error)
}
