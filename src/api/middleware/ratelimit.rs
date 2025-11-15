//! 限流中间件
//!
//! 提供 API 请求速率限制功能

// Placeholder for future rate limiting implementation
// Will use governor or similar crate for production use

/// 限流配置
#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    /// 每秒请求数限制
    pub requests_per_second: u32,
    
    /// 突发请求容量
    pub burst_size: u32,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            requests_per_second: 10,
            burst_size: 20,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rate_limit_config_default() {
        let config = RateLimitConfig::default();
        assert_eq!(config.requests_per_second, 10);
        assert_eq!(config.burst_size, 20);
    }
}
