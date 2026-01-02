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

//! 请求重试机制模块
//!
//! 提供请求重试功能，支持多种重试策略，提高网络请求的可靠性。

pub mod strategy;

use seesea_errors::Result;
use std::future::Future;
use std::time::Duration;
use strategy::RetryStrategy;

/// 重试配置
#[derive(Debug, Clone)]
pub struct RetryConfig {
    /// 最大重试次数
    pub max_retries: usize,
    /// 重试策略
    pub strategy: RetryStrategy,
    /// 是否重试超时错误
    pub retry_on_timeout: bool,
    /// 是否重试连接错误
    pub retry_on_connection_error: bool,
    /// 是否重试 HTTP 错误状态码
    pub retry_on_http_error: bool,
    /// 重试的 HTTP 错误状态码列表
    pub retry_http_status_codes: Vec<u16>,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            strategy: RetryStrategy::ExponentialBackoff {
                initial_delay: Duration::from_millis(100),
                max_delay: Duration::from_secs(5),
                multiplier: 2.0,
            },
            retry_on_timeout: true,
            retry_on_connection_error: true,
            retry_on_http_error: true,
            retry_http_status_codes: vec![429, 500, 502, 503, 504],
        }
    }
}

/// 重试执行器
pub struct RetryExecutor {
    config: RetryConfig,
}

impl RetryExecutor {
    /// 创建新的重试执行器
    pub fn new(config: RetryConfig) -> Self {
        Self { config }
    }

    /// 执行带重试的异步操作
    ///
    /// # 参数
    ///
    /// * `operation` - 要执行的异步操作
    ///
    /// # 返回
    ///
    /// 成功返回操作结果，失败返回错误
    pub async fn execute<F, T, E>(&self, mut operation: F) -> Result<T>
    where
        F: FnMut() -> std::pin::Pin<Box<dyn Future<Output = std::result::Result<T, E>> + Send>>,
        E: std::error::Error + Send + Sync + 'static,
    {
        let mut attempt = 0;

        loop {
            match operation().await {
                Ok(result) => return Ok(result),
                Err(error) => {
                    attempt += 1;
                    if attempt > self.config.max_retries {
                        return Err(seesea_errors::http_error(
                            0,
                            &format!("Operation failed after {attempt} attempts: {error}"),
                        ));
                    }

                    // 检查是否应该重试
                    if !self.should_retry(&error) {
                        return Err(seesea_errors::http_error(
                            0,
                            &format!("Operation failed and should not be retried: {error}"),
                        ));
                    }

                    // 计算延迟
                    let delay = self.config.strategy.calculate_delay(attempt);
                    tokio::time::sleep(delay).await;
                }
            }
        }
    }

    /// 检查是否应该重试
    fn should_retry<E>(&self, _error: &E) -> bool
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        // 目前简单实现为总是重试，后续可以根据错误类型和内容进行更智能的判断
        // 由于泛型错误类型 E 无法直接转换为 reqwest::Error，我们需要使用不同的方式来检查错误类型
        // 这里我们暂时使用配置中的重试开关来决定是否重试
        self.config.retry_on_timeout
            || self.config.retry_on_connection_error
            || self.config.retry_on_http_error
    }
}

/// 重试扩展 trait
#[allow(async_fn_in_trait)]
pub trait RetryExt<T, E> {
    /// 执行带重试的操作
    async fn with_retry<F>(self, config: RetryConfig) -> Result<T>
    where
        F: FnMut() -> std::pin::Pin<Box<dyn Future<Output = std::result::Result<T, E>> + Send>>,
        E: std::error::Error + Send + Sync + 'static;
}

#[allow(async_fn_in_trait)]
impl<T, E> RetryExt<T, E>
    for fn() -> std::pin::Pin<Box<dyn Future<Output = std::result::Result<T, E>> + Send>>
where
    E: std::error::Error + Send + Sync + 'static,
{
    async fn with_retry<F>(self, config: RetryConfig) -> Result<T>
    where
        F: FnMut() -> std::pin::Pin<Box<dyn Future<Output = std::result::Result<T, E>> + Send>>,
        E: std::error::Error + Send + Sync + 'static,
    {
        let executor = RetryExecutor::new(config);
        executor.execute(self).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use strategy::RetryStrategy;

    #[tokio::test]
    async fn test_retry_execute_success() {
        let config = RetryConfig::default();
        let executor = RetryExecutor::new(config);

        let mut attempts = 0;
        let result = executor
            .execute(|| {
                attempts += 1;
                Box::pin(async move { Ok::<&str, std::io::Error>("success") })
            })
            .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "success");
        assert_eq!(attempts, 1);
    }

    #[tokio::test]
    async fn test_retry_execute_with_retry() {
        let config = RetryConfig {
            max_retries: 3,
            strategy: RetryStrategy::FixedDelay(Duration::from_millis(10)),
            ..Default::default()
        };
        let executor = RetryExecutor::new(config);

        let mut attempts = 0;
        let result = executor
            .execute(|| {
                attempts += 1;
                Box::pin(async move {
                    if attempts < 3 {
                        Err(std::io::Error::other("temporary error"))
                    } else {
                        Ok("success after retry")
                    }
                })
            })
            .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "success after retry");
        assert_eq!(attempts, 3);
    }

    #[tokio::test]
    async fn test_retry_execute_exhausted() {
        let config = RetryConfig {
            max_retries: 2,
            strategy: RetryStrategy::FixedDelay(Duration::from_millis(10)),
            ..Default::default()
        };
        let executor = RetryExecutor::new(config);

        let mut attempts = 0;
        let result: Result<&str> = executor
            .execute(|| {
                attempts += 1;
                Box::pin(async move {
                    Err::<&str, std::io::Error>(std::io::Error::other("persistent error"))
                })
            })
            .await;

        assert!(result.is_err());
        assert_eq!(attempts, 3); // 1 initial attempt + 2 retries
    }
}
