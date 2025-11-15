//! Python bindings for search functionality

use pyo3::prelude::*;
use pyo3::types::PyDict;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::search::{SearchInterface, SearchConfig, SearchRequest};
use crate::derive::SearchQuery;
use crate::net::{NetworkInterface, types::NetworkConfig};
use crate::cache::{CacheInterface, types::CacheImplConfig};

#[pyclass]
pub struct PySearchClient {
    runtime: tokio::runtime::Runtime,
    interface: Arc<SearchInterface>,
}

#[pymethods]
impl PySearchClient {
    #[new]
    pub fn new() -> PyResult<Self> {
        let runtime = tokio::runtime::Runtime::new()
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                format!("Failed to create runtime: {}", e)
            ))?;
        
        let interface = runtime.block_on(async {
            let mut network_config = NetworkConfig::default();
            network_config.pool.max_idle_connections = 200;
            network_config.pool.max_connections_per_host = 20;
            
            let network = Arc::new(NetworkInterface::new(network_config)
                .map_err(|e| format!("Failed to create network: {}", e))?);
            let cache = Arc::new(RwLock::new(CacheInterface::new(CacheImplConfig::default())
                .map_err(|e| format!("Failed to create cache: {}", e))?));
            
            SearchInterface::new(SearchConfig::default(), network, cache)
                .map_err(|e| format!("Failed to create search interface: {}", e))
        }).map_err(|e: String| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e))?;
        
        Ok(Self {
            runtime,
            interface: Arc::new(interface),
        })
    }
    
    pub fn search(&self, query: String, page: Option<usize>, page_size: Option<usize>) -> PyResult<PyObject> {
        let search_query = SearchQuery {
            query,
            page: page.unwrap_or(1),
            page_size: page_size.unwrap_or(10),
            ..Default::default()
        };
        
        let request = SearchRequest {
            query: search_query,
            engines: Vec::new(),
            timeout: None,
            max_results: None,
        };
        
        let response = self.runtime.block_on(async {
            self.interface.search(&request).await
        }).map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
            format!("Search failed: {}", e)
        ))?;
        
        Python::with_gil(|py| {
            let dict = PyDict::new(py);
            dict.set_item("query", response.query.query)?;
            dict.set_item("total_count", response.total_count)?;
            dict.set_item("cached", response.cached)?;
            dict.set_item("query_time_ms", response.query_time_ms)?;
            dict.set_item("engines_used", response.engines_used)?;
            
            let results: Vec<_> = response.results.iter().flat_map(|r| {
                r.items.iter().map(|item| {
                    let item_dict = PyDict::new(py);
                    let _ = item_dict.set_item("title", &item.title);
                    let _ = item_dict.set_item("url", &item.url);
                    let _ = item_dict.set_item("content", &item.content);
                    let _ = item_dict.set_item("score", item.score);
                    item_dict.into()
                }).collect::<Vec<_>>()
            }).collect();
            
            dict.set_item("results", results)?;
            Ok(dict.into())
        })
    }
    
    pub fn get_stats(&self) -> PyResult<PyObject> {
        let stats = self.interface.get_stats();
        
        Python::with_gil(|py| {
            let dict = PyDict::new(py);
            dict.set_item("total_searches", stats.total_searches)?;
            dict.set_item("cache_hits", stats.cache_hits)?;
            dict.set_item("cache_misses", stats.cache_misses)?;
            dict.set_item("engine_failures", stats.engine_failures)?;
            dict.set_item("timeouts", stats.timeouts)?;
            Ok(dict.into())
        })
    }
}
