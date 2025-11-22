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

//! 引擎配置类型定义

use crate::config::common::{BaseEngineConfig, ConfigValidationResult};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 引擎配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnginesConfig {
    /// 引擎列表
    pub engines: HashMap<String, EngineConfig>,
    /// 引擎分类配置
    pub categories: HashMap<String, CategoryConfig>,
    /// 全局引擎设置
    pub global_settings: GlobalEngineSettings,
    /// 引擎发现配置
    pub discovery: EngineDiscoveryConfig,
    /// 引擎健康检查配置
    pub health_check: HealthCheckConfig,
}

/// 引擎配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineConfig {
    /// 基础引擎配置
    pub base: BaseEngineConfig,
    /// 引擎网络配置
    pub network: EngineNetworkConfig,
    /// 引擎性能配置
    pub performance: EnginePerformanceConfig,
    /// 引擎结果配置
    pub results: EngineResultsConfig,
    /// 引擎特定配置
    pub specific: EngineSpecificConfig,
    /// 引擎依赖配置
    pub dependencies: EngineDependencies,
}

/// 引擎网络配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineNetworkConfig {
    /// 请求配置
    pub request: RequestConfig,
    /// 响应配置
    pub response: ResponseConfig,
    /// 重试配置
    pub retry: RetryConfig,
    /// 超时配置
    pub timeout: TimeoutConfig,
}

/// 请求配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestConfig {
    /// HTTP 方法
    pub method: HttpMethod,
    /// 请求头
    pub headers: HashMap<String, String>,
    /// 用户代理
    pub user_agent: Option<String>,
    /// 请求体
    pub body: Option<String>,
    /// 查询参数
    pub query_params: HashMap<String, String>,
    /// 是否启用压缩
    pub enable_compression: bool,
    /// 是否跟随重定向
    pub follow_redirects: bool,
    /// 最大重定向次数
    pub max_redirects: usize,
}

/// 响应配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseConfig {
    /// 期望的状态码
    pub expected_status_codes: Vec<u16>,
    /// 响应编码
    pub response_encoding: String,
    /// 是否验证 SSL 证书
    pub verify_ssl: bool,
    /// 是否使用代理
    pub use_proxy: bool,
    /// 代理配置覆盖
    pub proxy_override: Option<String>,
}

/// 重试配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryConfig {
    /// 是否启用重试
    pub enabled: bool,
    /// 最大重试次数
    pub max_retries: u32,
    /// 重试延迟策略
    pub delay_strategy: RetryDelayStrategy,
    /// 基础延迟（毫秒）
    pub base_delay: u64,
    /// 最大延迟（毫秒）
    pub max_delay: u64,
    /// 指数退避倍数
    pub backoff_multiplier: f32,
    /// 抖动因子
    pub jitter_factor: f32,
}

/// 重试延迟策略
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RetryDelayStrategy {
    /// 固定延迟
    Fixed,
    /// 线性增加
    Linear,
    /// 指数退避
    ExponentialBackoff,
    /// 自适应延迟
    Adaptive,
}

/// 超时配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeoutConfig {
    /// 连接超时（毫秒）
    pub connect_timeout: u64,
    /// 请求超时（毫秒）
    pub request_timeout: u64,
    /// 读取超时（毫秒）
    pub read_timeout: u64,
    /// 总超时（毫秒）
    pub total_timeout: u64,
    /// 是否启用缓慢请求检测
    pub enable_slow_request_detection: bool,
    /// 缓慢请求阈值（毫秒）
    pub slow_request_threshold: u64,
}

/// 引擎性能配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnginePerformanceConfig {
    /// 并发配置
    pub concurrency: ConcurrencyConfig,
    /// 缓存配置
    pub caching: EngineCachingConfig,
    /// 限流配置
    pub rate_limiting: EngineRateLimitConfig,
    /// 负载均衡配置
    pub load_balancing: LoadBalancingConfig,
}

/// 并发配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConcurrencyConfig {
    /// 最大并发请求数
    pub max_concurrent_requests: usize,
    /// 请求队列大小
    pub request_queue_size: usize,
    /// 是否启用请求批处理
    pub enable_batching: bool,
    /// 批处理大小
    pub batch_size: usize,
    /// 批处理超时（毫秒）
    pub batch_timeout: u64,
}

/// 引擎缓存配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineCachingConfig {
    /// 是否启用缓存
    pub enabled: bool,
    /// 缓存策略
    pub cache_strategy: CacheStrategy,
    /// 缓存 TTL（秒）
    pub cache_ttl: u64,
    /// 缓存键前缀
    pub cache_key_prefix: String,
    /// 是否缓存错误结果
    pub cache_errors: bool,
    /// 缓存大小限制
    pub cache_size_limit: Option<usize>,
}

/// 缓存策略
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CacheStrategy {
    /// 基于查询
    QueryBased,
    /// 基于时间
    TimeBased,
    /// 基于结果大小
    SizeBased,
    /// 自适应策略
    Adaptive,
}

/// 引擎限流配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineRateLimitConfig {
    /// 是否启用限流
    pub enabled: bool,
    /// 每秒请求数限制
    pub requests_per_second: u32,
    /// 每分钟请求数限制
    pub requests_per_minute: u32,
    /// 每小时请求数限制
    pub requests_per_hour: u32,
    /// 突发请求限制
    pub burst_size: u32,
    /// 限流算法
    pub algorithm: RateLimitAlgorithm,
}

/// 限流算法
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RateLimitAlgorithm {
    /// 令牌桶
    TokenBucket,
    /// 漏桶
    LeakyBucket,
    /// 固定窗口
    FixedWindow,
    /// 滑动窗口
    SlidingWindow,
}

/// 负载均衡配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadBalancingConfig {
    /// 是否启用负载均衡
    pub enabled: bool,
    /// 负载均衡策略
    pub strategy: LoadBalancingStrategy,
    /// 健康检查
    pub health_check: bool,
    /// 故障转移
    pub failover: bool,
    /// 服务节点列表
    pub nodes: Vec<ServiceNode>,
}

/// 负载均衡策略
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LoadBalancingStrategy {
    /// 轮询
    RoundRobin,
    /// 加权轮询
    WeightedRoundRobin,
    /// 最少连接
    LeastConnections,
    /// 随机
    Random,
    /// 一致性哈希
    ConsistentHash,
}

/// 服务节点
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceNode {
    /// 节点地址
    pub address: String,
    /// 节点端口
    pub port: u16,
    /// 节点权重
    pub weight: f32,
    /// 是否启用
    pub enabled: bool,
    /// 节点标签
    pub tags: HashMap<String, String>,
}

/// 引擎结果配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineResultsConfig {
    /// 结果解析配置
    pub parsing: ResultParsingConfig,
    /// 结果过滤配置
    pub filtering: ResultFilteringConfig,
    /// 结果排序配置
    pub sorting: ResultSortingConfig,
    /// 结果限制配置
    pub limiting: ResultLimitingConfig,
}

/// 结果解析配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResultParsingConfig {
    /// 解析器类型
    pub parser_type: ParserType,
    /// 选择器配置
    pub selectors: HashMap<String, String>,
    /// 正则表达式配置
    pub regex_patterns: HashMap<String, String>,
    /// 自定义解析脚本
    pub custom_parser: Option<String>,
    /// 字段映射
    pub field_mapping: HashMap<String, String>,
}

/// 解析器类型
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ParserType {
    /// HTML 解析器
    Html,
    /// JSON 解析器
    Json,
    /// XML 解析器
    Xml,
    /// 正则表达式解析器
    Regex,
    /// 自定义解析器
    Custom,
}

/// 结果过滤配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResultFilteringConfig {
    /// 是否启用过滤
    pub enabled: bool,
    /// URL 过滤规则
    pub url_filters: Vec<String>,
    /// 标题过滤规则
    pub title_filters: Vec<String>,
    /// 内容过滤规则
    pub content_filters: Vec<String>,
    /// 域名白名单
    pub domain_whitelist: Vec<String>,
    /// 域名黑名单
    pub domain_blacklist: Vec<String>,
}

/// 结果排序配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResultSortingConfig {
    /// 排序字段
    pub sort_by: Vec<String>,
    /// 排序方向
    pub sort_direction: SortDirection,
    /// 自定义排序函数
    pub custom_sorter: Option<String>,
}

/// 排序方向
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SortDirection {
    /// 升序
    Asc,
    /// 降序
    Desc,
}

/// 结果限制配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResultLimitingConfig {
    /// 最大结果数
    pub max_results: usize,
    /// 最小结果数
    pub min_results: usize,
    /// 是否截断结果
    pub truncate_results: bool,
    /// 截断长度
    pub truncate_length: usize,
}

/// 引擎特定配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineSpecificConfig {
    /// 引擎类型特定配置
    pub api_key: Option<String>,
    /// API 版本
    pub api_version: Option<String>,
    /// 端点 URL
    pub endpoint_url: Option<String>,
    /// 自定义参数
    pub custom_params: HashMap<String, serde_json::Value>,
    /// 认证配置
    pub authentication: Option<AuthenticationConfig>,
    /// 地区设置
    pub region: Option<String>,
    /// 语言设置
    pub language: Option<String>,
}

/// 认证配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthenticationConfig {
    /// 认证类型
    pub auth_type: EngineAuthType,
    /// 凭据
    pub credentials: HashMap<String, String>,
    /// 令牌刷新配置
    pub token_refresh: Option<TokenRefreshConfig>,
}

/// 引擎认证类型
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EngineAuthType {
    /// 无认证
    None,
    /// API 密钥
    ApiKey,
    /// OAuth 2.0
    OAuth2,
    /// 基础认证
    Basic,
    /// 摘要认证
    Digest,
    /// Bearer Token
    Bearer,
}

/// 令牌刷新配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenRefreshConfig {
    /// 令牌端点
    pub token_endpoint: String,
    /// 客户端 ID
    pub client_id: String,
    /// 客户端密钥
    pub client_secret: String,
    /// 刷新令牌
    pub refresh_token: String,
    /// 提前刷新时间（秒）
    pub refresh_before_expiry: u64,
}

/// 引擎依赖配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineDependencies {
    /// 必需的依赖
    pub required: Vec<String>,
    /// 可选的依赖
    pub optional: Vec<String>,
    /// 系统要求
    pub system_requirements: SystemRequirements,
}

/// 系统要求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemRequirements {
    /// 最小内存要求（MB）
    pub min_memory_mb: Option<usize>,
    /// 最小 CPU 核心数
    pub min_cpu_cores: Option<usize>,
    /// 所需的磁盘空间（MB）
    pub min_disk_space_mb: Option<usize>,
    /// 支持的平台
    pub supported_platforms: Vec<String>,
}

/// 分类配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategoryConfig {
    /// 分类名称
    pub name: String,
    /// 分类描述
    pub description: String,
    /// 分类图标
    pub icon: Option<String>,
    /// 默认引擎
    pub default_engines: Vec<String>,
    /// 分类权重
    pub weight: f32,
    /// 是否启用
    pub enabled: bool,
}

/// 全局引擎设置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalEngineSettings {
    /// 默认超时时间（秒）
    pub default_timeout: u64,
    /// 默认重试次数
    pub default_retries: u32,
    /// 最大并发引擎数
    pub max_concurrent_engines: usize,
    /// 引擎失败阈值
    pub failure_threshold: f32,
    /// 引擎恢复时间（秒）
    pub recovery_time: u64,
    /// 是否启用引擎监控
    pub enable_monitoring: bool,
    /// 引擎性能统计
    pub performance_stats: bool,
}

/// 引擎发现配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineDiscoveryConfig {
    /// 是否启用自动发现
    pub enabled: bool,
    /// 发现路径
    pub discovery_paths: Vec<String>,
    /// 发现模式
    pub discovery_patterns: Vec<String>,
    /// 是否启用热重载
    pub enable_hot_reload: bool,
    /// 重载间隔（秒）
    pub reload_interval: u64,
}

/// 健康检查配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckConfig {
    /// 是否启用健康检查
    pub enabled: bool,
    /// 检查间隔（秒）
    pub check_interval: u64,
    /// 检查超时（秒）
    pub check_timeout: u64,
    /// 失败阈值
    pub failure_threshold: u32,
    /// 恢复阈值
    pub recovery_threshold: u32,
    /// 健康检查端点
    pub health_endpoint: Option<String>,
    /// 自定义健康检查脚本
    pub custom_health_check: Option<String>,
}

/// HTTP 方法
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum HttpMethod {
    /// GET
    Get,
    /// POST
    Post,
    /// PUT
    Put,
    /// DELETE
    Delete,
    /// PATCH
    Patch,
    /// HEAD
    Head,
    /// OPTIONS
    Options,
}

impl Default for EnginesConfig {
    fn default() -> Self {
        Self {
            engines: HashMap::new(),
            categories: HashMap::new(),
            global_settings: GlobalEngineSettings::default(),
            discovery: EngineDiscoveryConfig::default(),
            health_check: HealthCheckConfig::default(),
        }
    }
}

impl EnginesConfig {
    /// 验证引擎配置
    pub fn validate(&self) -> ConfigValidationResult {
        let mut result = ConfigValidationResult::success();

        // 验证全局设置
        if self.global_settings.default_timeout == 0 {
            result.add_error("默认超时时间必须大于 0".to_string());
        }

        if self.global_settings.max_concurrent_engines == 0 {
            result.add_error("最大并发引擎数必须大于 0".to_string());
        }

        if self.global_settings.failure_threshold < 0.0 || self.global_settings.failure_threshold > 1.0 {
            result.add_error("失败阈值必须在 0.0-1.0 之间".to_string());
        }

        // 验证各个引擎配置
        for (engine_name, engine_config) in &self.engines {
            if engine_config.base.name != *engine_name {
                result.add_error(format!("引擎 {} 的名称与配置不匹配", engine_name));
            }

            if engine_config.base.weight < 0.0 {
                result.add_error(format!("引擎 {} 的权重不能为负数", engine_name));
            }

            if let Some(timeout) = engine_config.base.timeout {
                if timeout == 0 {
                    result.add_error(format!("引擎 {} 的超时时间必须大于 0", engine_name));
                }
            }
        }

        // 验证健康检查配置
        if let Some(health_check) = Some(&self.health_check) {
            if health_check.enabled {
                if health_check.check_interval == 0 {
                    result.add_error("健康检查间隔必须大于 0".to_string());
                }

                if health_check.check_timeout == 0 {
                    result.add_error("健康检查超时时间必须大于 0".to_string());
                }
            }
        }

        result
    }

    /// 获取启用的引擎列表
    pub fn get_enabled_engines(&self) -> Vec<String> {
        self.engines
            .iter()
            .filter(|(_, config)| config.base.enabled)
            .map(|(name, _)| name.clone())
            .collect()
    }

    /// 获取指定分类的引擎
    pub fn get_engines_by_category(&self, category: &str) -> Vec<String> {
        self.engines
            .iter()
            .filter(|(_, config)| {
                config.base.enabled && config.base.categories.contains(&category.to_string())
            })
            .map(|(name, _)| name.clone())
            .collect()
    }

    /// 获取总权重
    pub fn get_total_weight(&self, category: Option<&str>) -> f32 {
        self.engines
            .iter()
            .filter(|(_, config)| config.base.enabled)
            .filter(|(_, config)| {
                category.map_or(true, |cat| config.base.categories.contains(&cat.to_string()))
            })
            .map(|(_, config)| config.base.weight)
            .sum()
    }
}

impl Default for GlobalEngineSettings {
    fn default() -> Self {
        Self {
            default_timeout: 30,
            default_retries: 3,
            max_concurrent_engines: 5,
            failure_threshold: 0.5,
            recovery_time: 300, // 5 minutes
            enable_monitoring: true,
            performance_stats: true,
        }
    }
}

impl Default for EngineDiscoveryConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            discovery_paths: vec!["./engines".to_string()],
            discovery_patterns: vec!["*.rs".to_string(), "*.json".to_string()],
            enable_hot_reload: false,
            reload_interval: 60, // 1 minute
        }
    }
}

impl Default for HealthCheckConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            check_interval: 300, // 5 minutes
            check_timeout: 10,    // 10 seconds
            failure_threshold: 3,
            recovery_threshold: 2,
            health_endpoint: None,
            custom_health_check: None,
        }
    }
}

impl Default for EngineNetworkConfig {
    fn default() -> Self {
        Self {
            request: RequestConfig::default(),
            response: ResponseConfig::default(),
            retry: RetryConfig::default(),
            timeout: TimeoutConfig::default(),
        }
    }
}

impl Default for RequestConfig {
    fn default() -> Self {
        Self {
            method: HttpMethod::Get,
            headers: HashMap::new(),
            user_agent: None,
            body: None,
            query_params: HashMap::new(),
            enable_compression: true,
            follow_redirects: true,
            max_redirects: 5,
        }
    }
}

impl Default for ResponseConfig {
    fn default() -> Self {
        Self {
            expected_status_codes: vec![200],
            response_encoding: "utf-8".to_string(),
            verify_ssl: true,
            use_proxy: false,
            proxy_override: None,
        }
    }
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_retries: 3,
            delay_strategy: RetryDelayStrategy::ExponentialBackoff,
            base_delay: 1000, // 1 second
            max_delay: 30000, // 30 seconds
            backoff_multiplier: 2.0,
            jitter_factor: 0.1,
        }
    }
}

impl Default for TimeoutConfig {
    fn default() -> Self {
        Self {
            connect_timeout: 10000,    // 10 seconds
            request_timeout: 30000,    // 30 seconds
            read_timeout: 30000,       // 30 seconds
            total_timeout: 60000,      // 1 minute
            enable_slow_request_detection: true,
            slow_request_threshold: 5000, // 5 seconds
        }
    }
}