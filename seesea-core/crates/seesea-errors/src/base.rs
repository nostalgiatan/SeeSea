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

//! 基础错误类型定义
//!
//! 提供统一的错误基础类型和枚举。

use serde::{Deserialize, Serialize};
use std::fmt;

// 重新导出 error crate 中的核心类型
pub use error::{ErrorInfo, ErrorKind, ErrorSeverity, Result};

/// 错误类别枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ErrorCategory {
    /// 网络相关错误
    Network,
    /// 搜索相关错误
    Search,
    /// 解析相关错误
    Parse,
    /// 验证相关错误
    Validation,
    /// IO相关错误
    Io,
    /// 权限相关错误
    Permission,
    /// 配置相关错误
    Configuration,
    /// 数据库相关错误
    Database,
    /// 业务逻辑相关错误
    Business,
    /// 系统相关错误
    System,
    /// 其他错误
    Other,
}

impl fmt::Display for ErrorCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ErrorCategory::Network => write!(f, "Network"),
            ErrorCategory::Search => write!(f, "Search"),
            ErrorCategory::Parse => write!(f, "Parse"),
            ErrorCategory::Validation => write!(f, "Validation"),
            ErrorCategory::Io => write!(f, "IO"),
            ErrorCategory::Permission => write!(f, "Permission"),
            ErrorCategory::Configuration => write!(f, "Configuration"),
            ErrorCategory::Database => write!(f, "Database"),
            ErrorCategory::Business => write!(f, "Business"),
            ErrorCategory::System => write!(f, "System"),
            ErrorCategory::Other => write!(f, "Other"),
        }
    }
}
