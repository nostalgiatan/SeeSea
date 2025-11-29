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

//! 搜索相关错误定义
//! 
//! 包含搜索引擎、搜索请求、搜索结果等相关错误的定义和创建函数。

use crate::errors::{ErrorInfo, ErrorCategory, ErrorSeverity};

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
    ErrorInfo::new(ENGINE_UNAVAILABLE, format!("搜索引擎 '{engine_name}' 不可用"))
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
/// - `query`: 无效的查询内容
/// - `reason`: 无效的原因
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
/// - `search_type`: 不支持的搜索类型
/// - `engine_name`: 搜索引擎名称
/// 
/// # 返回
/// 包含不支持的搜索类型信息的错误对象
pub fn unsupported_search_type(search_type: &str, engine_name: &str) -> ErrorInfo {
    ErrorInfo::new(UNSUPPORTED_SEARCH_TYPE, format!("搜索引擎 '{engine_name}' 不支持 '{search_type}' 搜索类型"))
        .with_category(ErrorCategory::Search)
        .with_severity(ErrorSeverity::Error)
}

/// 创建引擎错误
/// 
/// # 参数
/// - `engine_name`: 出错的引擎名称
/// - `message`: 错误详细信息
/// 
/// # 返回
/// 包含引擎错误信息的错误对象
pub fn engine_error(engine_name: &str, message: &str) -> ErrorInfo {
    ErrorInfo::new(ENGINE_ERROR, format!("搜索引擎 '{engine_name}' 错误: {message}"))
        .with_category(ErrorCategory::Search)
        .with_severity(ErrorSeverity::Error)
}

/// 创建结果解析失败错误
/// 
/// # 参数
/// - `engine_name`: 结果解析失败的引擎名称
/// - `reason`: 解析失败的原因
/// 
/// # 返回
/// 包含结果解析失败信息的错误对象
pub fn result_parse_failed(engine_name: &str, reason: &str) -> ErrorInfo {
    ErrorInfo::new(RESULT_PARSE_FAILED, format!("搜索引擎 '{engine_name}' 结果解析失败: {reason}"))
        .with_category(ErrorCategory::Search)
        .with_severity(ErrorSeverity::Error)
}

/// 创建搜索速率限制错误
/// 
/// # 参数
/// - `engine_name`: 触发速率限制的引擎名称
/// 
/// # 返回
/// 包含搜索速率限制信息的错误对象
pub fn search_rate_limited(engine_name: &str) -> ErrorInfo {
    ErrorInfo::new(SEARCH_RATE_LIMITED, format!("搜索引擎 '{engine_name}' 搜索速率受限"))
        .with_category(ErrorCategory::Search)
        .with_severity(ErrorSeverity::Warning)
}

/// 创建搜索深度过大错误
/// 
/// # 参数
/// - `depth`: 请求的搜索深度
/// - `max_depth`: 允许的最大搜索深度
/// 
/// # 返回
/// 包含搜索深度过大信息的错误对象
pub fn search_depth_too_large(depth: u32, max_depth: u32) -> ErrorInfo {
    ErrorInfo::new(SEARCH_DEPTH_TOO_LARGE, format!("搜索深度 {depth} 超过最大限制 {max_depth}"))
        .with_category(ErrorCategory::Search)
        .with_severity(ErrorSeverity::Error)
}

/// 创建无效搜索范围错误
/// 
/// # 参数
/// - `scope`: 无效的搜索范围
/// 
/// # 返回
/// 包含无效搜索范围信息的错误对象
pub fn invalid_search_scope(scope: &str) -> ErrorInfo {
    ErrorInfo::new(INVALID_SEARCH_SCOPE, format!("无效的搜索范围: {scope}"))
        .with_category(ErrorCategory::Search)
        .with_severity(ErrorSeverity::Error)
}

/// 向后兼容的搜索错误创建函数
/// 
/// 此函数保持与现有代码的兼容性，新代码建议使用更具体的错误创建函数。
/// 
/// # 参数
/// - `message`: 错误消息
/// 
/// # 返回
/// 包含指定消息的搜索错误对象
pub fn search_error(message: impl Into<String>) -> ErrorInfo {
    ErrorInfo::new(SEARCH_ERROR_BASE, message.into())
        .with_category(ErrorCategory::Search)
}
