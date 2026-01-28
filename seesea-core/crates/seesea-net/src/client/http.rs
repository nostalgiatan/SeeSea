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

//! HTTP 客户端核心实现
//!
//! 提供基于 reqwest 的强大 HTTP 客户端封装
//!
//! # 特性
//!
//! - **异步设计**：所有公共方法都是异步的，便于在 tokio 任务中使用
//! - **单例模式**：支持全局单例实例，便于在应用程序中共享
//! - **隐私保护**：支持 User-Agent 轮换、请求头伪造等隐私保护功能
//! - **代理支持**：支持 HTTP、HTTPS、SOCKS5、Tor 代理
//! - **TLS 配置**：支持 TLS 指纹混淆，提高隐私保护
//! - **指标监控**：提供请求统计、响应时间等指标
//! - **可配置**：支持灵活的配置选项
//!
//! # 单例模式使用示例
//!
//! ```rust,no_run
//! use seesea::seesea_seesea_net::client::HttpClient;
//! use seesea::seesea_seesea_net::seesea_seesea_config::NetworkConfig;
//!
//! // 获取或创建全局单例实例（使用默认配置）
//! let http_client = HttpClient::instance().unwrap();
//!
//! // 使用指定配置获取或创建全局单例实例
//! let config = NetworkConfig::default();
//! let http_client = HttpClient::instance_with_config(config).unwrap();
//!
//! // 发送 GET 请求
//! let response = http_client.get("https://example.com", None).await.unwrap();
//! ```
//!
//! # 普通实例使用示例
//!
//! ```rust,no_run
//! use seesea::seesea_seesea_net::client::HttpClient;
//! use seesea::seesea_seesea_net::seesea_seesea_config::NetworkConfig;
//!
//! // 创建普通实例
//! let config = NetworkConfig::default();
//! let http_client = HttpClient::new(config).unwrap();
//!
//! // 发送 GET 请求
//! let response = http_client.get("https://example.com", None).await.unwrap();
//! ```

use crate::metrics::MetricsCollector;
use crate::privacy::PrivacyManager;
use crate::retry::RetryConfig;
use seesea_config::{NetworkConfig, RequestOptions};
use seesea_errors::Result;

use futures::StreamExt;
use once_cell::sync::OnceCell;
use reqwest::{Client, ClientBuilder, Response};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio_util::io::ReaderStream;
use tokio_util::io::StreamReader;

/// HTTP 客户端状态，包含可动态更新的字段
#[derive(Clone)]
pub struct HttpClientState {
    /// 底层 reqwest 客户端（用于普通请求，带自动解压缩）
    client: Arc<Client>,
    /// 底层 reqwest 客户端（用于流式请求，不带自动解压缩）
    stream_client: Arc<Client>,
    /// 网络配置
    config: Arc<NetworkConfig>,
    /// 隐私管理器
    privacy_manager: Option<Arc<PrivacyManager>>,
}

/// HTTP 客户端封装
#[derive(Clone)]
pub struct HttpClient {
    /// 客户端状态，使用 RwLock 保护，支持动态更新
    state: Arc<RwLock<HttpClientState>>,
    /// 重试配置
    retry_config: RetryConfig,
    /// 指标收集器
    metrics_collector: Arc<MetricsCollector>,
}

/// 全局 HttpClient 单例
static GLOBAL_HTTP_CLIENT: OnceCell<Arc<HttpClient>> = OnceCell::new();

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
        Self::new_with_config(config)
    }

    /// 从网络配置创建新的 HTTP 客户端
    ///
    /// # 参数
    ///
    /// * `network_config` - 网络配置
    ///
    /// # 返回
    ///
    /// 成功返回配置好的 HttpClient，失败返回错误
    pub fn from_network_config(network_config: NetworkConfig) -> Result<Self> {
        Self::new_with_config(network_config)
    }

    /// 从网络配置创建新的 HTTP 客户端（内部实现）
    ///
    /// # 参数
    ///
    /// * `config` - 网络配置
    ///
    /// # 返回
    ///
    /// 成功返回配置好的 HttpClient，失败返回错误
    fn new_with_config(config: NetworkConfig) -> Result<Self> {
        // 创建基础配置构建器
        let create_client = |enable_decompression, config: &NetworkConfig| -> Result<Client> {
            let mut builder = ClientBuilder::new();

            // 配置超时
            builder = builder
                .timeout(Duration::from_secs(config.pool.read_timeout_secs))
                .connect_timeout(Duration::from_secs(config.pool.connect_timeout_secs));

            // 配置连接池
            builder = builder
                .pool_max_idle_per_host(config.pool.max_idle_connections)
                .pool_idle_timeout(Some(Duration::from_secs(config.pool.idle_timeout_secs)));

            // 配置 HTTP/2
            if config.pool.http2_only {
                builder = builder.http2_prior_knowledge();
            }

            // 配置 TCP_NODELAY
            builder = builder.tcp_nodelay(config.pool.tcp_nodelay);

            // 配置 TCP 保活
            if let Some(interval) = config.pool.tcp_keepalive_interval_secs {
                builder = builder.tcp_keepalive_interval(Duration::from_secs(interval));
            }

            if let Some(retries) = config.pool.tcp_keepalive_retries {
                builder = builder.tcp_keepalive_retries(retries);
            }

            // 配置 TLS
            builder = super::tls::configure_tls(builder, &config.tls)?;

            // 配置代理
            if config.proxy.enabled {
                builder = super::proxy::configure_proxy(builder, &config.proxy)?;
            }

            // 添加隐私保护请求头
            builder = crate::privacy::headers::configure_privacy(builder, &Default::default());

            // 配置解压缩
            if enable_decompression {
                // reqwest 0.12+ 默认启用压缩，无需显式设置
            } else {
                // 明确禁用所有压缩算法，确保获取原始字节流
                builder = builder.no_brotli().no_deflate().no_gzip();
            }

            // 构建客户端
            builder.build().map_err(|e| {
                seesea_errors::http_error(0, &format!("Failed to build HTTP client: {e}"))
            })
        };

        // 创建隐私管理器
        let privacy_manager = Arc::new(PrivacyManager::new(
            config.privacy.clone(),
            config.tls.clone(),
            config.dns.clone(),
        ));

        // 创建普通请求客户端（带自动解压缩）
        let client = create_client(true, &config)?;
        // 创建流式请求客户端（不带自动解压缩）
        let stream_client = create_client(false, &config)?;

        // 创建客户端状态
        let state = HttpClientState {
            client: Arc::new(client),
            stream_client: Arc::new(stream_client),
            config: Arc::new(config),
            privacy_manager: Some(privacy_manager),
        };

        Ok(Self {
            state: Arc::new(RwLock::new(state)),
            retry_config: RetryConfig::default(),
            metrics_collector: Arc::new(MetricsCollector::new()),
        })
    }

    /// 获取隐私管理器
    pub async fn privacy_manager(&self) -> Option<Arc<PrivacyManager>> {
        let state = self.state.read().await;
        state.privacy_manager.clone()
    }

    /// 动态更新网络配置
    ///
    /// # 参数
    ///
    /// * `config` - 新的网络配置
    ///
    /// # 返回
    ///
    /// 成功返回 Ok，失败返回错误
    pub async fn update_config(&self, config: NetworkConfig) -> Result<()> {
        // 创建基础配置构建器
        let create_client = |enable_decompression| -> Result<Client> {
            let mut builder = ClientBuilder::new();

            // 配置超时
            builder = builder
                .timeout(Duration::from_secs(config.pool.read_timeout_secs))
                .connect_timeout(Duration::from_secs(config.pool.connect_timeout_secs));

            // 配置连接池
            builder = builder
                .pool_max_idle_per_host(config.pool.max_idle_connections)
                .pool_idle_timeout(Some(Duration::from_secs(config.pool.idle_timeout_secs)));

            // 配置 HTTP/2
            if config.pool.http2_only {
                builder = builder.http2_prior_knowledge();
            }

            // 配置 TCP_NODELAY
            builder = builder.tcp_nodelay(config.pool.tcp_nodelay);

            // 配置 TCP 保活
            if let Some(interval) = config.pool.tcp_keepalive_interval_secs {
                builder = builder.tcp_keepalive_interval(Duration::from_secs(interval));
            }

            if let Some(retries) = config.pool.tcp_keepalive_retries {
                builder = builder.tcp_keepalive_retries(retries);
            }

            // 配置 TLS
            builder = super::tls::configure_tls(builder, &config.tls)?;

            // 配置代理
            if config.proxy.enabled {
                builder = super::proxy::configure_proxy(builder, &config.proxy)?;
            }

            // 添加隐私保护请求头
            builder = crate::privacy::headers::configure_privacy(builder, &Default::default());

            // 配置解压缩
            if enable_decompression {
                // reqwest 0.12+ 默认启用压缩，无需显式设置
            } else {
                // 明确禁用所有压缩算法，确保获取原始字节流
                builder = builder.no_brotli().no_deflate().no_gzip();
            }

            // 构建客户端
            builder.build().map_err(|e| {
                seesea_errors::http_error(0, &format!("Failed to build HTTP client: {e}"))
            })
        };

        // 创建隐私管理器
        let privacy_manager = Arc::new(PrivacyManager::new(
            config.privacy.clone(),
            config.tls.clone(),
            config.dns.clone(),
        ));

        // 创建普通请求客户端（带自动解压缩）
        let client = create_client(true)?;
        // 创建流式请求客户端（不带自动解压缩）
        let stream_client = create_client(false)?;

        // 更新客户端状态
        let mut state = self.state.write().await;
        *state = HttpClientState {
            client: Arc::new(client),
            stream_client: Arc::new(stream_client),
            config: Arc::new(config),
            privacy_manager: Some(privacy_manager),
        };

        Ok(())
    }

    /// 获取全局 HttpClient 单例
    ///
    /// # 返回
    ///
    /// 返回全局 HttpClient 实例
    pub async fn instance() -> Result<Arc<Self>> {
        // 尝试从全局配置创建客户端
        if let Some(project_config) = seesea_config::get_config().await {
            let client = Self::from_network_config(project_config.network)?;
            Ok(GLOBAL_HTTP_CLIENT.get_or_init(|| Arc::new(client)).clone())
        } else {
            // 如果全局配置未初始化，使用默认配置
            let config = NetworkConfig::default();
            let client = Self::new(config)?;
            Ok(GLOBAL_HTTP_CLIENT.get_or_init(|| Arc::new(client)).clone())
        }
    }

    /// 使用指定配置获取或创建全局 HttpClient 单例
    ///
    /// # 参数
    ///
    /// * `config` - 网络配置
    ///
    /// # 返回
    ///
    /// 返回全局 HttpClient 实例
    pub fn instance_with_config(config: NetworkConfig) -> Result<Arc<Self>> {
        let client = Self::new(config)?;
        Ok(GLOBAL_HTTP_CLIENT.get_or_init(|| Arc::new(client)).clone())
    }

    /// 使用项目级配置获取或创建全局 HttpClient 单例
    ///
    /// # 参数
    ///
    /// * `project_config` - 项目级配置
    ///
    /// # 返回
    ///
    /// 返回全局 HttpClient 实例
    pub fn instance_with_network_config(network_config: NetworkConfig) -> Result<Arc<Self>> {
        let client = Self::from_network_config(network_config)?;
        Ok(GLOBAL_HTTP_CLIENT.get_or_init(|| Arc::new(client)).clone())
    }

    /// 设置全局 HttpClient 单例
    ///
    /// # 参数
    ///
    /// * `client` - HttpClient 实例
    ///
    /// # 返回
    ///
    /// 如果单例已存在，返回 Err，否则返回 Ok
    pub fn set_instance(client: Self) -> Result<()> {
        GLOBAL_HTTP_CLIENT
            .set(Arc::new(client))
            .map_err(|_| seesea_errors::http_error(0, "Global HTTP client instance already exists"))
    }

    /// 清除全局 HttpClient 单例
    ///
    /// # 注意
    ///
    /// 由于 OnceCell 的限制，此方法目前不支持清除已初始化的单例
    /// 后续可以考虑使用其他同步机制来实现此功能
    #[deprecated(
        note = "Clearing the global instance is not supported due to OnceCell limitations"
    )]
    pub fn clear_instance() {
        // 由于 OnceCell 的限制，无法清除已初始化的单例
        // 此方法保留用于向后兼容性
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
        let start_time = self.metrics_collector.start_request();
        let opts = options.unwrap_or_default();

        let state = self.state.read().await;
        let mut request = state
            .client
            .get(url)
            .timeout(Duration::from_secs(opts.timeout_secs.unwrap_or(30)));

        // 添加隐私保护请求头
        if let Some(ref privacy_mgr) = state.privacy_manager {
            let privacy_headers = privacy_mgr.get_privacy_headers(url).await;
            for (key, value) in privacy_headers {
                request = request.header(key.as_str(), value.as_str());
            }
        }

        // 添加自定义请求头
        if let Some(headers_map) = &opts.headers {
            for (key, value) in headers_map {
                request = request.header(key.as_str(), value.as_str());
            }
        }
        // 非流处理请求：允许服务器返回压缩数据，client会自动解压缩

        // 发送请求
        let result = request.send().await;

        // 记录指标
        match &result {
            Ok(_) => {
                self.metrics_collector
                    .record_successful_request(start_time)
                    .await;
            }
            Err(_) => {
                self.metrics_collector
                    .record_failed_request(start_time)
                    .await;
            }
        }

        // 处理结果
        result.map_err(|e| seesea_errors::http_error(0, &format!("GET request failed: {e}")))
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
    pub async fn post(
        &self,
        url: &str,
        body: Vec<u8>,
        options: Option<RequestOptions>,
    ) -> Result<Response> {
        let start_time = self.metrics_collector.start_request();
        let opts = options.unwrap_or_default();

        // 使用stream_client，它禁用了自动解压缩，确保获取原始二进制数据
        let state = self.state.read().await;
        let mut request = state
            .stream_client
            .post(url)
            .timeout(Duration::from_secs(opts.timeout_secs.unwrap_or(30)))
            .body(body);

        // 添加隐私保护请求头
        if let Some(ref privacy_mgr) = state.privacy_manager {
            let privacy_headers = privacy_mgr.get_privacy_headers(url).await;
            for (key, value) in privacy_headers {
                request = request.header(key.as_str(), value.as_str());
            }
        }

        // 添加自定义请求头（会覆盖隐私头）
        if let Some(headers_map) = &opts.headers {
            for (key, value) in headers_map {
                request = request.header(key.as_str(), value.as_str());
            }
        }
        // 非流处理请求：使用默认的Accept-Encoding设置，允许服务器返回压缩数据
        // reqwest会自动解压缩响应体

        // 发送请求
        let result = request.send().await;

        // 记录指标
        match &result {
            Ok(_) => {
                self.metrics_collector
                    .record_successful_request(start_time)
                    .await;
            }
            Err(_) => {
                self.metrics_collector
                    .record_failed_request(start_time)
                    .await;
            }
        }

        // 处理结果
        result.map_err(|e| seesea_errors::http_error(0, &format!("POST request failed: {e}")))
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
    pub async fn post_json<T: serde::Serialize>(
        &self,
        url: &str,
        json: &T,
        options: Option<RequestOptions>,
    ) -> Result<Response> {
        let start_time = self.metrics_collector.start_request();
        let opts = options.unwrap_or_default();

        let state = self.state.read().await;
        let mut request = state
            .client
            .post(url)
            .timeout(Duration::from_secs(opts.timeout_secs.unwrap_or(30)))
            .json(json);

        // 添加自定义请求头
        if let Some(headers_map) = &opts.headers {
            for (key, value) in headers_map {
                request = request.header(key.as_str(), value.as_str());
            }
        }

        // 发送请求
        let result = request.send().await;

        // 记录指标
        match &result {
            Ok(_) => {
                self.metrics_collector
                    .record_successful_request(start_time)
                    .await;
            }
            Err(_) => {
                self.metrics_collector
                    .record_failed_request(start_time)
                    .await;
            }
        }

        // 处理结果
        result.map_err(|e| seesea_errors::http_error(0, &format!("POST JSON request failed: {e}")))
    }

    /// 流式 GET 请求（支持零拷贝）
    ///
    /// 此方法用于下载大文件，使用零拷贝技术，避免将整个响应体加载到内存中
    ///
    /// # 参数
    ///
    /// * `url` - 请求 URL
    /// * `options` - 请求选项（可选）
    ///
    /// # 返回
    ///
    /// 成功返回包含状态码、响应头和异步读取器的元组，失败返回错误
    pub async fn get_stream(
        &self,
        url: &str,
        options: Option<RequestOptions>,
    ) -> Result<(
        u16,
        reqwest::header::HeaderMap,
        impl tokio::io::AsyncRead + Unpin + Send + 'static,
    )> {
        let start_time = self.metrics_collector.start_request();
        let mut opts = options.unwrap_or_default();

        // 为流式请求设置更长的默认超时时间（5分钟）
        if opts.timeout_secs.unwrap_or(30) < 300 {
            opts.timeout_secs = Some(300);
        }

        // 使用独立的stream_client（完全禁用自动解压缩），确保获取原始字节流
        let state = self.state.read().await;
        let mut request = state
            .stream_client
            .get(url)
            .timeout(Duration::from_secs(opts.timeout_secs.unwrap_or(30)));

        // 添加隐私保护请求头
        if let Some(ref privacy_mgr) = state.privacy_manager {
            let privacy_headers = privacy_mgr.get_privacy_headers(url).await;
            for (key, value) in privacy_headers {
                request = request.header(key.as_str(), value.as_str());
            }
        }

        // 添加自定义请求头（会覆盖隐私头）
        if let Some(headers_map) = &opts.headers {
            for (key, value) in headers_map {
                request = request.header(key.as_str(), value.as_str());
            }
        }

        // 明确禁用压缩，确保获取原始二进制数据
        request = request.header("Accept-Encoding", "identity");

        // 确保不自动解码响应体，直接获取原始字节流
        let response = request.send().await.map_err(|e| {
            seesea_errors::http_error(0, &format!("GET stream request failed: {e}"))
        })?;

        // 手动检查状态码，避免使用 error_for_status() 尝试解码响应体
        let status = response.status();
        if !status.is_success() {
            return Err(seesea_errors::http_error(
                status.as_u16(),
                &format!("GET stream request failed with status: {status}"),
            ));
        }

        // 记录指标
        self.metrics_collector
            .record_successful_request(start_time)
            .await;

        // 保存状态码和响应头
        let status = response.status().as_u16();
        let headers = response.headers().clone();

        // 获取响应体流，使用raw()方法确保获取原始字节流
        let stream = response.bytes_stream();

        // 转换为tokio AsyncRead，直接传递原始字节流
        let reader =
            StreamReader::new(stream.map(|result| result.map_err(tokio::io::Error::other)));

        Ok((status, headers, reader))
    }

    /// 流式 POST 请求（支持零拷贝）
    ///
    /// 此方法用于上传大文件，使用零拷贝技术，避免将整个请求体加载到内存中
    ///
    /// # 参数
    ///
    /// * `url` - 请求 URL
    /// * `reader` - 异步读取器，用于分块读取请求体
    /// * `content_length` - 请求体长度（可选）
    /// * `content_type` - 内容类型（可选）
    /// * `options` - 请求选项（可选）
    ///
    /// # 返回
    ///
    /// 成功返回 HTTP 响应，失败返回错误
    pub async fn post_stream<R>(
        &self,
        url: &str,
        reader: R,
        content_length: Option<u64>,
        content_type: Option<&str>,
        options: Option<RequestOptions>,
    ) -> Result<Response>
    where
        R: tokio::io::AsyncRead + Unpin + Send + 'static,
    {
        let start_time = self.metrics_collector.start_request();
        let opts = options.unwrap_or_default();

        // 创建请求体流
        let body = reqwest::Body::wrap_stream(ReaderStream::new(reader));

        let state = self.state.read().await;
        let mut request = state
            .client
            .post(url)
            .timeout(Duration::from_secs(opts.timeout_secs.unwrap_or(30)))
            .body(body);

        // 设置内容长度（如果提供）
        if let Some(length) = content_length {
            request = request.header("Content-Length", length.to_string());
        }

        // 设置内容类型（如果提供）
        if let Some(ct) = content_type {
            request = request.header("Content-Type", ct);
        }

        // 添加隐私保护请求头
        if let Some(ref privacy_mgr) = state.privacy_manager {
            let privacy_headers = privacy_mgr.get_privacy_headers(url).await;
            for (key, value) in privacy_headers {
                request = request.header(key.as_str(), value.as_str());
            }
        }

        // 添加自定义请求头（会覆盖隐私头）
        if let Some(headers_map) = &opts.headers {
            for (key, value) in headers_map {
                request = request.header(key.as_str(), value.as_str());
            }
        }
        // 流处理请求：禁用压缩，确保获取原始二进制数据
        request = request.header("Accept-Encoding", "identity");

        // 发送请求
        let result = request.send().await;

        // 记录指标
        match &result {
            Ok(_) => {
                self.metrics_collector
                    .record_successful_request(start_time)
                    .await;
            }
            Err(_) => {
                self.metrics_collector
                    .record_failed_request(start_time)
                    .await;
            }
        }

        // 处理结果
        result
            .map_err(|e| seesea_errors::http_error(0, &format!("POST stream request failed: {e}")))
    }

    /// 获取网络配置
    pub async fn config(&self) -> NetworkConfig {
        let state = self.state.read().await;
        state.config.as_ref().clone()
    }

    /// 获取底层 reqwest 客户端（用于高级用途）
    pub async fn inner(&self) -> Arc<Client> {
        let state = self.state.read().await;
        state.client.clone()
    }

    /// 获取指标收集器
    pub fn metrics_collector(&self) -> &MetricsCollector {
        &self.metrics_collector
    }

    /// 获取重试配置
    pub fn retry_config(&self) -> &RetryConfig {
        &self.retry_config
    }

    /// 设置重试配置
    pub fn set_retry_config(&mut self, retry_config: RetryConfig) {
        self.retry_config = retry_config;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio_test::assert_ok;

    #[test]
    fn test_http_client_creation() {
        let config = NetworkConfig::default();
        let client = HttpClient::new(config);
        assert!(client.is_ok());
    }

    #[tokio::test]
    async fn test_http_client_config_access() {
        let config = NetworkConfig::default();
        let client = HttpClient::new(config.clone()).unwrap();
        let client_config = client.config().await;
        assert_eq!(
            client_config.pool.max_idle_connections,
            config.pool.max_idle_connections
        );
    }

    #[tokio::test]
    async fn test_http_client_singleton() {
        // 测试单例模式
        let client1 = HttpClient::instance().await.unwrap();
        let client2 = HttpClient::instance().await.unwrap();

        // 验证两个实例是同一个
        assert!(Arc::ptr_eq(&client1, &client2));
    }

    #[test]
    fn test_http_client_singleton_with_config() {
        // 测试使用指定配置的单例模式
        let config = NetworkConfig::default();
        let client1 = HttpClient::instance_with_config(config.clone()).unwrap();
        let client2 = HttpClient::instance_with_config(config).unwrap();

        // 验证两个实例是同一个
        assert!(Arc::ptr_eq(&client1, &client2));
    }

    #[test]
    fn test_http_client_retry_config() {
        // 测试重试配置
        let config = NetworkConfig::default();
        let mut client = HttpClient::new(config).unwrap();

        // 获取默认重试配置
        let _default_retry = client.retry_config().clone();

        // 修改重试配置
        let new_retry = RetryConfig {
            max_retries: 5,
            ..Default::default()
        };
        client.set_retry_config(new_retry.clone());

        // 验证配置已更新
        assert_eq!(client.retry_config().max_retries, new_retry.max_retries);
    }

    #[tokio::test]
    async fn test_http_client_get_request() {
        // 测试GET请求（使用example.com，应该返回200 OK）
        let client = HttpClient::new(NetworkConfig::default()).unwrap();

        // 这里使用example.com，它应该始终可用
        let response = assert_ok!(client.get("https://example.com", None).await);

        // 验证状态码
        assert_eq!(response.status().as_u16(), 200);
    }

    #[tokio::test]
    async fn test_http_client_get_request_not_found() {
        // 测试GET请求（使用不存在的URL，应该返回404 Not Found）
        let client = HttpClient::new(NetworkConfig::default()).unwrap();

        // 这里使用example.com的不存在路径，应该返回404
        let response = assert_ok!(
            client
                .get("https://example.com/this-path-does-not-exist-12345", None)
                .await
        );

        // 验证状态码
        assert_eq!(response.status().as_u16(), 404);
    }

    #[tokio::test]
    #[ignore]
    async fn test_http_client_post_request() {
        // 测试POST请求
        let client = HttpClient::new(NetworkConfig::default()).unwrap();

        // 使用reqres.in进行测试，它提供了可靠的测试POST请求的端点
        let body = b"name=test&job=developer";
        let response = assert_ok!(
            client
                .post("https://reqres.in/api/users", body.to_vec(), None)
                .await
        );

        // 验证状态码
        assert_eq!(response.status().as_u16(), 201);
    }

    #[tokio::test]
    #[ignore]
    async fn test_http_client_post_json_request() {
        // 测试POST JSON请求
        let client = HttpClient::new(NetworkConfig::default()).unwrap();

        // 使用reqres.in进行测试
        let json_data = serde_json::json!({"name": "test", "job": "developer"});
        let response = assert_ok!(
            client
                .post_json("https://reqres.in/api/users", &json_data, None)
                .await
        );

        // 验证状态码
        assert_eq!(response.status().as_u16(), 201);
    }

    #[tokio::test]
    async fn test_http_client_timeout() {
        // 测试请求超时
        let mut config = NetworkConfig::default();
        config.pool.connect_timeout_secs = 1; // 设置较短的连接超时时间
        let client = HttpClient::new(config).unwrap();

        // 使用一个不存在的IP地址，应该超时
        let result = client.get("https://192.0.2.1:9999", None).await;

        // 验证请求失败（超时）
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_http_client_dynamic_config_update() {
        // 测试动态配置更新
        let mut initial_config = NetworkConfig::default();
        initial_config.pool.max_idle_connections = 100;
        let client = HttpClient::new(initial_config.clone()).unwrap();

        // 验证初始配置
        let initial_client_config = client.config().await;
        assert_eq!(initial_client_config.pool.max_idle_connections, 100);

        // 创建新的配置
        let mut new_config = initial_config;
        new_config.pool.max_idle_connections = 200;

        // 更新配置
        assert_ok!(client.update_config(new_config.clone()).await);

        // 验证配置已更新
        let updated_config = client.config().await;
        assert_eq!(updated_config.pool.max_idle_connections, 200);
        assert_eq!(
            updated_config.pool.connect_timeout_secs,
            new_config.pool.connect_timeout_secs
        );
    }

    #[tokio::test]
    async fn test_http_client_privacy_manager_access() {
        let config = NetworkConfig::default();
        let client = HttpClient::new(config).unwrap();

        // 测试获取隐私管理器
        let privacy_mgr = client.privacy_manager().await;
        assert!(privacy_mgr.is_some());
    }

    #[tokio::test]
    async fn test_http_client_inner_access() {
        let config = NetworkConfig::default();
        let client = HttpClient::new(config).unwrap();

        // 测试获取内部客户端
        let inner_client = client.inner().await;
        // Arc类型不能使用is_some()，直接测试其存在性
        assert!(!std::ptr::eq(inner_client.as_ref(), std::ptr::null()));
    }
}
