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

//! Tor 网络支持模块
//!
//! 提供 Tor 网络的集成和管理功能，包括：
//! - 自动 Tor 连接检测和验证
//! - 智能电路管理和轮换
//! - 连接池支持
//! - 健壮的错误处理和重试机制

use crate::error::Result;
use crate::net::types::ProxyConfig;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};
use tokio::sync::RwLock;

/// Tor 电路信息
#[derive(Debug, Clone)]
pub struct TorCircuit {
    /// 电路 ID
    pub circuit_id: String,
    /// 路径（节点列表）
    pub path: Vec<String>,
    /// 创建时间
    pub created_at: SystemTime,
    /// 最后使用时间
    pub last_used: Instant,
}

/// Tor 连接状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TorStatus {
    /// 未连接
    Disconnected,
    /// 正在连接
    Connecting,
    /// 已连接
    Connected,
    /// 连接失败
    Failed,
}

/// Tor 连接管理器
pub struct TorManager {
    /// Tor 代理配置
    config: ProxyConfig,
    /// 当前状态
    status: Arc<RwLock<TorStatus>>,
    /// 当前电路信息
    current_circuit: Arc<RwLock<Option<TorCircuit>>>,
    /// 电路最大使用时间（秒）
    circuit_max_age: Duration,
    /// 电路最大请求数
    circuit_max_requests: u32,
    /// 当前电路请求计数
    circuit_request_count: Arc<RwLock<u32>>,
}

impl TorManager {
    /// 创建新的 Tor 管理器
    ///
    /// # 参数
    ///
    /// * `config` - Tor 代理配置
    pub fn new(config: ProxyConfig) -> Self {
        Self {
            config,
            status: Arc::new(RwLock::new(TorStatus::Disconnected)),
            current_circuit: Arc::new(RwLock::new(None)),
            circuit_max_age: Duration::from_secs(600), // 10 minutes
            circuit_max_requests: 100,
            circuit_request_count: Arc::new(RwLock::new(0)),
        }
    }

    /// 配置电路最大使用时间
    pub fn with_circuit_max_age(mut self, max_age: Duration) -> Self {
        self.circuit_max_age = max_age;
        self
    }

    /// 配置电路最大请求数
    pub fn with_circuit_max_requests(mut self, max_requests: u32) -> Self {
        self.circuit_max_requests = max_requests;
        self
    }

    /// 获取当前 Tor 状态
    pub async fn get_status(&self) -> TorStatus {
        *self.status.read().await
    }

    /// 检查 Tor 是否可用
    ///
    /// # 返回
    ///
    /// 如果 Tor 可用返回 true，否则返回 false
    pub async fn is_tor_available(&self) -> bool {
        // 更新状态为正在连接
        *self.status.write().await = TorStatus::Connecting;

        // 尝试连接到 Tor 代理
        let available = crate::net::client::proxy::check_proxy(&self.config)
            .await
            .unwrap_or(false);

        // 更新状态
        *self.status.write().await = if available {
            TorStatus::Connected
        } else {
            TorStatus::Failed
        };

        available
    }

    /// 检查是否需要轮换电路
    async fn should_rotate_circuit(&self) -> bool {
        let circuit = self.current_circuit.read().await;
        let count = *self.circuit_request_count.read().await;

        if let Some(circuit_info) = circuit.as_ref() {
            // 检查电路年龄 - handle system time going backwards gracefully
            let age = SystemTime::now()
                .duration_since(circuit_info.created_at)
                .unwrap_or_else(|_| Duration::from_secs(0)); // If time goes backwards, treat as age 0
            
            if age > self.circuit_max_age {
                return true;
            }

            // 检查请求计数
            if count >= self.circuit_max_requests {
                return true;
            }
        }

        false
    }

    /// 请求新的 Tor 电路（更换 IP）
    ///
    /// # 返回
    ///
    /// 成功返回 Ok(())，失败返回错误
    ///
    /// # 注意
    ///
    /// 需要 Tor 控制端口（默认 9051）开启并配置认证
    pub async fn new_circuit(&self) -> Result<()> {
        // Tor 的新电路请求需要通过控制端口（默认 9051）
        // 发送 SIGNAL NEWNYM 命令
        
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        use tokio::net::TcpStream;
        use tokio::time::timeout;
        
        // 提取控制端口地址（假设格式为 host:port）
        let control_addr = self.config.address.replace(":9050", ":9051");
        
        // 设置超时时间为10秒
        let connect_timeout = Duration::from_secs(10);
        
        // 连接到 Tor 控制端口（带超时）
        let mut stream = timeout(
            connect_timeout,
            TcpStream::connect(&control_addr)
        )
            .await
            .map_err(|_| crate::error::network_error("Connection to Tor control port timed out"))?
            .map_err(|e| crate::error::network_error(format!("Failed to connect to Tor control port: {}", e)))?;
        
        // 发送 AUTHENTICATE 命令（空密码）
        stream.write_all(b"AUTHENTICATE \"\"\r\n")
            .await
            .map_err(|e| crate::error::network_error(format!("Failed to authenticate: {}", e)))?;
        
        // 读取认证响应（带超时）
        let mut auth_response = vec![0u8; 1024];
        let n = timeout(
            Duration::from_secs(5),
            stream.read(&mut auth_response)
        )
            .await
            .map_err(|_| crate::error::network_error("Reading auth response timed out"))?
            .map_err(|e| crate::error::network_error(format!("Failed to read auth response: {}", e)))?;
        
        let response = String::from_utf8_lossy(&auth_response[..n]);
        if !response.starts_with("250") {
            return Err(crate::error::network_error(format!("Authentication failed: {}", response)));
        }
        
        // 发送 SIGNAL NEWNYM 命令请求新电路
        stream.write_all(b"SIGNAL NEWNYM\r\n")
            .await
            .map_err(|e| crate::error::network_error(format!("Failed to send NEWNYM signal: {}", e)))?;
        
        // 读取响应（带超时）
        let mut signal_response = vec![0u8; 1024];
        let n = timeout(
            Duration::from_secs(5),
            stream.read(&mut signal_response)
        )
            .await
            .map_err(|_| crate::error::network_error("Reading signal response timed out"))?
            .map_err(|e| crate::error::network_error(format!("Failed to read signal response: {}", e)))?;
        
        let response = String::from_utf8_lossy(&signal_response[..n]);
        if !response.starts_with("250") {
            return Err(crate::error::network_error(format!("NEWNYM signal failed: {}", response)));
        }

        // 重置电路信息
        *self.current_circuit.write().await = Some(TorCircuit {
            circuit_id: format!("circuit_{}", SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs()),
            path: vec![],
            created_at: SystemTime::now(),
            last_used: Instant::now(),
        });
        *self.circuit_request_count.write().await = 0;
        
        Ok(())
    }

    /// 记录一次请求（用于电路管理）
    pub async fn record_request(&self) {
        let mut count = self.circuit_request_count.write().await;
        *count += 1;

        // Update last_used timestamp
        if let Some(circuit) = self.current_circuit.write().await.as_mut() {
            circuit.last_used = Instant::now();
        }

        // 检查是否需要轮换电路
        if self.should_rotate_circuit().await {
            // 异步轮换电路（记录错误但不阻塞）
            let manager = self.clone();
            tokio::spawn(async move {
                if let Err(e) = manager.new_circuit().await {
                    // Log error - in production this should use proper logging
                    eprintln!("Warning: Failed to rotate Tor circuit: {}", e);
                }
            });
        }
    }

    /// 获取当前 Tor IP 地址
    ///
    /// # 返回
    ///
    /// 成功返回 IP 地址，失败返回错误
    ///
    /// # 注意
    ///
    /// 通过访问 https://api.ipify.org 获取外部 IP
    pub async fn get_current_ip(&self) -> Result<String> {
        use reqwest::Client;
        use tokio::time::timeout;
        
        // 创建使用 Tor 代理的 HTTP 客户端
        let proxy_url = format!("socks5://{}", self.config.address);
        let proxy = reqwest::Proxy::all(&proxy_url)
            .map_err(|e| crate::error::network_error(format!("Failed to create proxy: {}", e)))?;
        
        let client = Client::builder()
            .proxy(proxy)
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|e| crate::error::network_error(format!("Failed to build client: {}", e)))?;
        
        // 通过 IP 查询服务获取当前 IP（带重试机制）
        let mut last_error = None;
        for _ in 0..3 {
            match timeout(
                Duration::from_secs(30),
                client.get("https://api.ipify.org").send()
            ).await {
                Ok(Ok(response)) => {
                    let ip = response
                        .text()
                        .await
                        .map_err(|e| crate::error::network_error(format!("Failed to read response: {}", e)))?;
                    return Ok(ip.trim().to_string());
                }
                Ok(Err(e)) => {
                    last_error = Some(format!("Request failed: {}", e));
                    tokio::time::sleep(Duration::from_secs(2)).await;
                }
                Err(_) => {
                    // Timeout - preserve error information
                    if last_error.is_none() {
                        last_error = Some("Request timed out".to_string());
                    }
                    tokio::time::sleep(Duration::from_secs(2)).await;
                }
            }
        }

        Err(crate::error::network_error(format!(
            "Failed to get IP after 3 retries: {}",
            last_error.unwrap_or_else(|| "unknown error".to_string())
        )))
    }

    /// 验证是否通过 Tor 连接
    ///
    /// # 返回
    ///
    /// 如果通过 Tor 连接返回 true，否则返回 false
    pub async fn verify_tor_connection(&self) -> bool {
        self.is_tor_available().await
    }
}

// 实现Clone trait以支持在tokio::spawn中使用
impl Clone for TorManager {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            status: Arc::clone(&self.status),
            current_circuit: Arc::clone(&self.current_circuit),
            circuit_max_age: self.circuit_max_age,
            circuit_max_requests: self.circuit_max_requests,
            circuit_request_count: Arc::clone(&self.circuit_request_count),
        }
    }
}

impl Default for TorManager {
    fn default() -> Self {
        let mut config = ProxyConfig::default();
        config.proxy_type = crate::net::types::ProxyType::Tor;
        config.address = "127.0.0.1:9050".to_string();
        Self::new(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tor_manager_new() {
        let config = ProxyConfig::default();
        let manager = TorManager::new(config);
        assert_eq!(manager.config.address, "127.0.0.1:8080");
    }

    #[test]
    fn test_tor_manager_default() {
        let manager = TorManager::default();
        assert_eq!(manager.config.address, "127.0.0.1:9050");
        assert_eq!(manager.config.proxy_type, crate::net::types::ProxyType::Tor);
    }

    #[tokio::test]
    async fn test_tor_manager_status() {
        let manager = TorManager::default();
        let status = manager.get_status().await;
        assert_eq!(status, TorStatus::Disconnected);
    }

    #[tokio::test]
    async fn test_tor_manager_configuration() {
        use std::time::Duration;
        let manager = TorManager::default()
            .with_circuit_max_age(Duration::from_secs(300))
            .with_circuit_max_requests(50);
        assert_eq!(manager.circuit_max_age, Duration::from_secs(300));
        assert_eq!(manager.circuit_max_requests, 50);
    }

    #[tokio::test]
    #[ignore] // Requires actual Tor daemon
    async fn test_tor_manager_new_circuit() {
        let manager = TorManager::default();
        let result = manager.new_circuit().await;
        // This will fail without Tor running, but shouldn't panic
        assert!(result.is_err());
    }

    #[tokio::test]
    #[ignore] // Requires actual Tor daemon and internet
    async fn test_tor_manager_get_current_ip() {
        let manager = TorManager::default();
        let result = manager.get_current_ip().await;
        // This will fail without Tor running
        assert!(result.is_err() || result.unwrap().len() > 0);
    }

    #[tokio::test]
    async fn test_tor_circuit_creation() {
        let circuit = TorCircuit {
            circuit_id: "test_circuit".to_string(),
            path: vec!["node1".to_string(), "node2".to_string()],
            created_at: SystemTime::now(),
            last_used: Instant::now(),
        };
        assert_eq!(circuit.circuit_id, "test_circuit");
        assert_eq!(circuit.path.len(), 2);
    }
}
