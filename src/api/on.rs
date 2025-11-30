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

//! API 外部接口模块
//!
//! 提供高层次的 HTTP API 接口供外部调用

use axum::{
    Router,
    routing::{get, post},
};
use std::sync::Arc;
use tokio::sync::RwLock;

use super::handlers::{
    cache, handle_engines_list, handle_favicon, handle_health, handle_index,
    handle_magic_link_generate, handle_metrics, handle_realtime_metrics, handle_search,
    handle_search_post, handle_stats, handle_version, rss,
};
use super::metrics::{MetricsCollector, MetricsConfig};
use super::middleware::{
    AuthConfig, AuthState, CircuitBreakerConfig, CircuitBreakerState, IpFilterConfig,
    IpFilterState, MagicLinkConfig, MagicLinkState, RateLimitConfig, RateLimiterState,
    circuit_breaker_middleware, cors, ip_filter_middleware, jwt_auth_middleware,
    magic_link_middleware, rate_limit_middleware,
};
use super::network::{NetworkConfig, NetworkMode};
use crate::cache::CacheInterface;
use crate::net::NetworkInterface;
use crate::search::SearchInterface;

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
    /// 版本信息
    pub version: String,
    /// 指标收集器
    pub metrics: Arc<MetricsCollector>,
    /// 魔法链接状态
    pub magic_link: Arc<MagicLinkState>,
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
        let metrics = Arc::new(MetricsCollector::new(MetricsConfig::default()));
        let magic_link = Arc::new(MagicLinkState::new(MagicLinkConfig::default()));

        let state = ApiState {
            search,
            version,
            metrics,
            magic_link,
        };

        // 根据网络配置初始化中间件
        let rate_limiter = Arc::new(RateLimiterState::new(RateLimitConfig {
            enabled: network_config.external.enable_rate_limit,
            ..Default::default()
        }));

        let circuit_breaker = Arc::new(CircuitBreakerState::new(CircuitBreakerConfig {
            enabled: network_config.external.enable_circuit_breaker,
            ..Default::default()
        }));

        let ip_filter = Arc::new(IpFilterState::new(IpFilterConfig {
            enabled: network_config.external.enable_ip_filter,
            ..Default::default()
        }));

        let auth_state = Arc::new(AuthState::new(AuthConfig {
            enabled: network_config.external.enable_jwt_auth,
            ..Default::default()
        }));

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
        search_config: crate::search::SearchConfig,
        _network: Arc<NetworkInterface>,
        _cache: Arc<RwLock<CacheInterface>>,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let search = Arc::new(SearchInterface::new(search_config)?);
        Ok(Self::new(search, env!("CARGO_PKG_VERSION").to_string()))
    }

    /// 构建默认路由器（内网模式）
    ///
    /// # Returns
    ///
    /// 返回配置好的 Axum Router
    pub fn build_router(&self) -> Router {
        self.build_internal_router()
    }

    /// 构建内网路由器（无安全限制）
    ///
    /// # Returns
    ///
    /// 返回配置好的 Axum Router
    pub fn build_internal_router(&self) -> Router {
        Router::new()
            // 首页路由
            .route("/", get(handle_index))
            .route("/favicon.ico", get(handle_favicon))
            // 搜索相关路由
            .route("/api/search", get(handle_search))
            .route("/api/search", post(handle_search_post))
            // 引擎信息路由
            .route("/api/engines", get(handle_engines_list))
            // RSS 相关路由
            .route("/api/rss/feeds", get(rss::handle_rss_feeds_list))
            .route("/api/rss/fetch", post(rss::handle_rss_fetch))
            .route("/api/rss/templates", get(rss::handle_rss_templates_list))
            .route("/api/rss/template/add", post(rss::handle_rss_template_add))
            // 缓存管理路由
            .route("/api/cache/stats", get(cache::handle_cache_stats))
            .route("/api/cache/clear", post(cache::handle_cache_clear))
            .route("/api/cache/cleanup", post(cache::handle_cache_cleanup))
            // 统计信息路由
            .route("/api/stats", get(handle_stats))
            // 健康检查路由
            .route("/api/health", get(handle_health))
            .route("/health", get(handle_health))
            // 版本信息路由
            .route("/api/version", get(handle_version))
            // 指标路由
            .route("/api/metrics", get(handle_metrics))
            .route("/api/metrics/realtime", get(handle_realtime_metrics))
            // 魔法链接管理路由（仅内网）
            .route("/api/magic-link/generate", post(handle_magic_link_generate))
            .with_state(self.state.clone())
    }

    /// 构建外网路由器（带安全限制）
    ///
    /// # Returns
    ///
    /// 返回配置好的 Axum Router
    pub fn build_external_router(&self) -> Router {
        use axum::middleware;

        Router::new()
            // 首页路由
            .route("/", get(handle_index))
            .route("/favicon.ico", get(handle_favicon))
            // 搜索相关路由
            .route("/api/search", get(handle_search))
            .route("/api/search", post(handle_search_post))
            // 引擎信息路由
            .route("/api/engines", get(handle_engines_list))
            // RSS 相关路由（可能需要认证）
            .route("/api/rss/feeds", get(rss::handle_rss_feeds_list))
            .route("/api/rss/fetch", post(rss::handle_rss_fetch))
            // 统计信息路由
            .route("/api/stats", get(handle_stats))
            // 健康检查路由
            .route("/api/health", get(handle_health))
            .route("/health", get(handle_health))
            // 版本信息路由
            .route("/api/version", get(handle_version))
            // 指标路由（只读）
            .route("/api/metrics", get(handle_metrics))
            .with_state(self.state.clone())
            // 应用中间件（顺序很重要）
            // 1. 魔法链接（最先检查，可以绕过认证）
            .layer(middleware::from_fn_with_state(
                self.state.magic_link.clone(),
                magic_link_middleware,
            ))
            // 2. JWT认证（如果启用）
            .layer(middleware::from_fn_with_state(
                self.auth_state.clone(),
                jwt_auth_middleware,
            ))
            // 3. IP过滤
            .layer(middleware::from_fn_with_state(
                self.ip_filter.clone(),
                ip_filter_middleware,
            ))
            // 4. 熔断器
            .layer(middleware::from_fn_with_state(
                self.circuit_breaker.clone(),
                circuit_breaker_middleware,
            ))
            // 5. 限流
            .layer(middleware::from_fn_with_state(
                self.rate_limiter.clone(),
                rate_limit_middleware,
            ))
            // 6. CORS
            .layer(cors::create_cors_layer())
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
        println!("   - 仅允许本地访问");
        println!("   - 无安全限制");

        let listener = tokio::net::TcpListener::bind(&addr).await?;
        axum::serve(listener, app).await?;

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
        println!(
            "   - 启用限流: {}",
            self.network_config.external.enable_rate_limit
        );
        println!(
            "   - 启用熔断: {}",
            self.network_config.external.enable_circuit_breaker
        );
        println!(
            "   - 启用IP过滤: {}",
            self.network_config.external.enable_ip_filter
        );
        println!(
            "   - 启用JWT认证: {}",
            self.network_config.external.enable_jwt_auth
        );
        println!(
            "   - 启用魔法链接: {}",
            self.network_config.external.enable_magic_link
        );

        self.print_metrics_dashboard().await;

        let listener = tokio::net::TcpListener::bind(&addr).await?;
        axum::serve(listener, app).await?;

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

            println!("\n🔒 内网服务器: {internal_addr}");
            println!("   - 仅允许本地访问");
            println!("   - 无安全限制");

            let internal_listener = tokio::net::TcpListener::bind(&internal_addr).await?;
            tokio::spawn(async move { axum::serve(internal_listener, internal_app).await });
        }

        // 启动外网服务器
        if self.network_config.external.enabled {
            let external_app = self.build_external_router();
            let external_addr = format!(
                "{}:{}",
                self.network_config.external.host, self.network_config.external.port
            );

            println!("\n🌐 外网服务器: {external_addr}");
            println!(
                "   - 启用限流: {}",
                self.network_config.external.enable_rate_limit
            );
            println!(
                "   - 启用熔断: {}",
                self.network_config.external.enable_circuit_breaker
            );
            println!(
                "   - 启用IP过滤: {}",
                self.network_config.external.enable_ip_filter
            );
            println!(
                "   - 启用JWT认证: {}",
                self.network_config.external.enable_jwt_auth
            );
            println!(
                "   - 启用魔法链接: {}",
                self.network_config.external.enable_magic_link
            );

            self.print_metrics_dashboard().await;

            let external_listener = tokio::net::TcpListener::bind(&external_addr).await?;
            axum::serve(external_listener, external_app).await?;
        }

        Ok(())
    }

    /// 打印指标面板
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cache::types::CacheImplConfig;
    use crate::net::config::NetworkConfig;
    use crate::search::SearchConfig;

    #[tokio::test]
    async fn test_api_interface_creation() {
        let search_config = SearchConfig::default();
        let network = Arc::new(NetworkInterface::new(NetworkConfig::default()).unwrap());
        let cache = Arc::new(RwLock::new(
            CacheInterface::new(CacheImplConfig::default()).unwrap(),
        ));

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
