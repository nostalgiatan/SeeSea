//! 认证中间件
//!
//! 提供 API 认证功能

// Placeholder for future authentication implementation
// Will support API keys, JWT, etc.

/// 认证配置
#[derive(Debug, Clone)]
pub struct AuthConfig {
    /// 是否启用认证
    pub enabled: bool,
    
    /// API 密钥列表
    pub api_keys: Vec<String>,
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            api_keys: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auth_config_default() {
        let config = AuthConfig::default();
        assert!(!config.enabled);
        assert_eq!(config.api_keys.len(), 0);
    }
}
