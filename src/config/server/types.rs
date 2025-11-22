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

//! 服务器配置类型定义

use crate::config::common::ConfigValidationResult;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// 服务器配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    /// 绑定地址
    pub bind_address: String,
    /// 端口号
    pub port: u16,
    /// 是否启用限流
    pub limiter: bool,
    /// 是否为公共实例
    pub public_instance: bool,
    /// 密钥
    pub secret_key: String,
    /// 基础URL
    pub base_url: Option<String>,
    /// 静态文件路径
    pub static_path: Option<PathBuf>,
    /// TLS 配置
    pub tls: Option<TlsConfig>,
    /// 工作线程数（可选，默认为 CPU 核心数）
    pub worker_threads: Option<usize>,
    /// 请求超时时间（秒）
    pub request_timeout: u64,
    /// 最大请求体大小（字节）
    pub max_request_size: usize,
    /// 是否启用压缩
    pub enable_compression: bool,
}

/// TLS 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TlsConfig {
    /// 是否启用 TLS
    pub enabled: bool,
    /// 证书文件路径
    pub cert_path: Option<PathBuf>,
    /// 私钥文件路径
    pub key_path: Option<PathBuf>,
    /// CA 证书路径
    pub ca_path: Option<PathBuf>,
    /// 是否验证客户端证书
    pub verify_client: bool,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            bind_address: "127.0.0.1".to_string(),
            port: 8080,
            limiter: true,
            public_instance: false,
            secret_key: "change-me-in-production".to_string(),
            base_url: None,
            static_path: None,
            tls: None,
            worker_threads: None,
            request_timeout: 30,
            max_request_size: 10 * 1024 * 1024, // 10MB
            enable_compression: true,
        }
    }
}

impl ServerConfig {
    /// 验证服务器配置
    pub fn validate(&self) -> ConfigValidationResult {
        let mut result = ConfigValidationResult::success();

        // 检查端口范围 (u16 最大值为 65535，所以只需检查 0)
        if self.port == 0 {
            result.add_error("端口号必须在 1-65535 范围内".to_string());
        }

        // 检查密钥
        if self.secret_key == "change-me-in-production" && !self.public_instance {
            result.add_warning("生产环境请更改默认密钥".to_string());
        }

        if self.secret_key.len() < 16 {
            result.add_error("密钥长度必须至少 16 个字符".to_string());
        }

        // 检查 TLS 配置
        if let Some(tls) = &self.tls {
            if tls.enabled {
                if tls.cert_path.is_none() {
                    result.add_error("启用 TLS 时必须指定证书文件路径".to_string());
                }
                if tls.key_path.is_none() {
                    result.add_error("启用 TLS 时必须指定私钥文件路径".to_string());
                }
            }
        }

        // 检查请求超时
        if self.request_timeout == 0 {
            result.add_error("请求超时时间必须大于 0".to_string());
        }

        // 检查最大请求大小
        if self.max_request_size == 0 {
            result.add_error("最大请求大小必须大于 0".to_string());
        }

        result
    }

    /// 获取完整的绑定地址
    pub fn bind_address(&self) -> String {
        format!("{}:{}", self.bind_address, self.port)
    }

    /// 获取基础 URL
    pub fn base_url(&self) -> String {
        if let Some(base_url) = &self.base_url {
            base_url.clone()
        } else {
            let protocol = if self.tls.as_ref().map(|t| t.enabled).unwrap_or(false) {
                "https"
            } else {
                "http"
            };
            format!("{}://{}", protocol, self.bind_address())
        }
    }

    /// 是否启用 HTTPS
    pub fn is_https(&self) -> bool {
        self.tls.as_ref().map(|t| t.enabled).unwrap_or(false)
    }
}

impl Default for TlsConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            cert_path: None,
            key_path: None,
            ca_path: None,
            verify_client: false,
        }
    }
}