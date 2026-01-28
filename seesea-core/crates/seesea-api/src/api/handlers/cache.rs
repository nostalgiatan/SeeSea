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

//! 缓存 API 处理器
//!
//! 处理缓存管理相关的 API 请求

use crate::api::on::ApiState;

use axum::{
    Json,
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;

/// 缓存统计响应
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct CacheStatsResponse {
    /// 总缓存条目数
    pub total_entries: u64,
    /// 缓存大小（字节）
    pub size_bytes: u64,
    /// 命中次数
    pub hits: u64,
    /// 未命中次数
    pub misses: u64,
    /// 命中率
    pub hit_rate: f64,
    /// 写入次数
    pub writes: u64,
    /// 删除次数
    pub deletes: u64,
    /// 过期清理次数
    pub evictions: u64,
    /// 读取平均延迟（毫秒）
    pub avg_get_latency_ms: f64,
    /// 写入平均延迟（毫秒）
    pub avg_set_latency_ms: f64,
}

/// 缓存清理响应
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct CacheClearResponse {
    /// 是否成功
    pub success: bool,
    /// 清理的条目数
    pub cleared_entries: usize,
    /// 消息
    pub message: String,
}

/// 处理获取缓存统计请求
#[utoipa::path(
    get,
    path = "/cache/stats",
    responses(
        (status = 200, description = "获取成功", body = CacheStatsResponse),
    ),
    tag = "cache"
)]
pub async fn handle_cache_stats(State(state): State<ApiState>) -> Response {
    let cache_stats = state.search.get_cache_stats();

    // 从延迟统计中获取平均延迟（微秒转换为毫秒）
    let avg_get_latency_ms = cache_stats.latency_stats.avg_latency_us as f64 / 1_000.0;
    let avg_set_latency_ms = cache_stats.latency_stats.avg_latency_us as f64 / 1_000.0;

    // 计算命中率
    let total_requests = cache_stats.total_hits + cache_stats.total_misses;
    let hit_rate = if total_requests > 0 {
        cache_stats.total_hits as f64 / total_requests as f64
    } else {
        0.0
    };

    let stats = CacheStatsResponse {
        total_entries: cache_stats.current_size as u64,
        size_bytes: 0, // 没有直接的size_bytes字段，使用0作为默认值
        hits: cache_stats.total_hits,
        misses: cache_stats.total_misses,
        hit_rate,
        writes: cache_stats.total_inserts,
        deletes: cache_stats.total_deletes,
        evictions: cache_stats.total_evictions,
        avg_get_latency_ms,
        avg_set_latency_ms,
    };

    (StatusCode::OK, Json(stats)).into_response()
}

/// 处理清除所有缓存请求
#[utoipa::path(
    post,
    path = "/cache/clear",
    responses(
        (status = 200, description = "清除成功", body = CacheClearResponse),
        (status = 500, description = "服务器错误", body = CacheClearResponse),
    ),
    tag = "cache"
)]
pub async fn handle_cache_clear(State(state): State<ApiState>) -> Response {
    // 先获取当前条目数
    let cache_stats = state.search.get_cache_stats();
    let entries_before = cache_stats.total_inserts;

    // 清除缓存
    match state.search.clear_cache().await {
        Ok(()) => {
            let response = CacheClearResponse {
                success: true,
                cleared_entries: entries_before as usize,
                message: format!("成功清除 {} 条缓存", entries_before),
            };
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(e) => {
            let response = CacheClearResponse {
                success: false,
                cleared_entries: 0,
                message: format!("清除缓存失败: {}", e),
            };
            (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response()
        }
    }
}

/// 处理清理过期缓存请求
#[utoipa::path(
    post,
    path = "/cache/cleanup",
    responses(
        (status = 200, description = "清理成功", body = CacheClearResponse),
        (status = 500, description = "服务器错误", body = CacheClearResponse),
    ),
    tag = "cache"
)]
pub async fn handle_cache_cleanup(State(state): State<ApiState>) -> Response {
    match state.search.cleanup_expired_cache().await {
        Ok(cleaned_count) => {
            let response = CacheClearResponse {
                success: true,
                cleared_entries: cleaned_count,
                message: format!("成功清理 {} 条过期缓存", cleaned_count),
            };
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(e) => {
            let response = CacheClearResponse {
                success: false,
                cleared_entries: 0,
                message: format!("清理过期缓存失败: {}", e),
            };
            (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response()
        }
    }
}
