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

//! Python bindings for API server

use pyo3::prelude::*;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::signal;

use seesea_api::ApiInterface;
use seesea_api::api::network::{
    ExternalNetworkConfig, InternalNetworkConfig, NetworkConfig as ApiNetworkConfig, NetworkMode,
};
use seesea_cache::CacheInterface;
use seesea_cache::cache::types::CacheImplConfig;

use seesea_config::ConfigManager;
use seesea_search::search::SearchInterface;
use std::env;

/// Python bindings for API server
///
/// Provides a complete web server with search, RSS, cache management,
/// health checks, metrics, and more.
#[pyclass]
pub struct PyApiServer {
    runtime: Option<tokio::runtime::Runtime>,
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
    /// * `config_file` - Path to configuration file (optional)
    ///
    /// # Returns
    ///
    /// PyApiServer instance
    #[new]
    #[pyo3(signature = (host=None, port=None, network_mode=None, config_file=None))]
    pub fn new(
        host: Option<String>,
        port: Option<u16>,
        network_mode: Option<String>,
        config_file: Option<String>,
    ) -> PyResult<Self> {
        // 检查当前是否已经存在Tokio运行时
        let runtime = match tokio::runtime::Handle::try_current() {
            // 如果已经存在运行时，不创建新的运行时
            Ok(_) => None,
            // 如果不存在运行时，创建新的运行时
            Err(_) => Some(tokio::runtime::Runtime::new().map_err(|e| {
                PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                    "Failed to create runtime: {}",
                    e
                ))
            })?),
        };

        let mode = network_mode.unwrap_or_else(|| "internal".to_string());
        let network_mode_enum = match mode.as_str() {
            "internal" => NetworkMode::Internal,
            "external" => NetworkMode::External,
            "dual" => NetworkMode::Dual,
            _ => {
                return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                    "network_mode must be 'internal', 'external', or 'dual'",
                ));
            }
        };

        // 根据runtime是否存在来执行异步操作
        let (api, actual_host, actual_port) = match runtime.as_ref() {
            Some(runtime) => {
                runtime.block_on(async {
                    // Load configuration if provided
                    let config_path = config_file.map(std::path::PathBuf::from);
                    let config_manager =
                        ConfigManager::with_environment(config_path, "development")
                            .await
                            .map_err(|e| format!("Failed to load config: {}", e))?;
                    let config = config_manager.get_config().await;

                    // Create API interface with network configuration
                    // Note: network and cache are created by the ApiInterface internally
                    let search_config = config.search.clone();
                    let network_config = config.network.clone();
                    let search = Arc::new(
                        SearchInterface::new_with_network_config(
                            search_config.into(),
                            Some(network_config),
                        )
                        .map_err(|e| format!("Search error: {}", e))?,
                    );
                    // 使用结构体字面量初始化，包含所有字段，避免Clippy警告
                    let api_network_config = ApiNetworkConfig {
                        mode: network_mode_enum,
                        internal: InternalNetworkConfig {
                            port: config.server.port,
                            ..Default::default()
                        },
                        external: ExternalNetworkConfig {
                            port: config.server.port + 1,
                            host: config.server.bind_address.clone(),
                            enable_jwt_auth: config.api.auth.enabled,
                            enable_magic_link: config.api.auth.enabled,
                            auth_type: format!("{:?}", config.api.auth.auth_type),
                            api_key: config.api.auth.api_key.query_param.clone(),
                            key_source: "query".to_string(),
                            key_name: config.api.auth.api_key.query_param.clone(),
                            cors_origins: config.api.cors.allowed_origins.clone(),
                            enable_rate_limit: config.api.rate_limit.enabled,
                            rate_limit_per_second: config.api.rate_limit.requests_per_second,
                            rate_limit_burst_size: config.api.rate_limit.burst_size,
                            ..Default::default()
                        },
                    };

                    // 创建缓存接口
                    let cache_config = config.cache.clone();
                    let cache = Arc::new(
                        CacheInterface::new(CacheImplConfig::from(cache_config))
                            .map_err(|e| format!("Cache error: {}", e))?,
                    );

                    let api = ApiInterface::with_full_config(
                        search,
                        env!("CARGO_PKG_VERSION").to_string(),
                        api_network_config,
                        String::new(),
                        cache,
                    );

                    // Calculate actual host and port to use for binding
                    let actual_host = host.unwrap_or_else(|| config.server.bind_address.clone());
                    let actual_port = port.unwrap_or(config.server.port);

                    Ok::<_, String>((api, actual_host, actual_port))
                })
            }
            None => {
                tokio::runtime::Handle::current().block_on(async {
                    // Load configuration if provided
                    let config_path = config_file.map(std::path::PathBuf::from);
                    let config_manager =
                        ConfigManager::with_environment(config_path, "development")
                            .await
                            .map_err(|e| format!("Failed to load config: {}", e))?;
                    let config = config_manager.get_config().await;

                    // Create API interface with network configuration
                    // Note: network and cache are created by the ApiInterface internally
                    let search_config = config.search.clone();
                    let network_config = config.network.clone();
                    let search = Arc::new(
                        SearchInterface::new_with_network_config(
                            search_config.into(),
                            Some(network_config),
                        )
                        .map_err(|e| format!("Search error: {}", e))?,
                    );

                    // 使用结构体字面量初始化，包含所有字段，避免Clippy警告
                    let api_network_config = ApiNetworkConfig {
                        mode: network_mode_enum,
                        internal: InternalNetworkConfig {
                            port: config.server.port,
                            ..Default::default()
                        },
                        external: ExternalNetworkConfig {
                            port: config.server.port + 1,
                            host: config.server.bind_address.clone(),
                            enable_jwt_auth: config.api.auth.enabled,
                            enable_magic_link: config.api.auth.enabled,
                            auth_type: format!("{:?}", config.api.auth.auth_type),
                            api_key: config.api.auth.api_key.query_param.clone(),
                            key_source: "query".to_string(),
                            key_name: config.api.auth.api_key.query_param.clone(),
                            cors_origins: config.api.cors.allowed_origins.clone(),
                            enable_rate_limit: config.api.rate_limit.enabled,
                            rate_limit_per_second: config.api.rate_limit.requests_per_second,
                            rate_limit_burst_size: config.api.rate_limit.burst_size,
                            ..Default::default()
                        },
                    };

                    // 创建缓存接口
                    let cache_config = config.cache.clone();
                    let cache = Arc::new(
                        CacheInterface::new(CacheImplConfig::from(cache_config))
                            .map_err(|e| format!("Cache error: {}", e))?,
                    );

                    let api = ApiInterface::with_full_config(
                        search,
                        env!("CARGO_PKG_VERSION").to_string(),
                        api_network_config,
                        String::new(),
                        cache,
                    );

                    // Calculate actual host and port to use for binding
                    let actual_host = host.unwrap_or_else(|| config.server.bind_address.clone());
                    let actual_port = port.unwrap_or(config.server.port);

                    Ok::<_, String>((api, actual_host, actual_port))
                })
            }
        }
        .map_err(|e: String| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e))?;

        let address = format!("{}:{}", actual_host, actual_port);

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
    /// Press Ctrl+C to gracefully stop the server.
    ///
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
        let network_mode = self.network_mode.clone();
        let version = env!("CARGO_PKG_VERSION").to_string();

        println!("🌊 Starting SeeSea API Server");
        println!("   Address: {}", addr);
        println!("   Mode: {}", network_mode);
        println!("   Version: {}", version);
        println!("   Press Ctrl+C to stop");

        // 定义异步任务
        let async_task = async move {
            let listener = TcpListener::bind(&addr)
                .await
                .map_err(|e| format!("Failed to bind: {}", e))?;

            let server = axum::serve(listener, app);
            server
                .with_graceful_shutdown(shutdown_signal())
                .await
                .map_err(|e| format!("Server error: {}", e))
        };

        // 根据runtime是否存在来决定如何执行异步任务
        match self.runtime.as_ref() {
            // 如果runtime存在，使用它的block_on方法
            Some(runtime) => runtime.block_on(async_task),
            // 如果runtime不存在，使用当前运行时的block_on方法
            None => tokio::runtime::Handle::current().block_on(async_task),
        }
        .map_err(|e: String| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e))
    }

    /// Start the API server in internal mode (blocking)
    ///
    /// Same as start() but explicitly uses internal router (no security).
    /// Press Ctrl+C to gracefully stop the server.
    pub fn start_internal(&self) -> PyResult<()> {
        let app = self.api.build_internal_router();
        let addr = self.address.clone();

        println!("🔒 Starting SeeSea API Server (Internal Mode)");
        println!("   Address: {}", addr);
        println!("   Security: Disabled (local access only)");
        println!("   Press Ctrl+C to stop");

        // 根据runtime是否存在来执行异步任务
        match self.runtime.as_ref() {
            Some(runtime) => runtime.block_on(async {
                let listener = TcpListener::bind(&addr)
                    .await
                    .map_err(|e| format!("Failed to bind: {}", e))?;
                let server = axum::serve(listener, app);
                server
                    .with_graceful_shutdown(shutdown_signal())
                    .await
                    .map_err(|e| format!("Server error: {}", e))
            }),
            None => tokio::runtime::Handle::current().block_on(async {
                let listener = TcpListener::bind(&addr)
                    .await
                    .map_err(|e| format!("Failed to bind: {}", e))?;
                let server = axum::serve(listener, app);
                server
                    .with_graceful_shutdown(shutdown_signal())
                    .await
                    .map_err(|e| format!("Server error: {}", e))
            }),
        }
        .map_err(|e: String| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e))
    }

    /// Start the API server in external mode (blocking)
    ///
    /// Same as start() but explicitly uses external router with security enabled.
    /// Press Ctrl+C to gracefully stop the server.
    pub fn start_external(&self) -> PyResult<()> {
        let app = self.api.build_external_router();
        let addr = self.address.clone();

        println!("🌐 Starting SeeSea API Server (External Mode)");
        println!("   Address: {}", addr);
        println!("   Security: Enabled");
        println!("   Press Ctrl+C to stop");

        // 根据runtime是否存在来执行异步任务
        match self.runtime.as_ref() {
            Some(runtime) => runtime.block_on(async {
                let listener = TcpListener::bind(&addr)
                    .await
                    .map_err(|e| format!("Failed to bind: {}", e))?;
                let server = axum::serve(listener, app);
                server
                    .with_graceful_shutdown(shutdown_signal())
                    .await
                    .map_err(|e| format!("Server error: {}", e))
            }),
            None => tokio::runtime::Handle::current().block_on(async {
                let listener = TcpListener::bind(&addr)
                    .await
                    .map_err(|e| format!("Failed to bind: {}", e))?;
                let server = axum::serve(listener, app);
                server
                    .with_graceful_shutdown(shutdown_signal())
                    .await
                    .map_err(|e| format!("Server error: {}", e))
            }),
        }
        .map_err(|e: String| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e))
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
            (
                "search".to_string(),
                vec![
                    "GET/POST /api/search".to_string(),
                    "GET /api/engines".to_string(),
                ],
            ),
            (
                "health".to_string(),
                vec![
                    "GET /api/health".to_string(),
                    "GET /health".to_string(),
                    "GET /api/version".to_string(),
                ],
            ),
            (
                "metrics".to_string(),
                vec![
                    "GET /api/stats".to_string(),
                    "GET /api/metrics".to_string(),
                    "GET /api/metrics/realtime".to_string(),
                ],
            ),
            ("pro".to_string(), vec!["ANY /api/pro/*".to_string()]),
        ];

        if self.network_mode == "internal" || self.network_mode == "dual" {
            endpoints.push((
                "rss".to_string(),
                vec![
                    "GET /api/rss/feeds".to_string(),
                    "POST /api/rss/fetch".to_string(),
                    "GET /api/rss/templates".to_string(),
                    "POST /api/rss/template/add".to_string(),
                ],
            ));
            endpoints.push((
                "cache".to_string(),
                vec![
                    "GET /api/cache/stats".to_string(),
                    "POST /api/cache/clear".to_string(),
                    "POST /api/cache/cleanup".to_string(),
                ],
            ));
            endpoints.push((
                "admin".to_string(),
                vec!["POST /api/magic-link/generate".to_string()],
            ));
        }

        Ok(endpoints)
    }

    /// 添加Pro API路由
    ///
    /// # Arguments
    ///
    /// * `path` - 路由路径（如 "/process-url"，自动添加 "/api/pro/" 前缀）
    /// * `callback` - Python回调函数，接收请求上下文并返回响应字典
    /// * `method` - HTTP方法（默认: "POST"）
    ///
    /// # Returns
    ///
    /// None
    #[pyo3(signature = (path, callback, method=None))]
    pub fn add_pro_route(
        &self,
        path: String,
        callback: Py<PyAny>,
        method: Option<&str>,
    ) -> PyResult<()> {
        // 构建完整路径（去掉/api/pro/前缀，因为Axum已经处理了）
        let full_path = format!("/{}", path.trim_start_matches('/'));

        // 获取HTTP方法，默认POST
        let method = method.unwrap_or("POST").to_uppercase();

        // 打印调试信息
        println!("Adding Pro API route: /api/pro{full_path} with method: {method}");

        // 转换为RouteHandler类型
        let handler = Arc::new(callback);

        // 获取动态路由匹配器
        let dynamic_router = self.api.dynamic_router().clone();
        let full_path_clone = full_path.clone();
        let method_clone = method.clone();

        // 根据runtime是否存在来执行异步操作
        match self.runtime.as_ref() {
            Some(runtime) => {
                runtime.block_on(async move {
                    let mut router = dynamic_router.write().await;
                    router.add_route(&full_path_clone, &method_clone, handler);
                });
            }
            None => {
                tokio::runtime::Handle::current().block_on(async move {
                    let mut router = dynamic_router.write().await;
                    router.add_route(&full_path_clone, &method_clone, handler);
                });
            }
        };

        Ok(())
    }
}

/// Shutdown signal handler for graceful termination
async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c().await.expect(
            "Failed to install Ctrl+C handler - the server may not respond to keyboard interrupts",
        );
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("Failed to install SIGTERM signal handler - the server may not respond to terminate signals")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            println!("\n🛑 Received Ctrl+C, shutting down gracefully...");
        },
        _ = terminate => {
            println!("\n🛑 Received terminate signal, shutting down gracefully...");
        },
    }
}
