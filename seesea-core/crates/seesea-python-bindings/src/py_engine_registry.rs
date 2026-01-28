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

/// Python引擎注册模块（已废弃）
///
/// 原有的回调系统已被基于raming的事件系统取代。
/// 保留此文件仅为兼容性，新的引擎注册请使用Python SDK中的engines模块。
use pyo3::prelude::*;

/// Python函数：注册一个新的Python引擎（已废弃）
///
/// 此函数已废弃，请使用Python SDK中的register_search_engine函数
#[pyfunction]
pub fn register_engine(
    _name: String,
    _engine_type: String,
    _description: String,
    _categories: Vec<String>,
    _callback: Py<PyAny>,
) -> PyResult<bool> {
    Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
        "register_engine is deprecated. Please use Python SDK engines.register_search_engine instead.",
    ))
}

/// Python函数：获取已注册的引擎列表（已废弃）
#[pyfunction]
pub fn list_engines() -> PyResult<Vec<String>> {
    Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
        "list_engines is deprecated. Please use Python SDK engines.list_search_engines instead.",
    ))
}

/// Python函数：注销一个引擎（已废弃）
#[pyfunction]
pub fn unregister_engine(_name: String) -> PyResult<bool> {
    Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
        "unregister_engine is deprecated. Please use Python SDK engines.unregister_search_engine instead.",
    ))
}

/// Python函数：检查引擎是否已注册（已废弃）
#[pyfunction]
pub fn has_engine(_name: String) -> PyResult<bool> {
    Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
        "has_engine is deprecated. Please use Python SDK engines.has_search_engine instead.",
    ))
}
