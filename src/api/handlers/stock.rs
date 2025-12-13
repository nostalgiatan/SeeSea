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

//! 股票 API 处理器模块
//!
//! 处理所有 /api/stock/* 请求，支持：
//! - 股票搜索
//! - 实时行情
//! - K线数据
//! - 财务数据
//! - 资金流向
//! - 公告信息
//!
//! 架构设计：
//! - Rust 端负责配置管理、路径管理、HTTP 处理
//! - Python 端作为数据提供者，接收 Rust 传递的配置
//! - 无 unsafe 代码块
//! - 正确的资源管理

use crate::api::on::ApiState;
use axum::body::Body;
use axum::{
    extract::Path,
    extract::Query,
    extract::Request,
    extract::State,
    http::{Method, Response, StatusCode, header},
};

// 下面这些导入仅在启用 python feature 时需要
#[cfg(feature = "python")]
use crate::config::get_platform_paths;
#[cfg(feature = "python")]
use axum::response::{
    IntoResponse,
    sse::{Event, KeepAlive, Sse},
};
#[cfg(feature = "python")]
use pyo3::prelude::*;
#[cfg(feature = "python")]
use pyo3::types::IntoPyDict;
use serde::Deserialize;
use serde_json::json;
use std::collections::HashMap;
#[cfg(feature = "python")]
use std::convert::Infallible;
use std::sync::OnceLock;
#[cfg(feature = "python")]
use std::time::Duration;

/// Python 桥接是否已配置
#[cfg(feature = "python")]
static PYTHON_CONFIGURED: OnceLock<bool> = OnceLock::new();

/// 股票搜索请求参数
#[derive(Debug, Deserialize)]
pub struct StockSearchParams {
    pub q: String,
    #[serde(default = "default_limit")]
    pub limit: usize,
}

fn default_limit() -> usize {
    20
}

/// K线请求参数
#[derive(Debug, Deserialize)]
pub struct KLineParams {
    pub code: String,
    #[serde(default = "default_period")]
    pub period: String,
    #[serde(default = "default_adjust")]
    pub adjust: String,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    #[serde(default = "default_kline_limit")]
    pub limit: usize,
}

fn default_period() -> String {
    "daily".to_string()
}

fn default_adjust() -> String {
    "qfq".to_string()
}

fn default_kline_limit() -> usize {
    500
}

/// 确保 Python 桥接已配置
#[cfg(feature = "python")]
fn ensure_python_configured() -> Result<(), PyErr> {
    if PYTHON_CONFIGURED.get().is_some() {
        return Ok(());
    }

    // 获取平台路径配置
    let paths = get_platform_paths();
    let config_json = serde_json::to_string(&paths.to_json()).unwrap_or_default();

    Python::attach(|py| {
        let api_bridge = py.import("seesea.stock.api_bridge")?;
        api_bridge.call_method1("configure", (config_json,))?;
        Ok::<_, PyErr>(())
    })?;

    let _ = PYTHON_CONFIGURED.set(true);
    Ok(())
}

/// 调用 Python 函数的辅助宏
#[cfg(feature = "python")]
macro_rules! call_python {
    ($func:expr $(, $arg:expr)*) => {{
        let result = tokio::task::spawn_blocking(move || {
            // 确保已配置
            ensure_python_configured()?;

            Python::attach(|py| {
                let api_bridge = py.import("seesea.stock.api_bridge")?;
                let json_str = api_bridge.call_method1($func, ($($arg,)*))?;
                json_str.extract::<String>()
            })
        })
        .await;

        match result {
            Ok(Ok(json_str)) => Ok(json_str),
            Ok(Err(err)) => Err(format!("{:?}", err)),
            Err(err) => Err(format!("Task error: {:?}", err)),
        }
    }};
}

/// 处理所有股票 API 请求
#[cfg(feature = "python")]
pub async fn handle_stock_api(
    State(state): State<ApiState>,
    Path(path): Path<String>,
    method: Method,
    query: Query<HashMap<String, String>>,
    _request: Request,
) -> Response<Body> {
    let full_path = format!("/stock/{}", path);
    let path_segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();

    // 检查动态路由
    let router = state.dynamic_router.read().await;
    if let Some(handler) = router.match_route(&full_path, method.as_str()) {
        drop(router); // 提前释放锁
        return call_python_handler(handler, &full_path, &method, &query.0).await;
    }
    drop(router);

    // 路由分发
    match (method.as_str(), path_segments.as_slice()) {
        ("GET", ["search"]) => handle_stock_search(&query.0).await,
        ("GET", ["quote"]) => handle_stock_quote(&query.0).await,
        ("GET", ["quote", "stream"]) => handle_quote_stream(&query.0).await,
        ("GET", ["kline"]) => handle_stock_kline(&query.0).await,
        ("GET", ["detail", code]) => handle_stock_detail(code).await,
        ("GET", ["financial", code]) => handle_stock_financial(code, &query.0).await,
        ("GET", ["fund_flow", code]) => handle_fund_flow(code, &query.0).await,
        ("GET", ["holders", code]) => handle_stock_holders(code, &query.0).await,
        ("GET", ["announcements", code]) => handle_stock_announcements(code, &query.0).await,
        ("GET", ["market", "status"]) => handle_market_status(&query.0).await,
        ("GET", ["market", "indices"]) => handle_market_indices().await,
        ("GET", ["market", "lhb"]) => handle_lhb_data(&query.0).await,
        ("GET", ["sectors"]) => handle_sectors(&query.0).await,
        ("GET", ["sectors", code, "stocks"]) => handle_sector_stocks(code).await,
        _ => json_response(
            StatusCode::NOT_FOUND,
            json!({
                "error": "Stock API endpoint not found",
                "path": full_path,
                "method": method.as_str()
            }),
        ),
    }
}

/// 调用 Python 动态处理器
#[cfg(feature = "python")]
async fn call_python_handler(
    handler: std::sync::Arc<pyo3::Py<pyo3::PyAny>>,
    path: &str,
    method: &Method,
    query: &HashMap<String, String>,
) -> Response<Body> {
    let handler_clone = handler.clone();
    let path_clone = path.to_string();
    let method_str = method.as_str().to_string();
    let query_clone = query.clone();

    let result = tokio::task::spawn_blocking(move || {
        Python::attach(|py| {
            let handler_ref = handler_clone.clone_ref(py);
            let result = handler_ref.call1(py, (path_clone, &method_str, &query_clone))?;
            result.extract::<String>(py)
        })
    })
    .await;

    match result {
        Ok(Ok(json_str)) => build_json_response(StatusCode::OK, json_str, None),
        Ok(Err(err)) => json_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            json!({"error": format!("Python handler error: {:?}", err)}),
        ),
        Err(err) => json_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            json!({"error": format!("Task spawn error: {:?}", err)}),
        ),
    }
}

/// 股票搜索
#[cfg(feature = "python")]
async fn handle_stock_search(query: &HashMap<String, String>) -> Response<Body> {
    let q = match query.get("q") {
        Some(q) if !q.is_empty() => q.clone(),
        _ => {
            return json_response(
                StatusCode::BAD_REQUEST,
                json!({"error": "Search query 'q' is required"}),
            );
        }
    };
    let limit: usize = query
        .get("limit")
        .and_then(|s| s.parse().ok())
        .unwrap_or(20);

    match call_python!("stock_search", q, limit) {
        Ok(json_str) => build_json_response(StatusCode::OK, json_str, None),
        Err(err) => json_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            json!({"error": format!("Search failed: {}", err)}),
        ),
    }
}

/// 实时行情
#[cfg(feature = "python")]
async fn handle_stock_quote(query: &HashMap<String, String>) -> Response<Body> {
    let codes = query.get("codes").cloned().unwrap_or_default();

    match call_python!("stock_quotes", codes) {
        Ok(json_str) => build_json_response(StatusCode::OK, json_str, Some(5)),
        Err(err) => json_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            json!({"error": format!("Quote fetch failed: {}", err)}),
        ),
    }
}

/// 实时行情流 (SSE)
#[cfg(feature = "python")]
async fn handle_quote_stream(query: &HashMap<String, String>) -> Response<Body> {
    let codes = query.get("codes").cloned().unwrap_or_default();
    let interval: u64 = query
        .get("interval")
        .and_then(|s| s.parse().ok())
        .unwrap_or(5);

    let stream = async_stream::stream! {
        loop {
            let codes_clone = codes.clone();
            let result = tokio::task::spawn_blocking(move || {
                let _ = ensure_python_configured();
                Python::attach(|py| {
                    let api_bridge = py.import("seesea.stock.api_bridge")?;
                    let json_str = api_bridge.call_method1("stock_quotes", (codes_clone,))?;
                    json_str.extract::<String>()
                })
            }).await;

            match result {
                Ok(Ok(json_str)) => yield Ok::<_, Infallible>(Event::default().data(json_str)),
                Ok(Err(err)) => yield Ok(Event::default().data(format!(r#"{{"error": "{:?}"}}"#, err))),
                Err(err) => yield Ok(Event::default().data(format!(r#"{{"error": "{:?}"}}"#, err))),
            }

            tokio::time::sleep(Duration::from_secs(interval)).await;
        }
    };

    let sse = Sse::new(stream).keep_alive(KeepAlive::default());

    sse.into_response()
}

/// K线数据
#[cfg(feature = "python")]
async fn handle_stock_kline(query: &HashMap<String, String>) -> Response<Body> {
    let code = match query.get("code") {
        Some(c) if !c.is_empty() => c.clone(),
        _ => {
            return json_response(
                StatusCode::BAD_REQUEST,
                json!({"error": "Stock code is required"}),
            );
        }
    };

    let period = query
        .get("period")
        .cloned()
        .unwrap_or_else(|| "daily".to_string());
    let adjust = query
        .get("adjust")
        .cloned()
        .unwrap_or_else(|| "qfq".to_string());
    let start_date = query.get("start_date").cloned().unwrap_or_default();
    let end_date = query.get("end_date").cloned().unwrap_or_default();
    let limit: usize = query
        .get("limit")
        .and_then(|s| s.parse().ok())
        .unwrap_or(500);

    let result = tokio::task::spawn_blocking(move || {
        let _ = ensure_python_configured();
        Python::attach(|py| {
            let api_bridge = py.import("seesea.stock.api_bridge")?;
            let kwargs = [
                ("start_date", start_date.into_pyobject(py)?.into_any()),
                ("end_date", end_date.into_pyobject(py)?.into_any()),
                ("limit", limit.into_pyobject(py)?.into_any()),
            ]
            .into_py_dict(py)?;
            let json_str =
                api_bridge.call_method("stock_klines", (code, period, adjust), Some(&kwargs))?;
            json_str.extract::<String>()
        })
    })
    .await;

    match result {
        Ok(Ok(json_str)) => build_json_response(StatusCode::OK, json_str, Some(60)),
        Ok(Err(err)) => json_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            json!({"error": format!("KLine fetch failed: {:?}", err)}),
        ),
        Err(err) => json_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            json!({"error": format!("Task error: {:?}", err)}),
        ),
    }
}

/// 股票详情
#[cfg(feature = "python")]
async fn handle_stock_detail(code: &str) -> Response<Body> {
    let code = code.to_string();

    match call_python!("stock_detail", code) {
        Ok(json_str) => build_json_response(StatusCode::OK, json_str, Some(300)),
        Err(err) => json_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            json!({"error": format!("Detail fetch failed: {}", err)}),
        ),
    }
}

/// 财务数据
#[cfg(feature = "python")]
async fn handle_stock_financial(code: &str, query: &HashMap<String, String>) -> Response<Body> {
    let code = code.to_string();
    let report_type = query
        .get("type")
        .cloned()
        .unwrap_or_else(|| "all".to_string());

    match call_python!("stock_financial", code, report_type) {
        Ok(json_str) => build_json_response(StatusCode::OK, json_str, Some(3600)),
        Err(err) => json_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            json!({"error": format!("Financial fetch failed: {}", err)}),
        ),
    }
}

/// 资金流向
#[cfg(feature = "python")]
async fn handle_fund_flow(code: &str, query: &HashMap<String, String>) -> Response<Body> {
    let code = code.to_string();
    let period = query
        .get("period")
        .cloned()
        .unwrap_or_else(|| "daily".to_string());

    match call_python!("stock_fund_flow", code, period) {
        Ok(json_str) => build_json_response(StatusCode::OK, json_str, Some(60)),
        Err(err) => json_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            json!({"error": format!("Fund flow fetch failed: {}", err)}),
        ),
    }
}

/// 股东信息
#[cfg(feature = "python")]
async fn handle_stock_holders(code: &str, query: &HashMap<String, String>) -> Response<Body> {
    let code = code.to_string();
    let holder_type = query
        .get("type")
        .cloned()
        .unwrap_or_else(|| "top10".to_string());

    match call_python!("stock_holders", code, holder_type) {
        Ok(json_str) => build_json_response(StatusCode::OK, json_str, Some(3600)),
        Err(err) => json_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            json!({"error": format!("Holders fetch failed: {}", err)}),
        ),
    }
}

/// 公告信息
#[cfg(feature = "python")]
async fn handle_stock_announcements(code: &str, query: &HashMap<String, String>) -> Response<Body> {
    let code = code.to_string();
    let limit: i32 = query
        .get("limit")
        .and_then(|s| s.parse().ok())
        .unwrap_or(50);

    match call_python!("stock_announcements", code, limit) {
        Ok(json_str) => build_json_response(StatusCode::OK, json_str, Some(300)),
        Err(err) => json_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            json!({"error": format!("Announcements fetch failed: {}", err)}),
        ),
    }
}

/// 市场状态
#[cfg(feature = "python")]
async fn handle_market_status(query: &HashMap<String, String>) -> Response<Body> {
    let exchange = query
        .get("exchange")
        .cloned()
        .unwrap_or_else(|| "sse".to_string());

    match call_python!("market_status", exchange) {
        Ok(json_str) => build_json_response(StatusCode::OK, json_str, Some(30)),
        Err(err) => json_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            json!({"error": format!("Market status fetch failed: {}", err)}),
        ),
    }
}

/// 指数行情
#[cfg(feature = "python")]
async fn handle_market_indices() -> Response<Body> {
    let result = tokio::task::spawn_blocking(|| {
        let _ = ensure_python_configured();
        Python::attach(|py| {
            let api_bridge = py.import("seesea.stock.api_bridge")?;
            let json_str = api_bridge.call_method0("market_indices")?;
            json_str.extract::<String>()
        })
    })
    .await;

    match result {
        Ok(Ok(json_str)) => build_json_response(StatusCode::OK, json_str, Some(10)),
        Ok(Err(err)) => json_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            json!({"error": format!("Indices fetch failed: {:?}", err)}),
        ),
        Err(err) => json_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            json!({"error": format!("Task error: {:?}", err)}),
        ),
    }
}

/// 龙虎榜数据
#[cfg(feature = "python")]
async fn handle_lhb_data(query: &HashMap<String, String>) -> Response<Body> {
    let trade_date = query.get("date").cloned().unwrap_or_default();

    match call_python!("market_lhb", trade_date) {
        Ok(json_str) => build_json_response(StatusCode::OK, json_str, Some(300)),
        Err(err) => json_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            json!({"error": format!("LHB data fetch failed: {}", err)}),
        ),
    }
}

/// 板块数据
#[cfg(feature = "python")]
async fn handle_sectors(query: &HashMap<String, String>) -> Response<Body> {
    let sector_type = query
        .get("type")
        .cloned()
        .unwrap_or_else(|| "industry".to_string());

    match call_python!("sector_list", sector_type) {
        Ok(json_str) => build_json_response(StatusCode::OK, json_str, Some(300)),
        Err(err) => json_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            json!({"error": format!("Sectors fetch failed: {}", err)}),
        ),
    }
}

/// 板块成分股
#[cfg(feature = "python")]
async fn handle_sector_stocks(code: &str) -> Response<Body> {
    let code = code.to_string();

    match call_python!("sector_stocks", code) {
        Ok(json_str) => build_json_response(StatusCode::OK, json_str, Some(300)),
        Err(err) => json_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            json!({"error": format!("Sector stocks fetch failed: {}", err)}),
        ),
    }
}

// ==================== 无 Python 特性时的占位实现 ====================

#[cfg(not(feature = "python"))]
pub async fn handle_stock_api(
    _state: State<ApiState>,
    Path(path): Path<String>,
    method: Method,
    _query: Query<HashMap<String, String>>,
    _request: Request,
) -> Response<Body> {
    json_response(
        StatusCode::NOT_IMPLEMENTED,
        json!({
            "error": "Stock API requires Python feature",
            "path": format!("/stock/{}", path),
            "method": method.as_str()
        }),
    )
}

// ==================== 辅助函数 ====================

/// 构建 JSON 响应
fn json_response<T: serde::Serialize>(status: StatusCode, body: T) -> Response<Body> {
    Response::builder()
        .status(status)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_string(&body).unwrap_or_default()))
        .unwrap()
}

/// 构建带缓存控制的 JSON 响应
#[cfg(feature = "python")]
fn build_json_response(
    status: StatusCode,
    body: String,
    cache_max_age: Option<u32>,
) -> Response<Body> {
    let mut builder = Response::builder()
        .status(status)
        .header(header::CONTENT_TYPE, "application/json");

    if let Some(max_age) = cache_max_age {
        builder = builder.header(header::CACHE_CONTROL, format!("max-age={}", max_age));
    } else {
        builder = builder.header(header::CACHE_CONTROL, "no-cache");
    }

    builder.body(Body::from(body)).unwrap()
}
