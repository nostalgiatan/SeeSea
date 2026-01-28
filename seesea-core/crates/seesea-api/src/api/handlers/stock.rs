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
//! 处理所有 /api/stock/* 请求
//!
//! 设计原则：
//! 1. 所有缓存键与Python端的build_cache_key函数保持一致
//! 2. 直接使用 seesea-stock crate 调用 akshare 获取数据
//! 3. 使用 Rust 原生缓存系统

use crate::api::on::ApiState;

use axum::body::Body;

use axum::{
    extract::Path,
    extract::Query,
    extract::State,
    http::{Method, Response, StatusCode},
};

use serde_json::json;
use std::collections::HashMap;
use tracing::{debug, error, warn};

use seesea_stock::StockProcessor;

// ============================================================================
// 缓存作用域常量（用于 read_from_cache_only）
// ============================================================================

const SCOPE_STOCK_INFO: &str = "stock.info";
const SCOPE_STOCK_QUOTE: &str = "stock.quote";

// ============================================================================
// akshare 接口名称常量
// ============================================================================

// 基础信息接口
const INTERFACE_STOCK_INFO_A_CODE_NAME: &str = "stock_info_a_code_name";
const INTERFACE_STOCK_INFO_B_CODE_NAME: &str = "stock_info_b_code_name";
const INTERFACE_STOCK_INFO_SH_NAME_CODE: &str = "stock_info_sh_name_code";
const INTERFACE_STOCK_INFO_SZ_NAME_CODE: &str = "stock_info_sz_name_code";

// 实时行情接口
const INTERFACE_STOCK_ZH_A_SPOT_EM: &str = "stock_zh_a_spot_em";
const INTERFACE_STOCK_ZH_B_SPOT_EM: &str = "stock_zh_b_spot_em";
const INTERFACE_STOCK_US_SPOT_EM: &str = "stock_us_spot_em";
const INTERFACE_STOCK_HK_SPOT_EM: &str = "stock_hk_spot_em";

// 板块数据接口
const INTERFACE_STOCK_BOARD_INDUSTRY_NAME_EM: &str = "stock_board_industry_name_em";
const INTERFACE_STOCK_BOARD_CONCEPT_NAME_EM: &str = "stock_board_concept_name_em";

// 指数数据接口
const INTERFACE_STOCK_ZH_INDEX_SPOT_EM: &str = "stock_zh_index_spot_em";

// 资金流向接口
const INTERFACE_STOCK_MARKET_FUND_FLOW: &str = "stock_market_fund_flow";

// ============================================================================
// 辅助函数
// ============================================================================

/// 创建JSON响应
fn json_response<T: serde::Serialize>(status: StatusCode, data: T) -> Response<Body> {
    Response::builder()
        .status(status)
        .header("Content-Type", "application/json")
        .body(Body::from(
            serde_json::to_string(&data).unwrap_or_else(|_| "{}".to_string()),
        ))
        .unwrap_or_else(|_| {
            Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Body::empty())
                .unwrap()
        })
}

/// 创建错误响应
fn error_response(status: StatusCode, error: &str, message: &str) -> Response<Body> {
    json_response(
        status,
        json!({
            "error": error,
            "message": message
        }),
    )
}

/// 从缓存读取并解析JSON数据
fn read_cache_as_json(
    data: &[u8],
    log_prefix: &str,
) -> Result<serde_json::Value, Box<Response<Body>>> {
    serde_json::from_slice::<serde_json::Value>(data).map_err(|e| {
        error!("[{}] JSON解析失败: {:?}", log_prefix, e);
        Box::new(json_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            json!({"error": "Failed to parse cached data"}),
        ))
    })
}

fn read_from_cache_only(
    state: &ApiState,
    cache_key: &str,
    scope: &str,
    log_prefix: &str,
    not_found_error: &str,
    not_found_message: &str,
) -> Response<Body> {
    debug!(
        "[{}] 缓存查询 - 作用域: {}, 键: {}",
        log_prefix, scope, cache_key
    );

    match state.cache.scope(scope).get(cache_key) {
        Ok(Some(data)) => {
            debug!(
                "[{}] 缓存命中: {} (数据大小: {} 字节)",
                log_prefix,
                cache_key,
                data.len()
            );
            match read_cache_as_json(data.as_slice(), log_prefix) {
                Ok(json_data) => json_response(StatusCode::OK, json_data),
                Err(response) => *response,
            }
        }
        Ok(None) => {
            warn!(
                "[{}] 缓存未命中 - 作用域: {}, 键: {}",
                log_prefix, scope, cache_key
            );
            error_response(StatusCode::NOT_FOUND, not_found_error, not_found_message)
        }
        Err(e) => {
            error!(
                "[{}] 缓存查询失败 - 作用域: {}, 键: {}, 错误: {:?}",
                log_prefix, scope, cache_key, e
            );
            error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Cache query failed",
                "Unable to access cache system",
            )
        }
    }
}

// ============================================================================
// API处理器
// ============================================================================

async fn handle_stock_search(state: &ApiState, query: &HashMap<String, String>) -> Response<Body> {
    let keyword = query.get("keyword").cloned().unwrap_or_default();
    let limit = query
        .get("limit")
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(20);

    if keyword.is_empty() {
        return error_response(
            StatusCode::BAD_REQUEST,
            "keyword is required",
            "Please provide a search keyword",
        );
    }

    // 从缓存获取全量数据
    let data = read_from_cache_only(
        state,
        INTERFACE_STOCK_ZH_A_SPOT_EM,
        SCOPE_STOCK_QUOTE,
        "SEARCH",
        "No cached data",
        "当前无缓存数据，请稍后再试",
    );

    // 如果获取失败直接返回错误
    if data.status() != StatusCode::OK {
        return data;
    }

    // 提取 body 并搜索
    let body_bytes = match axum::body::to_bytes(data.into_body(), usize::MAX).await {
        Ok(bytes) => bytes,
        Err(_) => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Body read failed",
                "Unable to read response body",
            );
        }
    };

    match serde_json::from_slice::<serde_json::Value>(&body_bytes) {
        Ok(json_data) => {
            let processor = StockProcessor::new();
            match processor.search(&json_data, &keyword, Some(limit)) {
                Ok(results) => {
                    debug!("[SEARCH] 搜索 '{}' 找到 {} 个结果", keyword, results.len());
                    json_response(StatusCode::OK, results)
                }
                Err(e) => {
                    error!("[SEARCH] 搜索失败: {:?}", e);
                    error_response(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "Search failed",
                        &format!("{}", e),
                    )
                }
            }
        }
        Err(e) => {
            error!("[SEARCH] JSON解析失败: {:?}", e);
            error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Data parse failed",
                "Unable to parse stock data",
            )
        }
    }
}

async fn handle_stock_info(state: &ApiState, code: &str) -> Response<Body> {
    if code.is_empty() {
        return error_response(
            StatusCode::BAD_REQUEST,
            "code is required",
            "Please provide a stock code",
        );
    }

    let data = read_from_cache_only(
        state,
        INTERFACE_STOCK_ZH_A_SPOT_EM,
        SCOPE_STOCK_QUOTE,
        "STOCK_INFO",
        "No cached data",
        "当前无缓存数据，请稍后再试",
    );

    if data.status() != StatusCode::OK {
        return data;
    }

    let body_bytes = match axum::body::to_bytes(data.into_body(), usize::MAX).await {
        Ok(bytes) => bytes,
        Err(_) => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Body read failed",
                "Unable to read response body",
            );
        }
    };

    match serde_json::from_slice::<serde_json::Value>(&body_bytes) {
        Ok(json_data) => {
            let processor = StockProcessor::new();
            match processor.get_by_code(&json_data, code) {
                Ok(Some(stock)) => {
                    debug!("[STOCK_INFO] 获取股票信息: {}", code);
                    json_response(StatusCode::OK, stock)
                }
                Ok(None) => error_response(
                    StatusCode::NOT_FOUND,
                    "Stock not found",
                    &format!("Stock {} not found", code),
                ),
                Err(e) => error_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Search failed",
                    &format!("{}", e),
                ),
            }
        }
        Err(e) => {
            error!("[STOCK_INFO] JSON解析失败: {:?}", e);
            error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Data parse failed",
                "Unable to parse stock data",
            )
        }
    }
}

async fn handle_stock_detail(state: &ApiState, code: &str) -> Response<Body> {
    handle_stock_info(state, code).await
}

async fn handle_stock_quote(state: &ApiState, query: &HashMap<String, String>) -> Response<Body> {
    let code = query.get("code").cloned().unwrap_or_default();

    if code.is_empty() {
        return error_response(
            StatusCode::BAD_REQUEST,
            "code is required",
            "Please provide a stock code",
        );
    }

    let data = read_from_cache_only(
        state,
        INTERFACE_STOCK_ZH_A_SPOT_EM,
        SCOPE_STOCK_QUOTE,
        "QUOTE",
        "No cached data",
        "当前无缓存数据，请稍后再试",
    );

    if data.status() != StatusCode::OK {
        return data;
    }

    let body_bytes = match axum::body::to_bytes(data.into_body(), usize::MAX).await {
        Ok(bytes) => bytes,
        Err(_) => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Body read failed",
                "Unable to read response body",
            );
        }
    };

    match serde_json::from_slice::<serde_json::Value>(&body_bytes) {
        Ok(json_data) => {
            let processor = StockProcessor::new();
            match processor.get_by_code(&json_data, code.as_str()) {
                Ok(Some(stock)) => {
                    debug!("[QUOTE] 获取股票行情: {}", code);
                    json_response(StatusCode::OK, stock)
                }
                Ok(None) => error_response(
                    StatusCode::NOT_FOUND,
                    "Stock not found",
                    &format!("Stock {} not found", code),
                ),
                Err(e) => error_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Search failed",
                    &format!("{}", e),
                ),
            }
        }
        Err(e) => {
            error!("[QUOTE] JSON解析失败: {:?}", e);
            error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Data parse failed",
                "Unable to parse stock data",
            )
        }
    }
}

async fn handle_stock_realtime(state: &ApiState, code: &str) -> Response<Body> {
    if code.is_empty() {
        return error_response(
            StatusCode::BAD_REQUEST,
            "code is required",
            "Please provide a stock code",
        );
    }

    let mut query = HashMap::new();
    query.insert("code".to_string(), code.to_string());
    handle_stock_quote(state, &query).await
}

async fn handle_quote_stream(_state: &ApiState, query: &HashMap<String, String>) -> Response<Body> {
    let code = query.get("code").cloned().unwrap_or_default();

    if code.is_empty() {
        return error_response(
            StatusCode::BAD_REQUEST,
            "code is required",
            "Please provide a stock code",
        );
    }

    error_response(
        StatusCode::NOT_IMPLEMENTED,
        "Not implemented",
        "行情流功能暂未实现",
    )
}

async fn handle_stock_kline(_state: &ApiState, query: &HashMap<String, String>) -> Response<Body> {
    let code = query.get("code").cloned().unwrap_or_default();

    if code.is_empty() {
        return error_response(
            StatusCode::BAD_REQUEST,
            "code is required",
            "Please provide a stock code",
        );
    }

    error_response(
        StatusCode::NOT_IMPLEMENTED,
        "Not implemented",
        "K线数据功能暂未实现",
    )
}

async fn handle_stock_industry(_state: &ApiState, code: &str) -> Response<Body> {
    if code.is_empty() {
        read_from_cache_only(
            _state,
            INTERFACE_STOCK_BOARD_INDUSTRY_NAME_EM,
            "stock.industry",
            "INDUSTRY_LIST",
            "No cached data",
            "当前无缓存数据，请稍后再试",
        )
    } else {
        error_response(
            StatusCode::NOT_IMPLEMENTED,
            "Not implemented",
            "行业板块成份股功能暂未实现",
        )
    }
}

async fn handle_sectors(state: &ApiState, query: &HashMap<String, String>) -> Response<Body> {
    let sector_type = query
        .get("type")
        .cloned()
        .unwrap_or_else(|| "industry".to_string());

    let (interface, scope) = match sector_type.as_str() {
        "concept" => (INTERFACE_STOCK_BOARD_CONCEPT_NAME_EM, "stock.industry"),
        _ => (INTERFACE_STOCK_BOARD_INDUSTRY_NAME_EM, "stock.industry"),
    };

    read_from_cache_only(
        state,
        interface,
        scope,
        "SECTORS",
        "No cached data",
        "当前无缓存数据，请稍后再试",
    )
}

async fn handle_sector_stocks(_state: &ApiState, code: &str) -> Response<Body> {
    if code.is_empty() {
        return error_response(
            StatusCode::BAD_REQUEST,
            "code is required",
            "Please provide a sector code",
        );
    }

    error_response(
        StatusCode::NOT_IMPLEMENTED,
        "Not implemented",
        "板块成份股功能暂未实现",
    )
}

async fn handle_market_indices(state: &ApiState) -> Response<Body> {
    read_from_cache_only(
        state,
        INTERFACE_STOCK_ZH_INDEX_SPOT_EM,
        "stock.index",
        "INDICES",
        "No cached data",
        "当前无缓存数据，请稍后再试",
    )
}

async fn handle_fund_flow(
    state: &ApiState,
    code: &str,
    _query: &HashMap<String, String>,
) -> Response<Body> {
    if code.is_empty() {
        read_from_cache_only(
            state,
            INTERFACE_STOCK_MARKET_FUND_FLOW,
            "stock.fund_flow",
            "MARKET_FUND_FLOW",
            "No cached data",
            "当前无缓存数据，请稍后再试",
        )
    } else {
        error_response(
            StatusCode::NOT_IMPLEMENTED,
            "Not implemented",
            "个股资金流向功能暂未实现",
        )
    }
}

async fn handle_stock_financial(
    _state: &ApiState,
    code: &str,
    _query: &HashMap<String, String>,
) -> Response<Body> {
    if code.is_empty() {
        return error_response(
            StatusCode::BAD_REQUEST,
            "code is required",
            "Please provide a stock code",
        );
    }

    error_response(
        StatusCode::NOT_IMPLEMENTED,
        "Not implemented",
        "财务数据功能暂未实现",
    )
}

async fn handle_stock_holders(
    _state: &ApiState,
    code: &str,
    _query: &HashMap<String, String>,
) -> Response<Body> {
    if code.is_empty() {
        return error_response(
            StatusCode::BAD_REQUEST,
            "code is required",
            "Please provide a stock code",
        );
    }

    error_response(
        StatusCode::NOT_IMPLEMENTED,
        "Not implemented",
        "股东数据功能暂未实现",
    )
}

async fn handle_stock_announcements(
    _state: &ApiState,
    code: &str,
    _query: &HashMap<String, String>,
) -> Response<Body> {
    if code.is_empty() {
        return error_response(
            StatusCode::BAD_REQUEST,
            "code is required",
            "Please provide a stock code",
        );
    }

    error_response(
        StatusCode::NOT_IMPLEMENTED,
        "Not implemented",
        "公告数据功能暂未实现",
    )
}

async fn handle_market_status(
    state: &ApiState,
    _query: &HashMap<String, String>,
) -> Response<Body> {
    read_from_cache_only(
        state,
        INTERFACE_STOCK_ZH_INDEX_SPOT_EM,
        "stock.index",
        "MARKET_STATUS",
        "No cached data",
        "当前无缓存数据，请稍后再试",
    )
}

async fn handle_lhb_data(state: &ApiState, query: &HashMap<String, String>) -> Response<Body> {
    let date = query
        .get("date")
        .cloned()
        .unwrap_or_else(|| chrono::Local::now().format("%Y%m%d").to_string());

    let cache_key = format!("stock_zt_pool_em__date_{}", date);
    read_from_cache_only(
        state,
        &cache_key,
        "stock.ranking",
        "LHB",
        "No cached data",
        "当前无缓存数据，请稍后再试",
    )
}

async fn handle_stock_ranking(state: &ApiState, query: &HashMap<String, String>) -> Response<Body> {
    let ranking_type = query
        .get("type")
        .cloned()
        .unwrap_or_else(|| "zt".to_string());
    let date = query
        .get("date")
        .cloned()
        .unwrap_or_else(|| chrono::Local::now().format("%Y%m%d").to_string());

    let cache_key = match ranking_type.as_str() {
        "zt" => format!("stock_zt_pool_em__date_{}", date),
        "dt" => format!("stock_zt_pool_dtgc_em__date_{}", date),
        _ => format!("stock_zt_pool_em__date_{}", date),
    };

    read_from_cache_only(
        state,
        &cache_key,
        "stock.ranking",
        "RANKING",
        "No cached data",
        "当前无缓存数据，请稍后再试",
    )
}

async fn handle_restricted_stock_release(
    state: &ApiState,
    _query: &HashMap<String, String>,
) -> Response<Body> {
    read_from_cache_only(
        state,
        "restricted_releases",
        SCOPE_STOCK_INFO,
        "RESTRICTED",
        "Data not available",
        "Restricted stock release data is not currently available",
    )
}

async fn handle_stock_pledge(state: &ApiState, _query: &HashMap<String, String>) -> Response<Body> {
    read_from_cache_only(
        state,
        "stock_pledges",
        SCOPE_STOCK_INFO,
        "PLEDGE",
        "Data not available",
        "Stock pledge data is not currently available",
    )
}

async fn handle_stock_repurchase(
    state: &ApiState,
    _query: &HashMap<String, String>,
) -> Response<Body> {
    read_from_cache_only(
        state,
        "stock_repurchases",
        SCOPE_STOCK_INFO,
        "REPURCHASE",
        "Data not available",
        "Stock repurchase data is not currently available",
    )
}

// ============================================================================
// 批量数据和多市场处理器
// ============================================================================

/// 处理A股列表请求
async fn handle_a_stock_list(state: &ApiState) -> Response<Body> {
    read_from_cache_only(
        state,
        INTERFACE_STOCK_ZH_A_SPOT_EM,
        SCOPE_STOCK_QUOTE,
        "A_STOCK_LIST",
        "No cached data",
        "当前无缓存数据，请稍后再试",
    )
}

/// 处理B股列表请求
async fn handle_b_stock_list(state: &ApiState) -> Response<Body> {
    read_from_cache_only(
        state,
        INTERFACE_STOCK_ZH_B_SPOT_EM,
        SCOPE_STOCK_QUOTE,
        "B_STOCK_LIST",
        "No cached data",
        "当前无缓存数据，请稍后再试",
    )
}

/// 处理港股列表请求
async fn handle_hk_stock_list(state: &ApiState) -> Response<Body> {
    read_from_cache_only(
        state,
        INTERFACE_STOCK_HK_SPOT_EM,
        SCOPE_STOCK_QUOTE,
        "HK_STOCK_LIST",
        "No cached data",
        "当前无缓存数据，请稍后再试",
    )
}

/// 处理美股列表请求
async fn handle_us_stock_list(state: &ApiState) -> Response<Body> {
    read_from_cache_only(
        state,
        INTERFACE_STOCK_US_SPOT_EM,
        SCOPE_STOCK_QUOTE,
        "US_STOCK_LIST",
        "No cached data",
        "当前无缓存数据，请稍后再试",
    )
}

/// 处理股票代码名称映射
async fn handle_stock_codes(state: &ApiState, query: &HashMap<String, String>) -> Response<Body> {
    let market = query
        .get("market")
        .cloned()
        .unwrap_or_else(|| "a".to_string());

    let (interface, scope) = match market.as_str() {
        "a" => (INTERFACE_STOCK_INFO_A_CODE_NAME, "stock.info"),
        "b" => (INTERFACE_STOCK_INFO_B_CODE_NAME, "stock.info"),
        "sh" => (INTERFACE_STOCK_INFO_SH_NAME_CODE, "stock.info"),
        "sz" => (INTERFACE_STOCK_INFO_SZ_NAME_CODE, "stock.info"),
        _ => (INTERFACE_STOCK_INFO_A_CODE_NAME, "stock.info"),
    };

    read_from_cache_only(
        state,
        interface,
        scope,
        "CODE_NAME",
        "No cached data",
        "当前无缓存数据，请稍后再试",
    )
}

/// 处理分钟K线请求
async fn handle_stock_kline_minute(
    _state: &ApiState,
    query: &HashMap<String, String>,
) -> Response<Body> {
    let code = query.get("code").cloned().unwrap_or_default();

    if code.is_empty() {
        return error_response(
            StatusCode::BAD_REQUEST,
            "code is required",
            "Please provide a stock code",
        );
    }

    error_response(
        StatusCode::NOT_IMPLEMENTED,
        "Not implemented",
        "分钟K线功能暂未实现",
    )
}

/// 处理港股K线请求
async fn handle_hk_kline(_state: &ApiState, query: &HashMap<String, String>) -> Response<Body> {
    let code = query.get("code").cloned().unwrap_or_default();

    if code.is_empty() {
        return error_response(
            StatusCode::BAD_REQUEST,
            "code is required",
            "Please provide a stock code",
        );
    }

    error_response(
        StatusCode::NOT_IMPLEMENTED,
        "Not implemented",
        "港股K线功能暂未实现",
    )
}

/// 处理指数历史数据请求
async fn handle_index_history(
    _state: &ApiState,
    query: &HashMap<String, String>,
) -> Response<Body> {
    let code = query.get("code").cloned().unwrap_or_default();

    if code.is_empty() {
        return error_response(
            StatusCode::BAD_REQUEST,
            "code is required",
            "Please provide an index code",
        );
    }

    error_response(
        StatusCode::NOT_IMPLEMENTED,
        "Not implemented",
        "指数历史数据功能暂未实现",
    )
}

/// 处理概念板块列表请求
async fn handle_concept_list(state: &ApiState) -> Response<Body> {
    read_from_cache_only(
        state,
        INTERFACE_STOCK_BOARD_CONCEPT_NAME_EM,
        "stock.industry",
        "CONCEPT_LIST",
        "No cached data",
        "当前无缓存数据，请稍后再试",
    )
}

/// 处理概念板块成份股请求
async fn handle_concept_stocks(_state: &ApiState, code: &str) -> Response<Body> {
    if code.is_empty() {
        return error_response(
            StatusCode::BAD_REQUEST,
            "code is required",
            "Please provide a concept code",
        );
    }

    error_response(
        StatusCode::NOT_IMPLEMENTED,
        "Not implemented",
        "概念板块成份股功能暂未实现",
    )
}

// ============================================================================
// 主路由处理器
// ============================================================================

/// 处理所有股票API请求
pub async fn handle_stock_api(
    State(state): State<ApiState>,
    method: Method,
    Path(path): Path<String>,
    Query(query): Query<HashMap<String, String>>,
) -> Response<Body> {
    debug!("[STOCK_API] {} /{}", method.as_str(), path);

    let segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();

    match (method.as_str(), segments.as_slice()) {
        // 搜索
        ("GET", ["search"]) => handle_stock_search(&state, &query).await,

        // 行情数据
        ("GET", ["quote"]) => handle_stock_quote(&state, &query).await,
        ("GET", ["quote", "stream"]) => handle_quote_stream(&state, &query).await,
        ("GET", ["realtime", code]) => handle_stock_realtime(&state, code).await,

        // K线数据
        ("GET", ["kline"]) => handle_stock_kline(&state, &query).await,
        ("GET", ["kline", "minute"]) => handle_stock_kline_minute(&state, &query).await,
        ("GET", ["kline", "hk"]) => handle_hk_kline(&state, &query).await,

        // 股票信息
        ("GET", ["info", code]) => handle_stock_info(&state, code).await,
        ("GET", ["detail", code]) => handle_stock_detail(&state, code).await,
        ("GET", ["financial", code]) => handle_stock_financial(&state, code, &query).await,
        ("GET", ["holders", code]) => handle_stock_holders(&state, code, &query).await,
        ("GET", ["announcements", code]) => handle_stock_announcements(&state, code, &query).await,

        // 股票列表（批量数据）
        ("GET", ["list", "a"]) => handle_a_stock_list(&state).await,
        ("GET", ["list", "b"]) => handle_b_stock_list(&state).await,
        ("GET", ["list", "hk"]) => handle_hk_stock_list(&state).await,
        ("GET", ["list", "us"]) => handle_us_stock_list(&state).await,
        ("GET", ["codes"]) => handle_stock_codes(&state, &query).await,

        // 板块数据
        ("GET", ["industry"]) => handle_stock_industry(&state, "").await,
        ("GET", ["industry", code]) => handle_stock_industry(&state, code).await,
        ("GET", ["sectors"]) => handle_sectors(&state, &query).await,
        ("GET", ["sectors", code, "stocks"]) => handle_sector_stocks(&state, code).await,
        ("GET", ["concept"]) => handle_concept_list(&state).await,
        ("GET", ["concept", code, "stocks"]) => handle_concept_stocks(&state, code).await,

        // 资金流向
        ("GET", ["fund_flow"]) => handle_fund_flow(&state, "", &query).await,
        ("GET", ["fund_flow", code]) => handle_fund_flow(&state, code, &query).await,

        // 市场数据
        ("GET", ["market", "status"]) => handle_market_status(&state, &query).await,
        ("GET", ["market", "indices"]) => handle_market_indices(&state).await,
        ("GET", ["market", "lhb"]) => handle_lhb_data(&state, &query).await,
        ("GET", ["index", "history"]) => handle_index_history(&state, &query).await,

        // 排行榜
        ("GET", ["ranking"]) => handle_stock_ranking(&state, &query).await,

        // 其他数据
        ("GET", ["restricted"]) => handle_restricted_stock_release(&state, &query).await,
        ("GET", ["pledge"]) => handle_stock_pledge(&state, &query).await,
        ("GET", ["repurchase"]) => handle_stock_repurchase(&state, &query).await,
        _ => {
            warn!("[STOCK_API] 未匹配路由: {} /{}", method.as_str(), path);
            error_response(
                StatusCode::NOT_FOUND,
                "Endpoint not found",
                &format!("The path /{} is not a valid stock API endpoint", path),
            )
        }
    }
}
