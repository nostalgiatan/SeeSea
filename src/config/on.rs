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

//! SeeSea 配置管理公共接口
//!
//! 提供配置加载、验证、管理的外部接口

use crate::config::{
    common::ConfigValidationResult, ConfigError, ConfigLoadResult, SeeSeaConfig,
};
use crate::config::config::ConfigSummary;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

/// 配置管理器
pub struct ConfigManager {
    /// 当前配置
    config: Arc<RwLock<SeeSeaConfig>>,
    /// 配置文件路径
    config_path: PathBuf,
    /// 是否启用热重载
    hot_reload: bool,
}

impl ConfigManager {
    /// 创建新的配置管理器
    pub async fn new(config_path: Option<PathBuf>) -> Result<Self, ConfigError> {
        let config_path = config_path.unwrap_or_else(|| {
            std::env::var("SEEA_CONFIG_FILE")
                .map(PathBuf::from)
                .unwrap_or_else(|_| PathBuf::from("config/default.toml"))
        });

        let manager = Self {
            config: Arc::new(RwLock::new(SeeSeaConfig::default())),
            config_path: config_path.clone(),
            hot_reload: false,
        };

        // 尝试加载配置
        if config_path.exists() {
            manager.load_config().await?;
        } else {
            tracing::warn!("配置文件不存在: {:?}, 使用默认配置", config_path);
        }

        Ok(manager)
    }

    /// 创建带环境的配置管理器
    pub async fn with_environment(
        config_path: Option<PathBuf>,
        environment: &str,
    ) -> Result<Self, ConfigError> {
        let mut config = match environment {
            "development" | "dev" => SeeSeaConfig::development(),
            "testing" | "test" => SeeSeaConfig::testing(),
            "staging" | "stage" => SeeSeaConfig::default(),
            "production" | "prod" => SeeSeaConfig::production(),
            _ => {
                tracing::warn!("未知环境: {}, 使用默认配置", environment);
                SeeSeaConfig::default()
            }
        };

        // 应用环境特定的覆盖
        Self::apply_environment_overrides(&mut config, environment);

        let manager = Self {
            config: Arc::new(RwLock::new(config)),
            config_path: config_path.unwrap_or_else(|| PathBuf::from("config/default.toml")),
            hot_reload: false,
        };

        Ok(manager)
    }

    /// 加载配置文件
    pub async fn load_config(&self) -> Result<ConfigLoadResult, ConfigError> {
        let config = Self::load_from_file(&self.config_path).await?;
        let validation_result = config.validate();
        let summary = config.get_summary();

        let load_result = ConfigLoadResult {
            file_path: self.config_path.to_string_lossy().to_string(),
            used_defaults: !self.config_path.exists(),
            warnings: validation_result.warnings.clone(),
            config: config.clone(),
            summary,
            load_time: chrono::Utc::now(),
        };

        if !validation_result.is_valid {
            return Err(ConfigError::Validation(validation_result));
        }

        // 更新配置
        {
            let mut config_guard = self.config.write().await;
            *config_guard = config;
        }

        tracing::info!("配置加载成功: {:?}", self.config_path);
        for warning in &load_result.warnings {
            tracing::warn!("配置警告: {}", warning);
        }

        Ok(load_result)
    }

    /// 获取当前配置的副本
    pub async fn get_config(&self) -> SeeSeaConfig {
        self.config.read().await.clone()
    }

    /// 获取配置的只读引用
    pub async fn read_config(&self) -> tokio::sync::RwLockReadGuard<'_, SeeSeaConfig> {
        self.config.read().await
    }

    /// 获取配置摘要
    pub async fn get_summary(&self) -> ConfigSummary {
        let config = self.config.read().await;
        config.get_summary()
    }

    /// 验证当前配置
    pub async fn validate(&self) -> ConfigValidationResult {
        let config = self.config.read().await;
        config.validate()
    }

    /// 检查是否为生产就绪
    pub async fn is_production_ready(&self) -> bool {
        let config = self.config.read().await;
        config.is_production_ready()
    }

    /// 获取配置建议
    pub async fn get_recommendations(&self) -> Vec<String> {
        let config = self.config.read().await;
        config.get_config_recommendations()
    }

    /// 重新加载配置
    pub async fn reload(&self) -> Result<ConfigLoadResult, ConfigError> {
        tracing::info!("重新加载配置");
        self.load_config().await
    }

    /// 启用热重载
    pub fn enable_hot_reload(&mut self) {
        self.hot_reload = true;
        tracing::info!("配置热重载已启用");
    }

    /// 禁用热重载
    pub fn disable_hot_reload(&mut self) {
        self.hot_reload = false;
        tracing::info!("配置热重载已禁用");
    }

    /// 检查热重载状态
    pub fn is_hot_reload_enabled(&self) -> bool {
        self.hot_reload
    }

    /// 从文件加载配置
    async fn load_from_file(config_path: &PathBuf) -> Result<SeeSeaConfig, ConfigError> {
        let config_str = tokio::fs::read_to_string(config_path)
            .await
            .map_err(|e| ConfigError::IoError(e.to_string()))?;

        // 尝试解析为 TOML
        let config: SeeSeaConfig = toml::from_str(&config_str)
            .map_err(|e| ConfigError::ParseError(e.to_string()))?;

        Ok(config)
    }

    /// 应用环境特定的覆盖
    fn apply_environment_overrides(config: &mut SeeSeaConfig, environment: &str) {
        // 从环境变量读取配置覆盖
        if let Ok(port) = std::env::var("SEEA_PORT") {
            if let Ok(port) = port.parse::<u16>() {
                config.server.port = port;
            }
        }

        if let Ok(debug) = std::env::var("SEEA_DEBUG") {
            config.general.debug = debug.parse().unwrap_or(false);
        }

        if let Ok(secret_key) = std::env::var("SEEA_SECRET_KEY") {
            config.server.secret_key = secret_key;
        }

        if let Ok(log_level) = std::env::var("SEEA_LOG_LEVEL") {
            match log_level.to_lowercase().as_str() {
                "error" => config.logging.level = crate::config::LogLevel::Error,
                "warn" => config.logging.level = crate::config::LogLevel::Warn,
                "info" => config.logging.level = crate::config::LogLevel::Info,
                "debug" => config.logging.level = crate::config::LogLevel::Debug,
                "trace" => config.logging.level = crate::config::LogLevel::Trace,
                _ => {}
            }
        }

        tracing::debug!("应用环境覆盖: {}", environment);
    }
}

/// 全局配置管理器实例
static GLOBAL_CONFIG: std::sync::OnceLock<Arc<ConfigManager>> = std::sync::OnceLock::new();

/// 初始化全局配置管理器
pub async fn init_config() -> Result<Arc<ConfigManager>, ConfigError> {
    if let Some(config) = GLOBAL_CONFIG.get() {
        return Ok(config.clone());
    }
    
    let manager = Arc::new(ConfigManager::new(None).await?);
    let _ = GLOBAL_CONFIG.set(manager.clone());
    Ok(manager)
}

/// 初始化带环境的全局配置管理器
pub async fn init_config_with_env(
    environment: &str,
) -> Result<Arc<ConfigManager>, ConfigError> {
    if let Some(config) = GLOBAL_CONFIG.get() {
        return Ok(config.clone());
    }
    
    let manager = Arc::new(ConfigManager::with_environment(None, environment).await?);
    let _ = GLOBAL_CONFIG.set(manager.clone());
    Ok(manager)
}

/// 获取全局配置管理器
pub fn get_global_config() -> Option<Arc<ConfigManager>> {
    GLOBAL_CONFIG.get().cloned()
}

/// 便利函数：获取当前配置
pub async fn get_config() -> Option<SeeSeaConfig> {
    match get_global_config() {
        Some(manager) => Some(manager.get_config().await),
        None => None,
    }
}

/// 便利函数：验证当前配置
pub async fn validate_config() -> Option<ConfigValidationResult> {
    match get_global_config() {
        Some(manager) => Some(manager.validate().await),
        None => None,
    }
}

/// 便利函数：检查生产就绪
pub async fn is_production_ready() -> Option<bool> {
    match get_global_config() {
        Some(manager) => Some(manager.is_production_ready().await),
        None => None,
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    use tokio::fs;

    #[tokio::test]
    async fn test_config_manager_creation() {
        let manager = ConfigManager::new(None).await;
        assert!(manager.is_ok());
    }

    #[tokio::test]
    async fn test_environment_config() {
        let manager = ConfigManager::with_environment(None, "development").await;
        assert!(manager.is_ok());

        let manager = manager.unwrap();
        let config = manager.get_config().await;
        assert_eq!(config.general.instance_name, "SeeSea");
        assert!(config.general.debug);
        assert!(matches!(
            config.general.environment,
            crate::config::Environment::Development
        ));
    }

    #[tokio::test]
    async fn test_config_validation() {
        let manager = ConfigManager::new(None).await.unwrap();
        let result = manager.validate().await;
        assert!(result.is_valid);
    }

    #[tokio::test]
    async fn test_config_file_loading() -> Result<(), Box<dyn std::error::Error>> {
        // 创建临时配置文件
        let mut temp_file = NamedTempFile::new()?;
        let config_content = r#"
[general]
instance_name = "Test SeeSea"
debug = true

[server]
port = 8080

[search]
results_per_page = 20

[logging]
level = "debug"
"#;
        fs::write(temp_file.path(), config_content).await?;

        // 加载配置
        let manager = ConfigManager::new(Some(temp_file.path().to_path_buf())).await?;
        let config = manager.get_config().await;

        assert_eq!(config.general.instance_name, "Test SeeSea");
        assert_eq!(config.server.port, 8080);
        assert_eq!(config.search.results_per_page, 20);
        assert!(matches!(config.logging.level, crate::config::LogLevel::Debug));

        Ok(())
    }

    #[tokio::test]
    async fn test_production_ready_check() {
        let mut manager = ConfigManager::with_environment(None, "production").await.unwrap();

        // 生产配置应该是就绪的
        assert!(manager.is_production_ready().await);

        // 修改为不就绪的配置
        {
            let mut config = manager.config.write().await;
            config.server.secret_key = "change-me-in-production".to_string();
        }

        assert!(!manager.is_production_ready().await);
    }
}