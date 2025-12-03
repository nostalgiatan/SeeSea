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
//! 从 error crate 重新导出核心错误类型，提供统一的错误基础。

// 从 error crate 重新导出核心错误类型
pub use error::{ErrorCategory, ErrorInfo, ErrorKind, ErrorSeverity};

/// Result 类型别名
pub type Result<T> = std::result::Result<T, ErrorInfo>;
