// Copyright (C) 2025 nostalgiatan
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

//! 带缓存的股票客户端
//!
//! 封装 StockClient，添加缓存层

use crate::client::StockClient;
use crate::error::{StockError, StockResult};
use crate::types::CacheScope;

use chrono::Local;
use once_cell::sync::Lazy;
use parking_lot::RwLock;
use seesea_cache::CacheInterface;
use seesea_cache::cache::types::CacheImplConfig;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, info, warn};

/// 全局缓存客户端实例
static CACHED_CLIENT: Lazy<RwLock<Option<Arc<CachedStockClient>>>> =
    Lazy::new(|| RwLock::new(None));

/// 获取全局缓存股票客户端
pub fn get_cached_stock_client() -> StockResult<Arc<CachedStockClient>> {
    {
        let guard = CACHED_CLIENT.read();
        if let Some(client) = guard.as_ref() {
            return Ok(client.clone());
        }
    }

    let mut guard = CACHED_CLIENT.write();
    if guard.is_none() {
        let client = Arc::new(CachedStockClient::new()?);
        *guard = Some(client.clone());
        Ok(client)
    } else {
        Ok(guard.as_ref().unwrap().clone())
    }
}

/// 带缓存的股票客户端
pub struct CachedStockClient {
    /// 原始客户端
    client: StockClient,
    /// 缓存接口
    pub cache: CacheInterface,
}

impl CachedStockClient {
    /// 创建新的缓存股票客户端
    pub fn new() -> StockResult<Self> {
        let client = StockClient::new()?;
        let config = CacheImplConfig::new(seesea_config::paths::get_cache_dir());
        let cache = CacheInterface::new(config)
            .map_err(|e| StockError::CacheError(format!("初始化缓存失败: {}", e)))?;

        info!("✅ CachedStockClient 已初始化");
        Ok(Self { client, cache })
    }

    /// 构建缓存键
    fn build_cache_key(interface: &str, params: &HashMap<String, String>) -> String {
        if params.is_empty() {
            interface.to_string()
        } else {
            let mut sorted_params: Vec<_> = params.iter().collect();
            sorted_params.sort_by_key(|(k, _)| *k);
            let params_str: Vec<String> = sorted_params
                .iter()
                .map(|(k, v)| format!("{}_{}", k, v))
                .collect();
            format!("{}__{}", interface, params_str.join("__"))
        }
    }

    /// 只读缓存调用方法（不调用原始接口）
    pub fn cached_call_readonly(
        &self,
        interface: &str,
        scope: CacheScope,
        params: HashMap<String, String>,
    ) -> StockResult<Value> {
        let cache_key = Self::build_cache_key(interface, &params);
        let scope_str = scope.as_str();

        // 只尝试从缓存获取
        match self.cache.scope(scope_str).get(&cache_key) {
            Ok(Some(data)) => {
                debug!("✅ 缓存命中: {}", cache_key);
                let json: Value = serde_json::from_slice(&data)?;
                Ok(json)
            }
            Ok(None) => {
                debug!("⚠️ 缓存未命中: {}", cache_key);
                Err(StockError::CacheError(format!(
                    "缓存中暂无数据: {}",
                    interface
                )))
            }
            Err(e) => {
                warn!("❌ 缓存读取失败: {} - {}", cache_key, e);
                Err(StockError::CacheError(format!("缓存读取失败: {}", e)))
            }
        }
    }

    /// 通用缓存调用方法
    pub fn cached_call(
        &self,
        interface: &str,
        scope: CacheScope,
        params: HashMap<String, String>,
    ) -> StockResult<Value> {
        let cache_key = Self::build_cache_key(interface, &params);
        let scope_str = scope.as_str();

        // 尝试从缓存获取
        match self.cache.scope(scope_str).get(&cache_key) {
            Ok(Some(data)) => {
                debug!("✅ 缓存命中: {}", cache_key);
                let json: Value = serde_json::from_slice(&data)?;
                return Ok(json);
            }
            Ok(None) => {
                debug!("⚠️ 缓存未命中: {}", cache_key);
            }
            Err(e) => {
                warn!("❌ 缓存读取失败: {} - {}", cache_key, e);
            }
        }

        // 调用原始接口
        let data = self.client.call(interface, params)?;

        // 写入缓存
        let json_bytes = serde_json::to_vec(&data)?;
        let ttl = Some(Duration::from_secs(scope.ttl_seconds()));
        if let Err(e) = self
            .cache
            .scope(scope_str)
            .set(cache_key.clone(), json_bytes, ttl)
        {
            warn!("❌ 缓存写入失败: {} - {}", cache_key, e);
        } else {
            debug!("✅ 缓存写入: {}", cache_key);
        }

        Ok(data)
    }

    /// 强制刷新缓存
    pub fn refresh(
        &self,
        interface: &str,
        scope: CacheScope,
        params: HashMap<String, String>,
    ) -> StockResult<Value> {
        let cache_key = Self::build_cache_key(interface, &params);
        let scope_str = scope.as_str();

        // 调用原始接口
        let data = self.client.call(interface, params)?;

        // 写入缓存
        let json_bytes = serde_json::to_vec(&data)?;
        let ttl = Some(Duration::from_secs(scope.ttl_seconds()));
        if let Err(e) = self
            .cache
            .scope(scope_str)
            .set(cache_key.clone(), json_bytes, ttl)
        {
            warn!("❌ 缓存写入失败: {} - {}", cache_key, e);
        } else {
            info!("🔄 缓存已刷新: {}", cache_key);
        }

        Ok(data)
    }

    // ==================== 便捷方法 ====================

    /// 获取 A 股代码名称（缓存）
    pub fn stock_info_a_code_name(&self) -> StockResult<Value> {
        self.cached_call(
            "stock_info_a_code_name",
            CacheScope::StockInfo,
            HashMap::new(),
        )
    }

    /// 获取 A 股代码名称（只读缓存）
    pub fn stock_info_a_code_name_readonly(&self) -> StockResult<Value> {
        self.cached_call_readonly(
            "stock_info_a_code_name",
            CacheScope::StockInfo,
            HashMap::new(),
        )
    }

    /// 刷新 A 股代码名称缓存
    pub fn refresh_stock_info_a_code_name(&self) -> StockResult<Value> {
        self.refresh(
            "stock_info_a_code_name",
            CacheScope::StockInfo,
            HashMap::new(),
        )
    }

    /// 获取上交所股票信息（缓存）
    pub fn stock_info_sh_name_code(&self, symbol: Option<&str>) -> StockResult<Value> {
        let mut params = HashMap::new();
        if let Some(s) = symbol {
            params.insert("symbol".to_string(), s.to_string());
        }
        self.cached_call("stock_info_sh_name_code", CacheScope::StockInfo, params)
    }

    /// 获取上交所股票信息（只读缓存）
    pub fn stock_info_sh_name_code_readonly(&self, symbol: Option<&str>) -> StockResult<Value> {
        let mut params = HashMap::new();
        if let Some(s) = symbol {
            params.insert("symbol".to_string(), s.to_string());
        }
        self.cached_call_readonly("stock_info_sh_name_code", CacheScope::StockInfo, params)
    }

    /// 刷新上交所股票信息缓存
    pub fn refresh_stock_info_sh_name_code(&self, symbol: Option<&str>) -> StockResult<Value> {
        let mut params = HashMap::new();
        if let Some(s) = symbol {
            params.insert("symbol".to_string(), s.to_string());
        }
        self.refresh("stock_info_sh_name_code", CacheScope::StockInfo, params)
    }

    /// 获取深交所股票信息（缓存）
    pub fn stock_info_sz_name_code(&self, symbol: Option<&str>) -> StockResult<Value> {
        let mut params = HashMap::new();
        if let Some(s) = symbol {
            params.insert("symbol".to_string(), s.to_string());
        }
        self.cached_call("stock_info_sz_name_code", CacheScope::StockInfo, params)
    }

    /// 获取深交所股票信息（只读缓存）
    pub fn stock_info_sz_name_code_readonly(&self, symbol: Option<&str>) -> StockResult<Value> {
        let mut params = HashMap::new();
        if let Some(s) = symbol {
            params.insert("symbol".to_string(), s.to_string());
        }
        self.cached_call_readonly("stock_info_sz_name_code", CacheScope::StockInfo, params)
    }

    /// 刷新深交所股票信息缓存
    pub fn refresh_stock_info_sz_name_code(&self, symbol: Option<&str>) -> StockResult<Value> {
        let mut params = HashMap::new();
        if let Some(s) = symbol {
            params.insert("symbol".to_string(), s.to_string());
        }
        self.refresh("stock_info_sz_name_code", CacheScope::StockInfo, params)
    }

    /// 获取个股信息（缓存）
    pub fn stock_individual_info_em(&self, symbol: &str) -> StockResult<Value> {
        let mut params = HashMap::new();
        params.insert("symbol".to_string(), symbol.to_string());
        self.cached_call("stock_individual_info_em", CacheScope::StockInfo, params)
    }

    /// 获取个股信息（只读缓存）
    pub fn stock_individual_info_em_readonly(&self, symbol: &str) -> StockResult<Value> {
        let mut params = HashMap::new();
        params.insert("symbol".to_string(), symbol.to_string());
        self.cached_call_readonly("stock_individual_info_em", CacheScope::StockInfo, params)
    }

    /// 获取 A 股实时行情（缓存）
    pub fn stock_zh_a_spot_em(&self) -> StockResult<Value> {
        self.cached_call("stock_zh_a_spot_em", CacheScope::StockQuote, HashMap::new())
    }

    /// 获取 A 股实时行情（只读缓存）
    pub fn stock_zh_a_spot_em_readonly(&self) -> StockResult<Value> {
        self.cached_call_readonly("stock_zh_a_spot_em", CacheScope::StockQuote, HashMap::new())
    }

    /// 刷新 A 股实时行情缓存
    pub fn refresh_stock_zh_a_spot_em(&self) -> StockResult<Value> {
        self.refresh("stock_zh_a_spot_em", CacheScope::StockQuote, HashMap::new())
    }

    /// 获取 B 股实时行情（缓存）
    pub fn stock_zh_b_spot_em(&self) -> StockResult<Value> {
        self.cached_call("stock_zh_b_spot_em", CacheScope::StockQuote, HashMap::new())
    }

    /// 刷新 B 股实时行情缓存
    pub fn refresh_stock_zh_b_spot_em(&self) -> StockResult<Value> {
        self.refresh("stock_zh_b_spot_em", CacheScope::StockQuote, HashMap::new())
    }

    /// 获取美股实时行情（缓存）
    pub fn stock_us_spot_em(&self) -> StockResult<Value> {
        self.cached_call("stock_us_spot_em", CacheScope::StockQuote, HashMap::new())
    }

    /// 刷新美股实时行情缓存
    pub fn refresh_stock_us_spot_em(&self) -> StockResult<Value> {
        self.refresh("stock_us_spot_em", CacheScope::StockQuote, HashMap::new())
    }

    /// 获取港股实时行情（缓存）
    pub fn stock_hk_spot_em(&self) -> StockResult<Value> {
        self.cached_call("stock_hk_spot_em", CacheScope::StockQuote, HashMap::new())
    }

    /// 刷新港股实时行情缓存
    pub fn refresh_stock_hk_spot_em(&self) -> StockResult<Value> {
        self.refresh("stock_hk_spot_em", CacheScope::StockQuote, HashMap::new())
    }

    /// 获取行业板块列表（缓存）
    pub fn stock_board_industry_name_em(&self) -> StockResult<Value> {
        self.cached_call(
            "stock_board_industry_name_em",
            CacheScope::StockIndustry,
            HashMap::new(),
        )
    }

    /// 获取行业板块列表（只读缓存）
    pub fn stock_board_industry_name_em_readonly(&self) -> StockResult<Value> {
        self.cached_call_readonly(
            "stock_board_industry_name_em",
            CacheScope::StockIndustry,
            HashMap::new(),
        )
    }

    /// 刷新行业板块列表缓存
    pub fn refresh_stock_board_industry_name_em(&self) -> StockResult<Value> {
        self.refresh(
            "stock_board_industry_name_em",
            CacheScope::StockIndustry,
            HashMap::new(),
        )
    }

    /// 获取概念板块列表（缓存）
    pub fn stock_board_concept_name_em(&self) -> StockResult<Value> {
        self.cached_call(
            "stock_board_concept_name_em",
            CacheScope::StockIndustry,
            HashMap::new(),
        )
    }

    /// 获取概念板块列表（只读缓存）
    pub fn stock_board_concept_name_em_readonly(&self) -> StockResult<Value> {
        self.cached_call_readonly(
            "stock_board_concept_name_em",
            CacheScope::StockIndustry,
            HashMap::new(),
        )
    }

    /// 刷新概念板块列表缓存
    pub fn refresh_stock_board_concept_name_em(&self) -> StockResult<Value> {
        self.refresh(
            "stock_board_concept_name_em",
            CacheScope::StockIndustry,
            HashMap::new(),
        )
    }

    /// 获取指数实时行情（缓存）
    pub fn stock_zh_index_spot_em(&self) -> StockResult<Value> {
        self.cached_call(
            "stock_zh_index_spot_em",
            CacheScope::StockIndex,
            HashMap::new(),
        )
    }

    /// 获取指数实时行情（只读缓存）
    pub fn stock_zh_index_spot_em_readonly(&self) -> StockResult<Value> {
        self.cached_call_readonly(
            "stock_zh_index_spot_em",
            CacheScope::StockIndex,
            HashMap::new(),
        )
    }

    /// 刷新指数实时行情缓存
    pub fn refresh_stock_zh_index_spot_em(&self) -> StockResult<Value> {
        self.refresh(
            "stock_zh_index_spot_em",
            CacheScope::StockIndex,
            HashMap::new(),
        )
    }

    /// 获取市场资金流向（缓存）
    pub fn stock_market_fund_flow(&self) -> StockResult<Value> {
        self.cached_call(
            "stock_market_fund_flow",
            CacheScope::StockFundFlow,
            HashMap::new(),
        )
    }

    /// 获取市场资金流向（只读缓存）
    pub fn stock_market_fund_flow_readonly(&self) -> StockResult<Value> {
        self.cached_call_readonly(
            "stock_market_fund_flow",
            CacheScope::StockFundFlow,
            HashMap::new(),
        )
    }

    /// 刷新市场资金流向缓存
    pub fn refresh_stock_market_fund_flow(&self) -> StockResult<Value> {
        self.refresh(
            "stock_market_fund_flow",
            CacheScope::StockFundFlow,
            HashMap::new(),
        )
    }

    /// 获取涨停股池（缓存）
    pub fn stock_zt_pool_em(&self, date: Option<&str>) -> StockResult<Value> {
        let mut params = HashMap::new();
        let date_str = date
            .map(|d| d.to_string())
            .unwrap_or_else(|| Local::now().format("%Y%m%d").to_string());
        params.insert("date".to_string(), date_str);
        self.cached_call("stock_zt_pool_em", CacheScope::StockRanking, params)
    }

    /// 获取涨停股池（只读缓存）
    pub fn stock_zt_pool_em_readonly(&self, date: Option<&str>) -> StockResult<Value> {
        let mut params = HashMap::new();
        let date_str = date
            .map(|d| d.to_string())
            .unwrap_or_else(|| Local::now().format("%Y%m%d").to_string());
        params.insert("date".to_string(), date_str);
        self.cached_call_readonly("stock_zt_pool_em", CacheScope::StockRanking, params)
    }

    /// 刷新涨停股池缓存
    pub fn refresh_stock_zt_pool_em(&self, date: Option<&str>) -> StockResult<Value> {
        let mut params = HashMap::new();
        let date_str = date
            .map(|d| d.to_string())
            .unwrap_or_else(|| Local::now().format("%Y%m%d").to_string());
        params.insert("date".to_string(), date_str);
        self.refresh("stock_zt_pool_em", CacheScope::StockRanking, params)
    }

    /// 获取跌停股池（缓存）
    pub fn stock_dt_pool_em(&self, date: Option<&str>) -> StockResult<Value> {
        let mut params = HashMap::new();
        let date_str = date
            .map(|d| d.to_string())
            .unwrap_or_else(|| Local::now().format("%Y%m%d").to_string());
        params.insert("date".to_string(), date_str);
        self.cached_call("stock_dt_pool_em", CacheScope::StockRanking, params)
    }

    /// 获取跌停股池（只读缓存）
    pub fn stock_dt_pool_em_readonly(&self, date: Option<&str>) -> StockResult<Value> {
        let mut params = HashMap::new();
        let date_str = date
            .map(|d| d.to_string())
            .unwrap_or_else(|| Local::now().format("%Y%m%d").to_string());
        params.insert("date".to_string(), date_str);
        self.cached_call_readonly("stock_dt_pool_em", CacheScope::StockRanking, params)
    }

    /// 刷新跌停股池缓存
    pub fn refresh_stock_dt_pool_em(&self, date: Option<&str>) -> StockResult<Value> {
        let mut params = HashMap::new();
        let date_str = date
            .map(|d| d.to_string())
            .unwrap_or_else(|| Local::now().format("%Y%m%d").to_string());
        params.insert("date".to_string(), date_str);
        self.refresh("stock_zt_pool_dtgc_em", CacheScope::StockRanking, params)
    }

    /// 获取龙虎榜详情（缓存）
    pub fn stock_lhb_detail_daily_sina(&self, date: &str) -> StockResult<Value> {
        let mut params = HashMap::new();
        params.insert("date".to_string(), date.to_string());
        self.cached_call("stock_lhb_detail_daily_sina", CacheScope::StockLhb, params)
    }

    /// 获取龙虎榜详情（只读缓存）
    pub fn stock_lhb_detail_daily_sina_readonly(&self, date: &str) -> StockResult<Value> {
        let mut params = HashMap::new();
        params.insert("date".to_string(), date.to_string());
        self.cached_call_readonly("stock_lhb_detail_daily_sina", CacheScope::StockLhb, params)
    }

    /// 获取 A 股历史行情（缓存）
    pub fn stock_zh_a_hist(
        &self,
        symbol: &str,
        period: Option<&str>,
        start_date: Option<&str>,
        end_date: Option<&str>,
        adjust: Option<&str>,
    ) -> StockResult<Value> {
        let mut params = HashMap::new();
        params.insert("symbol".to_string(), symbol.to_string());
        if let Some(p) = period {
            params.insert("period".to_string(), p.to_string());
        }
        if let Some(s) = start_date {
            params.insert("start_date".to_string(), s.to_string());
        }
        if let Some(e) = end_date {
            params.insert("end_date".to_string(), e.to_string());
        }
        if let Some(a) = adjust {
            params.insert("adjust".to_string(), a.to_string());
        }
        self.cached_call("stock_zh_a_hist", CacheScope::StockKline, params)
    }

    /// 获取 A 股历史行情（只读缓存）
    pub fn stock_zh_a_hist_readonly(
        &self,
        symbol: &str,
        period: Option<&str>,
        start_date: Option<&str>,
        end_date: Option<&str>,
        adjust: Option<&str>,
    ) -> StockResult<Value> {
        let mut params = HashMap::new();
        params.insert("symbol".to_string(), symbol.to_string());
        if let Some(p) = period {
            params.insert("period".to_string(), p.to_string());
        }
        if let Some(s) = start_date {
            params.insert("start_date".to_string(), s.to_string());
        }
        if let Some(e) = end_date {
            params.insert("end_date".to_string(), e.to_string());
        }
        if let Some(a) = adjust {
            params.insert("adjust".to_string(), a.to_string());
        }
        self.cached_call_readonly("stock_zh_a_hist", CacheScope::StockKline, params)
    }

    /// 获取 A 股分钟历史行情（缓存）
    pub fn stock_zh_a_hist_min_em(
        &self,
        symbol: &str,
        period: Option<&str>,
        start_date: Option<&str>,
        end_date: Option<&str>,
        adjust: Option<&str>,
    ) -> StockResult<Value> {
        let mut params = HashMap::new();
        params.insert("symbol".to_string(), symbol.to_string());
        if let Some(p) = period {
            params.insert("period".to_string(), p.to_string());
        }
        if let Some(s) = start_date {
            params.insert("start_date".to_string(), s.to_string());
        }
        if let Some(e) = end_date {
            params.insert("end_date".to_string(), e.to_string());
        }
        if let Some(a) = adjust {
            params.insert("adjust".to_string(), a.to_string());
        }
        self.cached_call("stock_zh_a_hist_min_em", CacheScope::StockKline, params)
    }

    /// 获取 A 股分钟历史行情（只读缓存）
    pub fn stock_zh_a_hist_min_em_readonly(
        &self,
        symbol: &str,
        period: Option<&str>,
        start_date: Option<&str>,
        end_date: Option<&str>,
        adjust: Option<&str>,
    ) -> StockResult<Value> {
        let mut params = HashMap::new();
        params.insert("symbol".to_string(), symbol.to_string());
        if let Some(p) = period {
            params.insert("period".to_string(), p.to_string());
        }
        if let Some(s) = start_date {
            params.insert("start_date".to_string(), s.to_string());
        }
        if let Some(e) = end_date {
            params.insert("end_date".to_string(), e.to_string());
        }
        if let Some(a) = adjust {
            params.insert("adjust".to_string(), a.to_string());
        }
        self.cached_call_readonly("stock_zh_a_hist_min_em", CacheScope::StockKline, params)
    }

    /// 获取港股历史行情（缓存）
    pub fn stock_hk_hist(
        &self,
        symbol: &str,
        period: Option<&str>,
        start_date: Option<&str>,
        end_date: Option<&str>,
        adjust: Option<&str>,
    ) -> StockResult<Value> {
        let mut params = HashMap::new();
        params.insert("symbol".to_string(), symbol.to_string());
        if let Some(p) = period {
            params.insert("period".to_string(), p.to_string());
        }
        if let Some(s) = start_date {
            params.insert("start_date".to_string(), s.to_string());
        }
        if let Some(e) = end_date {
            params.insert("end_date".to_string(), e.to_string());
        }
        if let Some(a) = adjust {
            params.insert("adjust".to_string(), a.to_string());
        }
        self.cached_call("stock_hk_hist", CacheScope::StockKline, params)
    }

    /// 获取行业板块成分股（缓存）
    pub fn stock_board_industry_cons_em(&self, symbol: &str) -> StockResult<Value> {
        let mut params = HashMap::new();
        params.insert("symbol".to_string(), symbol.to_string());
        self.cached_call(
            "stock_board_industry_cons_em",
            CacheScope::StockIndustry,
            params,
        )
    }

    /// 获取概念板块成分股（缓存）
    pub fn stock_board_concept_cons_em(&self, symbol: &str) -> StockResult<Value> {
        let mut params = HashMap::new();
        params.insert("symbol".to_string(), symbol.to_string());
        self.cached_call(
            "stock_board_concept_cons_em",
            CacheScope::StockIndustry,
            params,
        )
    }

    /// 获取指数历史行情（缓存）
    pub fn stock_zh_index_daily_em(
        &self,
        symbol: &str,
        start_date: Option<&str>,
        end_date: Option<&str>,
    ) -> StockResult<Value> {
        let mut params = HashMap::new();
        params.insert("symbol".to_string(), symbol.to_string());
        if let Some(s) = start_date {
            params.insert("start_date".to_string(), s.to_string());
        }
        if let Some(e) = end_date {
            params.insert("end_date".to_string(), e.to_string());
        }
        self.cached_call("stock_zh_index_daily_em", CacheScope::StockIndex, params)
    }

    /// 获取个股资金流向（缓存）
    pub fn stock_individual_fund_flow(&self, stock: &str, market: &str) -> StockResult<Value> {
        let mut params = HashMap::new();
        params.insert("stock".to_string(), stock.to_string());
        params.insert("market".to_string(), market.to_string());
        self.cached_call(
            "stock_individual_fund_flow",
            CacheScope::StockFundFlow,
            params,
        )
    }

    /// 获取股票新闻（缓存）
    pub fn stock_news_em(&self, symbol: &str) -> StockResult<Value> {
        let mut params = HashMap::new();
        params.insert("symbol".to_string(), symbol.to_string());
        self.cached_call("stock_news_em", CacheScope::StockNews, params)
    }

    /// 获取行业板块成分股（只读缓存）
    pub fn stock_board_industry_cons_em_readonly(&self, symbol: &str) -> StockResult<Value> {
        let mut params = HashMap::new();
        params.insert("symbol".to_string(), symbol.to_string());
        self.cached_call_readonly(
            "stock_board_industry_cons_em",
            CacheScope::StockIndustry,
            params,
        )
    }

    /// 获取概念板块成分股（只读缓存）
    pub fn stock_board_concept_cons_em_readonly(&self, symbol: &str) -> StockResult<Value> {
        let mut params = HashMap::new();
        params.insert("symbol".to_string(), symbol.to_string());
        self.cached_call_readonly(
            "stock_board_concept_cons_em",
            CacheScope::StockIndustry,
            params,
        )
    }

    /// 获取指数历史行情（只读缓存）
    pub fn stock_zh_index_daily_em_readonly(
        &self,
        symbol: &str,
        start_date: Option<&str>,
        end_date: Option<&str>,
    ) -> StockResult<Value> {
        let mut params = HashMap::new();
        params.insert("symbol".to_string(), symbol.to_string());
        if let Some(s) = start_date {
            params.insert("start_date".to_string(), s.to_string());
        }
        if let Some(e) = end_date {
            params.insert("end_date".to_string(), e.to_string());
        }
        self.cached_call_readonly("stock_zh_index_daily_em", CacheScope::StockIndex, params)
    }

    /// 获取个股资金流向（只读缓存）
    pub fn stock_individual_fund_flow_readonly(
        &self,
        stock: &str,
        market: &str,
    ) -> StockResult<Value> {
        let mut params = HashMap::new();
        params.insert("stock".to_string(), stock.to_string());
        params.insert("market".to_string(), market.to_string());
        self.cached_call_readonly(
            "stock_individual_fund_flow",
            CacheScope::StockFundFlow,
            params,
        )
    }

    /// 获取股票新闻（只读缓存）
    pub fn stock_news_em_readonly(&self, symbol: &str) -> StockResult<Value> {
        let mut params = HashMap::new();
        params.insert("symbol".to_string(), symbol.to_string());
        self.cached_call_readonly("stock_news_em", CacheScope::StockNews, params)
    }
}
