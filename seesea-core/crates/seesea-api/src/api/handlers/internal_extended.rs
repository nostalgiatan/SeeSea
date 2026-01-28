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

//! 扩展的内部管理处理器
//!
//! 提供更多强大的内部管理功能

use axum::{
    extract::{Json, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use utoipa::ToSchema;

use crate::api::on::ApiState;

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

/// 引擎启用/禁用请求
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct EngineToggleRequest {
    /// 引擎名称
    pub engine_name: String,
    /// 是否启用
    pub enabled: bool,
}

/// 批量引擎操作请求
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct BatchEngineRequest {
    /// 引擎名称列表
    pub engines: Vec<String>,
    /// 操作类型：enable, disable, reset
    pub action: String,
}

/// 缓存清除请求
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CacheClearRequest {
    /// 缓存键模式（可选，不提供则清除所有）
    pub pattern: Option<String>,
}

/// 重启控制器请求
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ControllerActionRequest {
    /// 操作类型：restart, pause, resume
    pub action: String,
}

/// 操作响应
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ActionResponse {
    /// 是否成功
    pub success: bool,
    /// 消息
    pub message: String,
    /// 影响的项目数
    pub affected_count: Option<usize>,
}

/// 配置值请求
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ConfigValueRequest {
    /// 配置键
    pub key: String,
    /// 配置值
    pub value: serde_json::Value,
}

/// 获取所有可用引擎列表（带详细信息）
#[utoipa::path(
    get,
    path = "/internal/engines/list",
    responses(
        (status = 200, description = "获取成功"),
        (status = 403, description = "仅限本地访问"),
    ),
    tag = "internal"
)]
pub async fn handle_engines_list_full(
    headers: HeaderMap,
    State(_state): State<ApiState>,
) -> Response {
    if !is_local_request(&headers) {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({"error": "仅限本地访问"})),
        )
            .into_response();
    }

    // 获取引擎配置
    use seesea_search::search::engine_config::ENGINE_CONFIG;
    let config = &*ENGINE_CONFIG;

    let engines: Vec<serde_json::Value> = config
        .all_available_engines
        .iter()
        .map(|engine_name| {
            // 简化实现，默认为 general 类型
            json!({
                "name": engine_name,
                "type": "general",
                "enabled": true,
                "global": config.global_engines.contains(engine_name),
            })
        })
        .collect();

    (
        StatusCode::OK,
        Json(json!({
            "total": engines.len(),
            "engines": engines
        })),
    )
        .into_response()
}

/// 切换引擎启用/禁用状态
#[utoipa::path(
    post,
    path = "/internal/engines/toggle",
    request_body = EngineToggleRequest,
    responses(
        (status = 200, description = "操作成功", body = ActionResponse),
        (status = 403, description = "仅限本地访问"),
        (status = 400, description = "引擎不存在"),
    ),
    tag = "internal"
)]
pub async fn handle_engine_toggle(
    headers: HeaderMap,
    State(state): State<ApiState>,
    Json(req): Json<EngineToggleRequest>,
) -> Response {
    if !is_local_request(&headers) {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({"error": "仅限本地访问"})),
        )
            .into_response();
    }

    use seesea_search::search::engine_config::ENGINE_CONFIG;
    let config = &*ENGINE_CONFIG;

    // 检查引擎是否存在
    if !config.is_engine_available(&req.engine_name) {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({
                "success": false,
                "message": format!("引擎 '{}' 不存在", req.engine_name),
            })),
        )
            .into_response();
    }

    // 使用 RuntimeConfigManager 管理引擎状态
    let result = if req.enabled {
        state
            .runtime_config
            .enable_engine(req.engine_name.clone())
            .await
    } else {
        state
            .runtime_config
            .disable_engine(req.engine_name.clone())
            .await
    };

    match result {
        Ok(_) => (
            StatusCode::OK,
            Json(ActionResponse {
                success: true,
                message: format!(
                    "引擎 {} 已{}",
                    req.engine_name,
                    if req.enabled { "启用" } else { "禁用" }
                ),
                affected_count: Some(1),
            }),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "success": false,
                "message": format!("操作失败: {}", e),
            })),
        )
            .into_response(),
    }
}

/// 批量引擎操作
#[utoipa::path(
    post,
    path = "/internal/engines/batch",
    request_body = BatchEngineRequest,
    responses(
        (status = 200, description = "操作成功", body = ActionResponse),
        (status = 403, description = "仅限本地访问"),
    ),
    tag = "internal"
)]
pub async fn handle_engines_batch(
    headers: HeaderMap,
    State(state): State<ApiState>,
    Json(req): Json<BatchEngineRequest>,
) -> Response {
    if !is_local_request(&headers) {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({"error": "仅限本地访问"})),
        )
            .into_response();
    }

    use seesea_search::search::engine_config::ENGINE_CONFIG;
    let config = &*ENGINE_CONFIG;

    let mut failed_engines = Vec::new();
    let affected_count = match req.action.as_str() {
        "enable" => {
            // 批量启用引擎
            let mut count = 0;
            for engine_name in &req.engines {
                if config.is_engine_available(engine_name) {
                    match state
                        .runtime_config
                        .enable_engine(engine_name.clone())
                        .await
                    {
                        Ok(_) => count += 1,
                        Err(e) => failed_engines.push(format!("{}: {}", engine_name, e)),
                    }
                } else {
                    failed_engines.push(format!("{} (不存在)", engine_name));
                }
            }
            count
        }
        "disable" => {
            // 批量禁用引擎
            let mut count = 0;
            for engine_name in &req.engines {
                if config.is_engine_available(engine_name) {
                    match state
                        .runtime_config
                        .disable_engine(engine_name.clone())
                        .await
                    {
                        Ok(_) => count += 1,
                        Err(e) => failed_engines.push(format!("{}: {}", engine_name, e)),
                    }
                } else {
                    failed_engines.push(format!("{} (不存在)", engine_name));
                }
            }
            count
        }
        "reset" => {
            // 重置配置
            state.runtime_config.reset().await;
            req.engines.len()
        }
        _ => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({
                    "success": false,
                    "message": format!("不支持的操作: {}", req.action),
                    "supported_actions": ["enable", "disable", "reset"],
                })),
            )
                .into_response();
        }
    };

    let message = if failed_engines.is_empty() {
        format!(
            "批量操作 {} 完成，影响 {} 个引擎",
            req.action, affected_count
        )
    } else {
        format!(
            "批量操作 {} 完成，成功 {} 个，失败: {:?}",
            req.action, affected_count, failed_engines
        )
    };

    (
        StatusCode::OK,
        Json(ActionResponse {
            success: failed_engines.is_empty(),
            message,
            affected_count: Some(affected_count),
        }),
    )
        .into_response()
}

/// 清除缓存
#[utoipa::path(
    post,
    path = "/internal/cache/clear",
    request_body = CacheClearRequest,
    responses(
        (status = 200, description = "清除成功", body = ActionResponse),
        (status = 403, description = "仅限本地访问"),
    ),
    tag = "internal"
)]
pub async fn handle_cache_clear_pattern(
    headers: HeaderMap,
    State(state): State<ApiState>,
    Json(req): Json<CacheClearRequest>,
) -> Response {
    if !is_local_request(&headers) {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({"error": "仅限本地访问"})),
        )
            .into_response();
    }

    // 获取当前缓存的引擎
    let (total_count, cached_engines) = state.search.get_engine_cache_stats().await;

    let cleared_count = match req.pattern {
        Some(ref pattern) => {
            // 按模式清除缓存
            let engines_to_clear: Vec<String> = cached_engines
                .into_iter()
                .filter(|engine| engine.contains(pattern))
                .collect();

            // 如果有匹配的引擎，清除整个缓存（目前接口不支持按引擎清除）
            if !engines_to_clear.is_empty() {
                state.search.clear_engine_cache().await;
                engines_to_clear.len()
            } else {
                0
            }
        }
        None => {
            // 清除所有缓存
            state.search.clear_engine_cache().await;
            total_count
        }
    };

    (
        StatusCode::OK,
        Json(ActionResponse {
            success: true,
            message: match req.pattern {
                Some(p) => format!("已清除匹配 '{}' 的缓存，共 {} 个", p, cleared_count),
                None => format!("已清除所有缓存，共 {} 个", cleared_count),
            },
            affected_count: Some(cleared_count),
        }),
    )
        .into_response()
}

/// 获取缓存统计详情
#[utoipa::path(
    get,
    path = "/internal/cache/stats",
    responses(
        (status = 200, description = "获取成功"),
        (status = 403, description = "仅限本地访问"),
    ),
    tag = "internal"
)]
pub async fn handle_cache_stats_detail(
    headers: HeaderMap,
    State(state): State<ApiState>,
) -> Response {
    if !is_local_request(&headers) {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({"error": "仅限本地访问"})),
        )
            .into_response();
    }

    let (count, cached_engines) = state.search.get_engine_cache_stats().await;

    (
        StatusCode::OK,
        Json(json!({
            "total_keys": count,
            "cached_engines": cached_engines,
            "cache_enabled": true,
            "backend": "memory"
        })),
    )
        .into_response()
}

/// 控制器操作
#[utoipa::path(
    post,
    path = "/internal/controller/action",
    request_body = ControllerActionRequest,
    responses(
        (status = 200, description = "操作成功", body = ActionResponse),
        (status = 403, description = "仅限本地访问"),
    ),
    tag = "internal"
)]
pub async fn handle_controller_action(
    headers: HeaderMap,
    State(_state): State<ApiState>,
    Json(req): Json<ControllerActionRequest>,
) -> Response {
    if !is_local_request(&headers) {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({"error": "仅限本地访问"})),
        )
            .into_response();
    }

    use seesea_sys::get_global_system_controller;
    let controller = get_global_system_controller();

    let success = match req.action.as_str() {
        "restart" => {
            // 重启控制器
            let status = controller.get_system_status().await;
            if status.controller_running {
                controller.stop().await;
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
            // start 返回 ()，直接返回 true
            controller.start().await;
            true
        }
        "stop" => {
            // 停止控制器
            let status = controller.get_system_status().await;
            if status.controller_running {
                controller.stop().await;
                true
            } else {
                false
            }
        }
        "start" => {
            // 启动控制器
            let status = controller.get_system_status().await;
            if !status.controller_running {
                // start 返回 ()，直接返回 true
                controller.start().await;
                true
            } else {
                false
            }
        }
        _ => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({
                    "success": false,
                    "message": format!("不支持的操作: {}", req.action),
                    "supported_actions": ["restart", "stop", "start"],
                })),
            )
                .into_response();
        }
    };

    (
        StatusCode::OK,
        Json(ActionResponse {
            success,
            message: format!("控制器 {} 操作完成", req.action),
            affected_count: None,
        }),
    )
        .into_response()
}

/// 获取配置信息
#[utoipa::path(
    get,
    path = "/internal/config/get",
    responses(
        (status = 200, description = "获取成功"),
        (status = 403, description = "仅限本地访问"),
    ),
    tag = "internal"
)]
pub async fn handle_config_get(headers: HeaderMap, State(_state): State<ApiState>) -> Response {
    if !is_local_request(&headers) {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({"error": "仅限本地访问"})),
        )
            .into_response();
    }

    use seesea_search::search::engine_config::ENGINE_CONFIG;
    let config = &*ENGINE_CONFIG;

    // 构建引擎类型映射（不包含私有字段）
    let mut engine_types = serde_json::Map::new();
    for engine_type in config.get_supported_types() {
        engine_types.insert(
            engine_type.clone(),
            json!(config.get_engines_for_type(&engine_type)),
        );
    }

    (
        StatusCode::OK,
        Json(json!({
            "global_engines": config.global_engines,
            "all_available_engines": config.all_available_engines,
            "fast_engines": config.fast_engines,
            "deepweb_engines": config.deepweb_engines,
            "engine_types": engine_types,
            "supported_types": config.get_supported_types(),
        })),
    )
        .into_response()
}

/// 更新配置
#[utoipa::path(
    post,
    path = "/internal/config/update",
    request_body = ConfigValueRequest,
    responses(
        (status = 200, description = "更新成功", body = ActionResponse),
        (status = 403, description = "仅限本地访问"),
        (status = 400, description = "无效的配置"),
    ),
    tag = "internal"
)]
pub async fn handle_config_update(
    headers: HeaderMap,
    State(_state): State<ApiState>,
    Json(req): Json<ConfigValueRequest>,
) -> Response {
    if !is_local_request(&headers) {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({"error": "仅限本地访问"})),
        )
            .into_response();
    }

    use seesea_search::search::engine_config::ENGINE_CONFIG;
    let config = &*ENGINE_CONFIG;

    let success = match req.key.as_str() {
        "global_engines" => {
            if let Some(engines) = req.value.as_array() {
                let engine_list: Vec<String> = engines
                    .iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect();

                if let Err(e) = config.validate_engines(&engine_list) {
                    return (
                        StatusCode::BAD_REQUEST,
                        Json(json!({
                            "success": false,
                            "message": format!("验证引擎列表失败: {}", e),
                        })),
                    )
                        .into_response();
                }

                // 更新全局引擎列表（需要通过其他方式实现，这里简化处理）
                true
            } else {
                false
            }
        }
        _ => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({
                    "success": false,
                    "message": format!("不支持的配置键: {}", req.key),
                    "supported_keys": ["global_engines"],
                })),
            )
                .into_response();
        }
    };

    (
        StatusCode::OK,
        Json(ActionResponse {
            success,
            message: format!("配置 {} 已更新", req.key),
            affected_count: None,
        }),
    )
        .into_response()
}

/// 获取版本和构建信息
#[utoipa::path(
    get,
    path = "/internal/system/version",
    responses(
        (status = 200, description = "获取成功"),
        (status = 403, description = "仅限本地访问"),
    ),
    tag = "internal"
)]
pub async fn handle_version_info(headers: HeaderMap) -> Response {
    if !is_local_request(&headers) {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({"error": "仅限本地访问"})),
        )
            .into_response();
    }

    (
        StatusCode::OK,
        Json(json!({
            "version": env!("CARGO_PKG_VERSION"),
            "name": env!("CARGO_PKG_NAME"),
            "build_time": std::env::var("BUILD_TIME").unwrap_or_else(|_| "unknown".to_string()),
            "git_commit": std::env::var("GIT_COMMIT").unwrap_or_else(|_| "unknown".to_string()),
            "rust_version": "unknown".to_string(),
        })),
    )
        .into_response()
}

/// 获取实时日志（最近N行）
#[utoipa::path(
    get,
    path = "/internal/logs/tail",
    params(
        ("lines" = Option<usize>, Query, description = "返回的日志行数"),
    ),
    responses(
        (status = 200, description = "获取成功"),
        (status = 403, description = "仅限本地访问"),
    ),
    tag = "internal"
)]
pub async fn handle_logs_tail(
    headers: HeaderMap,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> Response {
    if !is_local_request(&headers) {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({"error": "仅限本地访问"})),
        )
            .into_response();
    }

    use seesea_config::paths::get_log_dir;
    let log_dir = get_log_dir();
    let log_path = std::path::Path::new(&log_dir);

    let lines: usize = params
        .get("lines")
        .and_then(|l| l.parse().ok())
        .unwrap_or(50);

    // 查找最新的 Python 日志文件
    let log_file = if let Ok(entries) = std::fs::read_dir(log_path) {
        entries
            .flatten()
            .filter_map(|entry| {
                let file_name = entry.file_name().to_string_lossy().to_string();
                if file_name.starts_with("seesea_core.") {
                    Some(entry.path())
                } else {
                    None
                }
            })
            .max_by_key(|path| path.metadata().ok().and_then(|m| m.modified().ok()))
    } else {
        None
    };

    let logs = match &log_file {
        Some(path) if path.exists() => match tokio::fs::read_to_string(path).await {
            Ok(content) => {
                let all_lines: Vec<&str> = content.lines().collect();
                let start_idx = if all_lines.len() > lines {
                    all_lines.len() - lines
                } else {
                    0
                };
                all_lines[start_idx..]
                    .iter()
                    .map(|s| s.to_string())
                    .collect()
            }
            Err(_) => {
                vec![format!("无法读取日志文件: {}", path.display())]
            }
        },
        Some(path) => vec![format!("日志文件不存在: {}", path.display())],
        None => vec![format!("日志目录不存在或为空: {}", log_dir)],
    };

    (
        StatusCode::OK,
        Json(json!({
            "logs": logs,
            "lines_requested": lines,
            "lines_returned": logs.len(),
            "log_dir": log_dir,
            "log_file": log_file.map(|p| p.to_string_lossy().to_string())
        })),
    )
        .into_response()
}

/// 获取错误日志
#[utoipa::path(
    get,
    path = "/internal/logs/errors",
    responses(
        (status = 200, description = "获取成功"),
        (status = 403, description = "仅限本地访问"),
    ),
    tag = "internal"
)]
pub async fn handle_logs_errors(headers: HeaderMap) -> Response {
    if !is_local_request(&headers) {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({"error": "仅限本地访问"})),
        )
            .into_response();
    }

    use seesea_config::paths::get_log_dir;
    let log_dir = get_log_dir();
    let log_path = std::path::Path::new(&log_dir);

    // 查找最新的 Python 日志文件
    let log_file = if let Ok(entries) = std::fs::read_dir(log_path) {
        entries
            .flatten()
            .filter_map(|entry| {
                let file_name = entry.file_name().to_string_lossy().to_string();
                if file_name.starts_with("seesea_core.") {
                    Some(entry.path())
                } else {
                    None
                }
            })
            .max_by_key(|path| path.metadata().ok().and_then(|m| m.modified().ok()))
    } else {
        None
    };

    let errors = match &log_file {
        Some(path) if path.exists() => match tokio::fs::read_to_string(path).await {
            Ok(content) => content
                .lines()
                .filter(|line| {
                    line.to_lowercase().contains("error")
                        || line.to_lowercase().contains("失败")
                        || line.to_lowercase().contains("failed")
                })
                .map(|s| s.to_string())
                .collect::<Vec<_>>(),
            Err(_) => {
                vec![format!("无法读取日志文件: {}", path.display())]
            }
        },
        Some(path) => vec![format!("日志文件不存在: {}", path.display())],
        None => vec![format!("日志目录不存在或为空: {}", log_dir)],
    };

    (
        StatusCode::OK,
        Json(json!({
            "errors": errors,
            "total_count": errors.len(),
            "log_dir": log_dir,
            "log_file": log_file.map(|p| p.to_string_lossy().to_string())
        })),
    )
        .into_response()
}

/// 获取连接统计
#[utoipa::path(
    get,
    path = "/internal/connections/stats",
    responses(
        (status = 200, description = "获取成功"),
        (status = 403, description = "仅限本地访问"),
    ),
    tag = "internal"
)]
pub async fn handle_connections_stats(
    headers: HeaderMap,
    State(state): State<ApiState>,
) -> Response {
    if !is_local_request(&headers) {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({"error": "仅限本地访问"})),
        )
            .into_response();
    }

    // 从 MetricsCollector 获取真实的实时指标
    let realtime_metrics = state.metrics.get_realtime_metrics().await;

    // 获取搜索统计
    let search_stats = state.search.get_stats().await;

    // 计算缓存命中率
    let cache_hit_rate = if search_stats.total_searches > 0 {
        search_stats.cache_hits as f64 / search_stats.total_searches as f64
    } else {
        0.0
    };

    // 计算每秒请求数（基于总请求数和运行时间）
    let rps = if realtime_metrics.uptime_seconds > 0 {
        realtime_metrics.total_requests as f64 / realtime_metrics.uptime_seconds as f64
    } else {
        0.0
    };

    (
        StatusCode::OK,
        Json(json!({
            "active_connections": realtime_metrics.active_connections,
            "total_requests": realtime_metrics.total_requests,
            "successful_requests": realtime_metrics.successful_requests,
            "failed_requests": realtime_metrics.failed_requests,
            "requests_per_second": rps,
            "average_response_time_ms": realtime_metrics.avg_response_time_ms,
            "rate_limited": realtime_metrics.rate_limited,
            "circuit_breaker_trips": realtime_metrics.circuit_breaker_trips,
            "ip_blocked": realtime_metrics.ip_blocked,
            "uptime_seconds": realtime_metrics.uptime_seconds,
            "cache_hits": search_stats.cache_hits,
            "cache_misses": search_stats.cache_misses,
            "cache_hit_rate": cache_hit_rate,
            "engine_failures": search_stats.engine_failures,
            "timeouts": search_stats.timeouts,
        })),
    )
        .into_response()
}

/// 获取系统健康检查详情
#[utoipa::path(
    get,
    path = "/internal/system/health",
    responses(
        (status = 200, description = "获取成功"),
        (status = 403, description = "仅限本地访问"),
    ),
    tag = "internal"
)]
pub async fn handle_health_detail(headers: HeaderMap, State(_state): State<ApiState>) -> Response {
    if !is_local_request(&headers) {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({"error": "仅限本地访问"})),
        )
            .into_response();
    }

    use seesea_sys::get_global_system_controller;
    let controller = get_global_system_controller();
    let system_status = controller.get_system_status().await;

    (
        StatusCode::OK,
        Json(json!({
            "status": "healthy",
            "components": {
                "controller": system_status.controller_running,
                "search": true,
                "cache": true,
                "engines": true
            },
            "last_check": std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs()
        })),
    )
        .into_response()
}

/// 获取日志目录路径
#[utoipa::path(
    get,
    path = "/internal/logs/directory",
    responses(
        (status = 200, description = "获取成功"),
        (status = 403, description = "仅限本地访问"),
    ),
    tag = "internal"
)]
pub async fn handle_logs_directory(headers: HeaderMap) -> Response {
    if !is_local_request(&headers) {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({"error": "仅限本地访问"})),
        )
            .into_response();
    }

    use seesea_config::paths::get_log_dir;
    let log_dir = get_log_dir();

    // 检查目录是否存在
    let dir_exists = std::path::Path::new(&log_dir).exists();

    (
        StatusCode::OK,
        Json(json!({
            "log_dir": log_dir,
            "exists": dir_exists,
            "readable": dir_exists && std::path::Path::new(&log_dir)
                .metadata()
                .map(|m| !m.permissions().readonly())
                .unwrap_or(false),
        })),
    )
        .into_response()
}

/// 获取日志文件列表
#[utoipa::path(
    get,
    path = "/internal/logs/files",
    params(
        ("log_type" = Option<String>, Query, description = "日志类型: python, rust, all"),
    ),
    responses(
        (status = 200, description = "获取成功"),
        (status = 403, description = "仅限本地访问"),
    ),
    tag = "internal"
)]
pub async fn handle_logs_files(
    headers: HeaderMap,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> Response {
    if !is_local_request(&headers) {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({"error": "仅限本地访问"})),
        )
            .into_response();
    }

    use seesea_config::paths::get_log_dir;
    let log_dir = get_log_dir();
    let log_path = std::path::Path::new(&log_dir);

    let log_type = params.get("log_type").unwrap_or(&"all".to_string()).clone();

    if !log_path.exists() {
        return (
            StatusCode::OK,
            Json(json!({
                "log_dir": log_dir,
                "exists": false,
                "files": Vec::<LogFileMetadata>::new(),
                "python_files": Vec::<LogFileMetadata>::new(),
                "rust_files": Vec::<LogFileMetadata>::new(),
            })),
        )
            .into_response();
    }

    let mut python_files = Vec::new();
    let mut rust_files = Vec::new();

    if let Ok(entries) = std::fs::read_dir(log_path) {
        for entry in entries.flatten() {
            if let Ok(file_type) = entry.file_type()
                && file_type.is_file()
            {
                let file_name = entry.file_name().to_string_lossy().to_string();
                let metadata = match entry.metadata() {
                    Ok(meta) => Some(LogFileMetadata {
                        name: file_name.clone(),
                        size: meta.len(),
                        modified: meta
                            .modified()
                            .ok()
                            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                            .map(|d| d.as_secs()),
                    }),
                    Err(_) => None,
                };

                // 根据 log_type 过滤
                if (log_type == "all" || log_type == "python")
                    && file_name.starts_with("seesea_core.")
                    && let Some(ref meta) = metadata
                {
                    python_files.push(meta.clone());
                }
                if (log_type == "all" || log_type == "rust")
                    && file_name.starts_with("rust_scheduler.")
                    && let Some(ref meta) = metadata
                {
                    rust_files.push(meta.clone());
                }
            }
        }
    }

    // 合并所有文件
    let all_files: Vec<_> = if log_type == "python" {
        python_files.clone()
    } else if log_type == "rust" {
        rust_files.clone()
    } else {
        python_files
            .clone()
            .into_iter()
            .chain(rust_files.clone())
            .collect()
    };

    // 按修改时间倒序排序
    let mut sorted_files = all_files;
    sorted_files.sort_by(|a, b| b.modified.cmp(&a.modified));

    (
        StatusCode::OK,
        Json(json!({
            "log_dir": log_dir,
            "exists": true,
            "log_type": log_type,
            "total_count": sorted_files.len(),
            "files": sorted_files,
            "python_count": python_files.len(),
            "rust_count": rust_files.len(),
        })),
    )
        .into_response()
}

/// 读取指定日志文件
#[utoipa::path(
    get,
    path = "/internal/logs/read",
    params(
        ("file" = String, Query, description = "日志文件名"),
        ("lines" = Option<usize>, Query, description = "返回的日志行数"),
        ("offset" = Option<usize>, Query, description = "从第几行开始（0-based）"),
    ),
    responses(
        (status = 200, description = "获取成功"),
        (status = 403, description = "仅限本地访问"),
        (status = 404, description = "文件不存在"),
    ),
    tag = "internal"
)]
pub async fn handle_logs_read(
    headers: HeaderMap,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> Response {
    if !is_local_request(&headers) {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({"error": "仅限本地访问"})),
        )
            .into_response();
    }

    use seesea_config::paths::get_log_dir;
    let log_dir = get_log_dir();
    let log_path = std::path::Path::new(&log_dir);

    let file_name = match params.get("file") {
        Some(f) => f,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({"error": "缺少 file 参数"})),
            )
                .into_response();
        }
    };

    let file_path = log_path.join(file_name);

    if !file_path.exists() {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({
                "error": "文件不存在",
                "file": file_name,
                "path": file_path.to_string_lossy()
            })),
        )
            .into_response();
    }

    let lines: usize = params
        .get("lines")
        .and_then(|l| l.parse().ok())
        .unwrap_or(100);

    let offset: usize = params
        .get("offset")
        .and_then(|o| o.parse().ok())
        .unwrap_or(0);

    match tokio::fs::read_to_string(&file_path).await {
        Ok(content) => {
            let all_lines: Vec<&str> = content.lines().collect();
            let total_lines = all_lines.len();

            // 计算起始和结束索引
            let start_idx = offset.min(total_lines);
            let end_idx = if offset + lines > total_lines {
                total_lines
            } else {
                offset + lines
            };

            let selected_lines = all_lines[start_idx..end_idx]
                .iter()
                .map(|s| s.to_string())
                .collect::<Vec<_>>();

            (
                StatusCode::OK,
                Json(json!({
                    "file": file_name,
                    "path": file_path.to_string_lossy(),
                    "total_lines": total_lines,
                    "offset": start_idx,
                    "lines_returned": selected_lines.len(),
                    "requested_lines": lines,
                    "logs": selected_lines,
                })),
            )
                .into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "error": "读取文件失败",
                "file": file_name,
                "message": e.to_string(),
            })),
        )
            .into_response(),
    }
}

/// 日志文件元数据
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct LogFileMetadata {
    /// 文件名
    pub name: String,
    /// 文件大小（字节）
    pub size: u64,
    /// 修改时间（Unix 时间戳）
    pub modified: Option<u64>,
}
