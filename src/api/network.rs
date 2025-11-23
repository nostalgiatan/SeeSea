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

//! 网络配置模块
//!
//! 提供内网和外网的分离配置

use serde::{Deserialize, Serialize};

/// 网络模式
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NetworkMode {
    /// 内网模式
    Internal,
    /// 外网模式
    External,
    /// 双模式（同时运行内网和外网）
    Dual,
}

/// 内网配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InternalNetworkConfig {
    /// 是否启用
    pub enabled: bool,
    
    /// 监听地址（仅localhost）
    pub host: String,
    
    /// 监听端口
    pub port: u16,
}

impl Default for InternalNetworkConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            host: "127.0.0.1".to_string(),
            port: 8081,
        }
    }
}

/// 外网配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalNetworkConfig {
    /// 是否启用
    pub enabled: bool,
    
    /// 监听地址
    pub host: String,
    
    /// 监听端口
    pub port: u16,
    
    /// CORS允许的源
    pub cors_origins: Vec<String>,
    
    /// 是否启用限流
    pub enable_rate_limit: bool,
    
    /// 是否启用熔断
    pub enable_circuit_breaker: bool,
    
    /// 是否启用IP过滤
    pub enable_ip_filter: bool,
    
    /// 是否启用JWT认证
    pub enable_jwt_auth: bool,
    
    /// 是否启用魔法链接
    pub enable_magic_link: bool,
}

impl Default for ExternalNetworkConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            host: "0.0.0.0".to_string(),
            port: 8080,
            cors_origins: vec!["*".to_string()],
            enable_rate_limit: true,
            enable_circuit_breaker: true,
            enable_ip_filter: true,
            enable_jwt_auth: false, // 默认不启用JWT，避免影响现有用户
            enable_magic_link: true,
        }
    }
}

/// 网络配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    /// 网络模式
    pub mode: NetworkMode,
    
    /// 内网配置
    pub internal: InternalNetworkConfig,
    
    /// 外网配置
    pub external: ExternalNetworkConfig,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            mode: NetworkMode::Dual,
            internal: InternalNetworkConfig::default(),
            external: ExternalNetworkConfig::default(),
        }
    }
}

impl NetworkConfig {
    /// 验证配置
    pub fn validate(&self) -> Result<(), String> {
        match self.mode {
            NetworkMode::Internal => {
                if !self.internal.enabled {
                    return Err("Internal mode selected but internal network is disabled".to_string());
                }
                // 验证内网地址必须是localhost
                if self.internal.host != "127.0.0.1" && self.internal.host != "localhost" {
                    return Err("Internal network must bind to localhost only".to_string());
                }
            }
            NetworkMode::External => {
                if !self.external.enabled {
                    return Err("External mode selected but external network is disabled".to_string());
                }
            }
            NetworkMode::Dual => {
                if !self.internal.enabled && !self.external.enabled {
                    return Err("Dual mode requires at least one network to be enabled".to_string());
                }
                // 验证内网地址必须是localhost
                if self.internal.enabled {
                    if self.internal.host != "127.0.0.1" && self.internal.host != "localhost" {
                        return Err("Internal network must bind to localhost only".to_string());
                    }
                }
            }
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_network_config_default() {
        let config = NetworkConfig::default();
        assert_eq!(config.mode, NetworkMode::Dual);
        assert!(config.internal.enabled);
        assert!(config.external.enabled);
    }

    #[test]
    fn test_internal_network_validation() {
        let mut config = NetworkConfig::default();
        config.mode = NetworkMode::Internal;
        assert!(config.validate().is_ok());
        
        // 内网绑定到非localhost应该失败
        config.internal.host = "0.0.0.0".to_string();
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_dual_mode_validation() {
        let mut config = NetworkConfig::default();
        config.mode = NetworkMode::Dual;
        assert!(config.validate().is_ok());
        
        // 禁用所有网络应该失败
        config.internal.enabled = false;
        config.external.enabled = false;
        assert!(config.validate().is_err());
    }
}
