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
//! 处理所有 /api/stock/* 请求

use crate::api::on::ApiState;
use axum::body::Body;
use axum::{
    extract::Path,
    extract::Query,
    extract::State,
    http::{Method, Response, StatusCode},
};

use serde::Deserialize;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{RwLock, Notify};
use tokio::time::sleep;
use lazy_static::lazy_static;

use seesea_event::{StringAsyncEventOperations, get_global_async_event_bus};

/// 股票缓存作用域常量
const STOCK_LIST_SCOPE: &str = "stock.list";
const STOCK_QUOTE_SCOPE: &str = "stock.quote";
const STOCK_KLINE_SCOPE: &str = "stock.kline";
const STOCK_DETAIL_SCOPE: &str = "stock.detail";
const STOCK_FINANCIAL_SCOPE: &str = "stock.financial";
const STOCK_FUND_FLOW_SCOPE: &str = "stock.fund_flow";
const STOCK_HOLDERS_SCOPE: &str = "stock.holders";
const STOCK_ANNOUNCEMENTS_SCOPE: &str = "stock.announcements";
const STOCK_MARKET_STATUS_SCOPE: &str = "stock.market_status";
const STOCK_LHB_SCOPE: &str = "stock.lhb";
const STOCK_SECTORS_SCOPE: &str = "stock.sectors";
const STOCK_RANKING_SCOPE: &str = "stock.ranking";

/// 股票缓存键名生成函数
fn stock_list_key(market: &str) -> String {
    format!("list:{}", market)
}

fn stock_quote_key(code: &str) -> String {
    format!("quote:{}", code)
}

fn stock_stream_key(code: &str) -> String {
    format!("stream:{}", code)
}

fn stock_kline_key(code: &str, period: &str) -> String {
    format!("kline:{}:{}", code, period)
}

fn stock_detail_key(code: &str) -> String {
    format!("detail:{}", code)
}

fn stock_financial_key(code: &str, report_type: &str) -> String {
    format!("financial:{}:{}", code, report_type)
}

fn stock_fund_flow_key(code: &str, flow_type: &str) -> String {
    format!("fund_flow:{}:{}", code, flow_type)
}

fn stock_holders_key(code: &str, holder_type: &str) -> String {
    format!("holders:{}:{}", code, holder_type)
}

fn stock_announcements_key(code: &str, page: usize, limit: usize) -> String {
    format!("announcements:{}:{}:{}", code, page, limit)
}

fn stock_market_status_key(market: &str) -> String {
    format!("market_status:{}", market)
}

fn stock_lhb_key(date: &str, market: &str) -> String {
    format!("lhb:{}:{}", date, market)
}

fn stock_sectors_key(sector_type: &str, market: &str) -> String {
    format!("sectors:{}:{}", sector_type, market)
}

fn stock_ranking_key(ranking_type: &str, market: &str) -> String {
    format!("ranking:{}:{}", ranking_type, market)
}

/// Helper function to create JSON responses
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

/// 等待中的请求信息
struct PendingRequest {
    notify: Arc<Notify>,
}

/// 全局待处理请求跟踪器
struct PendingRequestsTracker {
    requests: Arc<RwLock<HashMap<String, PendingRequest>>>,
}

impl PendingRequestsTracker {
    fn new() -> Self {
        Self {
            requests: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    async fn add_request(&self, _cache_key: String, _scope: String) -> Arc<Notify> {
        let notify = Arc::new(Notify::new());
        let request = PendingRequest {
            notify: notify.clone(),
        };
        self.requests.write().await.insert(_cache_key, request);
        notify
    }

    async fn remove_request(&self, cache_key: &str) {
        self.requests.write().await.remove(cache_key);
    }

    async fn notify_request(&self, cache_key: &str) {
        let requests = self.requests.read().await;
        if let Some(request) = requests.get(cache_key) {
            request.notify.notify_one();
        }
    }
}

lazy_static! {
    static ref PENDING_REQUESTS: PendingRequestsTracker = PendingRequestsTracker::new();
}

/// 初始化股票数据完成事件监听器
pub async fn init_stock_data_done_listener() {
    let bus = get_global_async_event_bus();
    
    match bus.on("stock.data.done", |_event_type: &str, data: &str| {
        let data = data.to_string();
        Box::pin(async move {
            // 解析事件数据
            if let Ok(event_data) = serde_json::from_str::<serde_json::Value>(&data) {
                if let Some(code) = event_data.get("code").and_then(|v| v.as_str()) {
                    if let Some(data_type) = event_data.get("data_type").and_then(|v| v.as_str()) {
                        // 根据数据类型生成缓存键
                        let cache_key = match data_type {
                            "info" => stock_detail_key(code),
                            "financial" => {
                                let report_type = event_data.get("report_type")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("annual");
                                stock_financial_key(code, report_type)
                            },
                            "industry" => format!("industry:{}", code),
                            "quote" => stock_quote_key(code),
                            "kline" => stock_kline_key(code, "daily"),
                            "holders" => stock_holders_key(code, "top10"),
                            "announcements" => stock_announcements_key(code, 1, 10),
                            _ => {
                                println!("⚠️ [STOCK_DONE] 未知的数据类型: {}", data_type);
                                return seesea_event::EventPayload::Empty;
                            }
                        };
                        
                        println!("📥 [STOCK_DONE] 收到数据完成事件: code={}, data_type={}, cache_key={}", 
                            code, data_type, cache_key);
                        
                        // 通知等待中的请求
                        PENDING_REQUESTS.notify_request(&cache_key).await;
                    }
                }
            }
            
            seesea_event::EventPayload::Empty
        })
    }).await {
        Ok(_) => println!("✅ [STOCK_DONE] 股票数据完成事件监听器已注册"),
        Err(e) => println!("❌ [STOCK_DONE] 注册事件监听器失败: {:?}", e),
    }
}

/// 处理缓存未命中：发送事件、等待并重试（事件驱动版本）
async fn handle_cache_miss_with_retry(
    state: &ApiState,
    cache_key: &str,
    scope: &str,
    event_data: &str,
    log_prefix: &str,
    max_retries: u32,
    retry_delay_ms: u64,
) -> Response<Body> {
    // 发送事件请求Python获取数据
    let bus = get_global_async_event_bus();
    match bus.send_string_notification("stock.data.request", event_data) {
        Ok(_) => println!("📤 [{}] 已发送数据请求事件", log_prefix),
        Err(e) => println!("❌ [{}] 发送事件失败: {:?}", log_prefix, e),
    }

    // 添加到待处理请求列表，以便接收完成通知
    let notify = PENDING_REQUESTS.add_request(cache_key.to_string(), scope.to_string()).await;

    // 等待数据就绪（事件驱动 + 超时重试）
    for retry in 0..max_retries {
        // 等待通知或超时
        let timeout = Duration::from_millis(retry_delay_ms);
        tokio::select! {
            _ = notify.notified() => {
                // 收到通知，立即检查缓存
                println!("🔔 [{}] 收到数据就绪通知，检查缓存", log_prefix);
            }
            _ = sleep(timeout) => {
                // 超时，继续下一次重试
                if retry < max_retries - 1 {
                    println!("⏳ [{}] 等待超时，继续重试... ({}/{})", log_prefix, retry + 1, max_retries);
                }
            }
        }
        
        // 检查缓存
        let cache_result = state.cache.scope(scope).get(cache_key);
        match cache_result {
            Ok(Some(data)) => {
                println!("✅ [{}] 重试第{}次成功: 从缓存获取到数据", log_prefix, retry + 1);
                
                // 从待处理列表中移除
                PENDING_REQUESTS.remove_request(cache_key).await;
                
                match serde_json::from_slice::<serde_json::Value>(data.as_slice()) {
                    Ok(json_data) => return json_response(StatusCode::OK, json_data),
                    Err(e) => {
                        println!("❌ [{}] JSON解析失败: {:?}", log_prefix, e);
                        return json_response(
                            StatusCode::INTERNAL_SERVER_ERROR,
                            json!({"error": "Failed to parse cached data"}),
                        );
                    }
                }
            }
            Ok(None) => {
                if retry < max_retries - 1 {
                    println!("⏳ [{}] 重试第{}次: 缓存中仍无数据，继续等待...", log_prefix, retry + 1);
                }
            }
            Err(e) => {
                println!("❌ [{}] 缓存查询失败: {:?}", log_prefix, e);
                
                // 从待处理列表中移除
                PENDING_REQUESTS.remove_request(cache_key).await;
                
                return json_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    json!({"error": "Cache query failed"}),
                );
            }
        }
    }

    // 所有重试都失败
    println!("❌ [{}] 重试{}次后仍未获取到数据", log_prefix, max_retries);
    
    // 从待处理列表中移除
    PENDING_REQUESTS.remove_request(cache_key).await;
    
    json_response(
        StatusCode::NOT_FOUND,
        json!({"error": "Data not available after retries"}),
    )
}

/// 股票搜索请求参数
#[derive(Debug, Deserialize)]
pub struct StockSearchQuery {
    pub keyword: String,
    pub market: Option<String>,
    pub limit: Option<usize>,
}

/// 股票行情请求参数
#[derive(Debug, Deserialize)]
pub struct StockQuoteQuery {
    pub code: String,
    pub market: Option<String>,
}

/// K线数据请求参数
#[derive(Debug, Deserialize)]
pub struct KLineQuery {
    pub code: String,
    pub period: Option<String>,
    pub limit: Option<usize>,
}

/// 股票详情请求参数
#[derive(Debug, Deserialize)]
pub struct StockDetailQuery {
    pub code: String,
    pub market: Option<String>,
}

/// 财务数据请求参数
#[derive(Debug, Deserialize)]
pub struct FinancialQuery {
    pub code: String,
    pub report_type: Option<String>,
    pub years: Option<usize>,
}

/// 资金流向请求参数
#[derive(Debug, Deserialize)]
pub struct FundFlowQuery {
    pub code: String,
    pub days: Option<usize>,
    pub market: Option<String>,
}

/// 股东数据请求参数
#[derive(Debug, Deserialize)]
pub struct HoldersQuery {
    pub code: String,
    pub quarter: Option<String>,
}

/// 公告数据请求参数
#[derive(Debug, Deserialize)]
pub struct AnnouncementsQuery {
    pub code: String,
    pub page: Option<usize>,
    pub limit: Option<usize>,
}

/// 市场状态请求参数
#[derive(Debug, Deserialize)]
pub struct MarketStatusQuery {
    pub market: Option<String>,
}

/// 龙虎榜数据请求参数
#[derive(Debug, Deserialize)]
pub struct LhbQuery {
    pub date: Option<String>,
    pub market: Option<String>,
}

/// 板块数据请求参数
#[derive(Debug, Deserialize)]
pub struct SectorsQuery {
    pub sector_type: Option<String>,
    pub market: Option<String>,
}

/// 排行榜数据请求参数
#[derive(Debug, Deserialize)]
pub struct RankingQuery {
    pub ranking_type: Option<String>,
    pub market: Option<String>,
    pub limit: Option<usize>,
}

/// 股票基本信息
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct StockBasic {
    pub code: String,
    pub name: String,
    pub market: String,
    pub industry: Option<String>,
    pub area: Option<String>,
}

/// 股票行情数据
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct StockQuote {
    pub code: String,
    pub name: String,
    pub price: f64,
    pub change: f64,
    pub change_pct: f64,
    pub volume: i64,
    pub amount: f64,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub prev_close: f64,
    pub time: String,
}

/// K线数据
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct KLineData {
    pub date: String,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: i64,
    pub amount: f64,
    pub change: f64,
    pub change_pct: f64,
}

/// 市场状态
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct MarketStatus {
    pub market: String,
    pub is_open: bool,
    pub open_time: Option<String>,
    pub close_time: Option<String>,
    pub status: String,
    pub description: String,
}

/// 指数行情
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct IndexQuote {
    pub code: String,
    pub name: String,
    pub price: f64,
    pub change: f64,
    pub change_pct: f64,
    pub volume: i64,
    pub amount: f64,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub prev_close: f64,
    pub time: String,
}

/// 股票搜索处理器
async fn handle_stock_search(state: &ApiState, query: &HashMap<String, String>) -> Response<Body> {
    let keyword = query.get("keyword").unwrap_or(&String::new()).to_string();

    if keyword.is_empty() {
        return json_response(
            StatusCode::BAD_REQUEST,
            json!({"error": "keyword is required"}),
        );
    }

    println!("🔍 [STOCK_SEARCH] 开始搜索 - keyword: '{}'", keyword);

    // 从缓存获取股票列表，直接在Rust中搜索
    let market = "cn_a";
    let cache_key = stock_list_key(market);
    println!(
        "🔍 [STOCK_SEARCH] 查询股票列表 - scope: '{}', key: '{}'",
        STOCK_LIST_SCOPE, cache_key
    );

    let cache_result = state.cache.scope(STOCK_LIST_SCOPE).get(&cache_key);

    match cache_result {
        Ok(Some(data)) => {
            println!("✅ [STOCK_SEARCH] 股票列表缓存命中 - {} bytes", data.len());

            // 解析股票列表数据
            match serde_json::from_slice::<serde_json::Value>(data.as_slice()) {
                Ok(json_data) => {
                    if let Some(stocks_array) = json_data.as_array() {
                        let keyword_lower = keyword.to_lowercase();
                        let limit = query
                            .get("limit")
                            .and_then(|l| l.parse::<usize>().ok())
                            .unwrap_or(20);

                        let mut results = Vec::new();

                        // 在股票列表中搜索
                        for stock in stocks_array {
                            if let Some(name) = stock.get("name").and_then(|n| n.as_str())
                                && name.to_lowercase().contains(&keyword_lower)
                            {
                                results.push(stock.clone());
                                if results.len() >= limit {
                                    break;
                                }
                                continue;
                            }

                            if let Some(code) = stock.get("code").and_then(|c| c.as_str())
                                && code.to_lowercase().contains(&keyword_lower)
                            {
                                results.push(stock.clone());
                                if results.len() >= limit {
                                    break;
                                }
                                continue;
                            }

                            if let Some(industry) = stock.get("industry").and_then(|i| i.as_str())
                                && industry.to_lowercase().contains(&keyword_lower)
                            {
                                results.push(stock.clone());
                                if results.len() >= limit {
                                    break;
                                }
                            }
                        }

                        println!("✅ [STOCK_SEARCH] 搜索完成 - 找到 {} 个结果", results.len());
                        json_response(StatusCode::OK, serde_json::Value::Array(results))
                    } else {
                        println!("❌ [STOCK_SEARCH] 股票列表数据格式错误");
                        json_response(
                            StatusCode::INTERNAL_SERVER_ERROR,
                            json!({"error": "Invalid stock list format"}),
                        )
                    }
                }
                Err(e) => {
                    println!("❌ [STOCK_SEARCH] 股票列表JSON解析失败: {:?}", e);
                    json_response(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        json!({"error": "Failed to parse stock list"}),
                    )
                }
            }
        }
        Ok(None) => {
            println!("⚠️ [STOCK_SEARCH] 股票列表缓存未命中");
            json_response(
                StatusCode::NOT_FOUND,
                json!({"error": "Stock list not found in cache"}),
            )
        }
        Err(e) => {
            println!("❌ [STOCK_SEARCH] 缓存查询失败: {:?}", e);
            json_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                json!({"error": "Cache query failed"}),
            )
        }
    }
}

/// 股票行情处理器
async fn handle_stock_quote(state: &ApiState, query: &HashMap<String, String>) -> Response<Body> {
    let code = query.get("code").unwrap_or(&String::new()).to_string();

    if code.is_empty() {
        return json_response(
            StatusCode::BAD_REQUEST,
            json!({"error": "code is required"}),
        );
    }

    // 从缓存获取股票行情
    let cache_key = stock_quote_key(&code);
    let cache_result = state.cache.scope(STOCK_QUOTE_SCOPE).get(&cache_key);

    match cache_result {
        Ok(Some(data)) => {
            println!("✅ [STOCK_QUOTE] 从缓存获取股票行情: {}", code);
            // Parse the JSON data from bytes
            match serde_json::from_slice::<serde_json::Value>(data.as_slice()) {
                Ok(json_data) => json_response(StatusCode::OK, json_data),
                Err(e) => {
                    println!("❌ [STOCK_QUOTE] JSON解析失败: {:?}", e);
                    json_response(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        json!({"error": "Failed to parse cached data"}),
                    )
                }
            }
        }
        Ok(None) => {
            println!("⚠️ [STOCK_QUOTE] 缓存中无股票行情: {}", code);
            
            let event_data = json!({ "code": code, "data_type": "quote" }).to_string();
            handle_cache_miss_with_retry(
                state,
                &cache_key,
                STOCK_QUOTE_SCOPE,
                &event_data,
                "STOCK_QUOTE",
                5,
                500,
            ).await
        }
        Err(e) => {
            println!("❌ [STOCK_QUOTE] 缓存查询失败: {:?}", e);
            json_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                json!({"error": "Cache query failed"}),
            )
        }
    }
}

/// 股票流处理器
async fn handle_quote_stream(state: &ApiState, query: &HashMap<String, String>) -> Response<Body> {
    let code = query.get("code").unwrap_or(&String::new()).to_string();

    if code.is_empty() {
        return json_response(
            StatusCode::BAD_REQUEST,
            json!({"error": "code is required"}),
        );
    }

    // 从缓存获取股票流数据
    let cache_key = stock_stream_key(&code);
    let cache_result = state.cache.scope(STOCK_QUOTE_SCOPE).get(&cache_key);

    match cache_result {
        Ok(Some(data)) => {
            println!("✅ [QUOTE_STREAM] 从缓存获取股票流: {}", code);
            // Parse the JSON data from bytes
            match serde_json::from_slice::<serde_json::Value>(data.as_slice()) {
                Ok(json_data) => json_response(StatusCode::OK, json_data),
                Err(e) => {
                    println!("❌ [QUOTE_STREAM] JSON解析失败: {:?}", e);
                    json_response(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        json!({"error": "Failed to parse cached data"}),
                    )
                }
            }
        }
        Ok(None) => {
            println!("⚠️ [QUOTE_STREAM] 缓存中无股票流数据: {}", code);
            json_response(
                StatusCode::NOT_FOUND,
                json!({"error": "Quote stream not found"}),
            )
        }
        Err(e) => {
            println!("❌ [QUOTE_STREAM] 缓存查询失败: {:?}", e);
            json_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                json!({"error": "Cache query failed"}),
            )
        }
    }
}

/// K线数据处理器
async fn handle_stock_kline(state: &ApiState, query: &HashMap<String, String>) -> Response<Body> {
    let code = query.get("code").unwrap_or(&String::new()).to_string();
    let period = query
        .get("period")
        .unwrap_or(&"daily".to_string())
        .to_string();

    if code.is_empty() {
        return json_response(
            StatusCode::BAD_REQUEST,
            json!({"error": "code is required"}),
        );
    }

    // 从缓存获取K线数据
    let cache_key = stock_kline_key(&code, &period);
    let cache_result = state.cache.scope(STOCK_KLINE_SCOPE).get(&cache_key);

    match cache_result {
        Ok(Some(data)) => {
            println!("✅ [STOCK_KLINE] 从缓存获取K线数据: {} ({})", code, period);
            // Parse the JSON data from bytes
            match serde_json::from_slice::<serde_json::Value>(data.as_slice()) {
                Ok(json_data) => json_response(StatusCode::OK, json_data),
                Err(e) => {
                    println!("❌ [STOCK_KLINE] JSON解析失败: {:?}", e);
                    json_response(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        json!({"error": "Failed to parse cached data"}),
                    )
                }
            }
        }
        Ok(None) => {
            println!("⚠️ [STOCK_KLINE] 缓存中无K线数据: {} ({})", code, period);
            
            let event_data = json!({ "code": code, "data_type": "kline", "period": period }).to_string();
            handle_cache_miss_with_retry(
                state,
                &cache_key,
                STOCK_KLINE_SCOPE,
                &event_data,
                "STOCK_KLINE",
                5,
                500,
            ).await
        }
        Err(e) => {
            println!("❌ [STOCK_KLINE] 缓存查询失败: {:?}", e);
            json_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                json!({"error": "Cache query failed"}),
            )
        }
    }
}

/// 股票详情处理器
async fn handle_stock_detail(state: &ApiState, code: &str) -> Response<Body> {
    if code.is_empty() {
        return json_response(
            StatusCode::BAD_REQUEST,
            json!({"error": "code is required"}),
        );
    }

    // 从缓存获取股票详情
    let cache_key = stock_detail_key(code);
    let cache_result = state.cache.scope(STOCK_DETAIL_SCOPE).get(&cache_key);

    match cache_result {
        Ok(Some(data)) => {
            println!("✅ [STOCK_DETAIL] 从缓存获取股票详情: {}", code);
            // Parse the JSON data from bytes
            match serde_json::from_slice::<serde_json::Value>(data.as_slice()) {
                Ok(json_data) => json_response(StatusCode::OK, json_data),
                Err(e) => {
                    println!("❌ [STOCK_DETAIL] JSON解析失败: {:?}", e);
                    json_response(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        json!({"error": "Failed to parse cached data"}),
                    )
                }
            }
        }
        Ok(None) => {
            println!("⚠️ [STOCK_DETAIL] 缓存中无股票详情: {}", code);
            
            let event_data = json!({ "code": code, "data_type": "info" }).to_string();
            handle_cache_miss_with_retry(
                state,
                &cache_key,
                STOCK_DETAIL_SCOPE,
                &event_data,
                "STOCK_DETAIL",
                5,  // 最多重试5次
                500, // 每次等待500ms
            ).await
        }
        Err(e) => {
            println!("❌ [STOCK_DETAIL] 缓存查询失败: {:?}", e);
            json_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                json!({"error": "Cache query failed"}),
            )
        }
    }
}

/// 财务数据处理器
async fn handle_stock_financial(
    state: &ApiState,
    code: &str,
    query: &HashMap<String, String>,
) -> Response<Body> {
    if code.is_empty() {
        return json_response(
            StatusCode::BAD_REQUEST,
            json!({"error": "code is required"}),
        );
    }

    let report_type = query
        .get("type")
        .unwrap_or(&"annual".to_string())
        .to_string();

    // 从缓存获取财务数据
    let cache_key = stock_financial_key(code, &report_type);
    let cache_result = state.cache.scope(STOCK_FINANCIAL_SCOPE).get(&cache_key);

    match cache_result {
        Ok(Some(data)) => {
            println!(
                "✅ [STOCK_FINANCIAL] 从缓存获取财务数据: {} ({})",
                code, report_type
            );
            // Parse the JSON data from bytes
            match serde_json::from_slice::<serde_json::Value>(data.as_slice()) {
                Ok(json_data) => json_response(StatusCode::OK, json_data),
                Err(e) => {
                    println!("❌ [STOCK_FINANCIAL] JSON解析失败: {:?}", e);
                    json_response(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        json!({"error": "Failed to parse cached data"}),
                    )
                }
            }
        }
        Ok(None) => {
            println!(
                "⚠️ [STOCK_FINANCIAL] 缓存中无财务数据: {} ({})",
                code, report_type
            );
            
            let event_data = json!({ "code": code, "data_type": "financial", "report_type": report_type }).to_string();
            handle_cache_miss_with_retry(
                state,
                &cache_key,
                STOCK_FINANCIAL_SCOPE,
                &event_data,
                "STOCK_FINANCIAL",
                5,  // 最多重试5次
                500, // 每次等待500ms
            ).await
        }
        Err(e) => {
            println!("❌ [STOCK_FINANCIAL] 缓存查询失败: {:?}", e);
            json_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                json!({"error": "Cache query failed"}),
            )
        }
    }
}

/// 资金流向处理器
async fn handle_fund_flow(
    state: &ApiState,
    code: &str,
    query: &HashMap<String, String>,
) -> Response<Body> {
    if code.is_empty() {
        return json_response(
            StatusCode::BAD_REQUEST,
            json!({"error": "code is required"}),
        );
    }

    let flow_type = query
        .get("type")
        .unwrap_or(&"inflow".to_string())
        .to_string();

    // 从缓存获取资金流向数据
    let cache_key = stock_fund_flow_key(code, &flow_type);
    let cache_result = state.cache.scope(STOCK_FUND_FLOW_SCOPE).get(&cache_key);

    match cache_result {
        Ok(Some(data)) => {
            println!(
                "✅ [FUND_FLOW] 从缓存获取资金流向: {} ({})",
                code, flow_type
            );
            // Parse the JSON data from bytes
            match serde_json::from_slice::<serde_json::Value>(data.as_slice()) {
                Ok(json_data) => json_response(StatusCode::OK, json_data),
                Err(e) => {
                    println!("❌ [FUND_FLOW] JSON解析失败: {:?}", e);
                    json_response(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        json!({"error": "Failed to parse cached data"}),
                    )
                }
            }
        }
        Ok(None) => {
            println!(
                "⚠️ [FUND_FLOW] 缓存中无资金流向数据: {} ({})",
                code, flow_type
            );
            json_response(
                StatusCode::NOT_FOUND,
                json!({"error": "Fund flow data not found"}),
            )
        }
        Err(e) => {
            println!("❌ [FUND_FLOW] 缓存查询失败: {:?}", e);
            json_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                json!({"error": "Cache query failed"}),
            )
        }
    }
}

/// 股东处理器
async fn handle_stock_holders(
    state: &ApiState,
    code: &str,
    query: &HashMap<String, String>,
) -> Response<Body> {
    if code.is_empty() {
        return json_response(
            StatusCode::BAD_REQUEST,
            json!({"error": "code is required"}),
        );
    }

    let holder_type = query
        .get("type")
        .unwrap_or(&"top10".to_string())
        .to_string();

    // 从缓存获取股东数据
    let cache_key = stock_holders_key(code, &holder_type);
    let cache_result = state.cache.scope(STOCK_HOLDERS_SCOPE).get(&cache_key);

    match cache_result {
        Ok(Some(data)) => {
            println!(
                "✅ [STOCK_HOLDERS] 从缓存获取股东数据: {} ({})",
                code, holder_type
            );
            // Parse the JSON data from bytes
            match serde_json::from_slice::<serde_json::Value>(data.as_slice()) {
                Ok(json_data) => json_response(StatusCode::OK, json_data),
                Err(e) => {
                    println!("❌ [STOCK_HOLDERS] JSON解析失败: {:?}", e);
                    json_response(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        json!({"error": "Failed to parse cached data"}),
                    )
                }
            }
        }
        Ok(None) => {
            println!(
                "⚠️ [STOCK_HOLDERS] 缓存中无股东数据: {} ({})",
                code, holder_type
            );
            
            let event_data = json!({ "code": code, "data_type": "holders", "holder_type": holder_type }).to_string();
            handle_cache_miss_with_retry(
                state,
                &cache_key,
                STOCK_HOLDERS_SCOPE,
                &event_data,
                "STOCK_HOLDERS",
                5,
                500,
            ).await
        }
        Err(e) => {
            println!("❌ [STOCK_HOLDERS] 缓存查询失败: {:?}", e);
            json_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                json!({"error": "Cache query failed"}),
            )
        }
    }
}

/// 公告处理器
async fn handle_stock_announcements(
    state: &ApiState,
    code: &str,
    query: &HashMap<String, String>,
) -> Response<Body> {
    if code.is_empty() {
        return json_response(
            StatusCode::BAD_REQUEST,
            json!({"error": "code is required"}),
        );
    }

    let page = query
        .get("page")
        .unwrap_or(&"1".to_string())
        .parse::<usize>()
        .unwrap_or(1);
    let limit = query
        .get("limit")
        .unwrap_or(&"10".to_string())
        .parse::<usize>()
        .unwrap_or(10);

    // 从缓存获取公告数据
    let cache_key = stock_announcements_key(code, page, limit);
    let cache_result = state.cache.scope(STOCK_ANNOUNCEMENTS_SCOPE).get(&cache_key);

    match cache_result {
        Ok(Some(data)) => {
            println!(
                "✅ [STOCK_ANNOUNCEMENTS] 从缓存获取公告数据: {} (page: {}, limit: {})",
                code, page, limit
            );
            // Parse the JSON data from bytes
            match serde_json::from_slice::<serde_json::Value>(data.as_slice()) {
                Ok(json_data) => json_response(StatusCode::OK, json_data),
                Err(e) => {
                    println!("❌ [STOCK_ANNOUNCEMENTS] JSON解析失败: {:?}", e);
                    json_response(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        json!({"error": "Failed to parse cached data"}),
                    )
                }
            }
        }
        Ok(None) => {
            println!(
                "⚠️ [STOCK_ANNOUNCEMENTS] 缓存中无公告数据: {} (page: {}, limit: {})",
                code, page, limit
            );
            
            let event_data = json!({ "code": code, "data_type": "announcements", "page": page, "limit": limit }).to_string();
            handle_cache_miss_with_retry(
                state,
                &cache_key,
                STOCK_ANNOUNCEMENTS_SCOPE,
                &event_data,
                "STOCK_ANNOUNCEMENTS",
                5,
                500,
            ).await
        }
        Err(e) => {
            println!("❌ [STOCK_ANNOUNCEMENTS] 缓存查询失败: {:?}", e);
            json_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                json!({"error": "Cache query failed"}),
            )
        }
    }
}

/// 市场状态处理器
async fn handle_market_status(state: &ApiState, query: &HashMap<String, String>) -> Response<Body> {
    let market = query
        .get("market")
        .unwrap_or(&"all".to_string())
        .to_string();

    // 从缓存获取市场状态
    let cache_key = stock_market_status_key(&market);
    let cache_result = state.cache.scope(STOCK_MARKET_STATUS_SCOPE).get(&cache_key);

    match cache_result {
        Ok(Some(data)) => {
            println!("✅ [MARKET_STATUS] 从缓存获取市场状态: {}", market);
            // Parse the JSON data from bytes
            match serde_json::from_slice::<serde_json::Value>(data.as_slice()) {
                Ok(json_data) => json_response(StatusCode::OK, json_data),
                Err(e) => {
                    println!("❌ [MARKET_STATUS] JSON解析失败: {:?}", e);
                    json_response(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        json!({"error": "Failed to parse cached data"}),
                    )
                }
            }
        }
        Ok(None) => {
            println!("⚠️ [MARKET_STATUS] 缓存中无市场状态: {}", market);
            json_response(
                StatusCode::NOT_FOUND,
                json!({"error": "Market status not found"}),
            )
        }
        Err(e) => {
            println!("❌ [MARKET_STATUS] 缓存查询失败: {:?}", e);
            json_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                json!({"error": "Cache query failed"}),
            )
        }
    }
}

/// 市场指数处理器
async fn handle_market_indices(state: &ApiState) -> Response<Body> {
    // 从缓存获取市场指数
    let cache_key = "market_indices";
    let cache_result = state.cache.scope("stock").get(cache_key);

    match cache_result {
        Ok(Some(data)) => {
            println!("✅ [MARKET_INDICES] 从缓存获取市场指数");
            // Parse the JSON data from bytes
            match serde_json::from_slice::<serde_json::Value>(data.as_slice()) {
                Ok(json_data) => json_response(StatusCode::OK, json_data),
                Err(e) => {
                    println!("❌ [MARKET_INDICES] JSON解析失败: {:?}", e);
                    json_response(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        json!({"error": "Failed to parse cached data"}),
                    )
                }
            }
        }
        Ok(None) => {
            println!("⚠️ [MARKET_INDICES] 缓存中无市场指数");
            json_response(
                StatusCode::NOT_FOUND,
                json!({"error": "Market indices not found"}),
            )
        }
        Err(e) => {
            println!("❌ [MARKET_INDICES] 缓存查询失败: {:?}", e);
            json_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                json!({"error": "Cache query failed"}),
            )
        }
    }
}

/// 龙虎榜处理器
async fn handle_lhb_data(state: &ApiState, query: &HashMap<String, String>) -> Response<Body> {
    let date = query
        .get("date")
        .unwrap_or(&"today".to_string())
        .to_string();
    let lhb_type = query.get("type").unwrap_or(&"all".to_string()).to_string();

    // 从缓存获取龙虎榜数据
    let cache_key = stock_lhb_key(&date, &lhb_type);
    let cache_result = state.cache.scope(STOCK_LHB_SCOPE).get(&cache_key);

    match cache_result {
        Ok(Some(data)) => {
            println!(
                "✅ [LHB_DATA] 从缓存获取龙虎榜数据: {} ({})",
                date, lhb_type
            );
            // Parse the JSON data from bytes
            match serde_json::from_slice::<serde_json::Value>(data.as_slice()) {
                Ok(json_data) => json_response(StatusCode::OK, json_data),
                Err(e) => {
                    println!("❌ [LHB_DATA] JSON解析失败: {:?}", e);
                    json_response(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        json!({"error": "Failed to parse cached data"}),
                    )
                }
            }
        }
        Ok(None) => {
            println!("⚠️ [LHB_DATA] 缓存中无龙虎榜数据: {} ({})", date, lhb_type);
            json_response(
                StatusCode::NOT_FOUND,
                json!({"error": "LHB data not found"}),
            )
        }
        Err(e) => {
            println!("❌ [LHB_DATA] 缓存查询失败: {:?}", e);
            json_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                json!({"error": "Cache query failed"}),
            )
        }
    }
}

/// 板块处理器
async fn handle_sectors(state: &ApiState, query: &HashMap<String, String>) -> Response<Body> {
    let sector_type = query.get("type").unwrap_or(&"all".to_string()).to_string();

    // 从缓存获取板块数据
    let market = "all"; // 默认市场
    let cache_key = stock_sectors_key(&sector_type, market);
    let cache_result = state.cache.scope(STOCK_SECTORS_SCOPE).get(&cache_key);

    match cache_result {
        Ok(Some(data)) => {
            println!("✅ [SECTORS] 从缓存获取板块数据: {}", sector_type);
            // Parse the JSON data from bytes
            match serde_json::from_slice::<serde_json::Value>(data.as_slice()) {
                Ok(json_data) => json_response(StatusCode::OK, json_data),
                Err(e) => {
                    println!("❌ [SECTORS] JSON解析失败: {:?}", e);
                    json_response(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        json!({"error": "Failed to parse cached data"}),
                    )
                }
            }
        }
        Ok(None) => {
            println!("⚠️ [SECTORS] 缓存中无板块数据: {}", sector_type);
            json_response(
                StatusCode::NOT_FOUND,
                json!({"error": "Sectors data not found"}),
            )
        }
        Err(e) => {
            println!("❌ [SECTORS] 缓存查询失败: {:?}", e);
            json_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                json!({"error": "Cache query failed"}),
            )
        }
    }
}

/// 排行榜处理器
async fn handle_stock_ranking(state: &ApiState, query: &HashMap<String, String>) -> Response<Body> {
    let ranking_type = query
        .get("type")
        .unwrap_or(&"change_pct".to_string())
        .to_string();
    let market = query
        .get("market")
        .unwrap_or(&"all".to_string())
        .to_string();

    // 从缓存获取排行榜数据
    let cache_key = stock_ranking_key(&ranking_type, &market);
    let cache_result = state.cache.scope(STOCK_RANKING_SCOPE).get(&cache_key);

    match cache_result {
        Ok(Some(data)) => {
            println!(
                "✅ [STOCK_RANKING] 从缓存获取排行榜数据: {} ({})",
                ranking_type, market
            );
            // Parse the JSON data from bytes
            match serde_json::from_slice::<serde_json::Value>(data.as_slice()) {
                Ok(json_data) => json_response(StatusCode::OK, json_data),
                Err(e) => {
                    println!("❌ [STOCK_RANKING] JSON解析失败: {:?}", e);
                    json_response(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        json!({"error": "Failed to parse cached data"}),
                    )
                }
            }
        }
        Ok(None) => {
            println!(
                "⚠️ [STOCK_RANKING] 缓存中无排行榜数据: {} ({})",
                ranking_type, market
            );
            json_response(
                StatusCode::NOT_FOUND,
                json!({"error": "Stock ranking data not found"}),
            )
        }
        Err(e) => {
            println!("❌ [STOCK_RANKING] 缓存查询失败: {:?}", e);
            json_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                json!({"error": "Cache query failed"}),
            )
        }
    }
}

/// 板块股票处理器
async fn handle_sector_stocks(state: &ApiState, code: &str) -> Response<Body> {
    if code.is_empty() {
        return json_response(
            StatusCode::BAD_REQUEST,
            json!({"error": "code is required"}),
        );
    }

    // 从缓存获取板块股票数据
    let cache_key = format!("sector_stocks:{}", code);
    let cache_result = state.cache.scope("stock").get(&cache_key);

    match cache_result {
        Ok(Some(data)) => {
            println!("✅ [SECTOR_STOCKS] 从缓存获取板块股票数据: {}", code);
            // Parse the JSON data from bytes
            match serde_json::from_slice::<serde_json::Value>(data.as_slice()) {
                Ok(json_data) => json_response(StatusCode::OK, json_data),
                Err(e) => {
                    println!("❌ [SECTOR_STOCKS] JSON解析失败: {:?}", e);
                    json_response(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        json!({"error": "Failed to parse cached data"}),
                    )
                }
            }
        }
        Ok(None) => {
            println!("⚠️ [SECTOR_STOCKS] 缓存中无板块股票数据: {}", code);
            json_response(
                StatusCode::NOT_FOUND,
                json!({"error": "Sector stocks data not found"}),
            )
        }
        Err(e) => {
            println!("❌ [SECTOR_STOCKS] 缓存查询失败: {:?}", e);
            json_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                json!({"error": "Cache query failed"}),
            )
        }
    }
}

/// 处理所有股票 API 请求
pub async fn handle_stock_api(
    State(state): State<ApiState>,
    method: Method,
    Path(path): Path<String>,
    Query(query): Query<HashMap<String, String>>,
) -> Response<Body> {
    println!(
        "📊 [STOCK_API] 收到股票 API 请求: {} {}",
        method.as_str(),
        path
    );

    let full_path = format!("/stock/{}", path);
    let path_segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
    println!("🔍 [STOCK_API] 路径段: {:?}", path_segments);

    println!("🔄 [STOCK_API] 使用内置路由分发");

    // 路由分发
    let result = match (method.as_str(), path_segments.as_slice()) {
        ("GET", ["search"]) => {
            println!("🔍 [STOCK_API] 调用股票搜索处理器");
            handle_stock_search(&state, &query).await
        }
        ("GET", ["quote"]) => {
            println!("📈 [STOCK_API] 调用股票行情处理器");
            handle_stock_quote(&state, &query).await
        }
        ("GET", ["quote", "stream"]) => {
            println!("📊 [STOCK_API] 调用股票流处理器");
            handle_quote_stream(&state, &query).await
        }
        ("GET", ["kline"]) => {
            println!("📈 [STOCK_API] 调用K线数据处理器");
            handle_stock_kline(&state, &query).await
        }
        ("GET", ["detail", code]) => {
            println!("📋 [STOCK_API] 调用股票详情处理器");
            handle_stock_detail(&state, code).await
        }
        ("GET", ["financial", code]) => {
            println!("💰 [STOCK_API] 调用财务数据处理器, code: {}", code);
            handle_stock_financial(&state, code, &query).await
        }
        ("GET", ["fund_flow", code]) => {
            println!("💸 [STOCK_API] 调用资金流向处理器, code: {}", code);
            handle_fund_flow(&state, code, &query).await
        }
        ("GET", ["holders", code]) => {
            println!("👥 [STOCK_API] 调用股东处理器, code: {}", code);
            handle_stock_holders(&state, code, &query).await
        }
        ("GET", ["announcements", code]) => {
            println!("📢 [STOCK_API] 调用公告处理器, code: {}", code);
            handle_stock_announcements(&state, code, &query).await
        }
        ("GET", ["market", "status"]) => {
            println!("💾 [STOCK_API] 调用市场状态处理器");
            handle_market_status(&state, &query).await
        }
        ("GET", ["market", "indices"]) => {
            println!("📊 [STOCK_API] 调用市场指数处理器");
            handle_market_indices(&state).await
        }
        ("GET", ["market", "lhb"]) => {
            println!("🐉 [STOCK_API] 调用龙虎榜处理器");
            handle_lhb_data(&state, &query).await
        }
        ("GET", ["sectors"]) => {
            println!("🏢 [STOCK_API] 调用板块处理器");
            handle_sectors(&state, &query).await
        }
        ("GET", ["ranking"]) => {
            println!("🏆 [STOCK_API] 调用排行榜处理器");
            handle_stock_ranking(&state, &query).await
        }
        ("GET", ["sectors", code, "stocks"]) => {
            println!("🏗️ [STOCK_API] 调用板块股票处理器, code: {}", code);
            handle_sector_stocks(&state, code).await
        }
        _ => {
            println!(
                "❌ [STOCK_API] 未找到匹配的路由: {} {}",
                method.as_str(),
                full_path
            );
            json_response(
                StatusCode::NOT_FOUND,
                json!({
                    "error": "Stock API endpoint not found",
                    "path": full_path,
                    "method": method.as_str()
                }),
            )
        }
    };
    println!("📤 [STOCK_API] 返回响应结果");
    result
}
