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

//! HTTP 客户端模块
//!
//! 提供基于 reqwest 的强大 HTTP 客户端封装

pub mod pool;
pub mod proxy;
pub mod tls;

use crate::error::Result;
use crate::net::types::{NetworkConfig, RequestOptions};
use crate::net::privacy::PrivacyManager;
use reqwest::{Client, ClientBuilder, Response};
use std::sync::Arc;
use std::time::Duration;

/// HTTP 客户端封装
#[derive(Clone)]
pub struct HttpClient {
    /// 底层 reqwest 客户端
    client: Arc<Client>,
    /// 网络配置
    config: Arc<NetworkConfig>,
    /// 隐私管理器
    privacy_manager: Option<Arc<PrivacyManager>>,
}

impl HttpClient {
    /// 从网络配置创建新的 HTTP 客户端
    ///
    /// # 参数
    ///
    /// * `config` - 网络配置
    ///
    /// # 返回
    ///
    /// 成功返回配置好的 HttpClient，失败返回错误
    pub fn new(config: NetworkConfig) -> Result<Self> {
        let mut builder = ClientBuilder::new();

        // 配置连接池
        builder = builder
            .pool_max_idle_per_host(config.pool.max_idle_connections)
            .pool_idle_timeout(Some(Duration::from_secs(config.pool.idle_timeout_secs)));

        // 配置 HTTP/2
        if config.pool.http2_only {
            builder = builder.http2_prior_knowledge();
        }

        // 配置 TLS
        builder = tls::configure_tls(builder, &config.tls)?;

        // 配置代理
        if config.proxy.enabled {
            builder = proxy::configure_proxy(builder, &config.proxy)?;
        }

        // 配置隐私保护
        builder = crate::net::privacy::headers::configure_privacy(builder, &config.privacy);

        // 创建隐私管理器
        let privacy_manager = Arc::new(PrivacyManager::new(
            config.privacy.clone(),
            config.tls.clone(),
            config.doh.clone(),
        ));

        // 构建客户端
        let client = builder
            .build()
            .map_err(|e| crate::error::network_error(format!("Failed to build HTTP client: {}", e)))?;

        Ok(Self {
            client: Arc::new(client),
            config: Arc::new(config),
            privacy_manager: Some(privacy_manager),
        })
    }

    /// 获取隐私管理器
    pub fn privacy_manager(&self) -> Option<&Arc<PrivacyManager>> {
        self.privacy_manager.as_ref()
    }

    /// 发送 GET 请求
    ///
    /// # 参数
    ///
    /// * `url` - 请求 URL
    /// * `options` - 请求选项（可选）
    ///
    /// # 返回
    ///
    /// 成功返回 HTTP 响应，失败返回错误
    pub async fn get(&self, url: &str, options: Option<RequestOptions>) -> Result<Response> {
        let opts = options.unwrap_or_default();
        
        let mut request = self.client
            .get(url)
            .timeout(opts.timeout);

        // 添加隐私保护请求头
        if let Some(ref privacy_mgr) = self.privacy_manager {
            let privacy_headers = privacy_mgr.get_privacy_headers(url).await;
            for (key, value) in privacy_headers {
                request = request.header(&key, &value);
            }
        }

        // 添加自定义请求头（会覆盖隐私头）
        for (key, value) in opts.headers {
            request = request.header(&key, &value);
        }

        // 发送请求
        request
            .send()
            .await
            .map_err(|e| {
                crate::error::network_error(format!("GET request failed: {}", e))
            })
    }

    /// 发送 POST 请求
    ///
    /// # 参数
    ///
    /// * `url` - 请求 URL
    /// * `body` - 请求体
    /// * `options` - 请求选项（可选）
    ///
    /// # 返回
    ///
    /// 成功返回 HTTP 响应，失败返回错误
    pub async fn post(&self, url: &str, body: Vec<u8>, options: Option<RequestOptions>) -> Result<Response> {
        let opts = options.unwrap_or_default();
        
        let mut request = self.client
            .post(url)
            .timeout(opts.timeout)
            .body(body);

        // 添加隐私保护请求头
        if let Some(ref privacy_mgr) = self.privacy_manager {
            let privacy_headers = privacy_mgr.get_privacy_headers(url).await;
            for (key, value) in privacy_headers {
                request = request.header(&key, &value);
            }
        }

        // 添加自定义请求头（会覆盖隐私头）
        for (key, value) in opts.headers {
            request = request.header(&key, &value);
        }

        // 发送请求
        request
            .send()
            .await
            .map_err(|e| crate::error::network_error(format!("POST request failed: {}", e)))
    }

    /// 发送 POST JSON 请求
    ///
    /// # 参数
    ///
    /// * `url` - 请求 URL
    /// * `json` - JSON 数据（实现了 Serialize trait）
    /// * `options` - 请求选项（可选）
    ///
    /// # 返回
    ///
    /// 成功返回 HTTP 响应，失败返回错误
    pub async fn post_json<T: serde::Serialize>(&self, url: &str, json: &T, options: Option<RequestOptions>) -> Result<Response> {
        let opts = options.unwrap_or_default();
        
        let mut request = self.client
            .post(url)
            .timeout(opts.timeout)
            .json(json);

        // 添加自定义请求头
        for (key, value) in opts.headers {
            request = request.header(&key, &value);
        }

        // 发送请求
        request
            .send()
            .await
            .map_err(|e| crate::error::network_error(format!("POST JSON request failed: {}", e)))
    }

    /// 获取网络配置
    pub fn config(&self) -> &NetworkConfig {
        &self.config
    }

    /// 获取底层 reqwest 客户端（用于高级用途）
    pub fn inner(&self) -> &Client {
        &self.client
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_http_client_creation() {
        let config = NetworkConfig::default();
        let client = HttpClient::new(config);
        assert!(client.is_ok());
    }

    #[test]
    fn test_http_client_config_access() {
        let config = NetworkConfig::default();
        let client = HttpClient::new(config.clone()).unwrap();
        assert_eq!(client.config().pool.max_idle_connections, config.pool.max_idle_connections);
    }
}
