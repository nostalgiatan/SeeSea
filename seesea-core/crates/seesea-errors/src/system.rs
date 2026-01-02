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

//! 系统相关错误定义
//!
//! 包含系统资源、系统服务、系统配置等相关错误的定义和创建函数。

use crate::{ErrorInfo, ErrorSeverity};
use error::ErrorCategory;

/// 系统错误码常量
///
/// 系统错误码范围：10000-10999
pub const SYSTEM_ERROR_BASE: u32 = 10000;
pub const RESOURCE_EXHAUSTED: u32 = SYSTEM_ERROR_BASE + 1;
pub const SERVICE_UNAVAILABLE: u32 = SYSTEM_ERROR_BASE + 2;
pub const CONFIGURATION_ERROR: u32 = SYSTEM_ERROR_BASE + 3;
pub const SYSTEM_TIMEOUT: u32 = SYSTEM_ERROR_BASE + 4;
pub const INTERNAL_SYSTEM_ERROR: u32 = SYSTEM_ERROR_BASE + 5;
pub const RESOURCE_LEAK: u32 = SYSTEM_ERROR_BASE + 6;
pub const SYSTEM_OVERLOAD: u32 = SYSTEM_ERROR_BASE + 7;
pub const SYSTEM_ERROR: u32 = SYSTEM_ERROR_BASE + 8;

/// 创建系统资源耗尽错误
///
/// # 参数
/// - `resource`: 耗尽的资源类型
/// - `current`: 当前使用量
/// - `limit`: 资源限制
///
/// # 返回
/// 包含系统资源耗尽信息的错误对象
pub fn resource_exhausted(resource: &str, current: u64, limit: u64) -> ErrorInfo {
    ErrorInfo::new(
        RESOURCE_EXHAUSTED,
        format!("系统资源 '{resource}' 耗尽: 当前使用量 {current}, 限制 {limit}"),
    )
    .with_category(ErrorCategory::System)
    .with_severity(ErrorSeverity::Error)
}

/// 创建系统服务不可用错误
///
/// # 参数
/// - `service`: 服务名称
/// - `reason`: 服务不可用的原因
///
/// # 返回
/// 包含系统服务不可用信息的错误对象
pub fn service_unavailable(service: &str, reason: &str) -> ErrorInfo {
    ErrorInfo::new(
        SERVICE_UNAVAILABLE,
        format!("系统服务 '{service}' 不可用: {reason}"),
    )
    .with_category(ErrorCategory::System)
    .with_severity(ErrorSeverity::Error)
}

/// 创建系统配置错误
///
/// # 参数
/// - `config`: 配置项名称
/// - `reason`: 配置错误原因
///
/// # 返回
/// 包含系统配置错误信息的错误对象
pub fn configuration_error(config: &str, reason: &str) -> ErrorInfo {
    ErrorInfo::new(
        CONFIGURATION_ERROR,
        format!("系统配置 '{config}' 错误: {reason}"),
    )
    .with_category(ErrorCategory::System)
    .with_severity(ErrorSeverity::Error)
}

/// 创建系统超时错误
///
/// # 参数
/// - `operation`: 超时操作
/// - `timeout`: 超时时间（秒）
///
/// # 返回
/// 包含系统超时信息的错误对象
pub fn system_timeout(operation: &str, timeout: u64) -> ErrorInfo {
    ErrorInfo::new(
        SYSTEM_TIMEOUT,
        format!("系统操作 '{operation}' 超时: {timeout}秒"),
    )
    .with_category(ErrorCategory::System)
    .with_severity(ErrorSeverity::Error)
}

/// 创建系统内部错误
///
/// # 参数
/// - `operation`: 发生错误的操作
/// - `details`: 错误详情
///
/// # 返回
/// 包含系统内部错误信息的错误对象
pub fn internal_system_error(operation: &str, details: &str) -> ErrorInfo {
    ErrorInfo::new(
        INTERNAL_SYSTEM_ERROR,
        format!("系统内部错误: {operation} - {details}"),
    )
    .with_category(ErrorCategory::System)
    .with_severity(ErrorSeverity::Error)
}

/// 创建系统资源泄漏错误
///
/// # 参数
/// - `resource`: 泄漏的资源类型
///
/// # 返回
/// 包含系统资源泄漏信息的错误对象
pub fn resource_leak(resource: &str) -> ErrorInfo {
    ErrorInfo::new(RESOURCE_LEAK, format!("系统资源 '{resource}' 泄漏"))
        .with_category(ErrorCategory::System)
        .with_severity(ErrorSeverity::Warning)
}

/// 创建系统过载错误
///
/// # 参数
/// - `metric`: 过载的指标
/// - `value`: 指标值
/// - `threshold`: 阈值
///
/// # 返回
/// 包含系统过载信息的错误对象
pub fn system_overload(metric: &str, value: f64, threshold: f64) -> ErrorInfo {
    ErrorInfo::new(
        SYSTEM_OVERLOAD,
        format!("系统过载指标 '{metric}' 超过阈值: 当前值 {value:.2}, 阈值 {threshold:.2}"),
    )
    .with_category(ErrorCategory::System)
    .with_severity(ErrorSeverity::Error)
}

/// 创建通用系统错误
///
/// # 参数
/// - `message`: 错误消息
///
/// # 返回
/// 包含系统错误信息的错误对象
pub fn system_error(message: impl Into<String>) -> ErrorInfo {
    ErrorInfo::new(SYSTEM_ERROR, message.into())
        .with_category(ErrorCategory::System)
        .with_severity(ErrorSeverity::Error)
}
