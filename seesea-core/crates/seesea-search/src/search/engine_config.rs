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

//! 搜索引擎配置管理
//!
//! 使用集中配置系统管理搜索引擎配置

pub use seesea_config::{EngineListConfig, EngineMode};

// 全局引擎配置实例
lazy_static::lazy_static! {
    pub static ref ENGINE_CONFIG: EngineListConfig = EngineListConfig::default();
}
