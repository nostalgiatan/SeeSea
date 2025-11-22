// Copyright 2025 nostalgiatan
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Python bindings for search functionality

use pyo3::prelude::*;
use pyo3::types::PyDict;
use pyo3::IntoPyObjectExt;
use std::sync::Arc;

use crate::search::{SearchInterface, SearchConfig, SearchRequest};
use crate::search::engine_config::EngineMode;
use crate::derive::SearchQuery;

#[pyclass]
pub struct PySearchClient {
    runtime: tokio::runtime::Runtime,
    interface: Arc<SearchInterface>,
}

#[pymethods]
impl PySearchClient {
    /// 创建搜索客户端
    #[new]
    pub fn new() -> PyResult<Self> {
        let runtime = tokio::runtime::Runtime::new()
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                format!("Failed to create runtime: {}", e)
            ))?;

        let interface = runtime.block_on(async {
            SearchInterface::new(SearchConfig::default())
                .map_err(|e| format!("Failed to create search interface: {}", e))
        }).map_err(|e: String| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e))?;

        Ok(Self {
            runtime,
            interface: Arc::new(interface),
        })
    }
    
    pub fn search(
        &self,
        query: String,
        page: Option<usize>,
        page_size: Option<usize>,
        language: Option<String>,
        region: Option<String>,
        engines: Option<Vec<String>>,
        force: Option<bool>,
        cache_timeline: Option<u64>,
    ) -> PyResult<Py<PyAny>> {
        let search_query = SearchQuery {
            query,
            page: page.unwrap_or(1),
            page_size: page_size.unwrap_or(10),
            language,
            region,
            ..Default::default()
        };

        // 确定引擎列表和模式
        let (engines_to_use, mode) = if let Some(engines) = engines {
            // 用户指定了具体引擎，使用自定义模式
            (engines.clone(), EngineMode::Custom(engines))
        } else {
            // 默认全局模式
            (vec![], EngineMode::Global)
        };

        let request = SearchRequest {
            query: search_query,
            engines: engines_to_use,
            timeout: None,
            max_results: None,
            force: force.unwrap_or(false),
            cache_timeline,
        };

        let response = if let EngineMode::Custom(_) = mode {
            // 自定义引擎模式，使用常规搜索
            self.runtime.block_on(async {
                self.interface.search(&request).await
            })
        } else {
            // 全局模式，使用模式搜索
            self.runtime.block_on(async {
                self.interface.search_with_mode(&request, mode).await
            })
        }.map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
            format!("Search failed: {}", e)
        ))?;
        
        Python::attach(|py| {
            let dict = PyDict::new(py);
            dict.set_item("query", response.query.query)?;
            dict.set_item("total_count", response.total_count)?;
            dict.set_item("cached", response.cached)?;
            dict.set_item("query_time_ms", response.query_time_ms)?;
            dict.set_item("engines_used", response.engines_used)?;
            
            let results: Vec<Py<PyAny>> = response.results.iter().flat_map(|r| {
                r.items.iter().map(|item| {
                    let item_dict = PyDict::new(py);
                    let _ = item_dict.set_item("title", &item.title);
                    let _ = item_dict.set_item("url", &item.url);
                    let _ = item_dict.set_item("content", &item.content);
                    let _ = item_dict.set_item("score", item.score);
                    item_dict.into_py_any(py).unwrap_or_else(|_| py.None())
                }).collect::<Vec<_>>()
            }).collect();
            
            dict.set_item("results", results)?;
            Ok(dict.into_py_any(py)?)
        })
    }
    
    pub fn get_stats(&self) -> PyResult<Py<PyAny>> {
        let stats = self.runtime.block_on(async {
            self.interface.get_stats().await
        });

        Python::attach(|py| {
            let dict = PyDict::new(py);
            dict.set_item("total_searches", stats.total_searches)?;
            dict.set_item("cache_hits", stats.cache_hits)?;
            dict.set_item("cache_misses", stats.cache_misses)?;
            dict.set_item("engine_failures", stats.engine_failures)?;
            dict.set_item("timeouts", stats.timeouts)?;
            dict.into_py_any(py)
        })
    }
    
    /// 清除缓存
    pub fn clear_cache(&self) -> PyResult<()> {
        self.runtime.block_on(async {
            self.interface.clear_cache().await
        }).map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
            format!("Failed to clear cache: {}", e)
        ))
    }

    /// 列出可用的搜索引擎
    pub fn list_engines(&self) -> PyResult<Vec<String>> {
        Ok(self.interface.list_engines())
    }

    /// 列出全局模式引擎
    pub fn list_global_engines(&self) -> PyResult<Vec<String>> {
        Ok(self.interface.list_global_engines())
    }

    /// 健康检查所有引擎
    pub fn health_check(&self) -> PyResult<Py<PyAny>> {
        let results = self.runtime.block_on(async {
            self.interface.health_check().await
        }).map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
            format!("Health check failed: {}", e)
        ))?;

        Python::attach(|py| {
            let dict = PyDict::new(py);
            for (engine, status) in results {
                dict.set_item(engine, status)?;
            }
            dict.into_py_any(py)
        })
    }

    /// 流式搜索 - 每个引擎完成时立即返回结果
    /// 
    /// # Arguments
    /// 
    /// * `query` - 搜索查询
    /// * `callback` - Python回调函数，每个引擎完成时调用
    /// * `page` - 页码（可选）
    /// * `page_size` - 每页大小（可选）
    /// * `engines` - 指定引擎列表（可选）
    /// 
    /// # Returns
    /// 
    /// 返回最终聚合的搜索结果
    pub fn search_streaming(
        &self,
        py: Python,
        query: String,
        callback: Py<PyAny>,
        page: Option<usize>,
        page_size: Option<usize>,
        engines: Option<Vec<String>>,
    ) -> PyResult<Py<PyAny>> {
        let search_query = SearchQuery {
            query,
            page: page.unwrap_or(1),
            page_size: page_size.unwrap_or(10),
            ..Default::default()
        };

        let (engines_to_use, mode) = if let Some(engines) = engines {
            (engines.clone(), EngineMode::Custom(engines))
        } else {
            (vec![], EngineMode::Global)
        };

        let request = SearchRequest {
            query: search_query,
            engines: engines_to_use,
            timeout: None,
            max_results: None,
            force: false,
            cache_timeline: None,
        };

        // 创建回调包装器
        let py_callback = callback.clone_ref(py);
        
        let response = self.runtime.block_on(async move {
            self.interface.search_streaming(&request, move |result, engine_name| {
                // 在回调中调用Python函数
                Python::with_gil(|py| {
                    let result_dict = PyDict::new(py);
                    let _ = result_dict.set_item("engine", engine_name);
                    let _ = result_dict.set_item("total_results", result.total_results);
                    
                    let items: Vec<Py<PyAny>> = result.items.iter().map(|item| {
                        let item_dict = PyDict::new(py);
                        let _ = item_dict.set_item("title", &item.title);
                        let _ = item_dict.set_item("url", &item.url);
                        let _ = item_dict.set_item("content", &item.content);
                        let _ = item_dict.set_item("score", item.score);
                        item_dict.into_py_any(py).unwrap_or_else(|_| py.None())
                    }).collect();
                    
                    let _ = result_dict.set_item("items", items);
                    
                    // 调用Python回调
                    let _ = py_callback.call1(py, (result_dict,));
                });
            }).await
        }).map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
            format!("Streaming search failed: {}", e)
        ))?;
        
        Python::attach(|py| {
            let dict = PyDict::new(py);
            dict.set_item("query", response.query.query)?;
            dict.set_item("total_count", response.total_count)?;
            dict.set_item("cached", response.cached)?;
            dict.set_item("query_time_ms", response.query_time_ms)?;
            dict.set_item("engines_used", response.engines_used)?;
            
            let results: Vec<Py<PyAny>> = response.results.iter().flat_map(|r| {
                r.items.iter().map(|item| {
                    let item_dict = PyDict::new(py);
                    let _ = item_dict.set_item("title", &item.title);
                    let _ = item_dict.set_item("url", &item.url);
                    let _ = item_dict.set_item("content", &item.content);
                    let _ = item_dict.set_item("score", item.score);
                    item_dict.into_py_any(py).unwrap_or_else(|_| py.None())
                }).collect::<Vec<_>>()
            }).collect();
            
            dict.set_item("results", results)?;
            Ok(dict.into_py_any(py)?)
        })
    }

    /// 获取引擎状态信息
    pub fn get_engine_states(&self) -> PyResult<Py<PyAny>> {
        let states = self.runtime.block_on(async {
            self.interface.get_engine_states().await
        });

        Python::attach(|py| {
            let dict = PyDict::new(py);
            for (engine, state) in states {
                let state_dict = PyDict::new(py);
                state_dict.set_item("enabled", state.0)?;
                state_dict.set_item("temporarily_disabled", state.1)?;
                state_dict.set_item("consecutive_failures", state.2)?;
                dict.set_item(engine, state_dict)?;
            }
            dict.into_py_any(py)
        })
    }

    /// 获取缓存统计信息
    pub fn get_cache_info(&self) -> PyResult<Py<PyAny>> {
        let (cache_size, cached_engines) = self.runtime.block_on(async {
            self.interface.get_engine_cache_stats().await
        });

        Python::attach(|py| {
            let dict = PyDict::new(py);
            dict.set_item("cache_size", cache_size)?;
            dict.set_item("cached_engines", cached_engines)?;
            dict.into_py_any(py)
        })
    }

    /// 强制刷新特定引擎的缓存
    pub fn invalidate_engine(&self, engine_name: String) -> PyResult<()> {
        self.runtime.block_on(async {
            self.interface.invalidate_engine(&engine_name).await
        }).map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
            format!("Failed to invalidate engine: {}", e)
        ))
    }

    /// 全文搜索 - 搜索网络和数据库（包括历史结果）
    /// 
    /// # Arguments
    /// 
    /// * `query` - 搜索查询
    /// * `page` - 页码（可选）
    /// * `page_size` - 每页大小（可选）
    /// * `engines` - 指定引擎列表（可选）
    /// 
    /// # Returns
    /// 
    /// 返回网络和数据库的聚合搜索结果
    pub fn search_fulltext(
        &self,
        query: String,
        page: Option<usize>,
        page_size: Option<usize>,
        engines: Option<Vec<String>>,
    ) -> PyResult<Py<PyAny>> {
        let search_query = SearchQuery {
            query,
            page: page.unwrap_or(1),
            page_size: page_size.unwrap_or(10),
            ..Default::default()
        };

        let (engines_to_use, _mode) = if let Some(engines) = engines {
            (engines.clone(), EngineMode::Custom(engines))
        } else {
            (vec![], EngineMode::Global)
        };

        let request = SearchRequest {
            query: search_query,
            engines: engines_to_use,
            timeout: None,
            max_results: None,
            force: false,
            cache_timeline: None,
        };

        let response = self.runtime.block_on(async {
            self.interface.search_fulltext(&request).await
        }).map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
            format!("Fulltext search failed: {}", e)
        ))?;
        
        Python::attach(|py| {
            let dict = PyDict::new(py);
            dict.set_item("query", response.query.query)?;
            dict.set_item("total_count", response.total_count)?;
            dict.set_item("cached", response.cached)?;
            dict.set_item("query_time_ms", response.query_time_ms)?;
            dict.set_item("engines_used", response.engines_used)?;
            
            let results: Vec<Py<PyAny>> = response.results.iter().flat_map(|r| {
                r.items.iter().map(|item| {
                    let item_dict = PyDict::new(py);
                    let _ = item_dict.set_item("title", &item.title);
                    let _ = item_dict.set_item("url", &item.url);
                    let _ = item_dict.set_item("content", &item.content);
                    let _ = item_dict.set_item("score", item.score);
                    item_dict.into_py_any(py).unwrap_or_else(|_| py.None())
                }).collect::<Vec<_>>()
            }).collect();
            
            dict.set_item("results", results)?;
            Ok(dict.into_py_any(py)?)
        })
    }

    /// 获取隐私保护统计信息
    pub fn get_privacy_stats(&self) -> PyResult<Py<PyAny>> {
        let stats_opt = self.runtime.block_on(async {
            self.interface.get_privacy_stats().await
        });

        Python::attach(|py| {
            if let Some(stats) = stats_opt {
                let dict = PyDict::new(py);
                dict.set_item("privacy_level", format!("{}", stats.privacy_level))?;
                dict.set_item("fake_headers_enabled", stats.fake_headers_enabled)?;
                dict.set_item("fingerprint_protection", format!("{:?}", stats.fingerprint_protection))?;
                dict.set_item("doh_enabled", stats.doh_enabled)?;
                dict.set_item("user_agent_strategy", format!("{:?}", stats.user_agent_strategy))?;
                dict.into_py_any(py)
            } else {
                Ok(py.None())
            }
        })
    }
}
