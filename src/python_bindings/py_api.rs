//! Python bindings for API server

use pyo3::prelude::*;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::api::ApiInterface;
use crate::search::SearchConfig;
use crate::net::{NetworkInterface, types::NetworkConfig};
use crate::cache::{CacheInterface, types::CacheImplConfig};

#[pyclass]
pub struct PyApiServer {
    runtime: tokio::runtime::Runtime,
    api: Arc<ApiInterface>,
    address: String,
}

#[pymethods]
impl PyApiServer {
    #[new]
    pub fn new(host: Option<String>, port: Option<u16>) -> PyResult<Self> {
        let runtime = tokio::runtime::Runtime::new()
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                format!("Failed to create runtime: {}", e)
            ))?;
        
        let api = runtime.block_on(async {
            let mut network_config = NetworkConfig::default();
            network_config.pool.max_idle_connections = 200;
            
            let network = Arc::new(NetworkInterface::new(network_config)
                .map_err(|e| format!("Network error: {}", e))?);
            let cache = Arc::new(RwLock::new(CacheInterface::new(CacheImplConfig::default())
                .map_err(|e| format!("Cache error: {}", e))?));
            
            ApiInterface::from_config(SearchConfig::default(), network, cache)
                .map_err(|e| format!("API error: {}", e))
        }).map_err(|e: String| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e))?;
        
        let address = format!("{}:{}", 
            host.unwrap_or_else(|| "127.0.0.1".to_string()),
            port.unwrap_or(8080)
        );
        
        Ok(Self {
            runtime,
            api: Arc::new(api),
            address,
        })
    }
    
    pub fn start(&self) -> PyResult<()> {
        let app = self.api.build_router();
        let addr = self.address.clone();
        
        println!("🌊 SeeSea API Server starting on http://{}", addr);
        
        self.runtime.block_on(async {
            let listener = tokio::net::TcpListener::bind(&addr).await
                .map_err(|e| format!("Failed to bind: {}", e))?;
            axum::serve(listener, app).await
                .map_err(|e| format!("Server error: {}", e))
        }).map_err(|e: String| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e))
    }
    
    pub fn get_address(&self) -> String {
        self.address.clone()
    }
}
