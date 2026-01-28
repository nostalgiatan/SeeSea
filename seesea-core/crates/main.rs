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

//! SeeSea 主程序入口

use clap::Parser;
use seesea_api::ApiInterface;
use seesea_api::api::network::{
    ExternalNetworkConfig, InternalNetworkConfig, NetworkConfig as ApiNetworkConfig, NetworkMode,
};
use seesea_api::api::on::ServerConfig;
use seesea_core::ConfigManager;
use seesea_core::NetworkInterface;
use seesea_core::{CacheImplConfig, CacheInterface};
use seesea_search::search::{SearchConfig, SearchInterface};
use std::path::PathBuf;
use std::sync::Arc;

/// SeeSea 命令行应用
#[derive(Parser)]
#[command(name = "SeeSea")]
#[command(about = "🌊 SeeSea - 隐私保护型元搜索引擎", long_about = None)]
#[command(version)]
struct Cli {
    /// 配置文件路径
    #[arg(short, long)]
    config: Option<PathBuf>,

    /// 运行环境
    #[arg(short, long, default_value = "development")]
    environment: String,

    /// 启用调试模式
    #[arg(short, long)]
    debug: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // 解析命令行参数
    let cli = Cli::parse();

    // 先加载配置以获取日志配置
    let manager = ConfigManager::with_environment(cli.config.clone(), &cli.environment).await?;
    let config = manager.get_config().await;

    // 初始化日志（使用配置文件中的日志配置）
    let log_level = match config.logging.level {
        seesea_config::LogLevel::Error => tracing::Level::ERROR,
        seesea_config::LogLevel::Warn => tracing::Level::WARN,
        seesea_config::LogLevel::Info => tracing::Level::INFO,
        seesea_config::LogLevel::Debug => tracing::Level::DEBUG,
        seesea_config::LogLevel::Trace => tracing::Level::TRACE,
    };

    tracing_subscriber::fmt()
        .with_max_level(log_level)
        .with_span_events(tracing_subscriber::fmt::format::FmtSpan::FULL)
        .with_ansi(config.logging.colored)
        .init();

    println!("🌊 SeeSea - 看海看得远，看得广");
    println!("🦀 隐私保护型元搜索引擎");
    println!();

    // 加载配置
    println!("📁 加载配置...");
    let manager = ConfigManager::with_environment(cli.config, &cli.environment).await?;
    let config = manager.get_config().await;
    println!("  ✅ 配置加载成功");
    println!("  📄 环境: {:?}", config.general.environment);
    println!("  📄 服务器端口: {}", config.server.port);
    println!();

    // 初始化网络接口
    println!("🌐 初始化网络接口...");
    let _network = Arc::new(NetworkInterface::default());
    println!("  ✅ 网络接口初始化成功");
    println!();

    // 初始化搜索接口
    println!("🔍 初始化搜索接口...");
    let search_config = SearchConfig::default();
    let network_config = config.network.clone();
    let _search = Arc::new(SearchInterface::new_with_network_config(
        search_config.clone(),
        Some(network_config),
    )?);
    println!("  ✅ 搜索接口初始化成功");
    println!();

    // 初始化缓存接口
    println!("💾 初始化缓存接口...");
    let cache_config = CacheImplConfig::new(seesea_config::paths::get_cache_dir());
    let _cache = Arc::new(tokio::sync::RwLock::new(
        CacheInterface::new(cache_config).map_err(|e| format!("Cache error: {e}"))?,
    ));
    println!("  ✅ 缓存接口初始化成功");
    println!();

    // 初始化API接口
    println!("🚀 初始化API接口...");

    // 创建搜索接口（使用完整的网络配置，包括隐私配置）
    let network_config_for_search = config.network.clone();
    let search = Arc::new(SearchInterface::new_with_network_config(
        search_config,
        Some(network_config_for_search),
    )?);

    // 保存bind_address和port，因为它们会被移动
    let bind_address = config.server.bind_address.clone();
    let port = config.server.port;
    let external_port = port + 1;
    let auth_enabled = config.api.auth.enabled;

    // 创建自定义网络配置，使用配置文件中的端口号
    // 内网和外网使用不同的端口，避免冲突
    let network_config = ApiNetworkConfig {
        mode: NetworkMode::Dual,
        internal: InternalNetworkConfig {
            enabled: true,
            host: "127.0.0.1".to_string(),
            port, // 内网使用配置文件中的端口号
        },
        external: ExternalNetworkConfig {
            enabled: true,
            host: config.server.bind_address,
            port: external_port, // 外网使用配置文件中的端口号+1，避免冲突
            cors_origins: config.api.cors.allowed_origins.clone(),
            enable_rate_limit: config.api.rate_limit.enabled,
            rate_limit_per_second: config.api.rate_limit.requests_per_second,
            rate_limit_burst_size: config.api.rate_limit.burst_size,
            enable_circuit_breaker: true,
            enable_ip_filter: true,
            enable_jwt_auth: auth_enabled,
            enable_magic_link: true,
            auth_type: format!("{:?}", config.api.auth.auth_type),
            api_key: config.api.auth.api_key.query_param.clone(),
            key_source: "query".to_string(),
            key_name: config.api.auth.api_key.query_param.clone(),
        },
    };

    // 使用自定义网络配置创建API接口
    let api = ApiInterface::with_network_config(
        search,
        env!("CARGO_PKG_VERSION").to_string(),
        network_config,
    );

    println!("  ✅ API接口初始化成功");
    println!();

    // 配置 VITE_API_BASE_URL 环境变量，指向外网后端URL
    let api_base_url = format!("http://{}:{}/api", bind_address, external_port);
    // std::env::set_var是unsafe的，需要在unsafe块中调用
    unsafe {
        std::env::set_var("VITE_API_BASE_URL", api_base_url);
    }
    println!("  🔧 配置环境变量:");
    println!(
        "    VITE_API_BASE_URL: {}",
        std::env::var("VITE_API_BASE_URL").unwrap()
    );
    println!();

    // 启动服务器
    println!("🖥️ 启动Web服务器...");
    let server_config = ServerConfig {
        host: "0.0.0.0".to_string(),
        port: config.server.port,
        cors_origins: vec!["*".to_string()],
        enable_logging: true,
    };

    println!("  🌐 服务器配置:");
    println!("    主机: {}", server_config.host);
    println!("    端口: {}", server_config.port);
    println!("    CORS: {:?}", server_config.cors_origins);
    println!();

    println!("  🚀 正在启动服务器...");
    api.serve(server_config).await?;

    Ok(())
}
