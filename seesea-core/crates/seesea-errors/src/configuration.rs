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

//! 配置相关错误定义
//!
//! 包含配置文件读取、解析、验证等相关错误的定义和创建函数。

use crate::{ErrorInfo, ErrorSeverity};
use error::ErrorCategory;

/// 配置错误码常量
///
/// 配置错误码范围：7000-7999
pub const CONFIGURATION_ERROR_BASE: u32 = 7000;
pub const CONFIG_FILE_NOT_FOUND: u32 = CONFIGURATION_ERROR_BASE + 1;
pub const CONFIG_PARSE_FAILED: u32 = CONFIGURATION_ERROR_BASE + 2;
pub const MISSING_CONFIG_ITEM: u32 = CONFIGURATION_ERROR_BASE + 3;
pub const INVALID_CONFIG_VALUE: u32 = CONFIGURATION_ERROR_BASE + 4;
pub const CONFIG_TYPE_ERROR: u32 = CONFIGURATION_ERROR_BASE + 5;
pub const CONFIG_VALUE_OUT_OF_RANGE: u32 = CONFIGURATION_ERROR_BASE + 6;
pub const CONFIG_CONFLICT: u32 = CONFIGURATION_ERROR_BASE + 7;
pub const CONFIG_VERSION_MISMATCH: u32 = CONFIGURATION_ERROR_BASE + 8;
pub const CONFIG_PERMISSION_ERROR: u32 = CONFIGURATION_ERROR_BASE + 9;
pub const CONFIG_VALIDATION_FAILED: u32 = CONFIGURATION_ERROR_BASE + 10;

/// 创建配置文件未找到错误
///
/// # 参数
/// - `path`: 配置文件路径
///
/// # 返回
/// 包含配置文件未找到信息的错误对象
pub fn config_file_not_found(path: &str) -> ErrorInfo {
    ErrorInfo::new(CONFIG_FILE_NOT_FOUND, format!("配置文件不存在: {path}"))
        .with_category(ErrorCategory::Configuration)
        .with_severity(ErrorSeverity::Error)
}

/// 创建配置文件解析失败错误
///
/// # 参数
/// - `path`: 配置文件路径
/// - `reason`: 解析失败的原因
///
/// # 返回
/// 包含配置文件解析失败信息的错误对象
pub fn config_parse_failed(path: &str, reason: &str) -> ErrorInfo {
    ErrorInfo::new(
        CONFIG_PARSE_FAILED,
        format!("配置文件 '{path}' 解析失败: {reason}"),
    )
    .with_category(ErrorCategory::Configuration)
    .with_severity(ErrorSeverity::Error)
}

/// 创建缺少配置项错误
///
/// # 参数
/// - `item`: 缺少的配置项名称
///
/// # 返回
/// 包含缺少配置项信息的错误对象
pub fn missing_config_item(item: &str) -> ErrorInfo {
    ErrorInfo::new(MISSING_CONFIG_ITEM, format!("缺少配置项: {item}"))
        .with_category(ErrorCategory::Configuration)
        .with_severity(ErrorSeverity::Error)
}

/// 创建无效配置值错误
///
/// # 参数
/// - `item`: 配置项名称
/// - `value`: 无效的配置值
/// - `reason`: 无效的原因
///
/// # 返回
/// 包含无效配置值信息的错误对象
pub fn invalid_config_value(item: &str, value: &str, reason: &str) -> ErrorInfo {
    ErrorInfo::new(
        INVALID_CONFIG_VALUE,
        format!("配置项 '{item}' 值 '{value}' 无效: {reason}"),
    )
    .with_category(ErrorCategory::Configuration)
    .with_severity(ErrorSeverity::Error)
}

/// 创建配置类型错误
///
/// # 参数
/// - `item`: 配置项名称
/// - `expected_type`: 期望的类型
/// - `actual_type`: 实际的类型
///
/// # 返回
/// 包含配置类型错误信息的错误对象
pub fn config_type_error(item: &str, expected_type: &str, actual_type: &str) -> ErrorInfo {
    ErrorInfo::new(
        CONFIG_TYPE_ERROR,
        format!("配置项 '{item}' 类型错误: 期望 {expected_type}, 实际 {actual_type}"),
    )
    .with_category(ErrorCategory::Configuration)
    .with_severity(ErrorSeverity::Error)
}

/// 创建配置值超出范围错误
///
/// # 参数
/// - `item`: 配置项名称
/// - `min`: 最小值
/// - `max`: 最大值
/// - `actual`: 实际值
///
/// # 返回
/// 包含配置值超出范围信息的错误对象
pub fn config_value_out_of_range(item: &str, min: i32, max: i32, actual: i32) -> ErrorInfo {
    ErrorInfo::new(
        CONFIG_VALUE_OUT_OF_RANGE,
        format!("配置项 '{item}' 值 {actual} 超出范围: 要求 {min}-{max} 之间"),
    )
    .with_category(ErrorCategory::Configuration)
    .with_severity(ErrorSeverity::Error)
}

/// 创建配置冲突错误
///
/// # 参数
/// - `item1`: 冲突的配置项1
/// - `item2`: 冲突的配置项2
/// - `reason`: 冲突的原因
///
/// # 返回
/// 包含配置冲突信息的错误对象
pub fn config_conflict(item1: &str, item2: &str, reason: &str) -> ErrorInfo {
    ErrorInfo::new(
        CONFIG_CONFLICT,
        format!("配置项 '{item1}' 和 '{item2}' 冲突: {reason}"),
    )
    .with_category(ErrorCategory::Configuration)
    .with_severity(ErrorSeverity::Error)
}

/// 创建配置版本不匹配错误
///
/// # 参数
/// - `expected_version`: 期望的版本
/// - `actual_version`: 实际的版本
///
/// # 返回
/// 包含配置版本不匹配信息的错误对象
pub fn config_version_mismatch(expected_version: &str, actual_version: &str) -> ErrorInfo {
    ErrorInfo::new(
        CONFIG_VERSION_MISMATCH,
        format!("配置版本不匹配: 期望 {expected_version}, 实际 {actual_version}"),
    )
    .with_category(ErrorCategory::Configuration)
    .with_severity(ErrorSeverity::Error)
}

/// 创建配置权限错误
///
/// # 参数
/// - `item`: 配置项名称
/// - `operation`: 操作类型
///
/// # 返回
/// 包含配置权限错误信息的错误对象
pub fn config_permission_error(item: &str, operation: &str) -> ErrorInfo {
    ErrorInfo::new(
        CONFIG_PERMISSION_ERROR,
        format!("配置项 '{item}' 权限错误: 不允许 {operation}"),
    )
    .with_category(ErrorCategory::Configuration)
    .with_severity(ErrorSeverity::Error)
}

/// 创建配置验证失败错误
///
/// # 参数
/// - `item`: 配置项名称
/// - `reason`: 验证失败的原因
///
/// # 返回
/// 包含配置验证失败信息的错误对象
pub fn config_validation_failed(item: &str, reason: &str) -> ErrorInfo {
    ErrorInfo::new(
        CONFIG_VALIDATION_FAILED,
        format!("配置项 '{item}' 验证失败: {reason}"),
    )
    .with_category(ErrorCategory::Configuration)
    .with_severity(ErrorSeverity::Error)
}

/// 通用配置错误创建函数
///
/// # 参数
/// - `message`: 配置错误的详细信息
///
/// # 返回
/// 包含配置错误信息的错误对象
pub fn configuration_error(message: impl Into<String>) -> ErrorInfo {
    ErrorInfo::new(CONFIGURATION_ERROR_BASE, message.into())
        .with_category(ErrorCategory::Configuration)
        .with_severity(ErrorSeverity::Error)
}
