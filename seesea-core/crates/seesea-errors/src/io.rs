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

//! IO相关错误定义
//!
//! 包含文件、目录、流等IO操作相关错误的定义和创建函数。

use crate::{ErrorInfo, ErrorSeverity};
use error::ErrorCategory;

/// IO错误码常量
///
/// IO错误码范围：5000-5999
pub const IO_ERROR_BASE: u32 = 5000;
pub const FILE_NOT_FOUND: u32 = IO_ERROR_BASE + 1;
pub const FILE_OPEN_FAILED: u32 = IO_ERROR_BASE + 2;
pub const FILE_READ_FAILED: u32 = IO_ERROR_BASE + 3;
pub const FILE_WRITE_FAILED: u32 = IO_ERROR_BASE + 4;
pub const FILE_PERMISSION_DENIED: u32 = IO_ERROR_BASE + 5;
pub const DIRECTORY_NOT_FOUND: u32 = IO_ERROR_BASE + 6;
pub const DIRECTORY_CREATE_FAILED: u32 = IO_ERROR_BASE + 7;
pub const INVALID_PATH: u32 = IO_ERROR_BASE + 8;
pub const FILE_TOO_LARGE: u32 = IO_ERROR_BASE + 9;
pub const DISK_FULL: u32 = IO_ERROR_BASE + 10;

/// 创建文件不存在错误
///
/// # 参数
/// - `path`: 文件路径
///
/// # 返回
/// 包含文件不存在信息的错误对象
pub fn file_not_found(path: &str) -> ErrorInfo {
    ErrorInfo::new(FILE_NOT_FOUND, format!("文件不存在: {path}"))
        .with_category(ErrorCategory::Io)
        .with_severity(ErrorSeverity::Error)
}

/// 创建文件打开失败错误
///
/// # 参数
/// - `path`: 文件路径
/// - `reason`: 打开失败的原因
///
/// # 返回
/// 包含文件打开失败信息的错误对象
pub fn file_open_failed(path: &str, reason: &str) -> ErrorInfo {
    ErrorInfo::new(FILE_OPEN_FAILED, format!("无法打开文件 '{path}': {reason}"))
        .with_category(ErrorCategory::Io)
        .with_severity(ErrorSeverity::Error)
}

/// 创建文件读取失败错误
///
/// # 参数
/// - `path`: 文件路径
/// - `reason`: 读取失败的原因
///
/// # 返回
/// 包含文件读取失败信息的错误对象
pub fn file_read_failed(path: &str, reason: &str) -> ErrorInfo {
    ErrorInfo::new(FILE_READ_FAILED, format!("无法读取文件 '{path}': {reason}"))
        .with_category(ErrorCategory::Io)
        .with_severity(ErrorSeverity::Error)
}

/// 创建文件写入失败错误
///
/// # 参数
/// - `path`: 文件路径
/// - `reason`: 写入失败的原因
///
/// # 返回
/// 包含文件写入失败信息的错误对象
pub fn file_write_failed(path: &str, reason: &str) -> ErrorInfo {
    ErrorInfo::new(
        FILE_WRITE_FAILED,
        format!("无法写入文件 '{path}': {reason}"),
    )
    .with_category(ErrorCategory::Io)
    .with_severity(ErrorSeverity::Error)
}

/// 创建文件权限被拒绝错误
///
/// # 参数
/// - `path`: 文件路径
/// - `operation`: 被拒绝的操作
///
/// # 返回
/// 包含文件权限被拒绝信息的错误对象
pub fn file_permission_denied(path: &str, operation: &str) -> ErrorInfo {
    ErrorInfo::new(
        FILE_PERMISSION_DENIED,
        format!("{operation} 文件 '{path}' 权限被拒绝"),
    )
    .with_category(ErrorCategory::Io)
    .with_severity(ErrorSeverity::Error)
}

/// 创建目录不存在错误
///
/// # 参数
/// - `path`: 目录路径
///
/// # 返回
/// 包含目录不存在信息的错误对象
pub fn directory_not_found(path: &str) -> ErrorInfo {
    ErrorInfo::new(DIRECTORY_NOT_FOUND, format!("目录不存在: {path}"))
        .with_category(ErrorCategory::Io)
        .with_severity(ErrorSeverity::Error)
}

/// 创建目录创建失败错误
///
/// # 参数
/// - `path`: 目录路径
/// - `reason`: 创建失败的原因
///
/// # 返回
/// 包含目录创建失败信息的错误对象
pub fn directory_create_failed(path: &str, reason: &str) -> ErrorInfo {
    ErrorInfo::new(
        DIRECTORY_CREATE_FAILED,
        format!("无法创建目录 '{path}': {reason}"),
    )
    .with_category(ErrorCategory::Io)
    .with_severity(ErrorSeverity::Error)
}

/// 创建无效路径错误
///
/// # 参数
/// - `path`: 无效的路径
///
/// # 返回
/// 包含无效路径信息的错误对象
pub fn invalid_path(path: &str) -> ErrorInfo {
    ErrorInfo::new(INVALID_PATH, format!("无效的路径: {path}"))
        .with_category(ErrorCategory::Io)
        .with_severity(ErrorSeverity::Error)
}

/// 创建文件过大错误
///
/// # 参数
/// - `path`: 文件路径
/// - `size`: 文件大小（字节）
/// - `max_size`: 最大允许大小（字节）
///
/// # 返回
/// 包含文件过大信息的错误对象
pub fn file_too_large(path: &str, size: u64, max_size: u64) -> ErrorInfo {
    ErrorInfo::new(
        FILE_TOO_LARGE,
        format!("文件 '{path}' 过大: {size} 字节, 最大允许 {max_size} 字节"),
    )
    .with_category(ErrorCategory::Io)
    .with_severity(ErrorSeverity::Error)
}

/// 创建磁盘空间不足错误
///
/// # 参数
/// - `path`: 操作路径
/// - `required`: 需要的磁盘空间（字节）
/// - `available`: 可用磁盘空间（字节）
///
/// # 返回
/// 包含磁盘空间不足信息的错误对象
pub fn disk_full(path: &str, required: u64, available: u64) -> ErrorInfo {
    ErrorInfo::new(
        DISK_FULL,
        format!("磁盘空间不足: 路径 '{path}' 需要 {required} 字节, 可用 {available} 字节"),
    )
    .with_category(ErrorCategory::Io)
    .with_severity(ErrorSeverity::Error)
}

/// 通用IO错误创建函数
///
/// # 参数
/// - `message`: IO错误的详细信息
///
/// # 返回
/// 包含IO错误信息的错误对象
pub fn io_error(message: impl Into<String>) -> ErrorInfo {
    ErrorInfo::new(IO_ERROR_BASE, message.into())
        .with_category(ErrorCategory::Io)
        .with_severity(ErrorSeverity::Error)
}
