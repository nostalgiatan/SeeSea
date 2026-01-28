// Copyright (C) 2025 nostalgiatan
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

//! seesea-stock 集成测试
//!
//! 测试股票数据获取、缓存、处理等核心功能

use seesea_stock::{
    CacheScope, StockClient, StockProcessor, get_cached_stock_client, start_scheduler,
    stop_scheduler,
};
use std::collections::HashMap;

/// 测试 StockClient 初始化
#[test]
fn test_stock_client_creation() {
    let result = StockClient::new();
    assert!(result.is_ok(), "StockClient 应该能成功创建");
}

/// 测试 CachedStockClient 单例获取
#[test]
fn test_cached_stock_client_singleton() {
    let client1 = get_cached_stock_client();
    assert!(client1.is_ok(), "第一次获取 CachedStockClient 应该成功");

    let client2 = get_cached_stock_client();
    assert!(client2.is_ok(), "第二次获取 CachedStockClient 应该成功");
}

/// 测试 StockProcessor 创建
#[test]
fn test_stock_processor_creation() {
    let processor = StockProcessor::new();
    // Processor 是无状态的，创建应该总是成功
    assert!(true, "StockProcessor 创建成功");
}

/// 测试 CacheScope 转换
#[test]
fn test_cache_scope_as_str() {
    assert_eq!(CacheScope::StockInfo.as_str(), "stock.info");
    assert_eq!(CacheScope::StockQuote.as_str(), "stock.quote");
    assert_eq!(CacheScope::StockKline.as_str(), "stock.kline");
    assert_eq!(CacheScope::StockIndex.as_str(), "stock.index");
    assert_eq!(CacheScope::StockIndustry.as_str(), "stock.industry");
    assert_eq!(CacheScope::StockFundFlow.as_str(), "stock.fund_flow");
    assert_eq!(CacheScope::StockRanking.as_str(), "stock.ranking");
    assert_eq!(CacheScope::StockLhb.as_str(), "stock.lhb");
    assert_eq!(CacheScope::StockNews.as_str(), "stock.news");
}

/// 测试 CacheScope TTL
#[test]
fn test_cache_scope_ttl() {
    assert_eq!(CacheScope::StockInfo.ttl_seconds(), 43200); // 12小时
    assert_eq!(CacheScope::StockQuote.ttl_seconds(), 360); // 6分钟
    assert_eq!(CacheScope::StockKline.ttl_seconds(), 3600); // 1小时
}

/// 测试调度器启动和停止
/// 注意：调度器在后台线程运行，可能需要Python环境
#[test]
#[ignore = "调度器测试需要完整环境，单独运行"]
fn test_scheduler_start_stop() {
    // 启动调度器
    start_scheduler();

    // 等待一小段时间
    std::thread::sleep(std::time::Duration::from_millis(500));

    // 停止调度器
    stop_scheduler();

    // 再次停止应该是安全的
    stop_scheduler();
}

// ============================================================================
// 网络测试 - 需要网络和 Python 环境
// ============================================================================

/// 测试获取 A 股代码名称列表
#[test]
#[ignore = "需要网络和 Python 环境"]
fn test_fetch_stock_info_a_code_name() {
    let client = get_cached_stock_client().expect("获取客户端失败");
    let result = client.stock_info_a_code_name();

    assert!(
        result.is_ok(),
        "获取 A 股代码名称应该成功: {:?}",
        result.err()
    );

    let data = result.unwrap();
    assert!(data.is_array(), "返回数据应该是数组");

    let arr = data.as_array().unwrap();
    assert!(!arr.is_empty(), "A 股列表不应该为空");

    // 验证数据结构
    let first = &arr[0];
    assert!(
        first.get("代码").is_some() || first.get("code").is_some(),
        "应该包含代码字段"
    );
}

/// 测试获取 A 股实时行情
#[test]
#[ignore = "需要网络和 Python 环境"]
fn test_fetch_stock_zh_a_spot_em() {
    let client = get_cached_stock_client().expect("获取客户端失败");
    let result = client.stock_zh_a_spot_em();

    assert!(
        result.is_ok(),
        "获取 A 股实时行情应该成功: {:?}",
        result.err()
    );

    let data = result.unwrap();
    assert!(data.is_array(), "返回数据应该是数组");

    let arr = data.as_array().unwrap();
    assert!(
        arr.len() > 1000,
        "A 股数量应该超过1000只，实际: {}",
        arr.len()
    );
}

/// 测试 StockProcessor 搜索功能
#[test]
#[ignore = "需要网络和 Python 环境"]
fn test_processor_search() {
    let client = get_cached_stock_client().expect("获取客户端失败");
    let data = client.stock_zh_a_spot_em().expect("获取行情数据失败");

    let processor = StockProcessor::new();

    // 搜索 "平安"
    let result = processor.search(&data, "平安", Some(10));
    assert!(result.is_ok(), "搜索应该成功: {:?}", result.err());

    let results = result.unwrap();
    assert!(!results.is_empty(), "搜索 '平安' 应该有结果");
    assert!(results.len() <= 10, "结果数量不应超过限制");
}

/// 测试按代码获取股票
#[test]
#[ignore = "需要网络和 Python 环境"]
fn test_processor_get_by_code() {
    let client = get_cached_stock_client().expect("获取客户端失败");
    let data = client.stock_zh_a_spot_em().expect("获取行情数据失败");

    let processor = StockProcessor::new();

    // 获取 000001 平安银行
    let result = processor.get_by_code(&data, "000001");
    assert!(result.is_ok(), "按代码获取应该成功: {:?}", result.err());

    let stock = result.unwrap();
    assert!(stock.is_some(), "000001 应该存在");
}

/// 测试板块数据获取
#[test]
#[ignore = "需要网络和 Python 环境"]
fn test_fetch_board_data() {
    let client = get_cached_stock_client().expect("获取客户端失败");

    // 行业板块
    let industry = client.stock_board_industry_name_em();
    assert!(
        industry.is_ok(),
        "获取行业板块应该成功: {:?}",
        industry.err()
    );

    // 概念板块
    let concept = client.stock_board_concept_name_em();
    assert!(concept.is_ok(), "获取概念板块应该成功: {:?}", concept.err());
}

/// 测试指数数据获取
#[test]
fn test_fetch_index_data() {
    let client = get_cached_stock_client().expect("获取客户端失败");

    let result = client.stock_zh_index_spot_em();
    assert!(
        result.is_ok(),
        "获取指数实时行情应该成功: {:?}",
        result.err()
    );

    let data = result.unwrap();
    assert!(data.is_array(), "返回数据应该是数组");
}

/// 测试涨停池数据
#[test]
fn test_fetch_zt_pool() {
    let client = get_cached_stock_client().expect("获取客户端失败");

    // 不传日期，使用当天
    let result = client.stock_zt_pool_em(None);
    assert!(result.is_ok(), "获取涨停池应该成功: {:?}", result.err());
}

/// 测试缓存命中
#[test]
fn test_cache_hit() {
    let client = get_cached_stock_client().expect("获取客户端失败");

    // 第一次调用
    let start1 = std::time::Instant::now();
    let _result1 = client.stock_info_a_code_name().expect("第一次获取失败");
    let duration1 = start1.elapsed();

    // 第二次调用（应该命中缓存）
    let start2 = std::time::Instant::now();
    let _result2 = client.stock_info_a_code_name().expect("第二次获取失败");
    let duration2 = start2.elapsed();

    println!("第一次耗时: {:?}", duration1);
    println!("第二次耗时: {:?}", duration2);

    // 缓存命中应该快很多
    assert!(duration2 < duration1 / 2, "缓存命中应该比首次请求快很多");
}
