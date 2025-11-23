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

//! 简单API服务器示例
//!
//! 展示如何启动一个带基本安全特性的外网API服务器

use seesea_core::api::{ApiInterface, NetworkConfig, NetworkMode};
use seesea_core::search::SearchInterface;
use std::sync::Arc;

#[tokio::main]
async fn main() {
    // 初始化日志
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    println!("🌊 SeeSea 简单API服务器示例");
    println!();

    // 创建网络配置（外网模式，基本安全特性）
    let mut network_config = NetworkConfig::default();
    network_config.mode = NetworkMode::External;
    network_config.external.enabled = true;
    network_config.external.host = "0.0.0.0".to_string();
    network_config.external.port = 8080;
    network_config.external.enable_rate_limit = true;
    network_config.external.enable_circuit_breaker = true;
    network_config.external.enable_ip_filter = false;
    network_config.external.enable_jwt_auth = false;
    network_config.external.enable_magic_link = true;

    // 验证配置
    if let Err(e) = network_config.validate() {
        eprintln!("配置验证失败: {}", e);
        return;
    }

    // 创建搜索接口
    let search_config = seesea_core::search::SearchConfig::default();
    let search = match SearchInterface::new(search_config) {
        Ok(s) => Arc::new(s),
        Err(e) => {
            eprintln!("创建搜索接口失败: {}", e);
            return;
        }
    };

    // 创建API接口
    let api = ApiInterface::with_network_config(
        search,
        env!("CARGO_PKG_VERSION").to_string(),
        network_config,
    );

    println!("🚀 服务器启动中...");
    println!("   访问 http://localhost:8080/api/health 检查服务器状态");
    println!("   访问 http://localhost:8080/api/metrics 查看Prometheus指标");
    println!("   访问 http://localhost:8080/api/metrics/realtime 查看实时指标");
    println!();

    // 启动服务器（这会阻塞）
    let server_config = seesea_core::api::ServerConfig::default();
    if let Err(e) = api.serve(server_config).await {
        eprintln!("服务器错误: {}", e);
    }
}
