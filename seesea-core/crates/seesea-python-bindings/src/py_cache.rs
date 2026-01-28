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

use once_cell::sync::Lazy;
use pyo3::IntoPyObjectExt;
use pyo3::prelude::*;
use pyo3::types::PyDict;
use std::path::PathBuf;
use std::sync::Arc;

use seesea_cache::CacheInterface;
use seesea_cache::cache::scope::ScopeCache;
use seesea_cache::cache::types::CacheImplConfig;

/// 获取操作系统特定的缓存目录
fn get_cache_dir() -> PathBuf {
    let cache_dir = seesea_config::paths::get_cache_dir();
    let base_dir = PathBuf::from(&cache_dir);

    if let Err(e) = std::fs::create_dir_all(&base_dir) {
        tracing::error!("Failed to create cache directory {:?}: {}", base_dir, e);
        panic!("Failed to create cache directory: {}", e);
    }
    base_dir
}

/// 全局缓存实例，确保复用
pub static GLOBAL_CACHE_INSTANCE: Lazy<Arc<CacheInterface>> = Lazy::new(|| {
    let cache_dir = get_cache_dir();

    let cache_config = CacheImplConfig {
        db_path: cache_dir.to_string_lossy().to_string(),
        ..Default::default()
    };

    match CacheInterface::new(cache_config) {
        Ok(cache) => Arc::new(cache),
        Err(e) => {
            tracing::error!("Failed to create global cache instance: {}", e);
            panic!("Failed to create global cache instance: {}", e);
        }
    }
});

#[pyclass]
#[derive(Clone, Default)]
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
        Self::default()
    }

    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total > 0 {
            self.hits as f64 / total as f64
        } else {
            0.0
        }
    }

    pub fn miss_rate(&self) -> f64 {
        1.0 - self.hit_rate()
    }
}

/// Python scope cache wrapper
#[pyclass]
pub struct PyScopeCache {
    scope_cache: ScopeCache,
}

#[pymethods]
impl PyScopeCache {
    /// 获取缓存值
    ///
    /// # 参数
    ///
    /// * `key` - 缓存键
    ///
    /// # 返回值
    ///
    /// 返回缓存值的字节数据，如果不存在或已过期则返回 None
    pub fn get(&self, key: &str) -> PyResult<Option<Vec<u8>>> {
        self.scope_cache.get(key).map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("Failed to get cache: {}", e))
        })
    }

    /// 设置缓存值
    ///
    /// # 参数
    ///
    /// * `key` - 缓存键
    /// * `value` - 缓存值（字节数据）
    /// * `ttl_seconds` - TTL（秒），None 表示使用默认值
    pub fn set(&self, key: String, value: Vec<u8>, ttl_seconds: Option<u64>) -> PyResult<()> {
        let ttl = ttl_seconds.map(std::time::Duration::from_secs);
        self.scope_cache.set(key, value, ttl).map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("Failed to set cache: {}", e))
        })
    }

    /// 检查键是否存在
    pub fn exists(&self, key: &str) -> PyResult<bool> {
        match self
            .scope_cache
            .manager()
            .get(self.scope_cache.scope(), key)
        {
            Ok(Some(_)) => Ok(true),
            Ok(None) => Ok(false),
            Err(e) => Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                "Failed to check cache existence: {}",
                e
            ))),
        }
    }

    /// 删除缓存键
    pub fn delete(&self, key: &str) -> PyResult<bool> {
        self.scope_cache.delete(key).map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                "Failed to delete cache: {}",
                e
            ))
        })
    }

    /// 获取作用域名称
    #[getter]
    pub fn scope(&self) -> String {
        self.scope_cache.scope().to_string()
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
    pub fn new() -> PyResult<Self> {
        // 使用全局缓存实例，确保复用
        Ok(Self {
            cache: GLOBAL_CACHE_INSTANCE.clone(),
        })
    }

    /// 获取指定作用域的缓存访问器
    pub fn scope(&self, scope_name: &str) -> PyScopeCache {
        PyScopeCache {
            scope_cache: self.cache.scope(scope_name),
        }
    }

    /// 获取缓存统计信息
    pub fn get_stats(&self) -> PyResult<Py<PyAny>> {
        let manager = self.cache.manager().map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                "Failed to get cache manager: {}",
                e
            ))
        })?;
        let stats = manager.stats();

        Python::attach(|py| {
            let dict = PyDict::new(py);
            dict.set_item("hits", stats.total_hits)?;
            dict.set_item("misses", stats.total_misses)?;
            dict.set_item("writes", stats.total_inserts)?;
            dict.set_item("deletes", stats.total_deletes)?;
            dict.set_item("total_keys", stats.current_size)?;
            dict.set_item("estimated_size_bytes", 0)?; // Placeholder since this field doesn't exist
            dict.set_item("evictions", stats.total_evictions)?;
            dict.set_item(
                "hit_rate",
                if stats.total_hits + stats.total_misses > 0 {
                    stats.total_hits as f64 / (stats.total_hits + stats.total_misses) as f64
                } else {
                    0.0
                },
            )?;
            dict.into_py_any(py)
        })
    }

    /// 清空所有缓存
    pub fn clear_all(&self) -> PyResult<()> {
        let manager = self.cache.manager().map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                "Failed to get cache manager: {}",
                e
            ))
        })?;
        manager.clear().map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                "Failed to clear cache: {}",
                e
            ))
        })
    }

    /// 刷新缓存到磁盘
    pub fn flush(&self) -> PyResult<()> {
        let manager = self.cache.manager().map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                "Failed to get cache manager: {}",
                e
            ))
        })?;
        manager.flush().map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                "Failed to flush cache: {}",
                e
            ))
        })
    }

    /// 清理过期条目
    pub fn cleanup(&self) -> PyResult<usize> {
        let manager = self.cache.manager().map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                "Failed to get cache manager: {}",
                e
            ))
        })?;
        manager.cleanup_expired_default().map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                "Failed to cleanup cache: {}",
                e
            ))
        })
    }
}
