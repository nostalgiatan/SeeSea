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

//! 系统调控中心模块
//!
//! 职责范围：
//! - 统一管理系统资源，包括CPU、内存、磁盘、网络等
//! - 实现基于优先级的资源分配机制
//! - 提供动态调整各个组件参数的接口
//! - 监控系统资源使用情况
//! - 运行守护进程，确保系统稳定运行
//!
//! 期望实现计划：
//! 1. 实现资源监控功能，实时获取系统资源使用情况
//! 2. 设计并实现优先级调控机制
//! 3. 实现守护进程，监控系统状态并进行自动调整
//! 4. 提供统一的API接口，供各个组件调用
//! 5. 与现有组件集成，替换分散的动态调整机制
//!
//! 已实现功能：
//! - 模块结构搭建
//!
//! 使用依赖：
//! - tracing: 日志记录
//! - serde: 序列化/反序列化
//! - std::sync: 同步原语
//!
//! 主要接口：
//! - SystemController: 系统控制器，提供资源调控的核心功能
//! - ResourceMonitor: 资源监控器，实时获取系统资源使用情况
//! - PriorityManager: 优先级管理器，管理各个组件的优先级
//! - Daemon: 守护进程，监控系统状态并进行自动调整

pub mod config;
pub mod controller;
pub mod daemon;
pub mod priority;
pub mod resource;
pub mod types;

// 重新导出常用类型和接口
pub use self::controller::SystemController;
pub use self::daemon::Daemon;
pub use self::priority::PriorityManager;
pub use self::resource::ResourceMonitor;
pub use self::types::*;
