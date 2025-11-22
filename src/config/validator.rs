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

//! SeeSea 配置验证器
//!
//! 提供详细的配置验证功能

use crate::config::{
    SeeSeaConfig, ConfigValidationResult, Environment, LogLevel, EngineLoadingMode,
};

/// 配置验证器
pub struct ConfigValidator {
    /// 验证规则
    rules: Vec<ValidationRule>,
    /// 验证器配置
    #[allow(dead_code)]
    config: ValidatorConfig,
}

impl ConfigValidator {
    /// 创建新的配置验证器
    pub fn new() -> Self {
        let mut validator = Self {
            rules: Vec::new(),
            config: ValidatorConfig::default(),
        };

        // 添加默认验证规则
        validator.add_default_rules();
        validator
    }

    /// 创建带配置的验证器
    pub fn with_config(config: ValidatorConfig) -> Self {
        let mut validator = Self {
            rules: Vec::new(),
            config,
        };

        validator.add_default_rules();
        validator
    }

    /// 添加验证规则
    pub fn add_rule(mut self, rule: ValidationRule) -> Self {
        self.rules.push(rule);
        self
    }

    /// 验证配置
    pub fn validate(&self, config: &SeeSeaConfig) -> ConfigValidationResult {
        let mut result = ConfigValidationResult::success();

        // 环境特定验证
        self.validate_environment(config, &mut result);

        // 安全性验证
        self.validate_security(config, &mut result);

        // 性能验证
        self.validate_performance(config, &mut result);

        // 兼容性验证
        self.validate_compatibility(config, &mut result);

        // 自定义规则验证
        self.validate_custom_rules(config, &mut result);

        result
    }

    /// 验证环境特定配置
    fn validate_environment(&self, config: &SeeSeaConfig, result: &mut ConfigValidationResult) {
        match config.general.environment {
            Environment::Production => {
                // 生产环境必须检查的项目
                if config.general.debug {
                    result.add_error("生产环境不能启用调试模式".to_string());
                }

                if config.server.secret_key == "change-me-in-production" {
                    result.add_error("生产环境必须更改默认密钥".to_string());
                }

                if !config.api.security.force_https {
                    result.add_error("生产环境必须启用 HTTPS".to_string());
                }

                if !config.logging.structured {
                    result.add_error("生产环境建议启用结构化日志".to_string());
                }

                if config.server.secret_key.len() < 32 {
                    result.add_error("生产环境密钥长度至少为 32 个字符".to_string());
                }
            }
            Environment::Development => {
                // 开发环境检查
                if config.server.port == 80 {
                    result.add_warning("开发环境使用标准 HTTP 端口可能与其他服务冲突".to_string());
                }

                if !config.api.documentation.enabled {
                    result.add_warning("开发环境建议启用 API 文档".to_string());
                }
            }
            Environment::Testing => {
                // 测试环境检查
                if config.cache.enable_result_cache {
                    result.add_warning("测试环境禁用缓存可能提高测试可靠性".to_string());
                }

                if config.privacy.enable_tor {
                    result.add_warning("测试环境启用 Tor 可能影响测试速度".to_string());
                }
            }
            Environment::Staging => {
                // 预生产环境检查
                if config.general.debug {
                    result.add_warning("预生产环境不建议启用调试模式".to_string());
                }

                if config.logging.level != LogLevel::Info {
                    result.add_warning("预生产环境建议使用 Info 日志级别".to_string());
                }
            }
        }
    }

    /// 验证安全性
    fn validate_security(&self, config: &SeeSeaConfig, result: &mut ConfigValidationResult) {
        // 密钥验证
        if config.server.secret_key.len() < 16 {
            result.add_error("服务器密钥长度至少为 16 个字符".to_string());
        }

        // API 安全验证
        if config.api.enabled && config.api.auth.enabled {
            match config.api.auth.auth_type {
                crate::config::AuthType::ApiKey => {
                    if config.api.auth.api_key.api_keys.is_empty() {
                        result.add_error("启用 API 密钥认证时必须指定至少一个密钥".to_string());
                    }
                }
                crate::config::AuthType::Jwt => {
                    if config.api.auth.jwt.secret.is_empty() {
                        result.add_error("启用 JWT 认证时必须指定密钥".to_string());
                    }
                    if config.api.auth.jwt.secret.len() < 32 {
                        result.add_error("JWT 密钥长度至少为 32 个字符".to_string());
                    }
                }
                crate::config::AuthType::Basic => {
                    if config.api.auth.basic_auth.users.is_empty() {
                        result.add_error("启用基础认证时必须指定至少一个用户".to_string());
                    }
                }
                crate::config::AuthType::None => {
                    result.add_warning("未启用认证可能存在安全风险".to_string());
                }
            }
        }

        // TLS 验证
        if let Some(tls) = &config.server.tls {
            if tls.enabled {
                if tls.cert_path.is_none() {
                    result.add_error("启用 TLS 时必须指定证书文件路径".to_string());
                }
                if tls.key_path.is_none() {
                    result.add_error("启用 TLS 时必须指定私钥文件路径".to_string());
                }
            }
        }

        // 隐私配置验证
        if config.privacy.enable_tor {
            if config.privacy.tor_config.socks_port == 0 {
                result.add_error("启用 Tor 时必须指定有效的 SOCKS 端口".to_string());
            }
        }

        if !config.privacy.proxy_chain.is_empty() {
            for (i, proxy) in config.privacy.proxy_chain.iter().enumerate() {
                if proxy.enabled {
                    if proxy.address.is_empty() {
                        result.add_error(format!("代理 {} 的地址不能为空", i + 1));
                    }
                    // u16 最大值为 65535，所以只需检查 0
                    if proxy.port == 0 {
                        result.add_error(format!("代理 {} 的端口无效", i + 1));
                    }
                }
            }
        }
    }

    /// 验证性能配置
    fn validate_performance(&self, config: &SeeSeaConfig, result: &mut ConfigValidationResult) {
        // 搜索性能验证
        if config.search.max_concurrent_engines == 0 {
            result.add_error("最大并发引擎数必须大于 0".to_string());
        }

        if config.search.max_concurrent_engines > 50 {
            result.add_warning("并发引擎数过多可能影响系统性能".to_string());
        }

        if config.search.search_timeout == 0 {
            result.add_error("搜索超时时间必须大于 0".to_string());
        }

        if config.search.search_timeout > 300 {
            result.add_warning("搜索超时时间过长（>5分钟）".to_string());
        }

        // 缓存性能验证
        if config.cache.enable_result_cache {
            if config.cache.max_size == 0 {
                result.add_error("启用缓存时最大缓存大小必须大于 0".to_string());
            }

            if config.cache.ttl == 0 {
                result.add_error("启用缓存时 TTL 必须大于 0".to_string());
            }

            if config.cache.max_size > 10 * 1024 * 1024 * 1024 { // 10GB
                result.add_warning("缓存大小过大可能影响内存使用".to_string());
            }
        }

        // 日志性能验证
        if config.logging.performance.enabled {
            if config.logging.performance.buffer_size == 0 {
                result.add_error("启用性能日志时缓冲区大小必须大于 0".to_string());
            }

            if config.logging.performance.flush_interval == 0 {
                result.add_error("启用性能日志时刷新间隔必须大于 0".to_string());
            }
        }

        // 隐私与性能权衡验证
        match config.privacy.fingerprint_protection.protection_level {
            crate::config::FingerprintLevel::Advanced | crate::config::FingerprintLevel::Maximum => {
                if config.search.max_concurrent_engines > 20 {
                    result.add_warning("高级指纹保护与高并发可能影响性能".to_string());
                }
            }
            crate::config::FingerprintLevel::None => {
                result.add_warning("未启用指纹保护可能影响隐私安全".to_string());
            }
            _ => {}
        }
    }

    /// 验证配置兼容性
    fn validate_compatibility(&self, config: &SeeSeaConfig, result: &mut ConfigValidationResult) {
        // 引擎加载模式验证
        match config.general.engine_loading_mode {
            EngineLoadingMode::Global => {
                if config.engines.engines.is_empty() {
                    result.add_warning("全局模式但未发现任何引擎配置".to_string());
                }
            }
            EngineLoadingMode::Settings => {
                if config.engines.engines.is_empty() {
                    result.add_error("设置模式但未指定任何引擎".to_string());
                }
            }
        }

        // 端口冲突检查
        let used_ports = vec![
            config.server.port,
            config.api.metrics.port,
        ];
        let mut port_set = std::collections::HashSet::new();
        for port in &used_ports {
            if port_set.contains(port) {
                result.add_error(format!("端口 {} 被重复使用", port));
            }
            port_set.insert(*port);
        }

        // 路径兼容性检查
        if !config.general.config_directory.is_absolute() {
            result.add_warning("建议使用绝对路径配置目录".to_string());
        }

        if !config.general.data_directory.is_absolute() {
            result.add_warning("建议使用绝对路径数据目录".to_string());
        }

        // 格式兼容性检查
        let supported_formats = &config.search.formats;
        if supported_formats.is_empty() {
            result.add_error("必须指定至少一种输出格式".to_string());
        }

        // API 版本兼容性
        if !config.api.version.starts_with("v") {
            result.add_warning("API 版本建议以 'v' 开头，如 'v1'".to_string());
        }
    }

    /// 验证自定义规则
    fn validate_custom_rules(&self, config: &SeeSeaConfig, result: &mut ConfigValidationResult) {
        for rule in &self.rules {
            let rule_result = rule.validate(config);
            if !rule_result.is_valid {
                for error in rule_result.errors {
                    result.add_error(format!("规则 '{}': {}", rule.name, error));
                }
            }
            for warning in rule_result.warnings {
                result.add_warning(format!("规则 '{}': {}", rule.name, warning));
            }
        }
    }

    /// 添加默认验证规则
    fn add_default_rules(&mut self) {
        // 引擎健康检查规则
        self.rules.push(ValidationRule {
            name: "引擎健康检查".to_string(),
            validator: Box::new(|config| {
                let mut result = ConfigValidationResult::success();

                let enabled_engines = config.engines.get_enabled_engines();
                if enabled_engines.is_empty() {
                    result.add_error("没有启用的搜索引擎".to_string());
                }

                result
            }),
        });

        // 缓存路径有效性规则
        self.rules.push(ValidationRule {
            name: "缓存路径有效性".to_string(),
            validator: Box::new(|config| {
                let mut result = ConfigValidationResult::success();

                if config.cache.enable_result_cache {
                    let cache_path = &config.cache.database_path;
                    if let Some(parent) = cache_path.parent() {
                        if parent.as_os_str().is_empty() {
                            result.add_error("缓存路径的父目录不能为空".to_string());
                        }
                    }
                }

                result
            }),
        });

        // 日志级别一致性规则
        self.rules.push(ValidationRule {
            name: "日志级别一致性".to_string(),
            validator: Box::new(|config| {
                let mut result = ConfigValidationResult::success();

                match config.general.environment {
                    Environment::Production => {
                        if matches!(config.logging.level, LogLevel::Debug | LogLevel::Trace) {
                            result.add_warning("生产环境使用调试日志级别可能影响性能".to_string());
                        }
                    }
                    Environment::Development => {
                        if matches!(config.logging.level, LogLevel::Error) {
                            result.add_warning("开发环境使用错误日志级别可能缺少调试信息".to_string());
                        }
                    }
                    _ => {}
                }

                result
            }),
        });
    }

    /// 生成配置报告
    pub fn generate_report(&self, config: &SeeSeaConfig) -> ConfigReport {
        let validation_result = self.validate(config);

        ConfigReport {
            timestamp: chrono::Utc::now(),
            environment: format!("{:?}", config.general.environment),
            is_valid: validation_result.is_valid,
            errors: validation_result.errors,
            warnings: validation_result.warnings,
            recommendations: self.generate_recommendations(config),
            summary: self.generate_summary(config),
        }
    }

    /// 生成配置建议
    fn generate_recommendations(&self, config: &SeeSeaConfig) -> Vec<String> {
        let mut recommendations = Vec::new();

        // 安全建议
        if config.api.auth.enabled {
            recommendations.push("已启用 API 认证，安全性良好".to_string());
        } else {
            recommendations.push("建议启用 API 认证以提高安全性".to_string());
        }

        // 性能建议
        if config.cache.enable_result_cache {
            recommendations.push("已启用结果缓存，性能良好".to_string());
        } else {
            recommendations.push("建议启用结果缓存以提高性能".to_string());
        }

        // 隐私建议
        if config.privacy.enable_tor || !config.privacy.proxy_chain.is_empty() {
            recommendations.push("已启用隐私保护功能".to_string());
        } else {
            recommendations.push("建议启用 Tor 或代理以提高隐私保护".to_string());
        }

        // 监控建议
        if config.general.enable_metrics {
            recommendations.push("已启用指标收集，监控良好".to_string());
        } else {
            recommendations.push("建议启用指标收集以监控系统状态".to_string());
        }

        recommendations
    }

    /// 生成配置摘要
    fn generate_summary(&self, config: &SeeSeaConfig) -> ConfigSummary {
        let validation_result = self.validate(config);

        ConfigSummary {
            total_rules: self.rules.len(),
            passed_rules: self.rules.len() - validation_result.errors.len(),
            failed_rules: validation_result.errors.len(),
            warning_count: validation_result.warnings.len(),
            environment: format!("{:?}", config.general.environment),
            security_score: self.calculate_security_score(config),
            performance_score: self.calculate_performance_score(config),
        }
    }

    /// 计算安全评分
    fn calculate_security_score(&self, config: &SeeSeaConfig) -> u8 {
        let mut score = 0;

        // 基础安全检查 (40 分)
        if config.api.auth.enabled { score += 10; }
        if !config.server.secret_key.contains("change") { score += 10; }
        if config.server.tls.as_ref().map(|t| t.enabled).unwrap_or(false) { score += 10; }
        if matches!(config.general.environment, Environment::Production) && !config.general.debug {
            score += 10;
        }

        // 高级安全检查 (40 分)
        if config.privacy.enable_tor { score += 10; }
        if !config.privacy.proxy_chain.is_empty() { score += 10; }
        if config.privacy.fingerprint_protection.protection_level != crate::config::FingerprintLevel::None {
            score += 10;
        }
        if config.api.rate_limit.enabled { score += 10; }

        // 其他安全检查 (20 分)
        if config.logging.structured { score += 10; }
        if config.general.enable_metrics { score += 10; }

        score
    }

    /// 计算性能评分
    fn calculate_performance_score(&self, config: &SeeSeaConfig) -> u8 {
        let mut score = 0;

        // 缓存优化 (30 分)
        if config.cache.enable_result_cache { score += 15; }
        if config.cache.enable_metadata_cache { score += 10; }
        if config.cache.enable_dns_cache { score += 5; }

        // 并发优化 (30 分)
        if config.search.max_concurrent_engines >= 5 { score += 10; }
        if config.search.max_concurrent_engines <= 20 { score += 10; }
        if config.logging.performance.enabled { score += 10; }

        // 网络优化 (20 分)
        if config.privacy.request_timing.max_delay > config.privacy.request_timing.min_delay {
            score += 10;
        }
        if config.cache.compression.enabled { score += 10; }

        // 其他优化 (20 分)
        if !config.search.aggregation.enable_deduplication ||
           (config.cache.enable_result_cache && config.search.aggregation.enable_deduplication) {
            score += 10;
        }
        if config.engines.global_settings.enable_monitoring { score += 10; }

        score
    }
}

impl Default for ConfigValidator {
    fn default() -> Self {
        Self::new()
    }
}

/// 验证规则
pub struct ValidationRule {
    /// 规则名称
    pub name: String,
    /// 验证器
    pub validator: Box<dyn Fn(&SeeSeaConfig) -> ConfigValidationResult + Send + Sync>,
}

impl ValidationRule {
    /// 执行验证
    pub fn validate(&self, config: &SeeSeaConfig) -> ConfigValidationResult {
        (self.validator)(config)
    }
}

/// 验证器配置
#[derive(Debug, Clone)]
pub struct ValidatorConfig {
    /// 是否启用严格模式
    pub strict_mode: bool,
    /// 是否启用警告
    pub enable_warnings: bool,
    /// 最大错误数量
    pub max_errors: Option<usize>,
    /// 最大警告数量
    pub max_warnings: Option<usize>,
}

impl Default for ValidatorConfig {
    fn default() -> Self {
        Self {
            strict_mode: false,
            enable_warnings: true,
            max_errors: None,
            max_warnings: None,
        }
    }
}

/// 配置报告
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ConfigReport {
    /// 报告时间
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// 环境
    pub environment: String,
    /// 是否有效
    pub is_valid: bool,
    /// 错误列表
    pub errors: Vec<String>,
    /// 警告列表
    pub warnings: Vec<String>,
    /// 建议
    pub recommendations: Vec<String>,
    /// 摘要
    pub summary: ConfigSummary,
}

/// 配置摘要
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ConfigSummary {
    /// 总规则数
    pub total_rules: usize,
    /// 通过的规则数
    pub passed_rules: usize,
    /// 失败的规则数
    pub failed_rules: usize,
    /// 警告数量
    pub warning_count: usize,
    /// 环境
    pub environment: String,
    /// 安全评分 (0-100)
    pub security_score: u8,
    /// 性能评分 (0-100)
    pub performance_score: u8,
}

/// 验证配置的便利函数
pub fn validate_config(config: &crate::config::config::SeeSeaConfig) -> ConfigValidationResult {
    let validator = ConfigValidator::new();
    validator.validate(config)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_validator_creation() {
        let validator = ConfigValidator::new();
        assert!(!validator.rules.is_empty());
    }

    #[test]
    fn test_production_validation() {
        let validator = ConfigValidator::new();

        let mut config = SeeSeaConfig::production();
        config.general.debug = true; // 这会导致验证失败

        let result = validator.validate(&config);
        assert!(!result.is_valid);
        assert!(result.errors.iter().any(|e| e.contains("生产环境不能启用调试模式")));
    }

    #[test]
    fn test_security_score() {
        let validator = ConfigValidator::new();

        let config = SeeSeaConfig::production();
        let score = validator.calculate_security_score(&config);

        // 生产配置应该有较高的安全评分
        assert!(score >= 70);
    }

    #[test]
    fn test_performance_score() {
        let validator = ConfigValidator::new();

        let mut config = SeeSeaConfig::default();
        config.cache.enable_result_cache = true;
        config.search.max_concurrent_engines = 10;

        let score = validator.calculate_performance_score(&config);
        assert!(score >= 40);
    }
}