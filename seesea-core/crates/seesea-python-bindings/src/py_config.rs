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

//! Python bindings for configuration

use pyo3::prelude::*;
use seesea_config::on::init_config_with_env;

#[pyclass]
#[derive(Clone)]
pub struct PyConfig {
    #[pyo3(get, set)]
    pub debug: bool,
    #[pyo3(get, set)]
    pub max_results: usize,
    #[pyo3(get, set)]
    pub timeout_seconds: u64,
}

#[pymethods]
impl PyConfig {
    #[new]
    pub fn new() -> Self {
        Self::default()
    }
}

// 添加Default实现以满足Clippy警告
impl Default for PyConfig {
    fn default() -> Self {
        Self {
            debug: false,
            max_results: 100,
            timeout_seconds: 30,
        }
    }
}

/// 初始化配置
#[pyfunction]
pub fn init_config(environment: &str) -> PyResult<()> {
    // 初始化全局配置
    tokio::runtime::Runtime::new()
        .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?
        .block_on(async {
            match init_config_with_env(environment).await {
                Ok(_) => Ok(()),
                Err(e) => Err(pyo3::exceptions::PyRuntimeError::new_err(e.to_string())),
            }
        })
}

/// 获取缓存目录路径
#[pyfunction]
pub fn get_cache_dir() -> String {
    seesea_config::paths::get_cache_dir()
}

/// 获取数据目录路径
#[pyfunction]
pub fn get_data_dir() -> String {
    seesea_config::paths::get_data_dir()
}

/// 获取配置目录路径
#[pyfunction]
pub fn get_config_dir() -> String {
    seesea_config::paths::get_config_dir()
}

/// 获取日志目录路径
#[pyfunction]
pub fn get_log_dir() -> String {
    seesea_config::paths::get_log_dir()
}
