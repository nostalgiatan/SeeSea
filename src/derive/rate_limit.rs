//! 速率限制模块
//!
//! 本模块提供了速率限制功能，用于控制对搜索引擎的请求频率，
//! 防止超出API限制或被封禁。

use crate::derive::error::{DeriveError, Result};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// 速率限制器配置
#[derive(Debug, Clone)]
pub struct RateLimiterConfig {
    /// 每分钟最大请求数
    pub requests_per_minute: usize,
    /// 突发请求容量
    pub burst_capacity: usize,
}

impl Default for RateLimiterConfig {
    fn default() -> Self {
        Self {
            requests_per_minute: 60,
            burst_capacity: 10,
        }
    }
}

/// 令牌桶状态
#[derive(Debug)]
struct TokenBucket {
    /// 可用令牌数
    tokens: f64,
    /// 最大令牌数
    capacity: f64,
    /// 令牌补充速率（每秒）
    refill_rate: f64,
    /// 上次更新时间
    last_refill: Instant,
}

impl TokenBucket {
    /// 创建新的令牌桶
    ///
    /// # 参数
    ///
    /// * `capacity` - 最大令牌容量
    /// * `refill_rate` - 每秒补充的令牌数
    fn new(capacity: f64, refill_rate: f64) -> Self {
        Self {
            tokens: capacity,
            capacity,
            refill_rate,
            last_refill: Instant::now(),
        }
    }

    /// 尝试消费一个令牌
    ///
    /// # 返回值
    ///
    /// 如果成功消费令牌返回 true，否则返回 false
    fn try_consume(&mut self) -> bool {
        self.refill();
        
        if self.tokens >= 1.0 {
            self.tokens -= 1.0;
            true
        } else {
            false
        }
    }

    /// 补充令牌
    fn refill(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_refill).as_secs_f64();
        
        if elapsed > 0.0 {
            let new_tokens = elapsed * self.refill_rate;
            self.tokens = (self.tokens + new_tokens).min(self.capacity);
            self.last_refill = now;
        }
    }

    /// 获取下次可用的等待时间
    ///
    /// # 返回值
    ///
    /// 返回需要等待的秒数
    fn wait_time(&mut self) -> f64 {
        self.refill();
        
        if self.tokens >= 1.0 {
            0.0
        } else {
            (1.0 - self.tokens) / self.refill_rate
        }
    }
}

/// 速率限制器
///
/// 使用令牌桶算法实现的速率限制器，支持多个引擎的独立限制
pub struct RateLimiter {
    /// 每个引擎的令牌桶
    buckets: Arc<Mutex<HashMap<String, TokenBucket>>>,
    /// 默认配置
    default_config: RateLimiterConfig,
}

impl RateLimiter {
    /// 创建新的速率限制器
    ///
    /// # 参数
    ///
    /// * `config` - 限制器配置
    pub fn new(config: RateLimiterConfig) -> Self {
        Self {
            buckets: Arc::new(Mutex::new(HashMap::new())),
            default_config: config,
        }
    }

    /// 尝试获取请求许可
    ///
    /// # 参数
    ///
    /// * `engine_name` - 引擎名称
    ///
    /// # 返回值
    ///
    /// 如果允许请求返回 Ok(())，否则返回包含重试时间的错误
    pub fn try_acquire(&self, engine_name: &str) -> Result<()> {
        let mut buckets = self.buckets.lock().map_err(|e| DeriveError::Internal {
            message: format!("获取速率限制器锁失败: {}", e),
        })?;

        let bucket = buckets.entry(engine_name.to_string()).or_insert_with(|| {
            let refill_rate = self.default_config.requests_per_minute as f64 / 60.0;
            TokenBucket::new(
                self.default_config.burst_capacity as f64,
                refill_rate,
            )
        });

        if bucket.try_consume() {
            Ok(())
        } else {
            let wait_secs = bucket.wait_time().ceil() as u64;
            Err(DeriveError::RateLimit {
                retry_after_secs: wait_secs.max(1),
            })
        }
    }

    /// 异步等待获取请求许可
    ///
    /// # 参数
    ///
    /// * `engine_name` - 引擎名称
    ///
    /// # 返回值
    ///
    /// 等待直到可以发送请求
    pub async fn acquire(&self, engine_name: &str) -> Result<()> {
        loop {
            match self.try_acquire(engine_name) {
                Ok(()) => return Ok(()),
                Err(DeriveError::RateLimit { retry_after_secs }) => {
                    tracing::debug!(
                        "速率限制触发，等待{}秒后重试 [引擎: {}]",
                        retry_after_secs,
                        engine_name
                    );
                    tokio::time::sleep(Duration::from_secs(retry_after_secs)).await;
                }
                Err(e) => return Err(e),
            }
        }
    }

    /// 重置指定引擎的限制
    ///
    /// # 参数
    ///
    /// * `engine_name` - 引擎名称
    pub fn reset(&self, engine_name: &str) -> Result<()> {
        let mut buckets = self.buckets.lock().map_err(|e| DeriveError::Internal {
            message: format!("获取速率限制器锁失败: {}", e),
        })?;

        buckets.remove(engine_name);
        Ok(())
    }

    /// 重置所有引擎的限制
    pub fn reset_all(&self) -> Result<()> {
        let mut buckets = self.buckets.lock().map_err(|e| DeriveError::Internal {
            message: format!("获取速率限制器锁失败: {}", e),
        })?;

        buckets.clear();
        Ok(())
    }
}

impl Default for RateLimiter {
    fn default() -> Self {
        Self::new(RateLimiterConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rate_limiter_config_default() {
        let config = RateLimiterConfig::default();
        assert_eq!(config.requests_per_minute, 60);
        assert_eq!(config.burst_capacity, 10);
    }

    #[test]
    fn test_token_bucket_creation() {
        let bucket = TokenBucket::new(10.0, 1.0);
        assert_eq!(bucket.capacity, 10.0);
        assert_eq!(bucket.refill_rate, 1.0);
        assert_eq!(bucket.tokens, 10.0);
    }

    #[test]
    fn test_token_bucket_consume() {
        let mut bucket = TokenBucket::new(10.0, 1.0);
        
        // 应该能够消费10个令牌
        for _ in 0..10 {
            assert!(bucket.try_consume());
        }
        
        // 第11个应该失败
        assert!(!bucket.try_consume());
    }

    #[test]
    fn test_token_bucket_wait_time() {
        let mut bucket = TokenBucket::new(1.0, 1.0);
        
        // 消费一个令牌
        assert!(bucket.try_consume());
        
        // 现在应该需要等待
        let wait = bucket.wait_time();
        assert!(wait > 0.0 && wait <= 1.0);
    }

    #[test]
    fn test_rate_limiter_creation() {
        let config = RateLimiterConfig {
            requests_per_minute: 30,
            burst_capacity: 5,
        };
        let limiter = RateLimiter::new(config.clone());
        assert_eq!(limiter.default_config.requests_per_minute, 30);
        assert_eq!(limiter.default_config.burst_capacity, 5);
    }

    #[test]
    fn test_rate_limiter_try_acquire() {
        let config = RateLimiterConfig {
            requests_per_minute: 60,
            burst_capacity: 2,
        };
        let limiter = RateLimiter::new(config);
        
        // 前2个请求应该成功（突发容量）
        assert!(limiter.try_acquire("test_engine").is_ok());
        assert!(limiter.try_acquire("test_engine").is_ok());
        
        // 第3个应该失败
        let result = limiter.try_acquire("test_engine");
        assert!(result.is_err());
        if let Err(DeriveError::RateLimit { retry_after_secs }) = result {
            assert!(retry_after_secs > 0);
        } else {
            panic!("预期速率限制错误");
        }
    }

    #[test]
    fn test_rate_limiter_reset() {
        let config = RateLimiterConfig {
            requests_per_minute: 60,
            burst_capacity: 1,
        };
        let limiter = RateLimiter::new(config);
        
        // 消费令牌
        assert!(limiter.try_acquire("test_engine").is_ok());
        assert!(limiter.try_acquire("test_engine").is_err());
        
        // 重置
        assert!(limiter.reset("test_engine").is_ok());
        
        // 现在应该可以再次请求
        assert!(limiter.try_acquire("test_engine").is_ok());
    }

    #[test]
    fn test_rate_limiter_multiple_engines() {
        let config = RateLimiterConfig {
            requests_per_minute: 60,
            burst_capacity: 1,
        };
        let limiter = RateLimiter::new(config);
        
        // 两个不同的引擎应该有独立的限制
        assert!(limiter.try_acquire("engine1").is_ok());
        assert!(limiter.try_acquire("engine2").is_ok());
        
        // 各自的第二次请求应该失败
        assert!(limiter.try_acquire("engine1").is_err());
        assert!(limiter.try_acquire("engine2").is_err());
    }

    #[tokio::test]
    async fn test_rate_limiter_acquire_async() {
        let config = RateLimiterConfig {
            requests_per_minute: 120, // 每秒2个请求
            burst_capacity: 1,
        };
        let limiter = RateLimiter::new(config);
        
        // 第一个请求应该立即成功
        let result = limiter.acquire("test_engine").await;
        assert!(result.is_ok());
        
        // 第二个请求应该等待后成功（因为补充速度快）
        let result = limiter.acquire("test_engine").await;
        assert!(result.is_ok());
    }
}
