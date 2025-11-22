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

//! 服务器配置外部接口

use crate::config::server::ServerConfig;

/// 创建默认服务器配置
pub fn default() -> ServerConfig {
    ServerConfig::default()
}

/// 创建开发环境服务器配置
pub fn development() -> ServerConfig {
    let mut config = ServerConfig::default();
    config.port = 3000;
    config.host = "127.0.0.1".to_string();
    config.debug = true;
    config
}

/// 创建生产环境服务器配置
pub fn production() -> ServerConfig {
    let mut config = ServerConfig::default();
    config.port = 8080;
    config.host = "0.0.0.0".to_string();
    config.debug = false;
    config.tls.enabled = true;
    config
}

/// 验证服务器配置
pub fn validate(config: &ServerConfig) -> crate::config::common::ConfigValidationResult {
    let mut result = crate::config::common::ConfigValidationResult::valid();

    // 验证端口范围
    if config.port < 1 || config.port > 65535 {
        result.add_error("服务器端口必须在 1-65535 范围内".to_string());
    }

    // 验证主机地址
    if config.host.is_empty() {
        result.add_error("服务器主机地址不能为空".to_string());
    }

    // 生产环境验证
    if !config.debug {
        if config.secret_key == "change-me-in-production" {
            result.add_error("生产环境必须设置安全的密钥".to_string());
        }

        if !config.tls.enabled {
            result.add_warning("生产环境建议启用 TLS".to_string());
        }
    }

    result
}