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

//! 内部管理 API 类型定义
//!
//! 定义内部管理端点使用的数据结构和类型

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::api::types::ApiStatsResponse;

/// 内部系统资源状态响应
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct InternalResourceStatusResponse {
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
    /// 系统控制器是否运行
    pub controller_running: bool,
}

/// 引擎详细状态
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct InternalEngineStatus {
    /// 引擎名称
    pub name: String,
    /// 是否启用
    pub enabled: bool,
    /// 是否临时禁用
    pub temporarily_disabled: bool,
    /// 连续失败次数
    pub consecutive_failures: u32,
    /// 总请求数
    pub total_requests: u32,
    /// 失败请求数
    pub failed_requests: u32,
    /// 成功率
    pub success_rate: f64,
    /// 平均响应时间（毫秒）
    pub avg_response_time_ms: Option<f64>,
    /// 禁用原因
    pub disabled_reason: Option<String>,
    /// 最后更新时间
    pub last_updated: u64,
}

/// 引擎性能指标
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct InternalEnginePerformance {
    /// 引擎名称
    pub engine_name: String,
    /// 延迟（毫秒）
    pub latency_ms: f64,
    /// P50 延迟
    pub p50_latency_ms: f64,
    /// P95 延迟
    pub p95_latency_ms: f64,
    /// P99 延迟
    pub p99_latency_ms: f64,
    /// 请求数
    pub request_count: u64,
    /// 成功数
    pub success_count: u64,
    /// 失败数
    pub failure_count: u64,
    /// 成功率
    pub success_rate: f64,
    /// 每秒请求数
    pub rps: f64,
}

/// 引擎配置更新请求
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct InternalEngineConfigUpdate {
    /// 引擎名称
    pub engine_name: String,
    /// 是否启用
    pub enabled: Option<bool>,
    /// 超时时间（秒）
    pub timeout_seconds: Option<u64>,
    /// 最大重试次数
    pub max_retries: Option<u32>,
    /// 优先级（0-100）
    pub priority: Option<u8>,
}

/// 缓存键列表响应
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct InternalCacheKeysResponse {
    /// 缓存键总数
    pub total_keys: usize,
    /// 缓存键列表（最多返回100个）
    pub keys: Vec<String>,
    /// 缓存大小（字节）
    pub cache_size_bytes: Option<u64>,
}

/// 缓存统计响应
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct InternalCacheStatsResponse {
    /// 总键数
    pub total_keys: usize,
    /// 总大小（字节）
    pub total_size_bytes: u64,
    /// 命中率
    pub hit_rate: f64,
}

/// 系统综合状态响应
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct InternalSystemStatusResponse {
    /// 资源状态
    pub resources: InternalResourceStatusResponse,
    /// 搜索统计
    pub search_stats: ApiStatsResponse,
    /// 引擎状态列表
    pub engine_statuses: Vec<InternalEngineStatus>,
    /// 缓存统计
    pub cache_stats: Option<InternalCacheStatsResponse>,
    /// 系统运行时间（秒）
    pub uptime_seconds: u64,
}

/// 配置获取/更新响应
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct InternalConfigResponse {
    /// 配置类型
    pub config_type: String,
    /// 配置内容
    pub config: serde_json::Value,
    /// 更新时间
    pub updated_at: u64,
}
