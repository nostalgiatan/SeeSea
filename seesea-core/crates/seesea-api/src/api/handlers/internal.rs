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

//! 内部管理处理器
//!
//! 处理内部管理相关的 API 请求，仅限本地访问

use axum::{
    extract::{Json, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use serde_json::json;

use crate::api::internal_types::*;
use crate::api::on::ApiState;
use crate::api::types::ApiStatsResponse;

/// 检查请求是否来自本地
fn is_local_request(headers: &HeaderMap) -> bool {
    let host = headers
        .get("host")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("");

    let x_forwarded_for = headers.get("x-forwarded-for").and_then(|h| h.to_str().ok());

    let x_real_ip = headers.get("x-real-ip").and_then(|h| h.to_str().ok());

    // 检查是否为本地地址
    host.contains("127.0.0.1")
        || host.contains("localhost")
        || host.contains("[::1]")
        || host.starts_with("0.0.0.0:")
        || x_forwarded_for.is_some_and(|ip| {
            ip.contains("127.0.0.1") || ip.contains("localhost") || ip.contains("[::1]")
        })
        || x_real_ip.is_some_and(|ip| {
            ip.contains("127.0.0.1") || ip.contains("localhost") || ip.contains("[::1]")
        })
}

/// 处理系统资源状态请求
#[utoipa::path(
    get,
    path = "/internal/system/resources",
    responses(
        (status = 200, description = "获取成功", body = InternalResourceStatusResponse),
        (status = 403, description = "仅限本地访问"),
    ),
    tag = "internal"
)]
pub async fn handle_system_resources(
    headers: HeaderMap,
    State(_state): State<ApiState>,
) -> Response {
    // 检查是否为本地请求
    if !is_local_request(&headers) {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({"error": "仅限本地访问"})),
        )
            .into_response();
    }

    // 从系统控制器获取资源状态
    use seesea_sys::get_global_system_controller;
    let controller = get_global_system_controller();
    let system_status = controller.get_system_status().await;

    let resource_status = InternalResourceStatusResponse {
        cpu_usage: system_status.resource_status.cpu_usage,
        memory_usage: system_status.resource_status.memory_usage,
        disk_io_usage: system_status.resource_status.disk_io_usage,
        network_io_usage: system_status.resource_status.network_io_usage,
        available_memory: system_status.resource_status.available_memory,
        available_disk: system_status.resource_status.available_disk,
        total_disk: system_status.resource_status.total_disk,
        load_avg_1: system_status.resource_status.load_avg_1,
        load_avg_5: system_status.resource_status.load_avg_5,
        load_avg_15: system_status.resource_status.load_avg_15,
        disk_usage_percent: system_status.resource_status.disk_usage_percent,
        controller_running: system_status.controller_running,
    };

    (StatusCode::OK, Json(resource_status)).into_response()
}

/// 处理引擎状态请求
#[utoipa::path(
    get,
    path = "/internal/engines/status",
    responses(
        (status = 200, description = "获取成功", body = Vec<InternalEngineStatus>),
        (status = 403, description = "仅限本地访问"),
    ),
    tag = "internal"
)]
pub async fn handle_engines_status(headers: HeaderMap, State(state): State<ApiState>) -> Response {
    // 检查是否为本地请求
    if !is_local_request(&headers) {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({"error": "仅限本地访问"})),
        )
            .into_response();
    }

    // 获取引擎状态
    let engine_states = state.search.get_engine_states().await;

    let engine_statuses: Vec<InternalEngineStatus> = engine_states
        .into_iter()
        .map(|(name, (enabled, disabled, failures))| {
            let total_requests = failures;
            let success_rate = if total_requests > 0 {
                (total_requests - failures) as f64 / total_requests as f64
            } else {
                1.0
            };

            InternalEngineStatus {
                name: name.clone(),
                enabled,
                temporarily_disabled: disabled,
                consecutive_failures: failures,
                total_requests,
                failed_requests: failures,
                success_rate,
                avg_response_time_ms: None,
                disabled_reason: if disabled && failures > 0 {
                    Some(format!("连续失败 {} 次", failures))
                } else {
                    None
                },
                last_updated: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
            }
        })
        .collect();

    (StatusCode::OK, Json(engine_statuses)).into_response()
}

/// 处理系统综合状态请求
#[utoipa::path(
    get,
    path = "/internal/system/status",
    responses(
        (status = 200, description = "获取成功", body = InternalSystemStatusResponse),
        (status = 403, description = "仅限本地访问"),
    ),
    tag = "internal"
)]
pub async fn handle_system_status(headers: HeaderMap, State(state): State<ApiState>) -> Response {
    // 检查是否为本地请求
    if !is_local_request(&headers) {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({"error": "仅限本地访问"})),
        )
            .into_response();
    }

    // 获取资源状态
    use seesea_sys::get_global_system_controller;
    let controller = get_global_system_controller();
    let system_status = controller.get_system_status().await;

    let resources = InternalResourceStatusResponse {
        cpu_usage: system_status.resource_status.cpu_usage,
        memory_usage: system_status.resource_status.memory_usage,
        disk_io_usage: system_status.resource_status.disk_io_usage,
        network_io_usage: system_status.resource_status.network_io_usage,
        available_memory: system_status.resource_status.available_memory,
        available_disk: system_status.resource_status.available_disk,
        total_disk: system_status.resource_status.total_disk,
        load_avg_1: system_status.resource_status.load_avg_1,
        load_avg_5: system_status.resource_status.load_avg_5,
        load_avg_15: system_status.resource_status.load_avg_15,
        disk_usage_percent: system_status.resource_status.disk_usage_percent,
        controller_running: system_status.controller_running,
    };

    // 获取搜索统计
    let search_stats_result = state.search.get_stats().await;
    let search_stats = ApiStatsResponse::from_search_stats(&search_stats_result);

    // 获取引擎状态
    let engine_states = state.search.get_engine_states().await;
    let engine_statuses: Vec<InternalEngineStatus> = engine_states
        .into_iter()
        .map(|(name, (enabled, disabled, failures))| {
            let total_requests = failures;
            let success_rate = if total_requests > 0 {
                (total_requests - failures) as f64 / total_requests as f64
            } else {
                1.0
            };

            InternalEngineStatus {
                name: name.clone(),
                enabled,
                temporarily_disabled: disabled,
                consecutive_failures: failures,
                total_requests,
                failed_requests: failures,
                success_rate,
                avg_response_time_ms: None,
                disabled_reason: if disabled && failures > 0 {
                    Some(format!("连续失败 {} 次", failures))
                } else {
                    None
                },
                last_updated: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
            }
        })
        .collect();

    // 获取缓存统计
    let cache_stats = Some(InternalCacheStatsResponse {
        total_keys: 0,
        total_size_bytes: 0,
        hit_rate: search_stats.cache_hit_rate,
    });

    // 计算真实的运行时间（使用进程启动时间）
    let uptime_seconds = {
        use seesea_sys::get_process_uptime_seconds;
        get_process_uptime_seconds()
    };

    let system_status_response = InternalSystemStatusResponse {
        resources,
        search_stats,
        engine_statuses,
        cache_stats,
        uptime_seconds,
    };

    (StatusCode::OK, Json(system_status_response)).into_response()
}

/// 处理缓存键列表请求
#[utoipa::path(
    get,
    path = "/internal/cache/keys",
    params(
        ("limit" = Option<usize>, Query, description = "返回的最大键数"),
    ),
    responses(
        (status = 200, description = "获取成功", body = InternalCacheKeysResponse),
        (status = 403, description = "仅限本地访问"),
    ),
    tag = "internal"
)]
pub async fn handle_cache_keys(
    headers: HeaderMap,
    State(state): State<ApiState>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> Response {
    // 检查是否为本地请求
    if !is_local_request(&headers) {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({"error": "仅限本地访问"})),
        )
            .into_response();
    }

    // 获取缓存键
    let (count, cached_engines) = state.search.get_engine_cache_stats().await;

    let limit: usize = params
        .get("limit")
        .and_then(|l| l.parse().ok())
        .unwrap_or(100);

    let keys: Vec<String> = cached_engines.into_iter().take(limit).collect();

    let cache_keys_response = InternalCacheKeysResponse {
        total_keys: count,
        keys,
        cache_size_bytes: None,
    };

    (StatusCode::OK, Json(cache_keys_response)).into_response()
}
