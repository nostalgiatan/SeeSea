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

//! 股票API响应类型定义
//!
//! 对应Python端的统一API响应格式，确保Rust端能够正确解析和处理

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 统一API响应格式 - 对应Python端的ApiResponse
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StockApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,