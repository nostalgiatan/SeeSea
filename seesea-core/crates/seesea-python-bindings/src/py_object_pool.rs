// Copyright (C) 2024 SeeSea Authors
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as
// published by the Free Software Foundation, either version 3 of the
// License, or (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

//! Python bindings for DatePageObjectPool functionality

use pyo3::prelude::*;
use pyo3::types::PyDict;
use std::sync::Arc;

use crate::py_date_page::PyDatePage;
use seesea_cleaner::date_page::DatePage;
use seesea_cleaner::object_pool::DatePageObjectPool;
use seesea_cleaner::object_pool::DatePageObjectPoolStats;

/// Python包装的DatePage对象池统计信息
#[pyclass]
pub struct PyDatePageObjectPoolStats {
    stats: DatePageObjectPoolStats,
}

#[pymethods]
impl PyDatePageObjectPoolStats {
    /// 当前池大小
    #[getter]
    pub fn current_size(&self) -> usize {
        self.stats.current_size
    }

    /// 最大池大小
    #[getter]
    pub fn max_size(&self) -> usize {
        self.stats.max_size
    }

    /// 最小池大小
    #[getter]
    pub fn min_size(&self) -> usize {
        self.stats.min_size
    }

    /// 池命中率
    #[getter]
    pub fn hits(&self) -> usize {
        self.stats.hits
    }

    /// 池未命中率
    #[getter]
    pub fn misses(&self) -> usize {
        self.stats.misses
    }

    /// 池命中率百分比
    #[getter]
    pub fn hit_rate(&self) -> f64 {
        self.stats.hit_rate
    }

    /// 对象创建计数
    #[getter]
    pub fn created(&self) -> usize {
        self.stats.created
    }

    /// 对象回收计数
    #[getter]
    pub fn recycled(&self) -> usize {
        self.stats.recycled
    }

    /// 转换为Python字典
    pub fn to_dict(&self, py: Python) -> PyResult<Py<PyDict>> {
        let dict = PyDict::new(py);
        dict.set_item("current_size", self.current_size())?;
        dict.set_item("max_size", self.max_size())?;
        dict.set_item("min_size", self.min_size())?;
        dict.set_item("hits", self.hits())?;
        dict.set_item("misses", self.misses())?;
        dict.set_item("hit_rate", self.hit_rate())?;
        dict.set_item("created", self.created())?;
        dict.set_item("recycled", self.recycled())?;
        Ok(dict.into())
    }
}

/// Python包装的DatePage对象池
#[pyclass]
pub struct PyDatePageObjectPool {
    pool: Arc<DatePageObjectPool>,
}

#[pymethods]
impl PyDatePageObjectPool {
    /// 创建新的DatePage对象池
    #[new]
    pub fn new(min_size: Option<usize>, max_size: Option<usize>) -> Self {
        let min = min_size.unwrap_or(10);
        let max = max_size.unwrap_or(100);

        let pool = DatePageObjectPool::new(min, max);

        Self {
            pool: Arc::new(pool),
        }
    }

    /// 从对象池获取一个DatePage对象
    pub fn get(&self) -> PyResult<PyDatePage> {
        let date_page = self.pool.get();

        // 创建PyDatePage实例，根据是否存在Tokio运行时来决定是否创建新的运行时
        let runtime = match tokio::runtime::Handle::try_current() {
            // 如果已经存在运行时，不创建新的运行时
            Ok(_) => None,
            // 如果不存在运行时，创建新的运行时
            Err(_) => Some(tokio::runtime::Runtime::new().map_err(|e| {
                PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                    "Failed to create runtime: {e}"
                ))
            })?),
        };

        Ok(PyDatePage { date_page, runtime })
    }

    /// 将DatePage对象归还到对象池
    pub fn put(&self, py_date_page: &mut PyDatePage) -> PyResult<()> {
        // 创建一个新的空DatePage来替换原对象
        let empty_date_page = DatePage::new("", std::time::SystemTime::UNIX_EPOCH, "", "");

        // 交换日期页面，保留原对象的运行时
        let date_page = std::mem::replace(&mut py_date_page.date_page, empty_date_page);

        // 将提取的DatePage放入对象池
        self.pool.put(date_page);
        Ok(())
    }

    /// 设置对象池最大大小
    pub fn set_max_size(&self, max_size: usize) {
        self.pool.set_max_size(max_size);
    }

    /// 设置对象池最小大小
    pub fn set_min_size(&self, min_size: usize) {
        self.pool.set_min_size(min_size);
    }

    /// 获取当前池大小
    pub fn current_size(&self) -> usize {
        self.pool.current_size()
    }

    /// 获取最大池大小
    pub fn max_size(&self) -> usize {
        self.pool.max_size()
    }

    /// 获取最小池大小
    pub fn min_size(&self) -> usize {
        self.pool.min_size()
    }

    /// 获取池命中率
    pub fn hit_rate(&self) -> f64 {
        self.pool.hit_rate()
    }

    /// 获取池统计信息
    pub fn stats(&self) -> PyDatePageObjectPoolStats {
        PyDatePageObjectPoolStats {
            stats: self.pool.stats(),
        }
    }

    /// 清理对象池，移除所有对象
    pub fn clear(&self) {
        self.pool.clear();
    }

    /// 动态调整池大小
    pub fn resize(&self, desired_size: usize) {
        self.pool.resize(desired_size);
    }
}

/// 导出Python模块
#[pymodule]
pub fn py_object_pool(_py: Python, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyDatePageObjectPool>()?;
    m.add_class::<PyDatePageObjectPoolStats>()?;
    Ok(())
}
