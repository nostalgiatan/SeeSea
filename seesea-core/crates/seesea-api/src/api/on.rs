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

//! API 外部接口模块
//!
//! 提供高层次的 HTTP API 接口供外部调用

use axum::extract::connect_info::IntoMakeServiceWithConnectInfo;
use axum::{
    Router,
    routing::{any, get, get_service, post},
};
use std::net::SocketAddr;
use std::sync::Arc;
use tracing::info;

use super::dynamic_router::{ThreadSafeDynamicRouter, new_dynamic_router};
use super::handlers::{
    cache, get_static_html_path, handle_cache_clear_pattern, handle_cache_keys,
    handle_cache_stats_detail, handle_config_get, handle_config_update, handle_connections_stats,
    handle_controller_action, handle_engine_toggle, handle_engines_batch, handle_engines_list,
    handle_engines_list_full, handle_engines_status, handle_favicon, handle_health,
    handle_health_detail, handle_hot_all, handle_hot_multiple, handle_hot_platform,
    handle_hot_platforms_list, handle_index, handle_logs_directory, handle_logs_errors,
    handle_logs_files, handle_logs_read, handle_logs_tail, handle_magic_link_generate,
    handle_metrics, handle_pro_api, handle_realtime_metrics, handle_search, handle_search_post,
    handle_stats, handle_stock_api, handle_system_resources, handle_system_status, handle_version,
    handle_version_info, rss,
};
use super::metrics::{MetricsCollector, MetricsConfig};
use super::middleware::{
    AuthState, CircuitBreakerState, IpFilterState, MagicLinkState, RateLimiterState,
    circuit_breaker_middleware, cors, ip_filter_middleware, jwt_auth_middleware,
    magic_link_middleware, metrics_middleware, rate_limit_middleware,
};
use super::network::{NetworkConfig, NetworkMode};
use super::openapi::ApiDoc;
use super::runtime_config::RuntimeConfigManager;
use super::swagger::create_swagger_router;
use seesea_cache::CacheInterface;
use seesea_cache::cache::types::CacheImplConfig;
use seesea_config::{
    AuthConfig, CircuitBreakerConfig, IpFilterConfig, MagicLinkConfig, RateLimitConfig,
};
use seesea_hot::client::AsyncHotTrendClient;
use seesea_net::NetworkInterface;
use seesea_search::search::{SearchConfig, SearchInterface};
use tokio::sync::OnceCell;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

/// Graceful shutdown 信号处理
async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            println!("\n⏹️  收到 Ctrl+C，正在关闭服务器...");
        }
        _ = terminate => {
            println!("\n⏹️  收到终止信号，正在关闭服务器...");
        }
    }
}

/// 服务器配置
#[derive(Debug, Clone)]
pub struct ServerConfig {
    /// 主机地址
    pub host: String,
    /// 端口
    pub port: u16,
    /// CORS允许的源
    pub cors_origins: Vec<String>,
    /// 是否启用日志
    pub enable_logging: bool,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "0.0.0.0".to_string(),
            port: 8080,
            cors_origins: vec!["*".to_string()],
            enable_logging: true,
        }
    }
}

/// API 服务状态
#[derive(Clone)]
pub struct ApiState {
    /// 搜索接口
    pub search: Arc<SearchInterface>,
    /// 热门搜索客户端（懒加载）
    pub hot_client: Arc<OnceCell<AsyncHotTrendClient>>,
    /// 版本信息
    pub version: String,
    /// 指标收集器
    pub metrics: Arc<MetricsCollector>,
    /// 魔法链接状态
    pub magic_link: Arc<MagicLinkState>,
    /// 动态路由匹配器
    pub dynamic_router: ThreadSafeDynamicRouter,
    /// 前端 API 地址（空字符串表示使用同源）
    pub frontend_api_url: String,
    /// 缓存接口
    pub cache: Arc<CacheInterface>,
    /// 运行时配置管理器
    pub runtime_config: Arc<RuntimeConfigManager>,
    /// RSS 存储管理器
    pub rss_storage: crate::api::handlers::rss::RssStorage,
}

/// API 接口
pub struct ApiInterface {
    /// 内部状态
    state: ApiState,
    /// 网络配置
    network_config: NetworkConfig,
    /// 中间件状态
    rate_limiter: Arc<RateLimiterState>,
    circuit_breaker: Arc<CircuitBreakerState>,
    ip_filter: Arc<IpFilterState>,
    auth_state: Arc<AuthState>,
}

impl ApiInterface {
    /// 创建新的 API 接口
    ///
    /// # Arguments
    ///
    /// * `search` - 搜索接口
    /// * `version` - 版本号
    ///
    /// # Returns
    ///
    /// 返回 API 接口实例
    pub fn new(search: Arc<SearchInterface>, version: String) -> Self {
        Self::with_network_config(search, version, NetworkConfig::default())
    }

    /// 使用网络配置创建 API 接口
    pub fn with_network_config(
        search: Arc<SearchInterface>,
        version: String,
        network_config: NetworkConfig,
    ) -> Self {
        info!("开始创建API接口的缓存");
        let cache = Arc::new(
            CacheInterface::new(CacheImplConfig::new(seesea_config::paths::get_cache_dir()))
                .unwrap(),
        );
        info!("API接口的缓存创建完成");
        Self::with_full_config(search, version, network_config, String::new(), cache)
    }

    /// 使用完整配置创建 API 接口
    pub fn with_full_config(
        search: Arc<SearchInterface>,
        version: String,
        network_config: NetworkConfig,
        frontend_api_url: String,
        cache: Arc<CacheInterface>,
    ) -> Self {
        let metrics = Arc::new(MetricsCollector::new(MetricsConfig::default()));
        let magic_link = Arc::new(MagicLinkState::new(MagicLinkConfig::default()));
        let dynamic_router = new_dynamic_router();

        // 创建AsyncHotTrendClient，不在这里初始化，而是在第一次使用时初始化
        // 这里我们直接使用空的OnceCell，等待第一次使用时再初始化
        let hot_client = Arc::new(OnceCell::new());

        let state = ApiState {
            search,
            hot_client,
            version,
            metrics,
            magic_link,
            dynamic_router,
            frontend_api_url,
            cache,
            runtime_config: Arc::new(RuntimeConfigManager::new()),
            rss_storage: crate::api::handlers::rss::RssStorage::new(),
        };

        // 根据网络配置初始化中间件
        let rate_limiter = Arc::new(RateLimiterState::new(
            RateLimitConfig {
                enabled: network_config.external.enable_rate_limit,
                requests_per_second: network_config.external.rate_limit_per_second,
                burst_size: network_config.external.rate_limit_burst_size,
                ..Default::default()
            }
            .into(),
        ));

        let circuit_breaker = Arc::new(CircuitBreakerState::new(CircuitBreakerConfig {
            enabled: network_config.external.enable_circuit_breaker,
            ..Default::default()
        }));

        let ip_filter = Arc::new(IpFilterState::new(IpFilterConfig {
            enabled: network_config.external.enable_ip_filter,
            ..Default::default()
        }));

        let auth_state = Arc::new(AuthState::new(
            AuthConfig {
                enabled: network_config.external.enable_jwt_auth,
                ..Default::default()
            }
            .into(),
        ));

        Self {
            state,
            network_config,
            rate_limiter,
            circuit_breaker,
            ip_filter,
            auth_state,
        }
    }

    /// 从配置创建 API 接口
    ///
    /// # Arguments
    ///
    /// * `search_config` - 搜索配置
    /// * `network` - 网络接口
    /// * `cache` - 缓存接口
    ///
    /// # Returns
    ///
    /// 返回 API 接口实例或错误
    pub fn from_config(
        search_config: SearchConfig,
        _network: Arc<NetworkInterface>,
        cache: Arc<CacheInterface>,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let search = Arc::new(SearchInterface::new(search_config)?);
        let metrics = Arc::new(MetricsCollector::new(MetricsConfig::default()));
        let magic_link = Arc::new(MagicLinkState::new(MagicLinkConfig::default()));
        let dynamic_router = new_dynamic_router();
        let hot_client = Arc::new(OnceCell::new());

        let state = ApiState {
            search,
            hot_client,
            version: env!("CARGO_PKG_VERSION").to_string(),
            metrics,
            magic_link,
            dynamic_router,
            frontend_api_url: String::new(),
            cache,
            runtime_config: Arc::new(RuntimeConfigManager::new()),
            rss_storage: crate::api::handlers::rss::RssStorage::new(),
        };

        let network_config = NetworkConfig::default();
        let rate_limiter = Arc::new(RateLimiterState::new(RateLimitConfig::default().into()));
        let circuit_breaker = Arc::new(CircuitBreakerState::new(CircuitBreakerConfig::default()));
        let ip_filter = Arc::new(IpFilterState::new(IpFilterConfig::default()));
        let auth_state = Arc::new(AuthState::new(AuthConfig::default().into()));

        Ok(Self {
            state,
            network_config,
            rate_limiter,
            circuit_breaker,
            ip_filter,
            auth_state,
        })
    }

    /// 构建默认路由器（内网模式）
    ///
    /// # Returns
    ///
    /// 返回配置好的 Axum Router
    pub fn build_router(&self) -> IntoMakeServiceWithConnectInfo<Router, SocketAddr> {
        self.build_internal_router()
    }

    /// 构建内网路由器（无安全限制）
    ///
    /// # Returns
    ///
    /// 返回配置好的 Axum Router
    pub fn build_internal_router(&self) -> IntoMakeServiceWithConnectInfo<Router, SocketAddr> {
        use axum::routing::get_service;
        use tower_http::services::ServeDir;

        // 获取静态文件路径
        let static_html_path = get_static_html_path();

        // API 路由组
        let api_router = Router::new()
            .route("/search", get(handle_search))
            .route("/search", post(handle_search_post))
            .route("/engines", get(handle_engines_list))
            .route("/hot", get(handle_hot_all))
            .route("/hot/all", get(handle_hot_all))
            .route("/hot/platforms", get(handle_hot_platforms_list))
            .route("/hot/multiple", get(handle_hot_multiple))
            .route("/hot/{platform_id}", get(handle_hot_platform))
            .route("/rss/feeds", get(rss::handle_rss_feeds_list))
            .route("/rss/fetch", post(rss::handle_rss_fetch))
            .route("/rss/templates", get(rss::handle_rss_templates_list))
            .route("/rss/template/add", post(rss::handle_rss_template_add))
            .route("/cache/stats", get(cache::handle_cache_stats))
            .route("/cache/clear", post(cache::handle_cache_clear))
            .route("/cache/cleanup", post(cache::handle_cache_cleanup))
            .route("/stats", get(handle_stats))
            .route("/health", get(handle_health))
            .route("/version", get(handle_version))
            .route("/metrics", get(handle_metrics))
            .route("/metrics/realtime", get(handle_realtime_metrics))
            .route("/magic-link/generate", post(handle_magic_link_generate))
            .route("/pro/{*path}", any(handle_pro_api))
            .route("/stock/{*path}", any(handle_stock_api));

        // 内部管理路由组（仅限本地访问）
        let internal_router = Router::new()
            // 系统监控
            .route("/system/resources", get(handle_system_resources))
            .route("/system/status", get(handle_system_status))
            .route("/system/health", get(handle_health_detail))
            // 引擎管理
            .route("/engines/list", get(handle_engines_list_full))
            .route("/engines/status", get(handle_engines_status))
            .route("/engines/toggle", post(handle_engine_toggle))
            .route("/engines/batch", post(handle_engines_batch))
            // 缓存管理
            .route("/cache/keys", get(handle_cache_keys))
            .route("/cache/stats", get(handle_cache_stats_detail))
            .route("/cache/clear", post(handle_cache_clear_pattern))
            // 配置管理
            .route("/config/get", get(handle_config_get))
            .route("/config/update", post(handle_config_update))
            // 控制器管理
            .route("/controller/action", post(handle_controller_action))
            // 系统信息
            .route("/system/version", get(handle_version_info))
            // 日志管理
            .route("/logs/directory", get(handle_logs_directory))
            .route("/logs/files", get(handle_logs_files))
            .route("/logs/read", get(handle_logs_read))
            .route("/logs/tail", get(handle_logs_tail))
            .route("/logs/errors", get(handle_logs_errors))
            // 连接统计
            .route("/connections/stats", get(handle_connections_stats));

        Router::new()
            .route("/favicon.ico", get(handle_favicon))
            .route("/health", get(handle_health))
            // API 路由组
            .nest("/api", api_router)
            // 内部管理路由组（仅限本地访问）
            .nest("/internal", internal_router)
            // Swagger UI 路由
            .merge(create_swagger_router())
            // 静态资源目录
            .nest_service(
                "/assets",
                get_service(ServeDir::new(static_html_path.join("assets"))),
            )
            // 根路径返回 index.html
            .route("/", get(handle_index))
            // robots.txt 等根目录文件
            .nest_service(
                "/robots.txt",
                get_service(ServeDir::new(static_html_path.clone())),
            )
            // 所有其他路由（SPA 客户端路由）都返回 index.html
            .fallback(get(handle_index))
            .with_state(self.state.clone())
            // 应用指标收集中间件
            .layer(axum::middleware::from_fn_with_state(
                self.state.metrics.clone(),
                metrics_middleware,
            ))
            // 应用 CORS 中间件
            .layer(cors::default_cors_layer())
            .into_make_service_with_connect_info::<SocketAddr>()
    }

    /// 构建外网路由器（带安全限制）
    ///
    /// # Returns
    ///
    /// 返回配置好的 Axum Router
    pub fn build_external_router(&self) -> IntoMakeServiceWithConnectInfo<Router, SocketAddr> {
        use tower_http::services::ServeDir;

        // 获取静态文件路径
        let static_html_path = get_static_html_path();

        // API 路由组
        let api_router = Router::new()
            .route("/search", get(handle_search))
            .route("/search", post(handle_search_post))
            .route("/engines", get(handle_engines_list))
            .route("/hot", get(handle_hot_all))
            .route("/hot/all", get(handle_hot_all))
            .route("/hot/platforms", get(handle_hot_platforms_list))
            .route("/hot/multiple", get(handle_hot_multiple))
            .route("/hot/{platform_id}", get(handle_hot_platform))
            .route("/rss/feeds", get(rss::handle_rss_feeds_list))
            .route("/rss/fetch", post(rss::handle_rss_fetch))
            .route("/stats", get(handle_stats))
            .route("/health", get(handle_health))
            .route("/version", get(handle_version))
            .route("/metrics", get(handle_metrics))
            .route("/cache/stats", get(cache::handle_cache_stats))
            .route("/cache/clear", post(cache::handle_cache_clear))
            .route("/cache/cleanup", post(cache::handle_cache_cleanup))
            .route("/metrics/realtime", get(handle_realtime_metrics))
            .route("/pro/{*path}", any(handle_pro_api))
            .route("/stock/{*path}", any(handle_stock_api));

        let swagger =
            SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi());

        Router::new()
            .route("/favicon.ico", get(handle_favicon))
            .route("/health", get(handle_health))
            // API 路由组
            .nest("/api", api_router)
            // Swagger UI 路由
            .merge(Router::from(swagger))
            // 静态资源目录 - 优先级最高
            .nest_service(
                "/assets",
                get_service(ServeDir::new(static_html_path.join("assets"))),
            )
            // 根路径返回 index.html
            .route("/", get(handle_index))
            // robots.txt 等根目录文件
            .nest_service(
                "/robots.txt",
                get_service(ServeDir::new(static_html_path.clone())),
            )
            // 所有其他路由（SPA 客户端路由）都返回 index.html
            .fallback(get(handle_index))
            .with_state(self.state.clone())
            // 应用中间件（顺序很重要）
            // 1. 魔法链接（最先检查，可以绕过认证）
            .layer(axum::middleware::from_fn_with_state(
                self.state.magic_link.clone(),
                magic_link_middleware,
            ))
            // 2. JWT认证（如果启用）
            .layer(axum::middleware::from_fn_with_state(
                self.auth_state.clone(),
                jwt_auth_middleware,
            ))
            // 3. IP过滤
            .layer(axum::middleware::from_fn_with_state(
                self.ip_filter.clone(),
                ip_filter_middleware,
            ))
            // 4. 熔断器
            .layer(axum::middleware::from_fn_with_state(
                self.circuit_breaker.clone(),
                circuit_breaker_middleware,
            ))
            // 5. 限流
            .layer(axum::middleware::from_fn_with_state(
                self.rate_limiter.clone(),
                rate_limit_middleware,
            ))
            // 6. 指标收集
            .layer(axum::middleware::from_fn_with_state(
                self.state.metrics.clone(),
                metrics_middleware,
            ))
            // 7. CORS
            .layer(cors::create_cors_layer(
                self.network_config.external.cors_origins.clone(),
            ))
            .into_make_service_with_connect_info::<SocketAddr>()
    }

    /// 启动服务器
    ///
    /// # Arguments
    ///
    /// * `config` - 服务器配置
    ///
    /// # Returns
    ///
    /// 返回结果
    pub async fn serve(
        &self,
        _config: ServerConfig,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // 根据网络模式启动服务器
        match self.network_config.mode {
            NetworkMode::Internal => self.serve_internal().await,
            NetworkMode::External => self.serve_external().await,
            NetworkMode::Dual => self.serve_dual().await,
        }
    }

    /// 启动内网服务器
    async fn serve_internal(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let app = self.build_internal_router();
        let addr = format!(
            "{}:{}",
            self.network_config.internal.host, self.network_config.internal.port
        );

        println!("🔒 内网服务器启动在: {addr}");

        let listener = tokio::net::TcpListener::bind(&addr).await?;
        axum::serve(listener, app)
            .with_graceful_shutdown(shutdown_signal())
            .await?;

        Ok(())
    }

    /// 启动外网服务器
    async fn serve_external(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let app = self.build_external_router();
        let addr = format!(
            "{}:{}",
            self.network_config.external.host, self.network_config.external.port
        );

        println!("🌐 外网服务器启动在: {addr}");

        let listener = tokio::net::TcpListener::bind(&addr).await?;
        axum::serve(listener, app)
            .with_graceful_shutdown(shutdown_signal())
            .await?;

        Ok(())
    }

    /// 启动双模式服务器（内网+外网）
    async fn serve_dual(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        println!("🚀 双模式服务器启动");

        // 启动内网服务器
        if self.network_config.internal.enabled {
            let internal_app = self.build_internal_router();
            let internal_addr = format!(
                "{}:{}",
                self.network_config.internal.host, self.network_config.internal.port
            );

            println!("🔒 内网服务器: {internal_addr}");

            let internal_listener = tokio::net::TcpListener::bind(&internal_addr).await?;
            tokio::spawn(async move {
                axum::serve(internal_listener, internal_app)
                    .with_graceful_shutdown(shutdown_signal())
                    .await
            });
        }

        // 启动外网服务器
        if self.network_config.external.enabled {
            let external_app = self.build_external_router();
            let external_addr = format!(
                "{}:{}",
                self.network_config.external.host, self.network_config.external.port
            );

            println!("🌐 外网服务器: {external_addr}");

            let external_listener = tokio::net::TcpListener::bind(&external_addr).await?;
            axum::serve(external_listener, external_app)
                .with_graceful_shutdown(shutdown_signal())
                .await?;
        }

        Ok(())
    }

    /// 打印指标面板（调试用）
    #[allow(dead_code)]
    async fn print_metrics_dashboard(&self) {
        let metrics = self.state.metrics.get_realtime_metrics().await;

        println!("\n📊 实时指标面板");
        println!("┌─────────────────────────────────────┐");
        println!("│ 请求总数: {:>24} │", metrics.total_requests);
        println!("│ 成功请求: {:>24} │", metrics.successful_requests);
        println!("│ 失败请求: {:>24} │", metrics.failed_requests);
        println!(
            "│ 平均响应时间: {:>17.2} ms │",
            metrics.avg_response_time_ms
        );
        println!("│ 活跃连接: {:>24} │", metrics.active_connections);
        println!("│ 限流拒绝: {:>24} │", metrics.rate_limited);
        println!("│ 熔断拒绝: {:>24} │", metrics.circuit_breaker_trips);
        println!("│ IP封禁拒绝: {:>22} │", metrics.ip_blocked);
        println!("└─────────────────────────────────────┘");
        println!();
    }

    /// 获取指标收集器
    pub fn metrics(&self) -> &Arc<MetricsCollector> {
        &self.state.metrics
    }

    /// 获取魔法链接状态
    pub fn magic_link(&self) -> &Arc<MagicLinkState> {
        &self.state.magic_link
    }

    /// 获取IP过滤器
    pub fn ip_filter(&self) -> &Arc<IpFilterState> {
        &self.ip_filter
    }

    /// 获取动态路由匹配器
    pub fn dynamic_router(&self) -> &ThreadSafeDynamicRouter {
        &self.state.dynamic_router
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use seesea_cache::cache::types::CacheImplConfig;
    use seesea_search::search::SearchConfig;

    #[tokio::test]
    async fn test_api_interface_creation() {
        let search_config = SearchConfig::default();
        let network_config = seesea_config::NetworkConfig::default();
        let network = Arc::new(NetworkInterface::new(network_config).unwrap());
        let cache = Arc::new(
            CacheInterface::new(CacheImplConfig::new(seesea_config::paths::get_cache_dir()))
                .unwrap(),
        );

        let api = ApiInterface::from_config(search_config, network, cache);
        assert!(api.is_ok());
    }

    #[test]
    fn test_api_router_creation() {
        let search = Arc::new(SearchInterface::new(SearchConfig::default()).unwrap());

        let api = ApiInterface::new(search, "0.1.0".to_string());
        let _internal_router = api.build_internal_router();
        let _external_router = api.build_external_router();
        // Routers are built successfully
    }
}
