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

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use uuid::Uuid;

/// 内存配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryConfig {
    /// 最大内存段大小（字节）
    pub max_segment_size: usize,
    /// 最大内存段数量
    pub max_segments: usize,
    /// 内存对齐大小
    pub alignment: usize,
    /// 是否启用内存映射
    pub enable_mmap: bool,
    /// 内存池大小
    pub pool_size: usize,
    /// 清理间隔
    pub cleanup_interval: Duration,
    /// 内存使用限制（百分比）
    pub memory_limit_percent: f32,
}

impl Default for MemoryConfig {
    fn default() -> Self {
        Self {
            max_segment_size: 1024 * 1024 * 100,
            max_segments: 1000,
            alignment: 64,
            enable_mmap: true,
            pool_size: 1024 * 1024 * 10,
            cleanup_interval: Duration::from_secs(300),
            memory_limit_percent: 0.8,
        }
    }
}

/// 事件配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventConfig {
    /// 事件队列大小
    pub queue_size: usize,
    /// 事件处理超时
    pub handler_timeout: Duration,
    /// 最大事件监听器数量
    pub max_listeners: usize,
    /// 是否启用事件压缩
    pub enable_compression: bool,
    /// 事件历史保留时间
    pub history_retention: Duration,
    /// 异步处理模式
    pub async_mode: bool,
}

impl Default for EventConfig {
    fn default() -> Self {
        Self {
            queue_size: 10000,
            handler_timeout: Duration::from_secs(30),
            max_listeners: 1000,
            enable_compression: true,
            history_retention: Duration::from_secs(3600),
            async_mode: true,
        }
    }
}

/// 绑定类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BindingType {
    /// 内存共享绑定
    MemoryShare,
    /// 事件监听绑定
    EventListener,
    /// 双向绑定
    Bidirectional,
    /// 单向发布绑定
    Publisher,
    /// 单向订阅绑定
    Subscriber,
}

/// 内存段信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemorySegmentInfo {
    /// 段ID
    pub id: Uuid,
    /// 段名称
    pub name: String,
    /// 段大小
    pub size: usize,
    /// 创建时间
    pub created_at: DateTime<Utc>,
    /// 最后访问时间
    pub last_accessed: DateTime<Utc>,
    /// 访问次数
    pub access_count: u64,
    /// 是否只读
    pub read_only: bool,
    /// 引用计数
    pub ref_count: u32,
}

/// 事件信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventInfo {
    /// 事件ID
    pub id: Uuid,
    /// 事件名称
    pub name: String,
    /// 事件类型
    pub event_type: String,
    /// 创建时间
    pub created_at: DateTime<Utc>,
    /// 最后触发时间
    pub last_triggered: Option<DateTime<Utc>>,
    /// 触发次数
    pub trigger_count: u64,
    /// 监听器数量
    pub listener_count: usize,
}

/// 内存统计信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryStats {
    /// 总内存使用
    pub total_memory: usize,
    /// 活动段数量
    pub active_segments: usize,
    /// 总段数量
    pub total_segments: usize,
    /// 缓存命中率
    pub cache_hit_rate: f32,
    /// 平均访问时间（毫秒）
    pub avg_access_time_ms: f64,
    /// 内存池使用
    pub pool_usage: usize,
    /// 总分配内存
    pub total_allocated: usize,
}

/// 内存池统计信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolStats {
    /// 总块数
    pub total_blocks: usize,
    /// 空闲块数
    pub free_blocks: usize,
    /// 使用块数
    pub used_blocks: usize,
    /// 分配次数
    pub allocation_count: u64,
    /// 释放次数
    pub deallocation_count: u64,
    /// 缓存命中率
    pub cache_hit_rate: f64,
    /// 平均分配时间（毫秒）
    pub avg_allocation_time_ms: f64,
}

impl Default for PoolStats {
    fn default() -> Self {
        Self {
            total_blocks: 0,
            free_blocks: 0,
            used_blocks: 0,
            allocation_count: 0,
            deallocation_count: 0,
            cache_hit_rate: 0.0,
            avg_allocation_time_ms: 0.0,
        }
    }
}

/// 事件统计信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventStats {
    /// 总事件数量
    pub total_events: usize,
    /// 活跃监听器数量
    pub active_listeners: usize,
    /// 事件处理速率（每秒）
    pub processing_rate: f64,
    /// 平均处理时间（毫秒）
    pub avg_processing_time_ms: f64,
    /// 队列长度
    pub queue_length: usize,
    /// 丢弃事件数量
    pub dropped_events: u64,
    /// 发布的事件数量
    pub published_events: usize,
    /// 成功处理的事件数量
    pub successful_events: usize,
    /// 失败的事件数量
    pub failed_events: usize,
    /// 超时的事件数量
    pub timed_out_events: usize,
    /// 总处理时间
    pub total_processing_time: Duration,
}

impl Default for EventStats {
    fn default() -> Self {
        Self {
            total_events: 0,
            active_listeners: 0,
            processing_rate: 0.0,
            avg_processing_time_ms: 0.0,
            queue_length: 0,
            dropped_events: 0,
            published_events: 0,
            successful_events: 0,
            failed_events: 0,
            timed_out_events: 0,
            total_processing_time: Duration::from_secs(0),
        }
    }
}

/// 内存访问权限
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MemoryAccess {
    /// 只读访问
    ReadOnly,
    /// 读写访问
    ReadWrite,
    /// 独占访问
    Exclusive,
}

/// 事件优先级
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Default)]
pub enum EventPriority {
    /// 最低优先级
    Low = 0,
    /// 普通优先级
    #[default]
    Normal = 1,
    /// 高优先级
    High = 2,
    /// 最高优先级
    Critical = 3,
}

/// 绑定配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BindingConfig {
    /// 最大绑定数量
    pub max_bindings: usize,
    /// 绑定超时时间
    pub binding_timeout: Duration,
    /// 自动清理间隔
    pub cleanup_interval: Duration,
    /// 是否启用持久化
    pub enable_persistence: bool,
    /// 持久化路径
    pub persistence_path: Option<String>,
    /// 是否启用监控
    pub enable_monitoring: bool,
    /// 监控间隔
    pub monitoring_interval: Duration,
}

impl Default for BindingConfig {
    fn default() -> Self {
        Self {
            max_bindings: 10000,
            binding_timeout: Duration::from_secs(3600),
            cleanup_interval: Duration::from_secs(300),
            enable_persistence: false,
            persistence_path: None,
            enable_monitoring: true,
            monitoring_interval: Duration::from_secs(60),
        }
    }
}

/// 绑定统计信息
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BindingStats {
    /// 成功次数
    pub success_count: u64,
    /// 错误次数
    pub error_count: u64,
    /// 总执行次数
    pub total_count: u64,
    /// 总执行时间
    pub total_duration: Duration,
}
