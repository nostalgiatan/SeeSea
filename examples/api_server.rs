//! SeeSea API 服务器示例

use std::sync::Arc;
use tokio::sync::RwLock;

use SeeSea::{
    api::ApiInterface,
    cache::CacheInterface,
    net::NetworkInterface,
    search::SearchConfig,
    cache::types::CacheImplConfig,
    net::types::NetworkConfig,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    println!("🌊 SeeSea API 服务器\n");

    let mut network_config = NetworkConfig::default();
    network_config.pool.max_idle_connections = 200;
    let network = Arc::new(NetworkInterface::new(network_config)?);
    let cache = Arc::new(RwLock::new(CacheInterface::new(CacheImplConfig::default())?));
    
    let api = ApiInterface::from_config(SearchConfig::default(), network, cache)?;
    let app = api.build_router();

    println!("📍 API 端点:");
    println!("  GET  /api/search?query=rust");
    println!("  GET  /api/health\n");

    let addr = "127.0.0.1:8080";
    println!("🚀 服务器: http://{}\n", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
