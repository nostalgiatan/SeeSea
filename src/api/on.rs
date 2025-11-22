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

use std::sync::Arc;
use tokio::sync::RwLock;
use axum::{
    Router,
    routing::{get, post},
    extract::{State, Query, Json},
    response::{IntoResponse, Response},
    http::StatusCode,
};
use serde_json::json;

use crate::cache::CacheInterface;
use crate::net::NetworkInterface;
use crate::search::{SearchInterface, SearchRequest};
use super::types::*;
use super::handlers::{rss, cache};
use super::middleware::cors;

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
}

/// API 接口
pub struct ApiInterface {
    /// 内部状态
    state: ApiState,
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
        Self {
            state: ApiState {
                search,
                version,
            },
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

    /// 构建 Axum 路由器
    ///
    /// # Returns
    ///
    /// 返回配置好的 Axum Router
    pub fn build_router(&self) -> Router {
        Router::new()
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
            
            // 应用 CORS 中间件
            .layer(cors::create_cors_layer())
            
            .with_state(self.state.clone())
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
    pub async fn serve(&self, config: ServerConfig) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let app = self.build_router();
        let addr = format!("{}:{}", config.host, config.port);
        let listener = tokio::net::TcpListener::bind(&addr).await?;
        axum::serve(listener, app).await?;
        
        Ok(())
    }
}

/// 处理 GET 搜索请求
async fn handle_search(
    State(state): State<ApiState>,
    Query(params): Query<ApiSearchRequest>,
) -> Response {
    match execute_search(&state, params).await {
        Ok(response) => (StatusCode::OK, Json(response)).into_response(),
        Err(e) => {
            let error = ApiErrorResponse {
                code: "SEARCH_ERROR".to_string(),
                message: "搜索失败".to_string(),
                details: Some(e.to_string()),
            };
            (StatusCode::INTERNAL_SERVER_ERROR, Json(error)).into_response()
        }
    }
}

/// 处理 POST 搜索请求
async fn handle_search_post(
    State(state): State<ApiState>,
    Json(params): Json<ApiSearchRequest>,
) -> Response {
    match execute_search(&state, params).await {
        Ok(response) => (StatusCode::OK, Json(response)).into_response(),
        Err(e) => {
            let error = ApiErrorResponse {
                code: "SEARCH_ERROR".to_string(),
                message: "搜索失败".to_string(),
                details: Some(e.to_string()),
            };
            (StatusCode::INTERNAL_SERVER_ERROR, Json(error)).into_response()
        }
    }
}

/// 执行搜索
async fn execute_search(
    state: &ApiState,
    params: ApiSearchRequest,
) -> Result<ApiSearchResponse, Box<dyn std::error::Error + Send + Sync>> {
    let start_time = std::time::Instant::now();

    // 转换为内部搜索查询
    let search_query = params.to_search_query()
        .map_err(|e| format!("参数错误: {}", e))?;

    // 获取引擎列表
    let engines = params.get_engines();

    // 创建搜索请求
    let request = SearchRequest {
        query: search_query,
        engines,
        timeout: None,
        max_results: None,
        force: false,
        cache_timeline: Some(3600),
    };

    // 执行搜索
    let response = state.search.search(&request).await?;
    
    // 转换结果
    let mut results = Vec::new();
    for search_result in &response.results {
        for item in &search_result.items {
            results.push(ApiSearchResultItem {
                title: item.title.clone(),
                url: item.url.clone(),
                description: Some(item.content.clone()),
                engine: search_result.engine_name.clone(),
                score: Some(item.score),
            });
        }
    }
    
    let elapsed = start_time.elapsed().as_millis() as u64;

    // 获取实际的查询字符串
    let query_text = params.get_query().unwrap_or_default();

    Ok(ApiSearchResponse {
        query: query_text,
        results,
        total_count: response.total_count,
        page: params.page,
        page_size: params.page_size,
        engines_used: response.engines_used,
        query_time_ms: elapsed,
        cached: response.cached,
    })
}

/// 处理引擎列表请求
async fn handle_engines_list(
    State(state): State<ApiState>,
) -> Response {
    let engines = state.search.list_engines();
    
    let engine_infos: Vec<ApiEngineInfo> = engines
        .into_iter()
        .map(|name| ApiEngineInfo {
            name: name.clone(),
            description: format!("{} 搜索引擎", name),
            engine_type: "general".to_string(),
            enabled: true,
            capabilities: vec!["web".to_string()],
        })
        .collect();
    
    (StatusCode::OK, Json(engine_infos)).into_response()
}

/// 处理统计信息请求
async fn handle_stats(
    State(state): State<ApiState>,
) -> Response {
    let stats = state.search.get_stats().await;
    let api_stats = ApiStatsResponse::from_search_stats(&stats);

    (StatusCode::OK, Json(api_stats)).into_response()
}

/// 处理健康检查请求
async fn handle_health(
    State(state): State<ApiState>,
) -> Response {
    let engines = state.search.list_engines();
    
    let health = ApiHealthResponse {
        status: "healthy".to_string(),
        version: state.version.clone(),
        available_engines: engines.len(),
        total_engines: engines.len(),
    };
    
    (StatusCode::OK, Json(health)).into_response()
}

/// 处理版本信息请求
async fn handle_version(
    State(state): State<ApiState>,
) -> Response {
    let version_info = json!({
        "version": state.version,
        "name": "SeeSea",
        "description": "隐私保护型元搜索引擎"
    });
    
    (StatusCode::OK, Json(version_info)).into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::search::SearchConfig;
    use crate::net::types::NetworkConfig;
    use crate::cache::types::CacheImplConfig;

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
        let search = Arc::new(
            SearchInterface::new(SearchConfig::default()).unwrap()
        );
        
        let api = ApiInterface::new(search, "0.1.0".to_string());
        let _router = api.build_router();
        // Router is built successfully
    }
}
