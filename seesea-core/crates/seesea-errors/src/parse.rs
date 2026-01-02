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

//! 解析相关错误定义
//!
//! 包含JSON、XML、HTML、YAML等格式解析错误的定义和创建函数。

use crate::{ErrorInfo, ErrorSeverity};
use error::ErrorCategory;

/// 解析错误码常量
///
/// 解析错误码范围：3000-3999
pub const PARSE_ERROR_BASE: u32 = 3000;
pub const JSON_PARSE_ERROR: u32 = PARSE_ERROR_BASE + 1;
pub const XML_PARSE_ERROR: u32 = PARSE_ERROR_BASE + 2;
pub const HTML_PARSE_ERROR: u32 = PARSE_ERROR_BASE + 3;
pub const YAML_PARSE_ERROR: u32 = PARSE_ERROR_BASE + 4;
pub const CSV_PARSE_ERROR: u32 = PARSE_ERROR_BASE + 5;
pub const INVALID_FORMAT: u32 = PARSE_ERROR_BASE + 6;
pub const MISSING_FIELD: u32 = PARSE_ERROR_BASE + 7;
pub const INVALID_FIELD_TYPE: u32 = PARSE_ERROR_BASE + 8;
pub const INVALID_FIELD_VALUE: u32 = PARSE_ERROR_BASE + 9;
pub const PARSE_TIMEOUT: u32 = PARSE_ERROR_BASE + 10;

/// 创建JSON解析错误
///
/// # 参数
/// - `message`: JSON解析错误的详细信息
///
/// # 返回
/// 包含JSON解析错误信息的错误对象
pub fn json_parse_error(message: &str) -> ErrorInfo {
    ErrorInfo::new(JSON_PARSE_ERROR, format!("JSON解析错误: {message}"))
        .with_category(ErrorCategory::Parse)
        .with_severity(ErrorSeverity::Error)
}

/// 创建XML解析错误
///
/// # 参数
/// - `message`: XML解析错误的详细信息
///
/// # 返回
/// 包含XML解析错误信息的错误对象
pub fn xml_parse_error(message: &str) -> ErrorInfo {
    ErrorInfo::new(XML_PARSE_ERROR, format!("XML解析错误: {message}"))
        .with_category(ErrorCategory::Parse)
        .with_severity(ErrorSeverity::Error)
}

/// 创建HTML解析错误
///
/// # 参数
/// - `message`: HTML解析错误的详细信息
///
/// # 返回
/// 包含HTML解析错误信息的错误对象
pub fn html_parse_error(message: &str) -> ErrorInfo {
    ErrorInfo::new(HTML_PARSE_ERROR, format!("HTML解析错误: {message}"))
        .with_category(ErrorCategory::Parse)
        .with_severity(ErrorSeverity::Error)
}

/// 创建YAML解析错误
///
/// # 参数
/// - `message`: YAML解析错误的详细信息
///
/// # 返回
/// 包含YAML解析错误信息的错误对象
pub fn yaml_parse_error(message: &str) -> ErrorInfo {
    ErrorInfo::new(YAML_PARSE_ERROR, format!("YAML解析错误: {message}"))
        .with_category(ErrorCategory::Parse)
        .with_severity(ErrorSeverity::Error)
}

/// 创建CSV解析错误
///
/// # 参数
/// - `message`: CSV解析错误的详细信息
///
/// # 返回
/// 包含CSV解析错误信息的错误对象
pub fn csv_parse_error(message: &str) -> ErrorInfo {
    ErrorInfo::new(CSV_PARSE_ERROR, format!("CSV解析错误: {message}"))
        .with_category(ErrorCategory::Parse)
        .with_severity(ErrorSeverity::Error)
}

/// 创建无效格式错误
///
/// # 参数
/// - `format`: 无效的数据格式
///
/// # 返回
/// 包含无效格式信息的错误对象
pub fn invalid_format(format: &str) -> ErrorInfo {
    ErrorInfo::new(INVALID_FORMAT, format!("无效的格式: {format}"))
        .with_category(ErrorCategory::Parse)
        .with_severity(ErrorSeverity::Error)
}

/// 创建缺少字段错误
///
/// # 参数
/// - `field`: 缺少的字段名称
/// - `format`: 数据格式
///
/// # 返回
/// 包含缺少字段信息的错误对象
pub fn missing_field(field: &str, format: &str) -> ErrorInfo {
    ErrorInfo::new(MISSING_FIELD, format!("{format}格式缺少字段: {field}"))
        .with_category(ErrorCategory::Parse)
        .with_severity(ErrorSeverity::Error)
}

/// 创建无效字段类型错误
///
/// # 参数
/// - `field`: 字段名称
/// - `expected`: 期望的字段类型
/// - `actual`: 实际的字段类型
///
/// # 返回
/// 包含无效字段类型信息的错误对象
pub fn invalid_field_type(field: &str, expected: &str, actual: &str) -> ErrorInfo {
    ErrorInfo::new(
        INVALID_FIELD_TYPE,
        format!("字段 '{field}' 类型无效: 期望 {expected}, 实际 {actual}"),
    )
    .with_category(ErrorCategory::Parse)
    .with_severity(ErrorSeverity::Error)
}

/// 创建无效字段值错误
///
/// # 参数
/// - `field`: 字段名称
/// - `value`: 无效的字段值
/// - `reason`: 无效的原因
///
/// # 返回
/// 包含无效字段值信息的错误对象
pub fn invalid_field_value(field: &str, value: &str, reason: &str) -> ErrorInfo {
    ErrorInfo::new(
        INVALID_FIELD_VALUE,
        format!("字段 '{field}' 值 '{value}' 无效: {reason}"),
    )
    .with_category(ErrorCategory::Parse)
    .with_severity(ErrorSeverity::Error)
}

/// 创建解析超时错误
///
/// # 参数
/// - `format`: 数据格式
///
/// # 返回
/// 包含解析超时信息的错误对象
pub fn parse_timeout(format: &str) -> ErrorInfo {
    ErrorInfo::new(PARSE_TIMEOUT, format!("{format}格式解析超时"))
        .with_category(ErrorCategory::Parse)
        .with_severity(ErrorSeverity::Error)
}

/// 通用解析错误创建函数
///
/// # 参数
/// - `message`: 解析错误的详细信息
///
/// # 返回
/// 包含解析错误信息的错误对象
pub fn parse_error(message: impl Into<String>) -> ErrorInfo {
    ErrorInfo::new(PARSE_ERROR_BASE, message.into())
        .with_category(ErrorCategory::Parse)
        .with_severity(ErrorSeverity::Error)
}
