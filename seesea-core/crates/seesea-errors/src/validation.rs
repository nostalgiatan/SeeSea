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

//! 验证相关错误定义
//!
//! 包含参数验证、数据验证、输入验证等相关错误的定义和创建函数。

use crate::{ErrorInfo, ErrorSeverity};
use error::ErrorCategory;

/// 验证错误码常量
///
/// 验证错误码范围：4000-4999
pub const VALIDATION_ERROR_BASE: u32 = 4000;
pub const EMPTY_FIELD: u32 = VALIDATION_ERROR_BASE + 1;
pub const FIELD_TOO_SHORT: u32 = VALIDATION_ERROR_BASE + 2;
pub const FIELD_TOO_LONG: u32 = VALIDATION_ERROR_BASE + 3;
pub const INVALID_EMAIL: u32 = VALIDATION_ERROR_BASE + 4;
pub const INVALID_URL: u32 = VALIDATION_ERROR_BASE + 5;
pub const INVALID_DATE: u32 = VALIDATION_ERROR_BASE + 6;
pub const INVALID_NUMBER: u32 = VALIDATION_ERROR_BASE + 7;
pub const INVALID_ENUM_VALUE: u32 = VALIDATION_ERROR_BASE + 8;
pub const DUPLICATE_VALUE: u32 = VALIDATION_ERROR_BASE + 9;
pub const UNSUPPORTED_PARAMETER: u32 = VALIDATION_ERROR_BASE + 10;

/// 创建空字段错误
///
/// # 参数
/// - `field`: 空字段的名称
///
/// # 返回
/// 包含空字段信息的错误对象
pub fn empty_field(field: &str) -> ErrorInfo {
    ErrorInfo::new(EMPTY_FIELD, format!("字段 '{field}' 不能为空"))
        .with_category(ErrorCategory::Validation)
        .with_severity(ErrorSeverity::Error)
}

/// 创建字段太短错误
///
/// # 参数
/// - `field`: 字段名称
/// - `min_length`: 最小长度要求
/// - `actual_length`: 实际长度
///
/// # 返回
/// 包含字段太短信息的错误对象
pub fn field_too_short(field: &str, min_length: usize, actual_length: usize) -> ErrorInfo {
    ErrorInfo::new(
        FIELD_TOO_SHORT,
        format!("字段 '{field}' 太短: 要求至少 {min_length} 个字符, 实际 {actual_length} 个字符"),
    )
    .with_category(ErrorCategory::Validation)
    .with_severity(ErrorSeverity::Error)
}

/// 创建字段太长错误
///
/// # 参数
/// - `field`: 字段名称
/// - `max_length`: 最大长度要求
/// - `actual_length`: 实际长度
///
/// # 返回
/// 包含字段太长信息的错误对象
pub fn field_too_long(field: &str, max_length: usize, actual_length: usize) -> ErrorInfo {
    ErrorInfo::new(
        FIELD_TOO_LONG,
        format!("字段 '{field}' 太长: 要求最多 {max_length} 个字符, 实际 {actual_length} 个字符"),
    )
    .with_category(ErrorCategory::Validation)
    .with_severity(ErrorSeverity::Error)
}

/// 创建无效邮箱错误
///
/// # 参数
/// - `email`: 无效的邮箱地址
///
/// # 返回
/// 包含无效邮箱信息的错误对象
pub fn invalid_email(email: &str) -> ErrorInfo {
    ErrorInfo::new(INVALID_EMAIL, format!("无效的邮箱地址: {email}"))
        .with_category(ErrorCategory::Validation)
        .with_severity(ErrorSeverity::Error)
}

/// 创建无效URL错误
///
/// # 参数
/// - `url`: 无效的URL地址
///
/// # 返回
/// 包含无效URL信息的错误对象
pub fn invalid_url(url: &str) -> ErrorInfo {
    ErrorInfo::new(INVALID_URL, format!("无效的URL地址: {url}"))
        .with_category(ErrorCategory::Validation)
        .with_severity(ErrorSeverity::Error)
}

/// 创建无效日期错误
///
/// # 参数
/// - `date`: 无效的日期字符串
///
/// # 返回
/// 包含无效日期信息的错误对象
pub fn invalid_date(date: &str) -> ErrorInfo {
    ErrorInfo::new(INVALID_DATE, format!("无效的日期格式: {date}"))
        .with_category(ErrorCategory::Validation)
        .with_severity(ErrorSeverity::Error)
}

/// 创建无效数字错误
///
/// # 参数
/// - `value`: 无效的数字字符串
/// - `field`: 字段名称
///
/// # 返回
/// 包含无效数字信息的错误对象
pub fn invalid_number(value: &str, field: &str) -> ErrorInfo {
    ErrorInfo::new(
        INVALID_NUMBER,
        format!("字段 '{field}' 不是有效的数字: {value}"),
    )
    .with_category(ErrorCategory::Validation)
    .with_severity(ErrorSeverity::Error)
}

/// 创建无效枚举值错误
///
/// # 参数
/// - `field`: 字段名称
/// - `value`: 无效的值
/// - `allowed_values`: 允许的枚举值列表
///
/// # 返回
/// 包含无效枚举值信息的错误对象
pub fn invalid_enum_value(field: &str, value: &str, allowed_values: &[&str]) -> ErrorInfo {
    let allowed = allowed_values.join(", ");
    ErrorInfo::new(
        INVALID_ENUM_VALUE,
        format!("字段 '{field}' 值 '{value}' 无效: 允许的值为 [{allowed}]"),
    )
    .with_category(ErrorCategory::Validation)
    .with_severity(ErrorSeverity::Error)
}

/// 创建重复值错误
///
/// # 参数
/// - `field`: 字段名称
/// - `value`: 重复的值
///
/// # 返回
/// 包含重复值信息的错误对象
pub fn duplicate_value(field: &str, value: &str) -> ErrorInfo {
    ErrorInfo::new(
        DUPLICATE_VALUE,
        format!("字段 '{field}' 值 '{value}' 已存在"),
    )
    .with_category(ErrorCategory::Validation)
    .with_severity(ErrorSeverity::Error)
}

/// 创建不支持的参数错误
///
/// # 参数
/// - `param`: 不支持的参数名称
///
/// # 返回
/// 包含不支持参数信息的错误对象
pub fn unsupported_parameter(param: &str) -> ErrorInfo {
    ErrorInfo::new(UNSUPPORTED_PARAMETER, format!("不支持的参数: {param}"))
        .with_category(ErrorCategory::Validation)
        .with_severity(ErrorSeverity::Error)
}

/// 创建超出范围错误
///
/// # 参数
/// - `field`: 字段名称
/// - `min`: 最小值
/// - `max`: 最大值
/// - `actual`: 实际值
///
/// # 返回
/// 包含超出范围信息的错误对象
pub fn out_of_range(field: &str, min: i32, max: i32, actual: i32) -> ErrorInfo {
    ErrorInfo::new(
        VALIDATION_ERROR_BASE + 11,
        format!("字段 '{field}' 值 {actual} 超出范围: 要求 {min}-{max} 之间"),
    )
    .with_category(ErrorCategory::Validation)
    .with_severity(ErrorSeverity::Error)
}

/// 通用验证错误创建函数
///
/// # 参数
/// - `message`: 验证错误的详细信息
///
/// # 返回
/// 包含验证错误信息的错误对象
pub fn validation_error(message: impl Into<String>) -> ErrorInfo {
    ErrorInfo::new(VALIDATION_ERROR_BASE, message.into())
        .with_category(ErrorCategory::Validation)
        .with_severity(ErrorSeverity::Error)
}
