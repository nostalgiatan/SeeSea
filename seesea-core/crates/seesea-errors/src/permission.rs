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

//! 权限相关错误定义
//!
//! 包含访问权限、认证、授权等相关错误的定义和创建函数。

use crate::{ErrorInfo, ErrorSeverity};
use error::ErrorCategory;

/// 权限错误码常量
///
/// 权限错误码范围：6000-6999
pub const PERMISSION_ERROR_BASE: u32 = 6000;
pub const PERMISSION_DENIED: u32 = PERMISSION_ERROR_BASE + 1;
pub const UNAUTHORIZED: u32 = PERMISSION_ERROR_BASE + 2;
pub const INVALID_CREDENTIALS: u32 = PERMISSION_ERROR_BASE + 3;
pub const TOKEN_EXPIRED: u32 = PERMISSION_ERROR_BASE + 4;
pub const INVALID_TOKEN: u32 = PERMISSION_ERROR_BASE + 5;
pub const MISSING_TOKEN: u32 = PERMISSION_ERROR_BASE + 6;
pub const ACCOUNT_LOCKED: u32 = PERMISSION_ERROR_BASE + 7;
pub const ACCOUNT_DISABLED: u32 = PERMISSION_ERROR_BASE + 8;
pub const INSUFFICIENT_ROLE: u32 = PERMISSION_ERROR_BASE + 9;
pub const ACCESS_DENIED: u32 = PERMISSION_ERROR_BASE + 10;

/// 创建权限被拒绝错误
///
/// # 参数
/// - `resource`: 资源名称
/// - `operation`: 操作类型
///
/// # 返回
/// 包含权限被拒绝信息的错误对象
pub fn permission_denied(resource: &str, operation: &str) -> ErrorInfo {
    ErrorInfo::new(
        PERMISSION_DENIED,
        format!("拒绝访问资源 '{resource}' 的 '{operation}' 操作"),
    )
    .with_category(ErrorCategory::Permission)
    .with_severity(ErrorSeverity::Error)
}

/// 创建未授权访问错误
///
/// # 返回
/// 包含未授权访问信息的错误对象
pub fn unauthorized() -> ErrorInfo {
    ErrorInfo::new(UNAUTHORIZED, "未授权访问".to_string())
        .with_category(ErrorCategory::Permission)
        .with_severity(ErrorSeverity::Error)
}

/// 创建无效凭证错误
///
/// # 返回
/// 包含无效凭证信息的错误对象
pub fn invalid_credentials() -> ErrorInfo {
    ErrorInfo::new(INVALID_CREDENTIALS, "无效的用户名或密码".to_string())
        .with_category(ErrorCategory::Permission)
        .with_severity(ErrorSeverity::Error)
}

/// 创建令牌过期错误
///
/// # 返回
/// 包含令牌过期信息的错误对象
pub fn token_expired() -> ErrorInfo {
    ErrorInfo::new(TOKEN_EXPIRED, "认证令牌已过期".to_string())
        .with_category(ErrorCategory::Permission)
        .with_severity(ErrorSeverity::Error)
}

/// 创建无效令牌错误
///
/// # 返回
/// 包含无效令牌信息的错误对象
pub fn invalid_token() -> ErrorInfo {
    ErrorInfo::new(INVALID_TOKEN, "无效的认证令牌".to_string())
        .with_category(ErrorCategory::Permission)
        .with_severity(ErrorSeverity::Error)
}

/// 创建缺少令牌错误
///
/// # 返回
/// 包含缺少令牌信息的错误对象
pub fn missing_token() -> ErrorInfo {
    ErrorInfo::new(MISSING_TOKEN, "缺少认证令牌".to_string())
        .with_category(ErrorCategory::Permission)
        .with_severity(ErrorSeverity::Error)
}

/// 创建账户锁定错误
///
/// # 参数
/// - `username`: 被锁定的用户名
///
/// # 返回
/// 包含账户锁定信息的错误对象
pub fn account_locked(username: &str) -> ErrorInfo {
    ErrorInfo::new(ACCOUNT_LOCKED, format!("账户 '{username}' 已被锁定"))
        .with_category(ErrorCategory::Permission)
        .with_severity(ErrorSeverity::Error)
}

/// 创建账户禁用错误
///
/// # 参数
/// - `username`: 被禁用的用户名
///
/// # 返回
/// 包含账户禁用信息的错误对象
pub fn account_disabled(username: &str) -> ErrorInfo {
    ErrorInfo::new(ACCOUNT_DISABLED, format!("账户 '{username}' 已被禁用"))
        .with_category(ErrorCategory::Permission)
        .with_severity(ErrorSeverity::Error)
}

/// 创建角色权限不足错误
///
/// # 参数
/// - `role`: 当前角色
/// - `required_role`: 所需角色
///
/// # 返回
/// 包含角色权限不足信息的错误对象
pub fn insufficient_role(role: &str, required_role: &str) -> ErrorInfo {
    ErrorInfo::new(
        INSUFFICIENT_ROLE,
        format!("角色 '{role}' 权限不足，需要 '{required_role}' 角色"),
    )
    .with_category(ErrorCategory::Permission)
    .with_severity(ErrorSeverity::Error)
}

/// 创建访问被拒绝错误
///
/// # 参数
/// - `reason`: 拒绝原因
///
/// # 返回
/// 包含访问被拒绝信息的错误对象
pub fn access_denied(reason: &str) -> ErrorInfo {
    ErrorInfo::new(ACCESS_DENIED, format!("访问被拒绝: {reason}"))
        .with_category(ErrorCategory::Permission)
        .with_severity(ErrorSeverity::Error)
}

/// 通用权限错误创建函数
///
/// # 参数
/// - `message`: 权限错误的详细信息
///
/// # 返回
/// 包含权限错误信息的错误对象
pub fn permission_error(message: impl Into<String>) -> ErrorInfo {
    ErrorInfo::new(PERMISSION_ERROR_BASE, message.into())
        .with_category(ErrorCategory::Permission)
        .with_severity(ErrorSeverity::Error)
}
