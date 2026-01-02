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

//! Raming 配置类型定义

use serde::{Deserialize, Serialize};

pub mod types;

pub use types::{
    BindingConfig, BindingStats, BindingType, EventConfig, EventPriority, EventStats, MemoryAccess,
    MemoryConfig, MemoryStats, PoolStats,
};

/// Raming 主配置
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RamingConfig {
    /// 内存配置
    #[serde(default)]
    pub memory: MemoryConfig,
    /// 事件配置
    #[serde(default)]
    pub event: EventConfig,
    /// 绑定配置
    #[serde(default)]
    pub binding: BindingConfig,
}
