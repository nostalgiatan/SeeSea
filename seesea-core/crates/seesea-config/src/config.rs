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

//! SeeSea 主配置类型定义

use serde::{Deserialize, Serialize};

use crate::LogLevel;
use crate::types::*;
use crate::*;

/// SeeSea 主配置结构
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SeeSeaConfig {
    /// 通用配置
    #[serde(default)]
    pub general: crate::general::GeneralConfig,
    /// 环境标识
    #[serde(default)]
    pub environment: Environment,
    /// 服务器配置
    #[serde(default)]
    pub server: crate::server::ServerConfig,
    /// 搜索配置
    #[serde(default)]
    pub search: crate::SearchConfig,
    /// 隐私保护配置
    #[serde(default)]
    pub privacy: crate::privacy::PrivacyConfig,
    /// 缓存配置
    #[serde(default)]
    pub cache: crate::CacheConfig,
    /// API 配置
    #[serde(default)]
    pub api: crate::ApiConfig,
    /// 日志配置
    #[serde(default)]
    pub logging: crate::logging::LoggingConfig,
    /// 搜索引擎配置
    #[serde(default)]
    pub engines: crate::engines::EnginesConfig,
    /// 向量数据库配置
    #[serde(default)]
    pub vector_store: crate::VectorStoreConfig,
    /// 网络配置
    #[serde(default)]
    pub network: crate::network::NetworkConfig,
    /// Raming 配置
    #[serde(default)]
    pub raming: crate::raming::RamingConfig,
    /// RSS 配置
    #[serde(default)]
    pub rss: crate::rss::RssConfig,
}

impl SeeSeaConfig {
    /// 创建开发环境配置
    #[allow(clippy::field_reassign_with_default)]
    pub fn development() -> Self {
        let mut config = Self::default();
        config.environment = Environment::Development;
        config.general.environment = Environment::Development;
        config.general.debug = true;
        config.logging.level = LogLevel::Debug;

        // 确保搜索配置符合验证规则
        config.search.max_concurrent_engines = 5;
        config.search.search_timeout = 30;
        config.search.formats = vec!["json".to_string()];

        // 确保服务器配置符合验证规则
        config.server.secret_key = "development-secret-key-1234567890123456".to_string();

        config
    }

    /// 创建测试环境配置
    #[allow(clippy::field_reassign_with_default)]
    pub fn testing() -> Self {
        let mut config = Self::default();
        config.environment = Environment::Testing;
        config.general.environment = Environment::Testing;
        config.general.debug = true;
        config.logging.level = LogLevel::Info;
        config
    }

    /// 创建生产环境配置
    #[allow(clippy::field_reassign_with_default)]
    pub fn production() -> Self {
        let mut config = Self::default();
        config.environment = Environment::Production;
        config.general.environment = Environment::Production;
        config.general.debug = false;
        config.logging.level = LogLevel::Warn;

        // 确保至少有一种输出格式
        if config.search.formats.is_empty() {
            config.search.formats.push("json".to_string());
        }

        // 配置 TLS（使用默认证书路径）
        config.server.tls = Some(crate::server::TlsConfig {
            enabled: true,
            cert_path: Some(std::path::PathBuf::from("/etc/ssl/certs/seesea.crt")),
            key_path: Some(std::path::PathBuf::from("/etc/ssl/private/seesea.key")),
            ca_path: None,
            verify_client: false,
        });

        // 配置 Tor
        config.privacy.enable_tor = true;
        config.privacy.tor_config.socks_port = 9050;

        config.privacy.fingerprint_protection.protection_level =
            crate::common::FingerprintLevel::Basic;
        config.logging.structured = true;
        config.general.enable_metrics = true;
        config.server.secret_key =
            "a-very-strong-secret-key-for-production-use-only-123456789012345678901234567890"
                .to_string();
        config
    }

    /// 验证配置
    pub fn validate(&self) -> ConfigValidationResult {
        crate::validator::validate_config(self)
    }

    /// 获取配置摘要
    pub fn get_summary(&self) -> ConfigSummary {
        ConfigSummary {
            config_path: String::new(),
            environment: format!("{:?}", self.environment),
            enabled_engines: 0, // TODO: 从 engines 配置计算
            total_engines: 0,
            enabled_proxies: 0,
            cache_enabled: true, // TODO: Get from cache config
            validation: self.validate(),
        }
    }

    /// 检查是否为生产就绪状态
    pub fn is_production_ready(&self) -> bool {
        let validation = self.validate();
        validation.is_valid && validation.errors.is_empty()
    }

    /// 获取配置建议
    pub fn get_config_recommendations(&self) -> Vec<String> {
        let validation = self.validate();
        validation.warnings.clone()
    }
}

/// 配置加载结果
#[derive(Debug, Clone)]
pub struct ConfigLoadResult {
    /// 配置对象
    pub config: SeeSeaConfig,
    /// 配置摘要
    pub summary: ConfigSummary,
    /// 加载时间戳
    pub load_time: chrono::DateTime<chrono::Utc>,
    /// 配置文件路径
    pub file_path: String,
    /// 是否使用默认值
    pub used_defaults: bool,
    /// 警告信息
    pub warnings: Vec<String>,
}

/// 配置摘要
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigSummary {
    /// 配置文件路径
    pub config_path: String,
    /// 环境名称
    pub environment: String,
    /// 启用的搜索引擎数量
    pub enabled_engines: usize,
    /// 总引擎数量
    pub total_engines: usize,
    /// 启用的代理数量
    pub enabled_proxies: usize,
    /// 缓存是否启用
    pub cache_enabled: bool,
    /// 验证结果
    pub validation: ConfigValidationResult,
}

/// 配置错误类型（使用 search/crates 的错误模块）
#[derive(Debug, Clone)]
pub enum ConfigError {
    /// IO 错误
    Io(String),
    /// 解析错误
    Parse(String),
    /// 验证失败
    Validation(ConfigValidationResult),
    /// 配置文件不存在
    NotFound(String),
    /// 权限错误
    Permission(String),
    /// 环境变量错误
    Environment(String),
    /// 配置冲突
    Conflict(String),
}

impl ConfigError {
    /// IoError 别名（兼容性）
    #[allow(non_snake_case)]
    pub fn IoError(msg: String) -> Self {
        Self::Io(msg)
    }

    /// ParseError 别名（兼容性）
    #[allow(non_snake_case)]
    pub fn ParseError(msg: String) -> Self {
        Self::Parse(msg)
    }

    /// ValidationFailed 别名（兼容性）
    #[allow(non_snake_case)]
    pub fn ValidationFailed(errors: Vec<String>) -> Self {
        let mut result = ConfigValidationResult::default();
        for error in errors {
            result.add_error(error);
        }
        Self::Validation(result)
    }

    /// FileNotFound 别名（兼容性）
    #[allow(non_snake_case)]
    pub fn FileNotFound(path: String) -> Self {
        Self::NotFound(path)
    }

    /// EnvironmentError 别名（兼容性）
    #[allow(non_snake_case)]
    pub fn EnvironmentError(msg: String) -> Self {
        Self::Environment(msg)
    }
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::Io(msg) => write!(f, "IO 错误: {msg}"),
            ConfigError::Parse(msg) => write!(f, "解析错误: {msg}"),
            ConfigError::Validation(result) => write!(f, "验证失败: {result:?}"),
            ConfigError::NotFound(msg) => write!(f, "配置文件不存在: {msg}"),
            ConfigError::Permission(msg) => write!(f, "权限错误: {msg}"),
            ConfigError::Environment(msg) => write!(f, "环境变量错误: {msg}"),
            ConfigError::Conflict(msg) => write!(f, "配置冲突: {msg}"),
        }
    }
}

impl std::error::Error for ConfigError {}

impl From<toml::de::Error> for ConfigError {
    fn from(err: toml::de::Error) -> Self {
        ConfigError::Parse(err.to_string())
    }
}

impl From<std::io::Error> for ConfigError {
    fn from(err: std::io::Error) -> Self {
        ConfigError::Io(err.to_string())
    }
}

/// 配置源
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConfigSource {
    /// 文件
    File(String),
    /// 环境变量
    Environment,
    /// 默认值
    Default,
    /// 命令行参数
    CommandLine,
}

/// 配置加载器
#[derive(Debug, Clone)]
pub struct ConfigLoader {
    /// 配置源
    pub sources: Vec<ConfigSource>,
    /// 当前环境
    pub environment: Option<Environment>,
    /// 是否启用验证
    pub validate: bool,
}

impl Default for ConfigLoader {
    fn default() -> Self {
        Self {
            sources: vec![
                ConfigSource::Default,
                ConfigSource::Environment,
                ConfigSource::File("config/default.toml".to_string()),
            ],
            environment: None,
            validate: true,
        }
    }
}

impl ConfigLoader {
    /// 创建新的配置加载器
    pub fn new() -> Self {
        Self::default()
    }

    /// 设置环境
    pub fn with_environment(mut self, env: Environment) -> Self {
        self.environment = Some(env);
        self
    }

    /// 添加配置源
    pub fn with_source(mut self, source: ConfigSource) -> Self {
        self.sources.push(source);
        self
    }

    /// 设置是否验证
    pub fn with_validation(mut self, validate: bool) -> Self {
        self.validate = validate;
        self
    }

    /// 加载配置
    pub fn load(self) -> Result<ConfigLoadResult, ConfigError> {
        let mut config = SeeSeaConfig::default();

        // 应用默认配置
        for source in &self.sources {
            match source {
                ConfigSource::Default => {
                    // 使用默认值
                }
                ConfigSource::Environment => {
                    self.apply_environment_overrides(&mut config)?;
                }
                ConfigSource::File(path) => {
                    if std::path::Path::new(path).exists() {
                        let file_config = self.load_from_file(path)?;
                        self.merge_config(&mut config, file_config);
                    }
                }
                ConfigSource::CommandLine => {
                    // TODO: 实现命令行参数解析
                }
            }
        }

        // 验证配置
        let validation = if self.validate {
            crate::validator::validate_config(&config)
        } else {
            ConfigValidationResult::valid()
        };

        if !validation.is_valid {
            return Err(ConfigError::Validation(validation));
        }

        let warnings = validation.warnings.clone();
        let summary = self.create_summary(&config, validation);

        Ok(ConfigLoadResult {
            config,
            summary,
            load_time: chrono::Utc::now(),
            file_path: String::new(),
            used_defaults: false,
            warnings,
        })
    }

    fn load_from_file(&self, path: &str) -> Result<SeeSeaConfig, ConfigError> {
        let content = std::fs::read_to_string(path)?;
        let config: SeeSeaConfig = toml::from_str(&content)?;
        Ok(config)
    }

    fn apply_environment_overrides(&self, _config: &mut SeeSeaConfig) -> Result<(), ConfigError> {
        // TODO: 实现环境变量覆盖逻辑
        Ok(())
    }

    fn merge_config(&self, target: &mut SeeSeaConfig, source: SeeSeaConfig) {
        // TODO: 实现完整的配置合并逻辑
        // For now, just replace the entire config
        *target = source;
    }

    fn create_summary(
        &self,
        config: &SeeSeaConfig,
        validation: ConfigValidationResult,
    ) -> ConfigSummary {
        ConfigSummary {
            config_path: "config".to_string(),
            environment: format!("{:?}", config.environment),
            enabled_engines: 0, // TODO: Get from engines config
            total_engines: 0,
            enabled_proxies: 0,
            cache_enabled: true, // TODO: Get from cache config
            validation,
        }
    }
}
