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

//! 系统相关错误定义
//!
//! 包含系统资源、系统服务、系统配置等相关错误的定义和创建函数。

use crate::errors::{ErrorCategory, ErrorInfo, ErrorSeverity};

/// 系统错误码常量
///
/// 系统错误码范围：10000-10999
pub const SYSTEM_ERROR_BASE: u32 = 10000;
pub const RESOURCE_EXHAUSTED: u32 = SYSTEM_ERROR_BASE + 1;
pub const SERVICE_UNAVAILABLE: u32 = SYSTEM_ERROR_BASE + 2;
pub const CONFIGURATION_ERROR: u32 = SYSTEM_ERROR_BASE + 3;
pub const SYSTEM_TIMEOUT: u32 = SYSTEM_ERROR_BASE + 4;
pub const INTERNAL_SYSTEM_ERROR: u32 = SYSTEM_ERROR_BASE + 5;
pub const SYSTEM_CALL_FAILED: u32 = SYSTEM_ERROR_BASE + 6;
pub const VERSION_INCOMPATIBLE: u32 = SYSTEM_ERROR_BASE + 7;
pub const PERMISSION_ERROR: u32 = SYSTEM_ERROR_BASE + 8;
pub const RESOURCE_LEAK: u32 = SYSTEM_ERROR_BASE + 9;
pub const SYSTEM_OVERLOAD: u32 = SYSTEM_ERROR_BASE + 10;

/// 创建系统资源耗尽错误
///
/// # 参数
/// - `resource`: 耗尽的资源类型
///
/// # 返回
/// 包含系统资源耗尽信息的错误对象
pub fn resource_exhausted(resource: &str) -> ErrorInfo {
    ErrorInfo::new(RESOURCE_EXHAUSTED, format!("系统资源 '{resource}' 耗尽"))
        .with_category(ErrorCategory::System)
        .with_severity(ErrorSeverity::Error)
}

/// 创建系统服务不可用错误
///
/// # 参数
/// - `service`: 不可用的服务名称
///
/// # 返回
/// 包含系统服务不可用信息的错误对象
pub fn service_unavailable(service: &str) -> ErrorInfo {
    ErrorInfo::new(SERVICE_UNAVAILABLE, format!("系统服务 '{service}' 不可用"))
        .with_category(ErrorCategory::System)
        .with_severity(ErrorSeverity::Error)
}

/// 创建系统配置错误
///
/// # 参数
/// - `message`: 配置错误的详细信息
///
/// # 返回
/// 包含系统配置错误信息的错误对象
pub fn configuration_error(message: &str) -> ErrorInfo {
    ErrorInfo::new(CONFIGURATION_ERROR, format!("系统配置错误: {message}"))
        .with_category(ErrorCategory::System)
        .with_severity(ErrorSeverity::Error)
}

/// 创建系统超时错误
///
/// # 参数
/// - `operation`: 超时的操作
///
/// # 返回
/// 包含系统超时信息的错误对象
pub fn system_timeout(operation: &str) -> ErrorInfo {
    ErrorInfo::new(SYSTEM_TIMEOUT, format!("系统操作 '{operation}' 超时"))
        .with_category(ErrorCategory::System)
        .with_severity(ErrorSeverity::Error)
}

/// 创建系统内部错误
///
/// # 参数
/// - `message`: 错误消息
///
/// # 返回
/// 包含系统内部错误信息的错误对象
pub fn internal_system_error(message: &str) -> ErrorInfo {
    ErrorInfo::new(INTERNAL_SYSTEM_ERROR, format!("系统内部错误: {message}"))
        .with_category(ErrorCategory::System)
        .with_severity(ErrorSeverity::Error)
}

/// 创建系统调用失败错误
///
/// # 参数
/// - `syscall`: 失败的系统调用
/// - `reason`: 失败原因
///
/// # 返回
/// 包含系统调用失败信息的错误对象
pub fn system_call_failed(syscall: &str, reason: &str) -> ErrorInfo {
    ErrorInfo::new(
        SYSTEM_CALL_FAILED,
        format!("系统调用 '{syscall}' 失败: {reason}"),
    )
    .with_category(ErrorCategory::System)
    .with_severity(ErrorSeverity::Error)
}

/// 创建系统版本不兼容错误
///
/// # 参数
/// - `expected`: 期望的版本
/// - `actual`: 实际的版本
///
/// # 返回
/// 包含系统版本不兼容信息的错误对象
pub fn version_incompatible(expected: &str, actual: &str) -> ErrorInfo {
    ErrorInfo::new(
        VERSION_INCOMPATIBLE,
        format!("系统版本不兼容: 期望 {expected}, 实际 {actual}"),
    )
    .with_category(ErrorCategory::System)
    .with_severity(ErrorSeverity::Error)
}

/// 创建系统权限错误
///
/// # 参数
/// - `operation`: 操作类型
/// - `resource`: 资源名称
///
/// # 返回
/// 包含系统权限错误信息的错误对象
pub fn permission_error(operation: &str, resource: &str) -> ErrorInfo {
    ErrorInfo::new(
        PERMISSION_ERROR,
        format!("系统权限错误: 无法对 '{resource}' 执行 '{operation}' 操作"),
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
        format!("系统过载: {metric} 达到 {value:.2}, 超过阈值 {threshold:.2}"),
    )
    .with_category(ErrorCategory::System)
    .with_severity(ErrorSeverity::Warning)
}

/// 通用系统错误创建函数
///
/// # 参数
/// - `message`: 系统错误的详细信息
///
/// # 返回
/// 包含系统错误信息的错误对象
pub fn system_error(message: impl Into<String>) -> ErrorInfo {
    ErrorInfo::new(SYSTEM_ERROR_BASE, message.into())
        .with_category(ErrorCategory::System)
        .with_severity(ErrorSeverity::Error)
}
