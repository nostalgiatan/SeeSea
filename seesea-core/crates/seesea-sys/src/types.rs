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

//! 系统调控中心类型定义

use serde::{Deserialize, Serialize};

/// 组件类型枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub enum ComponentType {
    /// 向量存储组件
    VectorStore,
    /// 缓存组件
    Cache,
    /// 索引组件
    Index,
    /// 查询处理器组件
    QueryProcessor,
    /// 文档处理器组件
    DocumentProcessor,
    /// 网络组件
    Network,
    /// Pro处理器组件
    ProProcessor,
    /// Crawl4AI组件
    Crawl4Ai,
    /// 其他组件
    Other,
}

/// 组件ID，用于标识不同的组件实例
#[derive(Debug, Clone, Serialize, Deserialize, Hash, PartialEq, Eq)]
pub struct ComponentId {
    /// 组件类型
    pub component_type: ComponentType,
    /// 组件实例名称
    pub name: String,
}

impl ComponentId {
    /// 创建新的组件ID
    pub fn new(component_type: ComponentType, name: impl Into<String>) -> Self {
        Self {
            component_type,
            name: name.into(),
        }
    }
}

/// 资源类型枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResourceType {
    /// CPU资源
    Cpu,
    /// 内存资源
    Memory,
    /// 磁盘I/O资源
    DiskIo,
    /// 网络I/O资源
    NetworkIo,
}

/// 资源状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceStatus {
    /// CPU使用率（0.0-1.0）
    pub cpu_usage: f64,
    /// 内存使用率（0.0-1.0）
    pub memory_usage: f64,
    /// 磁盘I/O使用率（0.0-1.0）
    pub disk_io_usage: f64,
    /// 网络I/O使用率（0.0-1.0）
    pub network_io_usage: f64,
    /// 可用内存（字节）
    pub available_memory: u64,
    /// 可用磁盘空间（字节）
    pub available_disk: u64,
    /// 磁盘总空间（字节）
    pub total_disk: u64,
    /// 系统负载平均值（1分钟）
    pub load_avg_1: f64,
    /// 系统负载平均值（5分钟）
    pub load_avg_5: f64,
    /// 系统负载平均值（15分钟）
    pub load_avg_15: f64,
    /// 磁盘空间使用率（0.0-1.0）
    pub disk_usage_percent: f64,
}

impl Default for ResourceStatus {
    fn default() -> Self {
        Self {
            cpu_usage: 0.0,
            memory_usage: 0.0,
            disk_io_usage: 0.0,
            network_io_usage: 0.0,
            available_memory: 0,
            available_disk: 0,
            total_disk: 0,
            load_avg_1: 0.0,
            load_avg_5: 0.0,
            load_avg_15: 0.0,
            disk_usage_percent: 0.0,
        }
    }
}

/// 组件配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentConfig {
    /// 组件ID
    pub id: ComponentId,
    /// 优先级（0-100，数值越大优先级越高）
    pub priority: u8,
    /// 最大资源使用率限制（0.0-1.0）
    pub max_resource_usage: f64,
    /// 最小资源分配（0.0-1.0）
    pub min_resource_allocation: f64,
    /// 是否启用动态调整
    pub enable_dynamic_adjustment: bool,
    /// 动态调整的参数配置
    pub adjustment_params: serde_json::Value,
}

/// 系统控制器配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemControllerConfig {
    /// 是否启用系统控制器
    pub enabled: bool,
    /// 资源监控间隔（毫秒）
    pub monitoring_interval_ms: u64,
    /// 动态调整间隔（毫秒）
    pub adjustment_interval_ms: u64,
    /// 资源使用率阈值（超过该阈值则进行调整）
    pub resource_threshold: f64,
    /// 优先级调整因子（用于计算优先级对资源分配的影响）
    pub priority_factor: f64,
    /// 默认组件配置
    pub default_component_config: ComponentConfig,
}

impl Default for SystemControllerConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            monitoring_interval_ms: 1000,
            adjustment_interval_ms: 5000,
            resource_threshold: 0.8,
            priority_factor: 0.5,
            default_component_config: ComponentConfig {
                id: ComponentId::new(ComponentType::Other, "default"),
                priority: 50,
                max_resource_usage: 0.8,
                min_resource_allocation: 0.1,
                enable_dynamic_adjustment: true,
                adjustment_params: serde_json::Value::Null,
            },
        }
    }
}

/// 守护进程配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaemonConfig {
    /// 是否启用守护进程
    pub enabled: bool,
    /// 检查间隔（毫秒）
    pub check_interval_ms: u64,
    /// 重启延迟（毫秒）
    pub restart_delay_ms: u64,
    /// 最大重启次数
    pub max_restart_attempts: u32,
    /// 是否自动修复问题
    pub auto_fix: bool,
}

impl Default for DaemonConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            check_interval_ms: 10000,
            restart_delay_ms: 5000,
            max_restart_attempts: 5,
            auto_fix: true,
        }
    }
}

/// 动态调整请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdjustmentRequest {
    /// 组件ID
    pub component_id: ComponentId,
    /// 请求的调整类型
    pub adjustment_type: AdjustmentType,
    /// 调整参数
    pub params: serde_json::Value,
}

/// 调整类型枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AdjustmentType {
    /// 增加资源分配
    IncreaseResource,
    /// 减少资源分配
    DecreaseResource,
    /// 调整优先级
    AdjustPriority,
    /// 调整批量大小
    AdjustBatchSize,
    /// 调整并发数量
    AdjustConcurrency,
    /// 调整其他参数
    AdjustOther,
}

/// 调整响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdjustmentResponse {
    /// 是否成功
    pub success: bool,
    /// 调整后的参数
    pub adjusted_params: serde_json::Value,
    /// 调整原因
    pub reason: String,
}

/// 系统状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemStatus {
    /// 资源状态
    pub resource_status: ResourceStatus,
    /// 组件状态列表
    pub component_statuses: Vec<ComponentStatus>,
    /// 系统控制器是否运行
    pub controller_running: bool,
    /// 守护进程是否运行
    pub daemon_running: bool,
}

/// 组件状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentStatus {
    /// 组件ID
    pub component_id: ComponentId,
    /// 组件当前资源使用率（0.0-1.0）
    pub current_resource_usage: f64,
    /// 组件当前优先级
    pub current_priority: u8,
    /// 组件当前配置参数
    pub current_params: serde_json::Value,
    /// 组件是否处于健康状态
    pub healthy: bool,
    /// 上次调整时间戳
    pub last_adjustment_timestamp: u64,
}
