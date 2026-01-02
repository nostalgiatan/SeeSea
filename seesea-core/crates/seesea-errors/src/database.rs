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
//! 数据库相关错误定义
//!
//! 包含数据库连接、查询、事务等相关错误的定义和创建函数。
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

use crate::{ErrorInfo, ErrorSeverity};
use error::ErrorCategory;

/// 数据库错误码常量
///
/// 数据库错误码范围：8000-8999
pub const DATABASE_ERROR_BASE: u32 = 8000;
pub const CONNECTION_FAILED: u32 = DATABASE_ERROR_BASE + 1;
pub const QUERY_FAILED: u32 = DATABASE_ERROR_BASE + 2;
pub const TRANSACTION_FAILED: u32 = DATABASE_ERROR_BASE + 3;
pub const DUPLICATE_KEY: u32 = DATABASE_ERROR_BASE + 4;
pub const FOREIGN_KEY_VIOLATION: u32 = DATABASE_ERROR_BASE + 5;
pub const TABLE_NOT_FOUND: u32 = DATABASE_ERROR_BASE + 6;
pub const COLUMN_NOT_FOUND: u32 = DATABASE_ERROR_BASE + 7;
pub const DATABASE_LOCKED: u32 = DATABASE_ERROR_BASE + 8;
pub const DATABASE_FULL: u32 = DATABASE_ERROR_BASE + 9;
pub const INVALID_SQL: u32 = DATABASE_ERROR_BASE + 10;

/// 创建数据库连接失败错误
///
/// # 参数
/// - `dsn`: 数据库连接字符串
/// - `reason`: 连接失败的原因
///
/// # 返回
/// 包含数据库连接失败信息的错误对象
pub fn connection_failed(dsn: &str, reason: &str) -> ErrorInfo {
    ErrorInfo::new(
        CONNECTION_FAILED,
        format!("数据库连接失败 ({dsn}): {reason}"),
    )
    .with_category(ErrorCategory::Database)
    .with_severity(ErrorSeverity::Error)
}

/// 创建SQL查询失败错误
///
/// # 参数
/// - `query`: SQL查询语句
/// - `reason`: 查询失败的原因
///
/// # 返回
/// 包含SQL查询失败信息的错误对象
pub fn query_failed(query: &str, reason: &str) -> ErrorInfo {
    ErrorInfo::new(
        QUERY_FAILED,
        format!("SQL查询失败: {query} (原因: {reason})"),
    )
    .with_category(ErrorCategory::Database)
    .with_severity(ErrorSeverity::Error)
}

/// 创建事务失败错误
///
/// # 参数
/// - `operation`: 事务操作名称
/// - `reason`: 事务失败的原因
///
/// # 返回
/// 包含事务失败信息的错误对象
pub fn transaction_failed(operation: &str, reason: &str) -> ErrorInfo {
    ErrorInfo::new(
        TRANSACTION_FAILED,
        format!("事务 '{operation}' 失败: {reason}"),
    )
    .with_category(ErrorCategory::Database)
    .with_severity(ErrorSeverity::Error)
}

/// 创建重复键错误
///
/// # 参数
/// - `table`: 表名
/// - `key`: 键名
/// - `value`: 重复的值
///
/// # 返回
/// 包含重复键信息的错误对象
pub fn duplicate_key(table: &str, key: &str, value: &str) -> ErrorInfo {
    ErrorInfo::new(
        DUPLICATE_KEY,
        format!("表 '{table}' 中键 '{key}' 的值 '{value}' 已存在"),
    )
    .with_category(ErrorCategory::Database)
    .with_severity(ErrorSeverity::Error)
}

/// 创建外键约束违反错误
///
/// # 参数
/// - `table`: 表名
/// - `column`: 列名
/// - `value`: 违反约束的值
///
/// # 返回
/// 包含外键约束违反信息的错误对象
pub fn foreign_key_violation(table: &str, column: &str, value: &str) -> ErrorInfo {
    ErrorInfo::new(
        FOREIGN_KEY_VIOLATION,
        format!("表 '{table}' 中列 '{column}' 的值 '{value}' 违反外键约束"),
    )
    .with_category(ErrorCategory::Database)
    .with_severity(ErrorSeverity::Error)
}

/// 创建表未找到错误
///
/// # 参数
/// - `table`: 表名
///
/// # 返回
/// 包含表未找到信息的错误对象
pub fn table_not_found(table: &str) -> ErrorInfo {
    ErrorInfo::new(TABLE_NOT_FOUND, format!("表 '{table}' 不存在"))
        .with_category(ErrorCategory::Database)
        .with_severity(ErrorSeverity::Error)
}

/// 创建列未找到错误
///
/// # 参数
/// - `table`: 表名
/// - `column`: 列名
///
/// # 返回
/// 包含列未找到信息的错误对象
pub fn column_not_found(table: &str, column: &str) -> ErrorInfo {
    ErrorInfo::new(
        COLUMN_NOT_FOUND,
        format!("表 '{table}' 中列 '{column}' 不存在"),
    )
    .with_category(ErrorCategory::Database)
    .with_severity(ErrorSeverity::Error)
}

/// 创建数据库锁定错误
///
/// # 参数
/// - `resource`: 被锁定的资源
///
/// # 返回
/// 包含数据库锁定信息的错误对象
pub fn database_locked(resource: &str) -> ErrorInfo {
    ErrorInfo::new(DATABASE_LOCKED, format!("数据库资源 '{resource}' 已被锁定"))
        .with_category(ErrorCategory::Database)
        .with_severity(ErrorSeverity::Error)
}

/// 创建数据库空间不足错误
///
/// # 返回
/// 包含数据库空间不足信息的错误对象
pub fn database_full() -> ErrorInfo {
    ErrorInfo::new(DATABASE_FULL, "数据库空间不足".to_string())
        .with_category(ErrorCategory::Database)
        .with_severity(ErrorSeverity::Error)
}

/// 创建无效SQL错误
///
/// # 参数
/// - `sql`: SQL语句
/// - `reason`: 无效的原因
///
/// # 返回
/// 包含无效SQL信息的错误对象
pub fn invalid_sql(sql: &str, reason: &str) -> ErrorInfo {
    ErrorInfo::new(
        INVALID_SQL,
        format!("无效的SQL语句: {sql} (原因: {reason})"),
    )
    .with_category(ErrorCategory::Database)
    .with_severity(ErrorSeverity::Error)
}

/// 创建通用数据库错误
///
/// # 参数
/// - `message`: 错误消息
///
/// # 返回
/// 包含数据库错误信息的错误对象
pub fn database_error(message: impl Into<String>) -> ErrorInfo {
    ErrorInfo::new(DATABASE_ERROR_BASE, message.into())
        .with_category(ErrorCategory::Database)
        .with_severity(ErrorSeverity::Error)
}
