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

//! # 系统控制模块
//!
//! 提供系统资源管理、守护进程控制等功能。

pub mod config;
pub mod controller;
pub mod daemon;
pub mod priority;
pub mod resource;
pub mod types;

// 重新导出常用类型
pub use controller::{
    SystemController, get_global_system_controller, get_or_create_runtime,
    get_process_uptime_seconds, spawn_runtime_task,
};
pub use types::{ComponentId, ComponentType, ResourceType};
