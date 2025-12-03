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

//! Python bindings for cache

use pyo3::IntoPyObjectExt;
use pyo3::prelude::*;
use pyo3::types::PyDict;
use std::sync::Arc;

use crate::cache::{CacheImplConfig, CacheInterface, CacheMode};

#[pyclass]
#[derive(Clone)]
pub struct PyCacheStats {
    #[pyo3(get)]
    pub hits: u64,
    #[pyo3(get)]
    pub misses: u64,
    #[pyo3(get)]
    pub size: usize,
}

#[pymethods]
impl PyCacheStats {
    #[new]
    pub fn new() -> Self {
        Self {
            hits: 0,
            misses: 0,
            size: 0,
        }
    }

    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total > 0 {
            self.hits as f64 / total as f64
        } else {
            0.0
        }
    }
}

/// Python cache interface
#[pyclass]
pub struct PyCacheInterface {
    cache: Arc<CacheInterface>,
}

#[pymethods]
impl PyCacheInterface {
    #[new]
    pub fn new(
        db_path: Option<String>,
        ttl_secs: Option<u64>,
        max_size_mb: Option<u64>,
    ) -> PyResult<Self> {
        let config = CacheImplConfig {
            db_path: db_path.unwrap_or_else(|| ".seesea/cache.db".to_string()),
            default_ttl_secs: ttl_secs.unwrap_or(3600),
            max_size_bytes: max_size_mb.unwrap_or(1024) * 1024 * 1024,
            enabled: true,
            compression: false,
            mode: CacheMode::HighThroughput,
            enable_bloom_filter: true,
            bloom_filter_expected_elements: 1_000_000,
            bloom_filter_false_positive_rate: 0.01,
        };

        let cache = CacheInterface::new(config).map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                "Failed to create cache: {}",
                e
            ))
        })?;

        Ok(Self {
            cache: Arc::new(cache),
        })
    }

    /// 获取缓存统计信息
    pub fn get_stats(&self) -> PyResult<Py<PyAny>> {
        let stats = self.cache.manager().stats();

        Python::attach(|py| {
            let dict = PyDict::new(py);
            dict.set_item("hits", stats.hits)?;
            dict.set_item("misses", stats.misses)?;
            dict.set_item("writes", stats.writes)?;
            dict.set_item("deletes", stats.deletes)?;
            dict.set_item("total_keys", stats.total_keys)?;
            dict.set_item("estimated_size_bytes", stats.estimated_size_bytes)?;
            dict.set_item("evictions", stats.evictions)?;
            dict.set_item("hit_rate", stats.hit_rate())?;
            dict.into_py_any(py)
        })
    }

    /// 清空所有缓存
    pub fn clear_all(&self) -> PyResult<()> {
        self.cache.clear_all().map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                "Failed to clear cache: {}",
                e
            ))
        })
    }

    /// 刷新缓存到磁盘
    pub fn flush(&self) -> PyResult<()> {
        self.cache.flush().map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                "Failed to flush cache: {}",
                e
            ))
        })
    }

    /// 清理过期条目
    pub fn cleanup(&self) -> PyResult<usize> {
        self.cache.cleanup().map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                "Failed to cleanup cache: {}",
                e
            ))
        })
    }
}
