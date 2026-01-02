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

//! 业务逻辑相关错误定义
//!
//! 包含业务规则、业务流程、业务状态等相关错误的定义和创建函数。

use crate::{ErrorInfo, ErrorSeverity};
use error::ErrorCategory;

/// 业务逻辑错误码常量
///
/// 业务逻辑错误码范围：9000-9999
pub const BUSINESS_ERROR_BASE: u32 = 9000;
pub const RESOURCE_NOT_FOUND: u32 = BUSINESS_ERROR_BASE + 1;
pub const BUSINESS_RULE_VIOLATION: u32 = BUSINESS_ERROR_BASE + 2;
pub const INVALID_STATE: u32 = BUSINESS_ERROR_BASE + 3;
pub const OPERATION_NOT_ALLOWED: u32 = BUSINESS_ERROR_BASE + 4;
pub const QUOTA_EXCEEDED: u32 = BUSINESS_ERROR_BASE + 5;
pub const RATE_LIMIT_EXCEEDED: u32 = BUSINESS_ERROR_BASE + 6;
pub const DEPENDENCY_FAILED: u32 = BUSINESS_ERROR_BASE + 7;
pub const VALIDATION_FAILED: u32 = BUSINESS_ERROR_BASE + 8;
pub const INTERNAL_BUSINESS_ERROR: u32 = BUSINESS_ERROR_BASE + 9;
pub const CONCURRENCY_ERROR: u32 = BUSINESS_ERROR_BASE + 10;

/// 创建资源未找到错误
///
/// # 参数
/// - `resource_type`: 资源类型
/// - `resource_id`: 资源ID
///
/// # 返回
/// 包含资源未找到信息的错误对象
pub fn resource_not_found(resource_type: &str, resource_id: &str) -> ErrorInfo {
    ErrorInfo::new(
        RESOURCE_NOT_FOUND,
        format!("{resource_type} '{resource_id}' 不存在"),
    )
    .with_category(ErrorCategory::Business)
    .with_severity(ErrorSeverity::Error)
}

/// 创建业务规则违反错误
///
/// # 参数
/// - `rule`: 业务规则名称
/// - `reason`: 违反的原因
///
/// # 返回
/// 包含业务规则违反信息的错误对象
pub fn business_rule_violation(rule: &str, reason: &str) -> ErrorInfo {
    ErrorInfo::new(
        BUSINESS_RULE_VIOLATION,
        format!("违反业务规则 '{rule}': {reason}"),
    )
    .with_category(ErrorCategory::Business)
    .with_severity(ErrorSeverity::Error)
}

/// 创建无效状态错误
///
/// # 参数
/// - `current_state`: 当前状态
/// - `expected_state`: 期望状态
/// - `operation`: 操作名称
///
/// # 返回
/// 包含无效状态信息的错误对象
pub fn invalid_state(current_state: &str, expected_state: &str, operation: &str) -> ErrorInfo {
    ErrorInfo::new(
        INVALID_STATE,
        format!("无效的状态 '{current_state}'，无法执行操作 '{operation}'，期望状态为 '{expected_state}'"),
    )
    .with_category(ErrorCategory::Business)
    .with_severity(ErrorSeverity::Error)
}

/// 创建操作不允许错误
///
/// # 参数
/// - `operation`: 操作名称
/// - `reason`: 不允许的原因
///
/// # 返回
/// 包含操作不允许信息的错误对象
pub fn operation_not_allowed(operation: &str, reason: &str) -> ErrorInfo {
    ErrorInfo::new(
        OPERATION_NOT_ALLOWED,
        format!("操作 '{operation}' 不允许: {reason}"),
    )
    .with_category(ErrorCategory::Business)
    .with_severity(ErrorSeverity::Error)
}

/// 创建配额超出错误
///
/// # 参数
/// - `quota_type`: 配额类型
/// - `limit`: 配额限制
/// - `usage`: 当前使用量
///
/// # 返回
/// 包含配额超出信息的错误对象
pub fn quota_exceeded(quota_type: &str, limit: u64, usage: u64) -> ErrorInfo {
    ErrorInfo::new(
        QUOTA_EXCEEDED,
        format!("{quota_type} 配额超出: 使用 {usage}，限制 {limit}"),
    )
    .with_category(ErrorCategory::Business)
    .with_severity(ErrorSeverity::Error)
}

/// 创建速率限制超出错误
///
/// # 参数
/// - `resource`: 资源名称
/// - `limit`: 速率限制
/// - `window`: 时间窗口（秒）
///
/// # 返回
/// 包含速率限制超出信息的错误对象
pub fn rate_limit_exceeded(resource: &str, limit: u64, window: u64) -> ErrorInfo {
    ErrorInfo::new(
        RATE_LIMIT_EXCEEDED,
        format!("{resource} 速率限制超出: {limit} 次/{window} 秒"),
    )
    .with_category(ErrorCategory::Business)
    .with_severity(ErrorSeverity::Error)
}

/// 创建依赖失败错误
///
/// # 参数
/// - `dependency`: 依赖名称
/// - `reason`: 失败原因
///
/// # 返回
/// 包含依赖失败信息的错误对象
pub fn dependency_failed(dependency: &str, reason: &str) -> ErrorInfo {
    ErrorInfo::new(
        DEPENDENCY_FAILED,
        format!("依赖 '{dependency}' 失败: {reason}"),
    )
    .with_category(ErrorCategory::Business)
    .with_severity(ErrorSeverity::Error)
}

/// 创建业务验证失败错误
///
/// # 参数
/// - `reason`: 验证失败的原因
///
/// # 返回
/// 包含业务验证失败信息的错误对象
pub fn validation_failed(reason: &str) -> ErrorInfo {
    ErrorInfo::new(VALIDATION_FAILED, format!("业务验证失败: {reason}"))
        .with_category(ErrorCategory::Business)
        .with_severity(ErrorSeverity::Error)
}

/// 创建内部业务错误
///
/// # 参数
/// - `message`: 错误消息
///
/// # 返回
/// 包含内部业务错误信息的错误对象
pub fn internal_business_error(message: &str) -> ErrorInfo {
    ErrorInfo::new(INTERNAL_BUSINESS_ERROR, format!("内部业务错误: {message}"))
        .with_category(ErrorCategory::Business)
        .with_severity(ErrorSeverity::Error)
}

/// 创建并发错误
///
/// # 参数
/// - `operation`: 操作名称
/// - `resource`: 资源名称
///
/// # 返回
/// 包含并发错误信息的错误对象
pub fn concurrency_error(operation: &str, resource: &str) -> ErrorInfo {
    ErrorInfo::new(
        CONCURRENCY_ERROR,
        format!("并发操作 '{operation}' 失败，资源 '{resource}' 已被修改"),
    )
    .with_category(ErrorCategory::Business)
    .with_severity(ErrorSeverity::Error)
}

/// 创建通用业务错误
///
/// # 参数
/// - `message`: 错误消息
///
/// # 返回
/// 包含业务错误信息的错误对象
pub fn business_error(message: impl Into<String>) -> ErrorInfo {
    ErrorInfo::new(BUSINESS_ERROR_BASE, message.into())
        .with_category(ErrorCategory::Business)
        .with_severity(ErrorSeverity::Error)
}
