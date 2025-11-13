//! HTTP 客户端处理模块
//!
//! 本模块提供了搜索引擎HTTP请求的统一客户端接口，
//! 包括请求构建、超时控制、重试机制等功能。

use crate::derive::error::{DeriveError, Result};
use reqwest::{Client, RequestBuilder, Response};
use std::time::Duration;

/// HTTP 客户端配置
///
/// 配置HTTP客户端的各种参数
#[derive(Debug, Clone)]
pub struct ClientConfig {
    /// 超时时间（秒）
    pub timeout_secs: u64,
    /// 用户代理字符串
    pub user_agent: String,
    /// 是否跟随重定向
    pub follow_redirects: bool,
    /// 最大重定向次数
    pub max_redirects: usize,
    /// 是否启用gzip压缩
    pub gzip: bool,
    /// 连接超时（秒）
    pub connect_timeout_secs: u64,
    /// 每个主机的最大连接数
    pub pool_max_idle_per_host: usize,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            timeout_secs: 30,
            user_agent: format!(
                "{}/{}",
                env!("CARGO_PKG_NAME"),
                env!("CARGO_PKG_VERSION")
            ),
            follow_redirects: true,
            max_redirects: 10,
            gzip: true,
            connect_timeout_secs: 10,
            pool_max_idle_per_host: 32,
        }
    }
}

/// HTTP 客户端构建器
///
/// 用于创建和配置HTTP客户端
pub struct ClientBuilder {
    config: ClientConfig,
}

impl ClientBuilder {
    /// 创建一个新的客户端构建器
    pub fn new() -> Self {
        Self {
            config: ClientConfig::default(),
        }
    }

    /// 设置超时时间
    ///
    /// # 参数
    ///
    /// * `secs` - 超时时间（秒）
    pub fn timeout(mut self, secs: u64) -> Self {
        self.config.timeout_secs = secs;
        self
    }

    /// 设置用户代理
    ///
    /// # 参数
    ///
    /// * `user_agent` - 用户代理字符串
    pub fn user_agent(mut self, user_agent: impl Into<String>) -> Self {
        self.config.user_agent = user_agent.into();
        self
    }

    /// 设置是否跟随重定向
    ///
    /// # 参数
    ///
    /// * `follow` - 是否跟随重定向
    pub fn follow_redirects(mut self, follow: bool) -> Self {
        self.config.follow_redirects = follow;
        self
    }

    /// 设置最大重定向次数
    ///
    /// # 参数
    ///
    /// * `max` - 最大重定向次数
    pub fn max_redirects(mut self, max: usize) -> Self {
        self.config.max_redirects = max;
        self
    }

    /// 设置是否启用gzip压缩
    ///
    /// # 参数
    ///
    /// * `enabled` - 是否启用
    pub fn gzip(mut self, enabled: bool) -> Self {
        self.config.gzip = enabled;
        self
    }

    /// 设置连接超时
    ///
    /// # 参数
    ///
    /// * `secs` - 连接超时时间（秒）
    pub fn connect_timeout(mut self, secs: u64) -> Self {
        self.config.connect_timeout_secs = secs;
        self
    }

    /// 设置每个主机的最大连接数
    ///
    /// # 参数
    ///
    /// * `max` - 最大连接数
    pub fn pool_max_idle_per_host(mut self, max: usize) -> Self {
        self.config.pool_max_idle_per_host = max;
        self
    }

    /// 构建HTTP客户端
    ///
    /// # 返回值
    ///
    /// 返回配置好的 reqwest::Client
    ///
    /// # 错误
    ///
    /// 如果客户端构建失败，返回 DeriveError
    pub fn build(self) -> Result<Client> {
        let mut builder = Client::builder()
            .timeout(Duration::from_secs(self.config.timeout_secs))
            .user_agent(&self.config.user_agent)
            .connect_timeout(Duration::from_secs(self.config.connect_timeout_secs))
            .pool_max_idle_per_host(self.config.pool_max_idle_per_host);

        if !self.config.follow_redirects {
            builder = builder.redirect(reqwest::redirect::Policy::none());
        } else {
            builder = builder.redirect(reqwest::redirect::Policy::limited(
                self.config.max_redirects,
            ));
        }

        if self.config.gzip {
            builder = builder.gzip(true);
        }

        builder.build().map_err(|e| DeriveError::Configuration {
            message: format!("创建HTTP客户端失败: {}", e),
        })
    }
}

impl Default for ClientBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// HTTP 请求助手
///
/// 提供便捷的HTTP请求方法
pub struct HttpClient {
    client: Client,
}

impl HttpClient {
    /// 创建一个新的HTTP客户端
    ///
    /// 使用默认配置
    pub fn new() -> Result<Self> {
        let client = ClientBuilder::new().build()?;
        Ok(Self { client })
    }

    /// 使用自定义配置创建HTTP客户端
    ///
    /// # 参数
    ///
    /// * `builder` - 客户端构建器
    pub fn with_builder(builder: ClientBuilder) -> Result<Self> {
        let client = builder.build()?;
        Ok(Self { client })
    }

    /// 使用现有的reqwest客户端创建
    ///
    /// # 参数
    ///
    /// * `client` - reqwest::Client实例
    pub fn with_client(client: Client) -> Self {
        Self { client }
    }

    /// 获取内部的reqwest客户端引用
    pub fn client(&self) -> &Client {
        &self.client
    }

    /// 发送GET请求
    ///
    /// # 参数
    ///
    /// * `url` - 请求URL
    ///
    /// # 返回值
    ///
    /// 返回RequestBuilder，可以继续添加headers等配置
    pub fn get(&self, url: &str) -> RequestBuilder {
        self.client.get(url)
    }

    /// 发送POST请求
    ///
    /// # 参数
    ///
    /// * `url` - 请求URL
    ///
    /// # 返回值
    ///
    /// 返回RequestBuilder，可以继续添加body、headers等配置
    pub fn post(&self, url: &str) -> RequestBuilder {
        self.client.post(url)
    }

    /// 执行请求并检查响应状态
    ///
    /// # 参数
    ///
    /// * `builder` - 请求构建器
    ///
    /// # 返回值
    ///
    /// 如果请求成功且状态码为2xx，返回Response
    ///
    /// # 错误
    ///
    /// 如果请求失败或状态码不是2xx，返回 DeriveError
    pub async fn execute(&self, builder: RequestBuilder) -> Result<Response> {
        let response = builder.send().await?;
        
        if response.status().is_success() {
            Ok(response)
        } else {
            Err(DeriveError::Network {
                message: format!("HTTP请求失败: {}", response.status()),
                status_code: Some(response.status().as_u16()),
            })
        }
    }

    /// 执行GET请求并返回文本响应
    ///
    /// # 参数
    ///
    /// * `url` - 请求URL
    ///
    /// # 返回值
    ///
    /// 返回响应的文本内容
    ///
    /// # 错误
    ///
    /// 如果请求或解析失败，返回 DeriveError
    pub async fn get_text(&self, url: &str) -> Result<String> {
        let response = self.execute(self.get(url)).await?;
        response.text().await.map_err(|e| DeriveError::Parse {
            message: format!("解析响应文本失败: {}", e),
            content_type: Some("text/plain".to_string()),
        })
    }

    /// 执行GET请求并返回JSON响应
    ///
    /// # 参数
    ///
    /// * `url` - 请求URL
    ///
    /// # 返回值
    ///
    /// 返回解析后的JSON值
    ///
    /// # 错误
    ///
    /// 如果请求或解析失败，返回 DeriveError
    pub async fn get_json(&self, url: &str) -> Result<serde_json::Value> {
        let response = self.execute(self.get(url)).await?;
        response.json().await.map_err(|e| DeriveError::Parse {
            message: format!("解析JSON响应失败: {}", e),
            content_type: Some("application/json".to_string()),
        })
    }
}

impl Default for HttpClient {
    fn default() -> Self {
        Self::with_client(Client::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_config_default() {
        let config = ClientConfig::default();
        assert_eq!(config.timeout_secs, 30);
        assert!(config.follow_redirects);
        assert_eq!(config.max_redirects, 10);
        assert!(config.gzip);
    }

    #[test]
    fn test_client_builder() {
        let builder = ClientBuilder::new()
            .timeout(60)
            .user_agent("TestAgent/1.0")
            .follow_redirects(false)
            .max_redirects(5)
            .gzip(false)
            .connect_timeout(5)
            .pool_max_idle_per_host(16);

        assert_eq!(builder.config.timeout_secs, 60);
        assert_eq!(builder.config.user_agent, "TestAgent/1.0");
        assert!(!builder.config.follow_redirects);
        assert_eq!(builder.config.max_redirects, 5);
        assert!(!builder.config.gzip);
        assert_eq!(builder.config.connect_timeout_secs, 5);
        assert_eq!(builder.config.pool_max_idle_per_host, 16);
    }

    #[test]
    fn test_client_builder_build() {
        let result = ClientBuilder::new()
            .timeout(30)
            .user_agent("TestAgent/1.0")
            .build();

        assert!(result.is_ok());
    }

    #[test]
    fn test_http_client_creation() {
        let result = HttpClient::new();
        assert!(result.is_ok());
    }

    #[test]
    fn test_http_client_with_builder() {
        let builder = ClientBuilder::new().timeout(60);
        let result = HttpClient::with_builder(builder);
        assert!(result.is_ok());
    }

    #[test]
    fn test_http_client_with_client() {
        let client = Client::new();
        let http_client = HttpClient::with_client(client);
        assert!(http_client.client().get("https://example.com").build().is_ok());
    }
}
