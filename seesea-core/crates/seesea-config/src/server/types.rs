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

//! 服务器配置类型定义

use crate::ConfigValidationResult;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// 服务器配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    /// 绑定地址
    #[serde(default)]
    pub bind_address: String,
    /// 端口号
    #[serde(default)]
    pub port: u16,
    /// 是否为公共实例
    #[serde(default)]
    pub public_instance: bool,
    /// 密钥
    #[serde(default)]
    pub secret_key: String,
    /// 基础URL
    #[serde(default)]
    pub base_url: Option<String>,
    /// 前端 API 地址配置
    /// 留空表示使用同源（默认，后端驱动前端时使用）
    /// 设置后会覆盖同源配置，适用于 nginx 反向代理等场景
    #[serde(default)]
    pub frontend_api_url: String,
    /// 静态文件路径
    #[serde(default)]
    pub static_path: Option<PathBuf>,
    /// TLS 配置
    #[serde(default)]
    pub tls: Option<TlsConfig>,
    /// 工作线程数（可选，默认为 CPU 核心数）
    #[serde(default)]
    pub worker_threads: Option<usize>,
    /// 请求超时时间（秒）
    #[serde(default)]
    pub request_timeout: u64,
    /// 最大请求体大小（字节）
    #[serde(default)]
    pub max_request_size: usize,
    /// 是否启用压缩
    #[serde(default)]
    pub enable_compression: bool,
}

/// TLS 配置
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TlsConfig {
    /// 是否启用 TLS
    #[serde(default)]
    pub enabled: bool,
    /// 证书文件路径
    #[serde(default)]
    pub cert_path: Option<PathBuf>,
    /// 私钥文件路径
    #[serde(default)]
    pub key_path: Option<PathBuf>,
    /// CA 证书路径
    #[serde(default)]
    pub ca_path: Option<PathBuf>,
    /// 是否验证客户端证书
    #[serde(default)]
    pub verify_client: bool,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            bind_address: "127.0.0.1".to_string(),
            port: 8080,
            public_instance: false,
            secret_key: "change-me-in-production".to_string(),
            base_url: None,
            frontend_api_url: String::new(), // 默认为空，使用同源
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
        if let Some(tls) = &self.tls
            && tls.enabled
        {
            if tls.cert_path.is_none() {
                result.add_error("启用 TLS 时必须指定证书文件路径".to_string());
            }
            if tls.key_path.is_none() {
                result.add_error("启用 TLS 时必须指定私钥文件路径".to_string());
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
