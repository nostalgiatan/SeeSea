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

//! 连接池配置类型

use serde::{Deserialize, Serialize};

/// 连接池配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolConfig {
    /// 最大空闲连接数
    pub max_idle_connections: usize,
    /// 每个主机的最大连接数
    pub max_connections_per_host: usize,
    /// 空闲连接超时时间（秒）
    pub idle_timeout_secs: u64,
    /// 连接超时时间（秒）
    pub connect_timeout_secs: u64,
    /// 读取超时时间（秒）
    pub read_timeout_secs: u64,
    /// 写入超时时间（秒）
    pub write_timeout_secs: u64,
    /// 是否启用 HTTP/2
    pub http2_only: bool,
    /// 是否启用 TCP_NODELAY
    pub tcp_nodelay: bool,
    /// TCP 保活间隔（秒）
    pub tcp_keepalive_interval_secs: Option<u64>,
    /// TCP 保活重试次数
    pub tcp_keepalive_retries: Option<u32>,
    /// 健康检查间隔（秒）
    pub health_check_interval_secs: Option<u64>,
    /// 连接最大生命周期（秒）
    pub max_lifetime_secs: Option<u64>,
    /// 连接获取超时时间（秒）
    pub connection_acquisition_timeout_secs: u64,
    /// 是否启用连接泄漏检测
    pub connection_leak_detection_enabled: bool,
    /// 连接泄漏检测阈值（秒）
    pub connection_leak_detection_threshold_secs: u64,
    /// 是否启用连接池统计
    pub pool_stats_enabled: bool,
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self {
            max_idle_connections: 100,
            max_connections_per_host: 6,
            idle_timeout_secs: 90,
            connect_timeout_secs: 30,
            read_timeout_secs: 30,
            write_timeout_secs: 30,
            http2_only: false,
            tcp_nodelay: true,
            tcp_keepalive_interval_secs: Some(60),
            tcp_keepalive_retries: Some(3),
            health_check_interval_secs: Some(30),
            max_lifetime_secs: Some(300),
            connection_acquisition_timeout_secs: 10,
            connection_leak_detection_enabled: true,
            connection_leak_detection_threshold_secs: 30,
            pool_stats_enabled: true,
        }
    }
}
