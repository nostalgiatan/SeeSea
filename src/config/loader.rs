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

//! SeeSea 配置加载器
//!
//! 提供灵活的配置文件加载功能

use crate::config::{SeeSeaConfig, ConfigError, ConfigLoadResult};
use std::path::{Path, PathBuf};
use std::collections::HashMap;
use tokio::fs;

/// 配置加载器
pub struct ConfigLoader {
    /// 搜索路径
    search_paths: Vec<PathBuf>,
    /// 配置文件名模式
    file_patterns: Vec<String>,
    /// 环境变量前缀
    env_prefix: String,
    /// 默认配置
    defaults: HashMap<String, serde_json::Value>,
}

impl ConfigLoader {
    /// 创建新的配置加载器
    pub fn new() -> Self {
        Self {
            search_paths: vec![
                PathBuf::from("./config"),
                PathBuf::from("/etc/seesea"),
                PathBuf::from("./seesea_config"),
            ],
            file_patterns: vec![
                "default.toml".to_string(),
                "seesea.toml".to_string(),
                "config.toml".to_string(),
            ],
            env_prefix: "SEEA".to_string(),
            defaults: HashMap::new(),
        }
    }

    /// 添加搜索路径
    pub fn add_search_path<P: AsRef<Path>>(mut self, path: P) -> Self {
        self.search_paths.push(path.as_ref().to_path_buf());
        self
    }

    /// 添加文件模式
    pub fn add_file_pattern<S: Into<String>>(mut self, pattern: S) -> Self {
        self.file_patterns.push(pattern.into());
        self
    }

    /// 设置环境变量前缀
    pub fn set_env_prefix<S: Into<String>>(mut self, prefix: S) -> Self {
        self.env_prefix = prefix.into();
        self
    }

    /// 添加默认配置
    pub fn add_default<K: Into<String>, V: Into<serde_json::Value>>(
        mut self,
        key: K,
        value: V,
    ) -> Self {
        self.defaults.insert(key.into(), value.into());
        self
    }

    /// 从多个来源加载配置
    pub async fn load_from_sources(
        &self,
        sources: &[ConfigSource],
    ) -> Result<ConfigLoadResult, ConfigError> {
        let mut final_config = SeeSeaConfig::default();
        let mut loaded_files = Vec::new();
        let mut warnings = Vec::new();

        // 按优先级加载配置
        for source in sources {
            let mut config = match source {
                ConfigSource::File(path) => {
                    let config = self.load_from_file(path).await?;
                    loaded_files.push(path.clone());
                    config
                }
                ConfigSource::Environment => {
                    self.load_from_environment()?
                }
                ConfigSource::Defaults => {
                    self.load_from_defaults()?
                }
            };

            // 合并配置
            self.merge_config(&mut final_config, &mut config)?;
        }

        // 应用后处理
        self.post_process(&mut final_config).await?;

        // 验证配置
        let validation_result = final_config.validate();
        if !validation_result.is_valid {
            return Err(ConfigError::ValidationFailed(validation_result.errors));
        }

        warnings.extend(validation_result.warnings);
        let summary = final_config.get_summary();

        let load_result = ConfigLoadResult {
            config: final_config,
            file_path: loaded_files.first().map(|p| p.to_string_lossy().to_string()).unwrap_or_default(),
            used_defaults: loaded_files.is_empty(),
            warnings,
            summary,
            load_time: chrono::Utc::now(),
        };

        Ok(load_result)
    }

    /// 自动发现并加载配置
    pub async fn auto_load(&self) -> Result<ConfigLoadResult, ConfigError> {
        let config_file = self.find_config_file().await?;

        let sources = vec![
            ConfigSource::Defaults,
            ConfigSource::File(config_file),
            ConfigSource::Environment,
        ];

        self.load_from_sources(&sources).await
    }

    /// 从指定文件加载配置
    pub async fn load_from_file<P: AsRef<Path>>(
        &self,
        path: P,
    ) -> Result<SeeSeaConfig, ConfigError> {
        let path = path.as_ref();
        let content = fs::read_to_string(path).await
            .map_err(|e| ConfigError::IoError(format!("读取配置文件失败: {}", e)))?;

        let config = self.parse_config_content(&content, path)?;
        Ok(config)
    }

    /// 从环境变量加载配置
    pub fn load_from_environment(&self) -> Result<SeeSeaConfig, ConfigError> {
        let mut config = SeeSeaConfig::default();

        // 应用环境变量覆盖
        self.apply_env_overrides(&mut config)?;

        Ok(config)
    }

    /// 从默认配置加载
    pub fn load_from_defaults(&self) -> Result<SeeSeaConfig, ConfigError> {
        let mut config = SeeSeaConfig::default();

        // 应用默认值
        self.apply_defaults(&mut config);

        Ok(config)
    }

    /// 查找配置文件
    pub async fn find_config_file(&self) -> Result<PathBuf, ConfigError> {
        for search_path in &self.search_paths {
            for pattern in &self.file_patterns {
                let config_path = search_path.join(pattern);
                if config_path.exists() {
                    return Ok(config_path);
                }
            }
        }

        // 如果没找到，返回默认路径
        let default_path = self.search_paths
            .first()
            .map(|p| p.join(&self.file_patterns[0]))
            .ok_or_else(|| ConfigError::NotFound(String::new()))?;

        Ok(default_path)
    }

    /// 解析配置内容
    fn parse_config_content(&self, content: &str, path: &Path) -> Result<SeeSeaConfig, ConfigError> {
        match path.extension().and_then(|s| s.to_str()) {
            Some("toml") => {
                toml::from_str(content).map_err(|e| ConfigError::ParseError(format!("TOML 解析错误: {}", e)))
            }
            Some("json") => {
                serde_json::from_str(content).map_err(|e| ConfigError::ParseError(format!("JSON 解析错误: {}", e)))
            }
            Some("yaml") | Some("yml") => {
                // TODO: Add serde_yaml dependency if needed
                Err(ConfigError::Parse(format!("YAML support not yet implemented")))
            }
            Some(ext) => {
                Err(ConfigError::ParseError(format!("不支持的配置文件格式: {}", ext)))
            }
            None => {
                // 默认尝试 TOML
                toml::from_str(content).map_err(|e| ConfigError::ParseError(format!("TOML 解析错误: {}", e)))
            }
        }
    }

    /// 合并配置
    fn merge_config(&self, target: &mut SeeSeaConfig, source: &mut SeeSeaConfig) -> Result<(), ConfigError> {
        // 这里可以使用 serde_json 来深度合并配置
        // 简化实现：直接替换主要字段

        // 合并通用配置
        if source.general.instance_name != SeeSeaConfig::default().general.instance_name {
            target.general.instance_name = source.general.instance_name.clone();
        }
        if source.general.debug != SeeSeaConfig::default().general.debug {
            target.general.debug = source.general.debug;
        }

        // 类似地合并其他配置字段...
        self.merge_server_config(&mut target.server, &mut source.server)?;
        self.merge_search_config(&mut target.search, &mut source.search)?;
        self.merge_privacy_config(&mut target.privacy, &mut source.privacy)?;
        self.merge_cache_config(&mut target.cache, &mut source.cache)?;
        self.merge_api_config(&mut target.api, &mut source.api)?;
        self.merge_logging_config(&mut target.logging, &mut source.logging)?;
        self.merge_engines_config(&mut target.engines, &mut source.engines)?;

        Ok(())
    }

    /// 合并服务器配置
    fn merge_server_config(&self, target: &mut crate::config::ServerConfig, source: &mut crate::config::ServerConfig) -> Result<(), ConfigError> {
        if source.bind_address != crate::config::ServerConfig::default().bind_address {
            target.bind_address = source.bind_address.clone();
        }
        if source.port != crate::config::ServerConfig::default().port {
            target.port = source.port;
        }
        if source.secret_key != crate::config::ServerConfig::default().secret_key {
            target.secret_key = source.secret_key.clone();
        }
        Ok(())
    }

    /// 合并搜索配置
    fn merge_search_config(&self, target: &mut crate::config::SearchConfig, source: &mut crate::config::SearchConfig) -> Result<(), ConfigError> {
        if source.results_per_page != crate::config::SearchConfig::default().results_per_page {
            target.results_per_page = source.results_per_page;
        }
        if source.search_timeout != crate::config::SearchConfig::default().search_timeout {
            target.search_timeout = source.search_timeout;
        }
        Ok(())
    }

    /// 合并隐私配置
    fn merge_privacy_config(&self, target: &mut crate::config::PrivacyConfig, source: &mut crate::config::PrivacyConfig) -> Result<(), ConfigError> {
        if source.enable_tor != crate::config::PrivacyConfig::default().enable_tor {
            target.enable_tor = source.enable_tor;
        }
        if source.user_agent_rotation.enabled != crate::config::PrivacyConfig::default().user_agent_rotation.enabled {
            target.user_agent_rotation.enabled = source.user_agent_rotation.enabled;
        }
        Ok(())
    }

    /// 合并缓存配置
    fn merge_cache_config(&self, target: &mut crate::config::CacheConfig, source: &mut crate::config::CacheConfig) -> Result<(), ConfigError> {
        if source.enable_result_cache != crate::config::CacheConfig::default().enable_result_cache {
            target.enable_result_cache = source.enable_result_cache;
        }
        if source.ttl != crate::config::CacheConfig::default().ttl {
            target.ttl = source.ttl;
        }
        Ok(())
    }

    /// 合并 API 配置
    fn merge_api_config(&self, target: &mut crate::config::ApiConfig, source: &mut crate::config::ApiConfig) -> Result<(), ConfigError> {
        if source.version != crate::config::ApiConfig::default().version {
            target.version = source.version.clone();
        }
        if source.enable_cors != crate::config::ApiConfig::default().enable_cors {
            target.enable_cors = source.enable_cors;
        }
        Ok(())
    }

    /// 合并日志配置
    fn merge_logging_config(&self, target: &mut crate::config::LoggingConfig, source: &mut crate::config::LoggingConfig) -> Result<(), ConfigError> {
        if source.level != crate::config::LoggingConfig::default().level {
            target.level = source.level.clone();
        }
        if source.structured != crate::config::LoggingConfig::default().structured {
            target.structured = source.structured;
        }
        Ok(())
    }

    /// 合并引擎配置
    fn merge_engines_config(&self, target: &mut crate::config::EnginesConfig, source: &mut crate::config::EnginesConfig) -> Result<(), ConfigError> {
        // 合并引擎配置
        for (name, engine) in source.engines.drain() {
            target.engines.insert(name, engine);
        }
        Ok(())
    }

    /// 应用环境变量覆盖
    fn apply_env_overrides(&self, config: &mut SeeSeaConfig) -> Result<(), ConfigError> {
        // 服务器配置
        if let Ok(port) = std::env::var(&format!("{}_PORT", self.env_prefix)) {
            config.server.port = port.parse().map_err(|_| {
                ConfigError::EnvironmentError("无效的端口号".to_string())
            })?;
        }

        if let Ok(secret_key) = std::env::var(&format!("{}_SECRET_KEY", self.env_prefix)) {
            config.server.secret_key = secret_key;
        }

        if let Ok(bind_address) = std::env::var(&format!("{}_BIND_ADDRESS", self.env_prefix)) {
            config.server.bind_address = bind_address;
        }

        // 通用配置
        if let Ok(debug) = std::env::var(&format!("{}_DEBUG", self.env_prefix)) {
            config.general.debug = debug.parse().unwrap_or(false);
        }

        if let Ok(instance_name) = std::env::var(&format!("{}_INSTANCE_NAME", self.env_prefix)) {
            config.general.instance_name = instance_name;
        }

        // 日志配置
        if let Ok(log_level) = std::env::var(&format!("{}_LOG_LEVEL", self.env_prefix)) {
            config.logging.level = match log_level.to_lowercase().as_str() {
                "error" => crate::config::LogLevel::Error,
                "warn" => crate::config::LogLevel::Warn,
                "info" => crate::config::LogLevel::Info,
                "debug" => crate::config::LogLevel::Debug,
                "trace" => crate::config::LogLevel::Trace,
                _ => return Err(ConfigError::EnvironmentError("无效的日志级别".to_string())),
            };
        }

        // 缓存配置
        if let Ok(cache_enabled) = std::env::var(&format!("{}_CACHE_ENABLED", self.env_prefix)) {
            config.cache.enable_result_cache = cache_enabled.parse().unwrap_or(true);
        }

        // 搜索配置
        if let Ok(results_per_page) = std::env::var(&format!("{}_RESULTS_PER_PAGE", self.env_prefix)) {
            config.search.results_per_page = results_per_page.parse().map_err(|_| {
                ConfigError::EnvironmentError("无效的结果数量".to_string())
            })?;
        }

        Ok(())
    }

    /// 应用默认配置
    fn apply_defaults(&self, config: &mut SeeSeaConfig) {
        // 这里可以应用预定义的默认值
        for (key, value) in &self.defaults {
            self.apply_nested_default(config, key, value);
        }
    }

    /// 应用嵌套默认值
    fn apply_nested_default(&self, config: &mut SeeSeaConfig, key: &str, value: &serde_json::Value) {
        match key {
            "server.port" => {
                if let Some(port) = value.as_u64() {
                    config.server.port = port as u16;
                }
            }
            "general.debug" => {
                if let Some(debug) = value.as_bool() {
                    config.general.debug = debug;
                }
            }
            "logging.level" => {
                if let Some(level) = value.as_str() {
                    config.logging.level = match level {
                        "error" => crate::config::LogLevel::Error,
                        "warn" => crate::config::LogLevel::Warn,
                        "info" => crate::config::LogLevel::Info,
                        "debug" => crate::config::LogLevel::Debug,
                        "trace" => crate::config::LogLevel::Trace,
                        _ => return,
                    };
                }
            }
            _ => {}
        }
    }

    /// 配置后处理
    async fn post_process(&self, config: &mut SeeSeaConfig) -> Result<(), ConfigError> {
        // 确保目录存在
        self.ensure_directories_exist(config).await?;

        // 处理相对路径
        self.resolve_relative_paths(config);

        // 验证必要配置
        self.validate_required_config(config)?;

        Ok(())
    }

    /// 确保目录存在
    async fn ensure_directories_exist(&self, config: &SeeSeaConfig) -> Result<(), ConfigError> {
        let mut dirs: Vec<&std::path::Path> = vec![
            config.general.config_directory.as_path(),
            config.general.data_directory.as_path(),
            config.general.temp_directory.as_path(),
        ];
        
        if let Some(parent) = config.cache.database_path.parent() {
            dirs.push(parent);
        }

        for dir in dirs {
            if !dir.exists() {
                fs::create_dir_all(dir).await.map_err(|e| {
                    ConfigError::IoError(format!("创建目录失败 {:?}: {}", dir, e))
                })?;
            }
        }

        Ok(())
    }

    /// 解析相对路径
    fn resolve_relative_paths(&self, config: &mut SeeSeaConfig) {
        // 将相对路径转换为绝对路径
        if !config.general.config_directory.is_absolute() {
            config.general.config_directory = std::env::current_dir()
                .unwrap_or_else(|_| PathBuf::from("."))
                .join(&config.general.config_directory);
        }

        if !config.general.data_directory.is_absolute() {
            config.general.data_directory = std::env::current_dir()
                .unwrap_or_else(|_| PathBuf::from("."))
                .join(&config.general.data_directory);
        }

        if !config.cache.database_path.is_absolute() {
            config.cache.database_path = config.general.data_directory.join(&config.cache.database_path);
        }
    }

    /// 验证必要配置
    fn validate_required_config(&self, config: &SeeSeaConfig) -> Result<(), ConfigError> {
        if config.server.secret_key == "change-me-in-production" {
            return Err(ConfigError::Conflict("生产环境需要更改默认密钥".to_string()));
        }

        if config.server.bind_address.is_empty() {
            return Err(ConfigError::Conflict("服务器绑定地址不能为空".to_string()));
        }

        Ok(())
    }
}

impl Default for ConfigLoader {
    fn default() -> Self {
        Self::new()
    }
}

/// 配置来源
#[derive(Debug, Clone)]
pub enum ConfigSource {
    /// 从文件加载
    File(PathBuf),
    /// 从环境变量加载
    Environment,
    /// 从默认配置加载
    Defaults,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[tokio::test]
    async fn test_config_loader_creation() {
        let loader = ConfigLoader::new();
        assert_eq!(loader.file_patterns.len(), 3);
        assert_eq!(loader.search_paths.len(), 3);
    }

    #[tokio::test]
    async fn test_config_loading() -> Result<(), Box<dyn std::error::Error>> {
        let mut temp_file = NamedTempFile::new()?;
        let config_content = r#"
[general]
instance_name = "Test"

[server]
port = 9999
"#;
        fs::write(temp_file.path(), config_content).await?;

        let loader = ConfigLoader::new();
        let config = loader.load_from_file(temp_file.path()).await?;

        assert_eq!(config.general.instance_name, "Test");
        assert_eq!(config.server.port, 9999);

        Ok(())
    }

    #[test]
    #[serial_test::serial]
    fn test_environment_overrides() {
        // Using temp-env provides safe environment variable manipulation for tests
        // It automatically cleans up even on panic
        temp_env::with_vars(
            vec![
                ("SEEA_PORT", Some("8888")),
                ("SEEA_DEBUG", Some("true")),
            ],
            || {
                // Use tokio test runtime
                let runtime = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .unwrap();
                    
                runtime.block_on(async {
                    let loader = ConfigLoader::new();
                    let result = loader.load_from_environment();

                    assert!(result.is_ok());
                    let config = result.unwrap();
                    assert_eq!(config.server.port, 8888);
                    assert!(config.general.debug);
                });
            },
        );
    }
}