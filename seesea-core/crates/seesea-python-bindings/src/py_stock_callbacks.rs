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

//! 股票API回调函数注册机制（已废弃）
//!
//! 该模块已被废弃，当前使用缓存优先模式：
//! - Rust端通过HTTP API直接调用Python函数
//! - 不再使用回调机制进行通信
//! - 所有股票数据通过缓存系统获取

use pyo3::prelude::*;

/// 注册股票搜索回调函数（已废弃）
///
/// 当前使用缓存优先模式，此函数仅保留用于向后兼容性
#[pyfunction]
pub fn register_stock_search(_callback: Py<PyAny>) -> PyResult<()> {
    // 回调机制已废弃，使用缓存优先模式
    Ok(())
}

/// 注册股票行情回调函数（已废弃）
///
/// 当前使用缓存优先模式，此函数仅保留用于向后兼容性
#[pyfunction]
pub fn register_stock_quotes(_callback: Py<PyAny>) -> PyResult<()> {
    // 回调机制已废弃，使用缓存优先模式
    Ok(())
}

/// 注册股票K线数据回调函数（已废弃）
///
/// 当前使用缓存优先模式，此函数仅保留用于向后兼容性
#[pyfunction]
pub fn register_stock_klines(_callback: Py<PyAny>) -> PyResult<()> {
    // 回调机制已废弃，使用缓存优先模式
    Ok(())
}

/// 注册股票详情回调函数（已废弃）
///
/// 当前使用缓存优先模式，此函数仅保留用于向后兼容性
#[pyfunction]
pub fn register_stock_detail(_callback: Py<PyAny>) -> PyResult<()> {
    // 回调机制已废弃，使用缓存优先模式
    Ok(())
}

/// 注册股票财务数据回调函数（已废弃）
///
/// 当前使用缓存优先模式，此函数仅保留用于向后兼容性
#[pyfunction]
pub fn register_stock_financial(_callback: Py<PyAny>) -> PyResult<()> {
    // 回调机制已废弃，使用缓存优先模式
    Ok(())
}

/// 注册股票资金流向回调函数（已废弃）
///
/// 当前使用缓存优先模式，此函数仅保留用于向后兼容性
#[pyfunction]
pub fn register_stock_fund_flow(_callback: Py<PyAny>) -> PyResult<()> {
    // 回调机制已废弃，使用缓存优先模式
    Ok(())
}

/// 注册股票股东信息回调函数（已废弃）
///
/// 当前使用缓存优先模式，此函数仅保留用于向后兼容性
#[pyfunction]
pub fn register_stock_holders(_callback: Py<PyAny>) -> PyResult<()> {
    // 回调机制已废弃，使用缓存优先模式
    Ok(())
}

/// 注册股票公告回调函数（已废弃）
///
/// 当前使用缓存优先模式，此函数仅保留用于向后兼容性
#[pyfunction]
pub fn register_stock_announcements(_callback: Py<PyAny>) -> PyResult<()> {
    // 回调机制已废弃，使用缓存优先模式
    Ok(())
}

/// 注册市场状态回调函数（已废弃）
///
/// 当前使用缓存优先模式，此函数仅保留用于向后兼容性
#[pyfunction]
pub fn register_market_status(_callback: Py<PyAny>) -> PyResult<()> {
    // 回调机制已废弃，使用缓存优先模式
    Ok(())
}

/// 注册市场指数回调函数（已废弃）
///
/// 当前使用缓存优先模式，此函数仅保留用于向后兼容性
#[pyfunction]
pub fn register_market_indices(_callback: Py<PyAny>) -> PyResult<()> {
    // 回调机制已废弃，使用缓存优先模式
    Ok(())
}

/// 注册市场龙虎榜回调函数（已废弃）
///
/// 当前使用缓存优先模式，此函数仅保留用于向后兼容性
#[pyfunction]
pub fn register_market_lhb(_callback: Py<PyAny>) -> PyResult<()> {
    // 回调机制已废弃，使用缓存优先模式
    Ok(())
}

/// 注册行业板块列表回调函数（已废弃）
///
/// 当前使用缓存优先模式，此函数仅保留用于向后兼容性
#[pyfunction]
pub fn register_sector_list(_callback: Py<PyAny>) -> PyResult<()> {
    // 回调机制已废弃，使用缓存优先模式
    Ok(())
}

/// 注册行业板块股票列表回调函数（已废弃）
///
/// 当前使用缓存优先模式，此函数仅保留用于向后兼容性
#[pyfunction]
pub fn register_sector_stocks(_callback: Py<PyAny>) -> PyResult<()> {
    // 回调机制已废弃，使用缓存优先模式
    Ok(())
}

/// 注册股票排行榜回调函数（已废弃）
///
/// 当前使用缓存优先模式，此函数仅保留用于向后兼容性
#[pyfunction]
pub fn register_stock_ranking(_callback: Py<PyAny>) -> PyResult<()> {
    // 回调机制已废弃，使用缓存优先模式
    Ok(())
}
