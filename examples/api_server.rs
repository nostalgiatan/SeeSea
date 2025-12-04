//! SeeSea API 服务器示例

use std::sync::Arc;
use tokio::sync::RwLock;

use seesea_core::{
    api::ApiInterface, cache::CacheInterface, cache::types::CacheImplConfig, net::NetworkInterface,
    net::config::NetworkConfig, search::SearchConfig,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    println!("🌊 SeeSea API 服务器\n");

    let mut network_config = NetworkConfig::default();
    network_config.pool.max_idle_connections = 200;
    let network = Arc::new(
        NetworkInterface::new(network_config).map_err(|e| format!("Network error: {e:?}"))?,
    );
    let cache = Arc::new(RwLock::new(
        CacheInterface::new(CacheImplConfig::default())
            .map_err(|e| format!("Cache error: {e:?}"))?,
    ));

    let api = ApiInterface::from_config(SearchConfig::default(), network, cache)
        .map_err(|e| format!("API error: {e:?}"))?;
    let app = api.build_router();

    println!("📍 API 端点:");
    println!("  GET  /api/search?query=rust");
    println!("  GET  /api/health\n");

    let addr = "127.0.0.1:8080";
    println!("🚀 服务器: http://{addr}\n");

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
