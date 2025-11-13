//! 搜索引擎核心 trait 定义
//!
//! 本模块定义了搜索引擎的核心接口，包括基础搜索、配置、缓存和重试功能。

use async_trait::async_trait;
use crate::derive::types::*;
use crate::derive::error::{DeriveError, Result};

/// 搜索引擎核心 trait
#[async_trait]
pub trait SearchEngine: Send + Sync {
    /// 获取引擎信息
    fn info(&self) -> &EngineInfo;

    /// 执行搜索
    async fn search(&self, query: &SearchQuery) -> Result<SearchResult>;

    /// 检查引擎是否可用
    async fn is_available(&self) -> bool {
        true
    }

    /// 获取引擎健康状态
    async fn health_check(&self) -> Result<EngineHealth> {
        Ok(EngineHealth {
            status: EngineStatus::Active,
            response_time_ms: 0,
            error_message: None,
        })
    }

    /// 验证查询参数
    fn validate_query(&self, query: &SearchQuery) -> Result<()> {
        // 基础验证
        if query.query.trim().is_empty() {
            return Err(DeriveError::Validation {
                message: "查询不能为空".to_string(),
                field: Some("query".to_string()),
            });
        }

        if query.query.len() > 1000 {
            return Err(DeriveError::Validation {
                message: "查询过长，最多1000字符".to_string(),
                field: Some("query".to_string()),
            });
        }

        // 页面大小验证
        if query.page_size > self.info().capabilities.max_page_size {
            return Err(DeriveError::Validation {
                message: format!("页面大小超出限制，最大{}个结果", 
                    self.info().capabilities.max_page_size),
                field: Some("page_size".to_string()),
            });
        }

        // 时间范围验证
        if query.time_range.is_some() && !self.info().capabilities.supports_time_range {
            return Err(DeriveError::Validation {
                message: "不支持时间范围过滤".to_string(),
                field: Some("time_range".to_string()),
            });
        }

        // 自定义参数验证
        for param in query.params.keys() {
            if !self.info().capabilities.supported_params.contains(param) {
                return Err(DeriveError::Validation {
                    message: format!("不支持的参数: {}", param),
                    field: Some("params".to_string()),
                });
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

/// 可配置的搜索引擎
pub trait ConfigurableEngine: SearchEngine {
    /// 配置类型
    type Config;

    /// 从配置创建引擎
    fn from_config(config: Self::Config) -> Result<Self>
    where
        Self: Sized;

    /// 更新配置
    fn update_config(&mut self, config: Self::Config) -> Result<()>;
}

/// 支持缓存的搜索引擎
#[async_trait]
pub trait CacheableEngine: SearchEngine {
    /// 生成缓存键
    fn cache_key(&self, query: &SearchQuery) -> String;

    /// 检查缓存
    async fn get_from_cache(&self, key: &str) -> Option<SearchResult>;

    /// 存储到缓存
    async fn store_to_cache(&self, key: &str, result: &SearchResult, ttl: Option<std::time::Duration>) -> Result<()>;

    /// 带缓存的搜索
    async fn cached_search(&self, query: &SearchQuery, ttl: Option<std::time::Duration>) -> Result<SearchResult> {
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
    fn should_retry(&self, error: &DeriveError, attempt: usize) -> bool {
        attempt < self.max_retries() && self.is_retryable_error(error)
    }

    /// 判断错误是否可重试
    fn is_retryable_error(&self, error: &DeriveError) -> bool {
        matches!(
            error,
            DeriveError::Network { .. } | DeriveError::Timeout { .. }
        )
    }

    /// 带重试的搜索
    async fn retryable_search(&self, query: &SearchQuery) -> Result<SearchResult> {
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

        Err(last_error.unwrap_or_else(|| DeriveError::Internal {
            message: "所有重试尝试均失败".to_string(),
        }))
    }
}