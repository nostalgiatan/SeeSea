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

//! 基础错误类型定义
//! 
//! 从 error crate 重新导出核心错误类型，提供统一的错误基础。

// 从 error crate 重新导出核心错误类型
pub use error::{ErrorInfo, ErrorKind, ErrorSeverity, ErrorCategory};

/// Result 类型别名
pub type Result<T> = std::result::Result<T, ErrorInfo>;