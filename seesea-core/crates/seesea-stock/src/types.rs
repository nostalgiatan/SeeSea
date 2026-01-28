// Copyright (C) 2025 nostalgiatan
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

//! 数据类型定义

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// 缓存作用域 - 与 Python 端保持一致
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CacheScope {
    /// 股票基础信息 (TTL: 12小时)
    StockInfo,
    /// 实时行情 (TTL: 6分钟)
    StockQuote,
    /// K线数据 (TTL: 1小时)
    StockKline,
    /// 指数数据 (TTL: 6分钟)
    StockIndex,
    /// 板块数据 (TTL: 20分钟)
    StockIndustry,
    /// 资金流向 (TTL: 20分钟)
    StockFundFlow,
    /// 排行榜 (TTL: 20分钟)
    StockRanking,
    /// 龙虎榜 (TTL: 20分钟)
    StockLhb,
    /// 新闻资讯 (TTL: 30分钟)
    StockNews,
}

impl CacheScope {
    /// 转换为缓存作用域字符串
    pub fn as_str(&self) -> &'static str {
        match self {
            CacheScope::StockInfo => "stock.info",
            CacheScope::StockQuote => "stock.quote",
            CacheScope::StockKline => "stock.kline",
            CacheScope::StockIndex => "stock.index",
            CacheScope::StockIndustry => "stock.industry",
            CacheScope::StockFundFlow => "stock.fund_flow",
            CacheScope::StockRanking => "stock.ranking",
            CacheScope::StockLhb => "stock.lhb",
            CacheScope::StockNews => "stock.news",
        }
    }

    /// 获取 TTL（秒）
    pub fn ttl_seconds(&self) -> u64 {
        match self {
            CacheScope::StockInfo => 3600,     // 1小时
            CacheScope::StockQuote => 3600,    // 1小时
            CacheScope::StockKline => 3600,    // 1小时
            CacheScope::StockIndex => 3600,    // 1小时
            CacheScope::StockIndustry => 3600, // 1小时
            CacheScope::StockFundFlow => 3600, // 1小时
            CacheScope::StockRanking => 3600,  // 1小时
            CacheScope::StockLhb => 3600,      // 1小时
            CacheScope::StockNews => 3600,     // 1小时
        }
    }
}

/// 轮转周期
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u64)]
pub enum RotationInterval {
    /// 长周期 (11.5小时) - 股票代码名称等
    LongTerm = 41400,
    /// 短周期 (15分钟) - 板块、资金流向等
    ShortTerm = 900,
    /// 实时 (5分钟) - 实时行情等
    Realtime = 300,
    /// 自定义间隔
    Custom(u64),
}

impl RotationInterval {
    /// 获取轮转间隔的持续时间
    pub fn as_duration(&self) -> Duration {
        match self {
            RotationInterval::LongTerm => Duration::from_secs(41400),
            RotationInterval::ShortTerm => Duration::from_secs(900),
            RotationInterval::Realtime => Duration::from_secs(300),
            RotationInterval::Custom(secs) => Duration::from_secs(*secs),
        }
    }
}

/// 股票数据接口定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StockInterface {
    /// 接口名称 (如 "stock_zh_a_spot_em")
    pub name: &'static str,
    /// 缓存作用域
    pub scope: CacheScope,
    /// 轮转周期
    pub rotation: RotationInterval,
    /// 是否需要参数
    pub has_params: bool,
}

/// 预定义的接口列表
pub mod interfaces {
    use super::*;

    // ==================== 基础信息接口 ====================

    pub const STOCK_INFO_A_CODE_NAME: StockInterface = StockInterface {
        name: "stock_info_a_code_name",
        scope: CacheScope::StockInfo,
        rotation: RotationInterval::LongTerm,
        has_params: false,
    };

    pub const STOCK_INFO_SH_NAME_CODE: StockInterface = StockInterface {
        name: "stock_info_sh_name_code",
        scope: CacheScope::StockInfo,
        rotation: RotationInterval::LongTerm,
        has_params: true,
    };

    pub const STOCK_INFO_SZ_NAME_CODE: StockInterface = StockInterface {
        name: "stock_info_sz_name_code",
        scope: CacheScope::StockInfo,
        rotation: RotationInterval::LongTerm,
        has_params: true,
    };

    pub const STOCK_INDIVIDUAL_INFO_EM: StockInterface = StockInterface {
        name: "stock_individual_info_em",
        scope: CacheScope::StockInfo,
        rotation: RotationInterval::LongTerm,
        has_params: true,
    };

    // ==================== 实时行情接口 ====================

    pub const STOCK_ZH_A_SPOT_EM: StockInterface = StockInterface {
        name: "stock_zh_a_spot_em",
        scope: CacheScope::StockQuote,
        rotation: RotationInterval::Realtime,
        has_params: false,
    };

    pub const STOCK_ZH_B_SPOT_EM: StockInterface = StockInterface {
        name: "stock_zh_b_spot_em",
        scope: CacheScope::StockQuote,
        rotation: RotationInterval::Realtime,
        has_params: false,
    };

    pub const STOCK_US_SPOT_EM: StockInterface = StockInterface {
        name: "stock_us_spot_em",
        scope: CacheScope::StockQuote,
        rotation: RotationInterval::Realtime,
        has_params: false,
    };

    pub const STOCK_HK_SPOT_EM: StockInterface = StockInterface {
        name: "stock_hk_spot_em",
        scope: CacheScope::StockQuote,
        rotation: RotationInterval::Realtime,
        has_params: false,
    };

    // ==================== 板块数据接口 ====================

    pub const STOCK_BOARD_INDUSTRY_NAME_EM: StockInterface = StockInterface {
        name: "stock_board_industry_name_em",
        scope: CacheScope::StockIndustry,
        rotation: RotationInterval::ShortTerm,
        has_params: false,
    };

    pub const STOCK_BOARD_CONCEPT_NAME_EM: StockInterface = StockInterface {
        name: "stock_board_concept_name_em",
        scope: CacheScope::StockIndustry,
        rotation: RotationInterval::ShortTerm,
        has_params: false,
    };

    pub const STOCK_BOARD_INDUSTRY_CONS_EM: StockInterface = StockInterface {
        name: "stock_board_industry_cons_em",
        scope: CacheScope::StockIndustry,
        rotation: RotationInterval::ShortTerm,
        has_params: true,
    };

    pub const STOCK_BOARD_CONCEPT_CONS_EM: StockInterface = StockInterface {
        name: "stock_board_concept_cons_em",
        scope: CacheScope::StockIndustry,
        rotation: RotationInterval::ShortTerm,
        has_params: true,
    };

    // ==================== 指数数据接口 ====================

    pub const STOCK_ZH_INDEX_SPOT_EM: StockInterface = StockInterface {
        name: "stock_zh_index_spot_em",
        scope: CacheScope::StockIndex,
        rotation: RotationInterval::Realtime,
        has_params: false,
    };

    pub const STOCK_ZH_INDEX_DAILY_EM: StockInterface = StockInterface {
        name: "stock_zh_index_daily_em",
        scope: CacheScope::StockIndex,
        rotation: RotationInterval::ShortTerm,
        has_params: true,
    };

    // ==================== 资金流向接口 ====================

    pub const STOCK_MARKET_FUND_FLOW: StockInterface = StockInterface {
        name: "stock_market_fund_flow",
        scope: CacheScope::StockFundFlow,
        rotation: RotationInterval::ShortTerm,
        has_params: false,
    };

    pub const STOCK_INDIVIDUAL_FUND_FLOW: StockInterface = StockInterface {
        name: "stock_individual_fund_flow",
        scope: CacheScope::StockFundFlow,
        rotation: RotationInterval::ShortTerm,
        has_params: true,
    };

    // ==================== 龙虎榜/涨跌停接口 ====================

    pub const STOCK_LHB_DETAIL_DAILY_SINA: StockInterface = StockInterface {
        name: "stock_lhb_detail_daily_sina",
        scope: CacheScope::StockLhb,
        rotation: RotationInterval::ShortTerm,
        has_params: true,
    };

    pub const STOCK_ZT_POOL_EM: StockInterface = StockInterface {
        name: "stock_zt_pool_em",
        scope: CacheScope::StockRanking,
        rotation: RotationInterval::Realtime,
        has_params: true,
    };

    pub const STOCK_DT_POOL_EM: StockInterface = StockInterface {
        name: "stock_zt_pool_dtgc_em",
        scope: CacheScope::StockRanking,
        rotation: RotationInterval::Realtime,
        has_params: true,
    };

    // ==================== 历史K线接口 ====================

    pub const STOCK_ZH_A_HIST: StockInterface = StockInterface {
        name: "stock_zh_a_hist",
        scope: CacheScope::StockKline,
        rotation: RotationInterval::ShortTerm,
        has_params: true,
    };

    pub const STOCK_ZH_A_HIST_MIN_EM: StockInterface = StockInterface {
        name: "stock_zh_a_hist_min_em",
        scope: CacheScope::StockKline,
        rotation: RotationInterval::ShortTerm,
        has_params: true,
    };

    pub const STOCK_HK_HIST: StockInterface = StockInterface {
        name: "stock_hk_hist",
        scope: CacheScope::StockKline,
        rotation: RotationInterval::ShortTerm,
        has_params: true,
    };

    // ==================== 新闻接口 ====================

    pub const STOCK_NEWS_EM: StockInterface = StockInterface {
        name: "stock_news_em",
        scope: CacheScope::StockNews,
        rotation: RotationInterval::Realtime,
        has_params: true,
    };

    /// 所有接口列表（用于调度器）
    pub static ALL_INTERFACES: &[&StockInterface] = &[
        // 基础信息
        &STOCK_INFO_A_CODE_NAME,
        &STOCK_INFO_SH_NAME_CODE,
        &STOCK_INFO_SZ_NAME_CODE,
        &STOCK_INDIVIDUAL_INFO_EM,
        // 实时行情
        &STOCK_ZH_A_SPOT_EM,
        &STOCK_ZH_B_SPOT_EM,
        &STOCK_US_SPOT_EM,
        &STOCK_HK_SPOT_EM,
        // 板块
        &STOCK_BOARD_INDUSTRY_NAME_EM,
        &STOCK_BOARD_CONCEPT_NAME_EM,
        &STOCK_BOARD_INDUSTRY_CONS_EM,
        &STOCK_BOARD_CONCEPT_CONS_EM,
        // 指数
        &STOCK_ZH_INDEX_SPOT_EM,
        &STOCK_ZH_INDEX_DAILY_EM,
        // 资金流向
        &STOCK_MARKET_FUND_FLOW,
        &STOCK_INDIVIDUAL_FUND_FLOW,
        // 龙虎榜/涨跌停
        &STOCK_LHB_DETAIL_DAILY_SINA,
        &STOCK_ZT_POOL_EM,
        &STOCK_DT_POOL_EM,
        // 历史K线
        &STOCK_ZH_A_HIST,
        &STOCK_ZH_A_HIST_MIN_EM,
        &STOCK_HK_HIST,
        // 新闻
        &STOCK_NEWS_EM,
    ];
}

// ==================== 缓存键构建 ====================

/// 构建缓存键 - 与 Python 端 build_cache_key 保持一致
///
/// # 格式
/// - 无参数: `interface_name`
/// - 有参数: `interface_name__key1_value1_key2_value2` (按key排序)
pub fn build_cache_key(
    interface_name: &str,
    params: &std::collections::HashMap<String, String>,
) -> String {
    if params.is_empty() {
        return interface_name.to_string();
    }

    // 按 key 排序并拼接
    let mut sorted_params: Vec<_> = params.iter().filter(|(_, v)| !v.is_empty()).collect();
    sorted_params.sort_by_key(|(k, _)| k.as_str());

    if sorted_params.is_empty() {
        return interface_name.to_string();
    }

    let param_str: String = sorted_params
        .iter()
        .map(|(k, v)| format!("{}_{}", k, v))
        .collect::<Vec<_>>()
        .join("_");

    format!("{}__{}", interface_name, param_str)
}

/// 从接口名获取缓存作用域
pub fn get_cache_scope(interface_name: &str) -> CacheScope {
    match interface_name {
        // 基础信息
        "stock_info_a_code_name"
        | "stock_info_sh_name_code"
        | "stock_info_sz_name_code"
        | "stock_individual_info_em" => CacheScope::StockInfo,

        // 实时行情
        "stock_zh_a_spot_em" | "stock_zh_b_spot_em" | "stock_us_spot_em" | "stock_hk_spot_em" => {
            CacheScope::StockQuote
        }

        // 历史K线
        "stock_zh_a_hist" | "stock_zh_a_hist_min_em" | "stock_hk_hist" => CacheScope::StockKline,

        // 指数
        "stock_zh_index_spot_em" | "stock_zh_index_daily_em" => CacheScope::StockIndex,

        // 板块
        "stock_board_industry_name_em"
        | "stock_board_concept_name_em"
        | "stock_board_industry_cons_em"
        | "stock_board_concept_cons_em" => CacheScope::StockIndustry,

        // 资金流向
        "stock_market_fund_flow" | "stock_individual_fund_flow" => CacheScope::StockFundFlow,

        // 排行
        "stock_zt_pool_em" | "stock_dt_pool_em" => CacheScope::StockRanking,

        // 龙虎榜
        "stock_lhb_detail_daily_sina" => CacheScope::StockLhb,

        // 新闻
        "stock_news_em" => CacheScope::StockNews,

        // 默认
        _ => CacheScope::StockInfo,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_cache_key_no_params() {
        let params = std::collections::HashMap::new();
        assert_eq!(
            build_cache_key("stock_zh_a_spot_em", &params),
            "stock_zh_a_spot_em"
        );
    }

    #[test]
    fn test_build_cache_key_with_params() {
        let mut params = std::collections::HashMap::new();
        params.insert("symbol".to_string(), "000001".to_string());
        params.insert("period".to_string(), "daily".to_string());

        let key = build_cache_key("stock_zh_a_hist", &params);
        assert_eq!(key, "stock_zh_a_hist__period_daily_symbol_000001");
    }

    #[test]
    fn test_interface_count() {
        assert_eq!(interfaces::ALL_INTERFACES.len(), 23);
    }
}
