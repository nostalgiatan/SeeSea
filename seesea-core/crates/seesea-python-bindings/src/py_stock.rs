// Copyright (C) 2025 nostalgiatan
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

//! Python 绑定 - 股票客户端
//!
//! 将 seesea-stock crate 暴露给 Python 使用

use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;
use seesea_config::SchedulerConfig;
use seesea_stock::{StockProcessor, get_cached_stock_client, start_scheduler, stop_scheduler};
use tracing::info;

/// Python 股票客户端
#[pyclass]
pub struct PyStockClient {
    processor: StockProcessor,
}

#[pymethods]
impl PyStockClient {
    /// 创建新的股票客户端
    #[new]
    pub fn new() -> PyResult<Self> {
        // 初始化缓存客户端
        get_cached_stock_client()
            .map_err(|e| PyRuntimeError::new_err(format!("初始化股票客户端失败: {}", e)))?;

        Ok(Self {
            processor: StockProcessor::new(),
        })
    }

    /// 获取 A 股代码名称列表
    pub fn get_stock_info_a_code_name(&self) -> PyResult<String> {
        let client =
            get_cached_stock_client().map_err(|e| PyRuntimeError::new_err(format!("{}", e)))?;
        let data = client
            .stock_info_a_code_name()
            .map_err(|e| PyRuntimeError::new_err(format!("{}", e)))?;
        Ok(serde_json::to_string(&data).unwrap_or_default())
    }

    /// 获取 A 股实时行情
    pub fn get_stock_zh_a_spot_em(&self) -> PyResult<String> {
        let client =
            get_cached_stock_client().map_err(|e| PyRuntimeError::new_err(format!("{}", e)))?;
        let data = client
            .stock_zh_a_spot_em()
            .map_err(|e| PyRuntimeError::new_err(format!("{}", e)))?;
        Ok(serde_json::to_string(&data).unwrap_or_default())
    }

    /// 获取 B 股实时行情
    pub fn get_stock_zh_b_spot_em(&self) -> PyResult<String> {
        let client =
            get_cached_stock_client().map_err(|e| PyRuntimeError::new_err(format!("{}", e)))?;
        let data = client
            .stock_zh_b_spot_em()
            .map_err(|e| PyRuntimeError::new_err(format!("{}", e)))?;
        Ok(serde_json::to_string(&data).unwrap_or_default())
    }

    /// 获取港股实时行情
    pub fn get_stock_hk_spot_em(&self) -> PyResult<String> {
        let client =
            get_cached_stock_client().map_err(|e| PyRuntimeError::new_err(format!("{}", e)))?;
        let data = client
            .stock_hk_spot_em()
            .map_err(|e| PyRuntimeError::new_err(format!("{}", e)))?;
        Ok(serde_json::to_string(&data).unwrap_or_default())
    }

    /// 获取美股实时行情
    pub fn get_stock_us_spot_em(&self) -> PyResult<String> {
        let client =
            get_cached_stock_client().map_err(|e| PyRuntimeError::new_err(format!("{}", e)))?;
        let data = client
            .stock_us_spot_em()
            .map_err(|e| PyRuntimeError::new_err(format!("{}", e)))?;
        Ok(serde_json::to_string(&data).unwrap_or_default())
    }

    /// 获取个股信息
    pub fn get_stock_individual_info(&self, symbol: &str) -> PyResult<String> {
        let client =
            get_cached_stock_client().map_err(|e| PyRuntimeError::new_err(format!("{}", e)))?;
        let data = client
            .stock_individual_info_em(symbol)
            .map_err(|e| PyRuntimeError::new_err(format!("{}", e)))?;
        Ok(serde_json::to_string(&data).unwrap_or_default())
    }

    /// 获取 A 股历史行情
    pub fn get_stock_zh_a_hist(
        &self,
        symbol: &str,
        period: Option<&str>,
        start_date: Option<&str>,
        end_date: Option<&str>,
        adjust: Option<&str>,
    ) -> PyResult<String> {
        let client =
            get_cached_stock_client().map_err(|e| PyRuntimeError::new_err(format!("{}", e)))?;
        let data = client
            .stock_zh_a_hist(symbol, period, start_date, end_date, adjust)
            .map_err(|e| PyRuntimeError::new_err(format!("{}", e)))?;
        Ok(serde_json::to_string(&data).unwrap_or_default())
    }

    /// 获取港股历史行情
    pub fn get_stock_hk_hist(
        &self,
        symbol: &str,
        period: Option<&str>,
        start_date: Option<&str>,
        end_date: Option<&str>,
        adjust: Option<&str>,
    ) -> PyResult<String> {
        let client =
            get_cached_stock_client().map_err(|e| PyRuntimeError::new_err(format!("{}", e)))?;
        let data = client
            .stock_hk_hist(symbol, period, start_date, end_date, adjust)
            .map_err(|e| PyRuntimeError::new_err(format!("{}", e)))?;
        Ok(serde_json::to_string(&data).unwrap_or_default())
    }

    /// 获取行业板块列表
    pub fn get_board_industry_name(&self) -> PyResult<String> {
        let client =
            get_cached_stock_client().map_err(|e| PyRuntimeError::new_err(format!("{}", e)))?;
        let data = client
            .stock_board_industry_name_em()
            .map_err(|e| PyRuntimeError::new_err(format!("{}", e)))?;
        Ok(serde_json::to_string(&data).unwrap_or_default())
    }

    /// 获取概念板块列表
    pub fn get_board_concept_name(&self) -> PyResult<String> {
        let client =
            get_cached_stock_client().map_err(|e| PyRuntimeError::new_err(format!("{}", e)))?;
        let data = client
            .stock_board_concept_name_em()
            .map_err(|e| PyRuntimeError::new_err(format!("{}", e)))?;
        Ok(serde_json::to_string(&data).unwrap_or_default())
    }

    /// 获取行业板块成分股
    pub fn get_board_industry_cons(&self, symbol: &str) -> PyResult<String> {
        let client =
            get_cached_stock_client().map_err(|e| PyRuntimeError::new_err(format!("{}", e)))?;
        let data = client
            .stock_board_industry_cons_em(symbol)
            .map_err(|e| PyRuntimeError::new_err(format!("{}", e)))?;
        Ok(serde_json::to_string(&data).unwrap_or_default())
    }

    /// 获取概念板块成分股
    pub fn get_board_concept_cons(&self, symbol: &str) -> PyResult<String> {
        let client =
            get_cached_stock_client().map_err(|e| PyRuntimeError::new_err(format!("{}", e)))?;
        let data = client
            .stock_board_concept_cons_em(symbol)
            .map_err(|e| PyRuntimeError::new_err(format!("{}", e)))?;
        Ok(serde_json::to_string(&data).unwrap_or_default())
    }

    /// 获取指数实时行情
    pub fn get_index_spot(&self) -> PyResult<String> {
        let client =
            get_cached_stock_client().map_err(|e| PyRuntimeError::new_err(format!("{}", e)))?;
        let data = client
            .stock_zh_index_spot_em()
            .map_err(|e| PyRuntimeError::new_err(format!("{}", e)))?;
        Ok(serde_json::to_string(&data).unwrap_or_default())
    }

    /// 获取市场资金流向
    pub fn get_market_fund_flow(&self) -> PyResult<String> {
        let client =
            get_cached_stock_client().map_err(|e| PyRuntimeError::new_err(format!("{}", e)))?;
        let data = client
            .stock_market_fund_flow()
            .map_err(|e| PyRuntimeError::new_err(format!("{}", e)))?;
        Ok(serde_json::to_string(&data).unwrap_or_default())
    }

    /// 获取涨停股池
    pub fn get_zt_pool(&self, date: Option<&str>) -> PyResult<String> {
        let client =
            get_cached_stock_client().map_err(|e| PyRuntimeError::new_err(format!("{}", e)))?;
        let data = client
            .stock_zt_pool_em(date)
            .map_err(|e| PyRuntimeError::new_err(format!("{}", e)))?;
        Ok(serde_json::to_string(&data).unwrap_or_default())
    }

    /// 获取跌停股池
    pub fn get_dt_pool(&self, date: Option<&str>) -> PyResult<String> {
        let client =
            get_cached_stock_client().map_err(|e| PyRuntimeError::new_err(format!("{}", e)))?;
        let data = client
            .stock_dt_pool_em(date)
            .map_err(|e| PyRuntimeError::new_err(format!("{}", e)))?;
        Ok(serde_json::to_string(&data).unwrap_or_default())
    }

    /// 搜索股票（按代码或名称）
    pub fn search(&self, keyword: &str, limit: Option<usize>) -> PyResult<String> {
        // 先获取全量数据
        let client =
            get_cached_stock_client().map_err(|e| PyRuntimeError::new_err(format!("{}", e)))?;
        let data = client
            .stock_zh_a_spot_em()
            .map_err(|e| PyRuntimeError::new_err(format!("{}", e)))?;

        // 使用 processor 搜索
        let results = self
            .processor
            .search(&data, keyword, limit)
            .map_err(|e| PyRuntimeError::new_err(format!("{}", e)))?;

        Ok(serde_json::to_string(&results).unwrap_or_default())
    }

    /// 按代码获取单只股票
    pub fn get_by_code(&self, code: &str) -> PyResult<Option<String>> {
        let client =
            get_cached_stock_client().map_err(|e| PyRuntimeError::new_err(format!("{}", e)))?;
        let data = client
            .stock_zh_a_spot_em()
            .map_err(|e| PyRuntimeError::new_err(format!("{}", e)))?;

        let result = self
            .processor
            .get_by_code(&data, code)
            .map_err(|e| PyRuntimeError::new_err(format!("{}", e)))?;

        Ok(result.map(|v| serde_json::to_string(&v).unwrap_or_default()))
    }
}

/// 启动股票数据轮转调度器
#[pyfunction]
pub fn stock_start_scheduler() -> PyResult<()> {
    info!("📈 正在启动股票数据调度器...");
    start_scheduler();
    info!("✅ 股票数据调度器已启动");
    Ok(())
}

/// 启动股票数据轮转调度器（带配置文件）
#[pyfunction]
pub fn stock_start_scheduler_with_config(config_path: &str) -> PyResult<()> {
    info!("📈 正在从配置文件启动股票数据调度器: {}", config_path);

    let config_str = std::fs::read_to_string(config_path)
        .map_err(|e| PyRuntimeError::new_err(format!("读取配置文件失败: {}", e)))?;

    let config: SchedulerConfig = toml::from_str(&config_str)
        .map_err(|e| PyRuntimeError::new_err(format!("解析配置文件失败: {}", e)))?;

    let scheduler = seesea_stock::get_scheduler();
    if scheduler.is_running() {
        info!("⚠️ 调度器已在运行中");
        return Ok(());
    }

    info!("📋 从配置加载调度任务...");
    scheduler.add_tasks_from_config(&config);
    info!("🚀 启动调度器工作线程...");
    scheduler.start();
    info!("✅ 调度器启动完成，后台任务开始执行");

    Ok(())
}

/// 停止股票数据轮转调度器
#[pyfunction]
pub fn stock_stop_scheduler() -> PyResult<()> {
    stop_scheduler();
    Ok(())
}
