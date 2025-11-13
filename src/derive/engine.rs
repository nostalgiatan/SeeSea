//! 搜索引擎核心 trait 定义

use async_trait::async_trait;
use crate::derive::types::*;
use std::error::Error;

/// 搜索引擎核心 trait
#[async_trait]
pub trait SearchEngine: Send + Sync {
    /// 获取引擎信息
    fn info(&self) -> &EngineInfo;

    /// 执行搜索
    async fn search(&self, query: &SearchQuery) -> Result<SearchResult, Box<dyn Error + Send + Sync>>;

    /// 检查引擎是否可用
    async fn is_available(&self) -> bool {
        true
    }

    /// 获取引擎健康状态
    async fn health_check(&self) -> Result<EngineHealth, Box<dyn Error + Send + Sync>> {
        Ok(EngineHealth {
            status: EngineStatus::Active,
            response_time_ms: 0,
            error_message: None,
        })
    }

    /// 验证查询参数
    fn validate_query(&self, query: &SearchQuery) -> Result<(), ValidationError> {
        // 基础验证
        if query.query.trim().is_empty() {
            return Err(ValidationError::EmptyQuery);
        }

        if query.query.len() > 1000 {
            return Err(ValidationError::QueryTooLong);
        }

        // 页面大小验证
        if query.page_size > self.info().capabilities.max_page_size {
            return Err(ValidationError::PageSizeTooLarge {
                max_size: self.info().capabilities.max_page_size
            });
        }

        // 时间范围验证
        if query.time_range.is_some() && !self.info().capabilities.supports_time_range {
            return Err(ValidationError::UnsupportedTimeRange);
        }

        // 自定义参数验证
        for param in query.params.keys() {
            if !self.info().capabilities.supported_params.contains(param) {
                return Err(ValidationError::UnsupportedParameter(param.clone()));
            }
        }

        Ok(())
    }
}

/// 引擎健康状态
#[derive(Debug, Clone)]
pub struct EngineHealth {
    /// 状态
    pub status: EngineStatus,
    /// 响应时间（毫秒）
    pub response_time_ms: u64,
    /// 错误信息（如果有）
    pub error_message: Option<String>,
}


/// 基础搜索引擎实现模板
///
/// 这个 trait 提供了基于 HTTP 请求的搜索引擎的抽象模板。
/// 具体的 HTTP 客户端实现应该在 `net/client/` 模块中提供。
///
/// # 设计说明
///
/// - `HttpClient`: 关联类型，表示 HTTP 客户端的抽象
/// - `HttpResponse`: 关联类型，表示 HTTP 响应的抽象
/// 
/// 这种设计允许不同的 HTTP 客户端实现（如 reqwest, hyper 等）
/// 都可以通过实现这些关联类型来使用此模板。
#[async_trait]
pub trait BaseEngine: SearchEngine {
    /// HTTP 客户端类型（抽象）
    /// 
    /// 具体实现应由 net/client 模块提供
    type HttpClient;
    
    /// HTTP 响应类型（抽象）
    /// 
    /// 具体实现应由 net/client 模块提供
    type HttpResponse;

    /// 获取 HTTP 客户端引用
    fn http_client(&self) -> &Self::HttpClient;

    /// 构建请求 URL
    /// 
    /// 根据查询参数构建完整的搜索引擎 API URL
    fn build_url(&self, query: &SearchQuery) -> Result<String, ValidationError>;

    /// 发送 HTTP GET 请求
    /// 
    /// 这是一个抽象方法，具体的 HTTP 请求逻辑由实现者提供。
    /// 通常会调用 net/client 模块的功能。
    async fn http_get(&self, url: &str) -> Result<Self::HttpResponse, Box<dyn Error + Send + Sync>>;

    /// 解析 HTTP 响应为搜索结果
    /// 
    /// 将搜索引擎返回的原始响应解析为标准化的 SearchResult
    async fn parse_response(&self, response: Self::HttpResponse, query: &SearchQuery) -> Result<SearchResult, Box<dyn Error + Send + Sync>>;

    /// 默认搜索实现（使用模板方法模式）
    /// 
    /// 这个方法提供了标准的搜索流程：
    /// 1. 验证查询参数
    /// 2. 构建请求 URL
    /// 3. 发送 HTTP 请求
    /// 4. 解析响应
    /// 
    /// 实现者只需要实现抽象方法即可复用这个流程。
    async fn search(&self, query: &SearchQuery) -> Result<SearchResult, Box<dyn Error + Send + Sync>> {
        // 1. 验证查询参数
        self.validate_query(query)?;

        // 2. 构建请求 URL
        let url = self.build_url(query)?;

        // 3. 发送 HTTP 请求（抽象方法，由实现者提供）
        let response = self.http_get(&url).await?;

        // 4. 解析响应
        let result = self.parse_response(response, query).await?;

        Ok(result)
    }
}

/// 可配置的搜索引擎
pub trait ConfigurableEngine: SearchEngine {
    /// 配置类型
    type Config;

    /// 从配置创建引擎
    fn from_config(config: Self::Config) -> Result<Self, Box<dyn Error + Send + Sync>>
    where
        Self: Sized;

    /// 更新配置
    fn update_config(&mut self, config: Self::Config) -> Result<(), Box<dyn Error + Send + Sync>>;
}

/// 支持缓存的搜索引擎
#[async_trait]
pub trait CacheableEngine: SearchEngine {
    /// 生成缓存键
    fn cache_key(&self, query: &SearchQuery) -> String;

    /// 检查缓存
    async fn get_from_cache(&self, key: &str) -> Option<SearchResult>;

    /// 存储到缓存
    async fn store_to_cache(&self, key: &str, result: &SearchResult, ttl: Option<std::time::Duration>) -> Result<(), Box<dyn Error + Send + Sync>>;

    /// 带缓存的搜索
    async fn cached_search(&self, query: &SearchQuery, ttl: Option<std::time::Duration>) -> Result<SearchResult, Box<dyn Error + Send + Sync>> {
        let cache_key = self.cache_key(query);

        // 尝试从缓存获取
        if let Some(cached_result) = self.get_from_cache(&cache_key).await {
            return Ok(cached_result);
        }

        // 执行搜索
        let result = self.search(query).await?;

        // 存储到缓存
        if let Err(e) = self.store_to_cache(&cache_key, &result, ttl).await {
            tracing::warn!("存储搜索结果到缓存失败: {}", e);
        }

        Ok(result)
    }
}

/// 支持重试的搜索引擎
#[async_trait]
pub trait RetryableEngine: SearchEngine {
    /// 最大重试次数
    fn max_retries(&self) -> usize { 3 }

    /// 重试延迟
    fn retry_delay(&self, attempt: usize) -> std::time::Duration {
        std::time::Duration::from_millis(1000 * (1 << attempt) as u64) // 指数退避
    }

    /// 判断是否应该重试
    fn should_retry(&self, error: &Box<dyn Error + Send + Sync>, attempt: usize) -> bool {
        attempt < self.max_retries() && self.is_retryable_error(error.as_ref())
    }

    /// 判断错误是否可重试
    fn is_retryable_error(&self, error: &dyn Error) -> bool {
        // 网络错误、超时等可以重试
        error.to_string().contains("timeout") ||
        error.to_string().contains("network") ||
        error.to_string().contains("connection")
    }

    /// 带重试的搜索
    async fn retryable_search(&self, query: &SearchQuery) -> Result<SearchResult, Box<dyn Error + Send + Sync>> {
        let mut last_error = None;

        for attempt in 0..=self.max_retries() {
            match self.search(query).await {
                Ok(result) => return Ok(result),
                Err(error) => {
                    if self.should_retry(&error, attempt) {
                        tracing::warn!("搜索失败，{}ms后重试 (尝试 {}/{})",
                                     self.retry_delay(attempt).as_millis(),
                                     attempt + 1,
                                     self.max_retries());
                        tokio::time::sleep(self.retry_delay(attempt)).await;
                        last_error = Some(error);
                    } else {
                        return Err(error);
                    }
                }
            }
        }

        Err(last_error.unwrap_or_else(|| "未知错误".into()))
    }
}