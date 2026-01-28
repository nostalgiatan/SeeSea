// Copyright (C) 2025 nostalgiatan
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

//! 股票客户端 - 通过 PyO3 调用 akshare
//!
//! 这是对 akshare 的 Rust 包装，直接调用 Python 获取数据

use crate::error::{StockError, StockResult};
use pyo3::prelude::*;
use pyo3::sync::PyOnceLock;
use pyo3::types::PyDict;
use serde_json::Value;
use std::collections::HashMap;
use tracing::{debug, error, info};

/// 全局 akshare 模块引用
static AKSHARE_MODULE: PyOnceLock<Py<PyModule>> = PyOnceLock::new();

/// 初始化 akshare 模块
fn ensure_akshare_initialized() -> StockResult<()> {
    Python::attach(|py| {
        AKSHARE_MODULE.get_or_try_init(py, || {
            let akshare = py
                .import("akshare")
                .map_err(|e| StockError::PythonError(format!("导入akshare失败: {}", e)))?;
            info!("✅ akshare 模块已初始化");
            Ok::<Py<PyModule>, StockError>(akshare.unbind())
        })?;
        Ok(())
    })
}

/// 股票客户端 - 调用 akshare 获取数据
pub struct StockClient;

impl StockClient {
    /// 创建新的股票客户端
    pub fn new() -> StockResult<Self> {
        ensure_akshare_initialized()?;
        Ok(Self)
    }

    /// 通用调用方法 - 调用 akshare 的任意函数
    ///
    /// # Arguments
    /// * `func_name` - akshare 函数名
    /// * `kwargs` - 关键字参数
    ///
    /// # Returns
    /// * `StockResult<Value>` - JSON 格式的返回数据
    pub fn call(&self, func_name: &str, kwargs: HashMap<String, String>) -> StockResult<Value> {
        let binding = std::thread::current();
        let thread_name = binding.name().unwrap_or("unknown");
        info!("📞 [{}] 开始调用 akshare.{}", thread_name, func_name);

        let result = Python::attach(|py| {
            // 获取 akshare 模块
            let akshare_py = AKSHARE_MODULE
                .get(py)
                .ok_or_else(|| StockError::PythonError("akshare未初始化".into()))?;
            let akshare = akshare_py.bind(py);

            // 获取函数
            let func = akshare.getattr(func_name).map_err(|e| {
                StockError::PythonError(format!("akshare.{} 不存在: {}", func_name, e))
            })?;

            // 构建参数
            let py_kwargs = PyDict::new(py);
            for (key, value) in &kwargs {
                py_kwargs
                    .set_item(key, value)
                    .map_err(|e| StockError::PythonError(format!("设置参数失败: {}", e)))?;
            }

            // 调用函数
            debug!("📞 调用 akshare.{}({:?})", func_name, kwargs);
            let result = func.call((), Some(&py_kwargs)).map_err(|e| {
                error!("❌ akshare.{} 调用失败: {}", func_name, e);
                StockError::PythonError(format!("调用失败: {}", e))
            })?;

            // 转换 DataFrame 为 JSON
            let json_data = dataframe_to_json(py, &result)?;

            debug!("✅ akshare.{} 返回数据", func_name);
            Ok(json_data)
        });

        info!("✅ [{}] akshare.{} 调用完成", thread_name, func_name);
        result
    }

    // ==================== 便捷方法 ====================

    /// 获取 A 股代码名称
    pub fn stock_info_a_code_name(&self) -> StockResult<Value> {
        self.call("stock_info_a_code_name", HashMap::new())
    }

    /// 获取上交所股票信息
    pub fn stock_info_sh_name_code(&self, symbol: Option<&str>) -> StockResult<Value> {
        let mut kwargs = HashMap::new();
        if let Some(s) = symbol {
            kwargs.insert("symbol".to_string(), s.to_string());
        }
        self.call("stock_info_sh_name_code", kwargs)
    }

    /// 获取深交所股票信息
    pub fn stock_info_sz_name_code(&self, symbol: Option<&str>) -> StockResult<Value> {
        let mut kwargs = HashMap::new();
        if let Some(s) = symbol {
            kwargs.insert("symbol".to_string(), s.to_string());
        }
        self.call("stock_info_sz_name_code", kwargs)
    }

    /// 获取个股信息
    pub fn stock_individual_info_em(&self, symbol: &str) -> StockResult<Value> {
        let mut kwargs = HashMap::new();
        kwargs.insert("symbol".to_string(), symbol.to_string());
        self.call("stock_individual_info_em", kwargs)
    }

    /// 获取 A 股实时行情
    pub fn stock_zh_a_spot_em(&self) -> StockResult<Value> {
        self.call("stock_zh_a_spot_em", HashMap::new())
    }

    /// 获取 B 股实时行情
    pub fn stock_zh_b_spot_em(&self) -> StockResult<Value> {
        self.call("stock_zh_b_spot_em", HashMap::new())
    }

    /// 获取美股实时行情
    pub fn stock_us_spot_em(&self) -> StockResult<Value> {
        self.call("stock_us_spot_em", HashMap::new())
    }

    /// 获取港股实时行情
    pub fn stock_hk_spot_em(&self) -> StockResult<Value> {
        self.call("stock_hk_spot_em", HashMap::new())
    }

    /// 获取 A 股历史行情
    pub fn stock_zh_a_hist(
        &self,
        symbol: &str,
        period: Option<&str>,
        start_date: Option<&str>,
        end_date: Option<&str>,
        adjust: Option<&str>,
    ) -> StockResult<Value> {
        let mut kwargs = HashMap::new();
        kwargs.insert("symbol".to_string(), symbol.to_string());
        if let Some(p) = period {
            kwargs.insert("period".to_string(), p.to_string());
        }
        if let Some(s) = start_date {
            kwargs.insert("start_date".to_string(), s.to_string());
        }
        if let Some(e) = end_date {
            kwargs.insert("end_date".to_string(), e.to_string());
        }
        if let Some(a) = adjust {
            kwargs.insert("adjust".to_string(), a.to_string());
        }
        self.call("stock_zh_a_hist", kwargs)
    }

    /// 获取行业板块列表
    pub fn stock_board_industry_name_em(&self) -> StockResult<Value> {
        self.call("stock_board_industry_name_em", HashMap::new())
    }

    /// 获取概念板块列表
    pub fn stock_board_concept_name_em(&self) -> StockResult<Value> {
        self.call("stock_board_concept_name_em", HashMap::new())
    }

    /// 获取行业板块成分股
    pub fn stock_board_industry_cons_em(&self, symbol: &str) -> StockResult<Value> {
        let mut kwargs = HashMap::new();
        kwargs.insert("symbol".to_string(), symbol.to_string());
        self.call("stock_board_industry_cons_em", kwargs)
    }

    /// 获取概念板块成分股
    pub fn stock_board_concept_cons_em(&self, symbol: &str) -> StockResult<Value> {
        let mut kwargs = HashMap::new();
        kwargs.insert("symbol".to_string(), symbol.to_string());
        self.call("stock_board_concept_cons_em", kwargs)
    }

    /// 获取指数实时行情
    pub fn stock_zh_index_spot_em(&self) -> StockResult<Value> {
        self.call("stock_zh_index_spot_em", HashMap::new())
    }

    /// 获取指数历史行情
    pub fn stock_zh_index_daily_em(
        &self,
        symbol: &str,
        start_date: Option<&str>,
        end_date: Option<&str>,
    ) -> StockResult<Value> {
        let mut kwargs = HashMap::new();
        kwargs.insert("symbol".to_string(), symbol.to_string());
        if let Some(s) = start_date {
            kwargs.insert("start_date".to_string(), s.to_string());
        }
        if let Some(e) = end_date {
            kwargs.insert("end_date".to_string(), e.to_string());
        }
        self.call("stock_zh_index_daily_em", kwargs)
    }

    /// 获取市场资金流向
    pub fn stock_market_fund_flow(&self) -> StockResult<Value> {
        self.call("stock_market_fund_flow", HashMap::new())
    }

    /// 获取个股资金流向
    pub fn stock_individual_fund_flow(&self, stock: &str, market: &str) -> StockResult<Value> {
        let mut kwargs = HashMap::new();
        kwargs.insert("stock".to_string(), stock.to_string());
        kwargs.insert("market".to_string(), market.to_string());
        self.call("stock_individual_fund_flow", kwargs)
    }

    /// 获取龙虎榜详情
    pub fn stock_lhb_detail_daily_sina(&self, date: &str) -> StockResult<Value> {
        let mut kwargs = HashMap::new();
        kwargs.insert("date".to_string(), date.to_string());
        self.call("stock_lhb_detail_daily_sina", kwargs)
    }

    /// 获取涨停股池
    pub fn stock_zt_pool_em(&self, date: &str) -> StockResult<Value> {
        let mut kwargs = HashMap::new();
        kwargs.insert("date".to_string(), date.to_string());
        self.call("stock_zt_pool_em", kwargs)
    }

    /// 获取跌停股池
    pub fn stock_dt_pool_em(&self, date: &str) -> StockResult<Value> {
        let mut kwargs = HashMap::new();
        kwargs.insert("date".to_string(), date.to_string());
        self.call("stock_zt_pool_dtgc_em", kwargs)
    }

    /// 获取股票新闻
    pub fn stock_news_em(&self, symbol: &str) -> StockResult<Value> {
        let mut kwargs = HashMap::new();
        kwargs.insert("symbol".to_string(), symbol.to_string());
        self.call("stock_news_em", kwargs)
    }

    /// 获取 A 股分钟历史行情
    pub fn stock_zh_a_hist_min_em(
        &self,
        symbol: &str,
        period: Option<&str>,
        start_date: Option<&str>,
        end_date: Option<&str>,
        adjust: Option<&str>,
    ) -> StockResult<Value> {
        let mut kwargs = HashMap::new();
        kwargs.insert("symbol".to_string(), symbol.to_string());
        if let Some(p) = period {
            kwargs.insert("period".to_string(), p.to_string());
        }
        if let Some(s) = start_date {
            kwargs.insert("start_date".to_string(), s.to_string());
        }
        if let Some(e) = end_date {
            kwargs.insert("end_date".to_string(), e.to_string());
        }
        if let Some(a) = adjust {
            kwargs.insert("adjust".to_string(), a.to_string());
        }
        self.call("stock_zh_a_hist_min_em", kwargs)
    }

    /// 获取港股历史行情
    pub fn stock_hk_hist(
        &self,
        symbol: &str,
        period: Option<&str>,
        start_date: Option<&str>,
        end_date: Option<&str>,
        adjust: Option<&str>,
    ) -> StockResult<Value> {
        let mut kwargs = HashMap::new();
        kwargs.insert("symbol".to_string(), symbol.to_string());
        if let Some(p) = period {
            kwargs.insert("period".to_string(), p.to_string());
        }
        if let Some(s) = start_date {
            kwargs.insert("start_date".to_string(), s.to_string());
        }
        if let Some(e) = end_date {
            kwargs.insert("end_date".to_string(), e.to_string());
        }
        if let Some(a) = adjust {
            kwargs.insert("adjust".to_string(), a.to_string());
        }
        self.call("stock_hk_hist", kwargs)
    }
}

impl Default for StockClient {
    fn default() -> Self {
        Self::new().expect("初始化StockClient失败")
    }
}

/// 将 pandas DataFrame 转换为 JSON
fn dataframe_to_json(py: Python<'_>, df: &Bound<'_, PyAny>) -> StockResult<Value> {
    // 检查是否为 DataFrame
    let pandas = py
        .import("pandas")
        .map_err(|e| StockError::PythonError(format!("导入pandas失败: {}", e)))?;
    let dataframe_class = pandas
        .getattr("DataFrame")
        .map_err(|e| StockError::PythonError(format!("获取DataFrame类失败: {}", e)))?;

    if !df
        .is_instance(&dataframe_class)
        .map_err(|e| StockError::PythonError(format!("类型检查失败: {}", e)))?
    {
        // 如果不是 DataFrame，尝试直接转换
        let json_str: String = df
            .str()
            .map_err(|e| StockError::ParseError(format!("转换字符串失败: {}", e)))?
            .to_string();
        return Ok(Value::String(json_str));
    }

    // 检查是否为空
    let is_empty: bool = df
        .getattr("empty")
        .map_err(|e| StockError::ParseError(format!("检查empty失败: {}", e)))?
        .extract()
        .map_err(|e| StockError::ParseError(format!("提取empty值失败: {}", e)))?;

    if is_empty {
        return Ok(Value::Array(vec![]));
    }

    // 使用 to_json 方法转换
    let kwargs = PyDict::new(py);
    kwargs
        .set_item("orient", "records")
        .map_err(|e| StockError::PythonError(format!("设置orient失败: {}", e)))?;

    let json_result = df
        .call_method("to_json", (), Some(&kwargs))
        .map_err(|e| StockError::ParseError(format!("DataFrame.to_json失败: {}", e)))?;

    // 提取字符串 - 可能是 None 或 str
    let json_str: String = if json_result.is_none() {
        return Ok(Value::Array(vec![]));
    } else {
        json_result
            .extract()
            .map_err(|e| StockError::ParseError(format!("提取JSON字符串失败: {}", e)))?
    };

    // 解析 JSON 字符串
    let json_value: Value = serde_json::from_str(&json_str)?;

    Ok(json_value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stock_client_creation() {
        let client = StockClient::new();
        assert!(client.is_ok());
    }

    #[test]
    fn test_stock_info_a_code_name() {
        let client = StockClient::new().unwrap();
        let result = client.stock_info_a_code_name();
        // 这个测试需要网络，可能会失败
        if let Ok(data) = result {
            assert!(data.is_array());
        }
    }
}
