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

//! Python bindings for API server

use pyo3::prelude::*;
use std::sync::Arc;

use crate::api::ApiInterface;
use crate::search::SearchConfig;
use crate::api::network::{NetworkConfig as ApiNetworkConfig, NetworkMode};

/// Python bindings for API server
///
/// Provides a complete web server with search, RSS, cache management,
/// health checks, metrics, and more.
#[pyclass]
pub struct PyApiServer {
    runtime: tokio::runtime::Runtime,
    api: Arc<ApiInterface>,
    address: String,
    network_mode: String,
}

#[pymethods]
impl PyApiServer {
    /// Create a new API server
    ///
    /// # Arguments
    ///
    /// * `host` - Server host address (default: "127.0.0.1")
    /// * `port` - Server port (default: 8080)
    /// * `network_mode` - Network mode: "internal", "external", or "dual" (default: "internal")
    ///
    /// # Returns
    ///
    /// PyApiServer instance
    #[new]
    #[pyo3(signature = (host=None, port=None, network_mode=None))]
    pub fn new(
        host: Option<String>, 
        port: Option<u16>,
        network_mode: Option<String>,
    ) -> PyResult<Self> {
        let runtime = tokio::runtime::Runtime::new()
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                format!("Failed to create runtime: {}", e)
            ))?;
        
        let mode = network_mode.unwrap_or_else(|| "internal".to_string());
        let network_mode_enum = match mode.as_str() {
            "internal" => NetworkMode::Internal,
            "external" => NetworkMode::External,
            "dual" => NetworkMode::Dual,
            _ => {
                return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                    "network_mode must be 'internal', 'external', or 'dual'"
                ));
            }
        };
        
        let api = runtime.block_on(async {
            // Create API interface with network configuration
            // Note: network and cache are created by the ApiInterface internally
            let search_config = SearchConfig::default();
            let search = Arc::new(crate::search::SearchInterface::new(search_config)
                .map_err(|e| format!("Search error: {}", e))?);
            
            let mut api_network_config = ApiNetworkConfig::default();
            api_network_config.mode = network_mode_enum;
            
            Ok::<_, String>(ApiInterface::with_network_config(
                search,
                env!("CARGO_PKG_VERSION").to_string(),
                api_network_config,
            ))
        }).map_err(|e: String| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e))?;
        
        let address = format!("{}:{}", 
            host.unwrap_or_else(|| "127.0.0.1".to_string()),
            port.unwrap_or(8080)
        );
        
        Ok(Self {
            runtime,
            api: Arc::new(api),
            address,
            network_mode: mode,
        })
    }
    
    /// Start the API server (blocking)
    ///
    /// Starts the web server and blocks until shutdown.
    /// Routes available depend on network mode:
    ///
    /// Internal mode (full access):
    /// - GET/POST /api/search - Search
    /// - GET /api/engines - List search engines
    /// - GET /api/stats - Statistics
    /// - GET /api/health - Health check
    /// - GET /api/version - Version info
    /// - GET /api/metrics - Prometheus metrics
    /// - GET /api/metrics/realtime - Real-time JSON metrics
    /// - GET /api/rss/feeds - List RSS feeds
    /// - POST /api/rss/fetch - Fetch RSS feed
    /// - GET /api/cache/stats - Cache statistics
    /// - POST /api/cache/clear - Clear cache
    /// - POST /api/magic-link/generate - Generate magic link
    ///
    /// External mode (security enabled):
    /// - Same routes but with rate limiting, circuit breaker, IP filtering, JWT auth
    ///
    /// # Returns
    ///
    /// None on success, raises exception on error
    pub fn start(&self) -> PyResult<()> {
        let app = self.api.build_router();
        let addr = self.address.clone();
        
        println!("🌊 Starting SeeSea API Server");
        println!("   Address: {}", addr);
        println!("   Mode: {}", self.network_mode);
        println!("   Version: {}", env!("CARGO_PKG_VERSION"));
        println!();
        
        self.runtime.block_on(async {
            let listener = tokio::net::TcpListener::bind(&addr).await
                .map_err(|e| format!("Failed to bind: {}", e))?;
            axum::serve(listener, app).await
                .map_err(|e| format!("Server error: {}", e))
        }).map_err(|e: String| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e))
    }
    
    /// Start the API server in internal mode (blocking)
    ///
    /// Same as start() but explicitly uses internal router (no security).
    pub fn start_internal(&self) -> PyResult<()> {
        let app = self.api.build_internal_router();
        let addr = self.address.clone();
        
        println!("🔒 Starting SeeSea API Server (Internal Mode)");
        println!("   Address: {}", addr);
        println!("   Security: Disabled (local access only)");
        println!();
        
        self.runtime.block_on(async {
            let listener = tokio::net::TcpListener::bind(&addr).await
                .map_err(|e| format!("Failed to bind: {}", e))?;
            axum::serve(listener, app).await
                .map_err(|e| format!("Server error: {}", e))
        }).map_err(|e: String| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e))
    }
    
    /// Start the API server in external mode (blocking)
    ///
    /// Same as start() but explicitly uses external router with security enabled.
    pub fn start_external(&self) -> PyResult<()> {
        let app = self.api.build_external_router();
        let addr = self.address.clone();
        
        println!("🌐 Starting SeeSea API Server (External Mode)");
        println!("   Address: {}", addr);
        println!("   Security: Enabled");
        println!();
        
        self.runtime.block_on(async {
            let listener = tokio::net::TcpListener::bind(&addr).await
                .map_err(|e| format!("Failed to bind: {}", e))?;
            axum::serve(listener, app).await
                .map_err(|e| format!("Server error: {}", e))
        }).map_err(|e: String| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e))
    }
    
    /// Get the server address
    ///
    /// # Returns
    ///
    /// String with host:port
    pub fn get_address(&self) -> String {
        self.address.clone()
    }
    
    /// Get the server network mode
    ///
    /// # Returns
    ///
    /// String: "internal", "external", or "dual"
    pub fn get_network_mode(&self) -> String {
        self.network_mode.clone()
    }
    
    /// Get the server URL
    ///
    /// # Returns
    ///
    /// String with full HTTP URL
    pub fn get_url(&self) -> String {
        format!("http://{}", self.address)
    }
    
    /// Get API endpoints available in current mode
    ///
    /// # Returns
    ///
    /// Dict with endpoint categories and their paths
    pub fn get_endpoints(&self) -> PyResult<Vec<(String, Vec<String>)>> {
        let mut endpoints = vec![
            ("search".to_string(), vec![
                "GET/POST /api/search".to_string(),
                "GET /api/engines".to_string(),
            ]),
            ("health".to_string(), vec![
                "GET /api/health".to_string(),
                "GET /health".to_string(),
                "GET /api/version".to_string(),
            ]),
            ("metrics".to_string(), vec![
                "GET /api/stats".to_string(),
                "GET /api/metrics".to_string(),
                "GET /api/metrics/realtime".to_string(),
            ]),
        ];
        
        if self.network_mode == "internal" || self.network_mode == "dual" {
            endpoints.push(("rss".to_string(), vec![
                "GET /api/rss/feeds".to_string(),
                "POST /api/rss/fetch".to_string(),
                "GET /api/rss/templates".to_string(),
                "POST /api/rss/template/add".to_string(),
            ]));
            endpoints.push(("cache".to_string(), vec![
                "GET /api/cache/stats".to_string(),
                "POST /api/cache/clear".to_string(),
                "POST /api/cache/cleanup".to_string(),
            ]));
            endpoints.push(("admin".to_string(), vec![
                "POST /api/magic-link/generate".to_string(),
            ]));
        }
        
        Ok(endpoints)
    }
}
