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

//! API 配置类型定义

use crate::config::common::{AuthType, ConfigValidationResult};
use serde::{Deserialize, Serialize};

/// API 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiConfig {
    /// 是否启用 API
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// API 版本
    pub version: String,
    /// 是否启用 CORS
    pub enable_cors: bool,
    /// CORS 配置
    pub cors: CorsConfig,
    /// 请求速率限制
    pub rate_limit: RateLimitConfig,
    /// 认证配置
    pub auth: AuthConfig,
    /// 响应格式配置
    pub response_format: ResponseFormatConfig,
    /// API 路由配置
    pub routes: RouteConfig,
    /// 中间件配置
    pub middleware: MiddlewareConfig,
    /// API 安全配置
    pub security: SecurityConfig,
    /// API 文档配置
    pub documentation: DocumentationConfig,
    /// 指标配置
    pub metrics: MetricsConfig,
}

fn default_true() -> bool {
    true
}

/// 指标配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsConfig {
    /// 是否启用指标
    #[serde(default)]
    pub enabled: bool,
    /// 指标端口
    #[serde(default = "default_metrics_port")]
    pub port: u16,
    /// 指标路径
    #[serde(default = "default_metrics_path")]
    pub path: String,
}

fn default_metrics_port() -> u16 {
    9090
}

fn default_metrics_path() -> String {
    "/metrics".to_string()
}

impl Default for MetricsConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            port: default_metrics_port(),
            path: default_metrics_path(),
        }
    }
}

/// CORS 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorsConfig {
    /// 是否启用
    pub enabled: bool,
    /// 允许的源
    pub allowed_origins: Vec<String>,
    /// 允许的方法
    pub allowed_methods: Vec<String>,
    /// 允许的头部
    pub allowed_headers: Vec<String>,
    /// 暴露的头部
    pub exposed_headers: Vec<String>,
    /// 是否允许凭证
    pub allow_credentials: bool,
    /// 预检请求缓存时间（秒）
    pub max_age: usize,
    /// 是否通配符源
    pub allow_wildcard_origin: bool,
}

/// 速率限制配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    /// 是否启用
    pub enabled: bool,
    /// 限制策略
    pub strategy: RateLimitStrategy,
    /// 每秒请求数限制
    pub requests_per_second: u32,
    /// 每分钟请求数限制
    pub requests_per_minute: u32,
    /// 每小时请求数限制
    pub requests_per_hour: u32,
    /// 每天请求数限制
    pub requests_per_day: u32,
    /// 突发请求限制
    pub burst_size: u32,
    /// 基于用户的限制
    pub user_based_limits: UserBasedLimits,
    /// 基于端点的限制
    pub endpoint_based_limits: EndpointBasedLimits,
}

/// 速率限制策略
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RateLimitStrategy {
    /// 固定窗口
    FixedWindow,
    /// 滑动窗口
    SlidingWindow,
    /// 令牌桶
    TokenBucket,
    /// 漏桶
    LeakyBucket,
}

/// 基于用户的限制
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserBasedLimits {
    /// 认证用户的限制倍数
    pub authenticated_multiplier: f32,
    /// 高级用户的限制倍数
    pub premium_multiplier: f32,
    /// 管理员豁免
    pub admin_exempt: bool,
}

/// 基于端点的限制
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EndpointBasedLimits {
    /// 搜索端点限制
    pub search_endpoint: Option<EndpointLimit>,
    /// 配置端点限制
    pub config_endpoint: Option<EndpointLimit>,
    /// 健康检查端点限制
    pub health_endpoint: Option<EndpointLimit>,
    /// 指标端点限制
    pub metrics_endpoint: Option<EndpointLimit>,
}

/// 端点限制配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EndpointLimit {
    /// 每秒请求数
    pub requests_per_second: u32,
    /// 每分钟请求数
    pub requests_per_minute: u32,
    /// 是否仅限制认证用户
    pub authenticated_only: bool,
}

/// 认证配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    /// 是否启用认证
    pub enabled: bool,
    /// 认证类型
    pub auth_type: AuthType,
    /// API 密钥配置
    pub api_key: ApiKeyConfig,
    /// JWT 配置
    pub jwt: JwtConfig,
    /// 基础认证配置
    pub basic_auth: BasicAuthConfig,
    /// OAuth 配置
    pub oauth: Option<OAuthConfig>,
}

/// API 密钥配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKeyConfig {
    /// 是否启用
    pub enabled: bool,
    /// 密钥列表
    pub api_keys: Vec<ApiKeyInfo>,
    /// 密钥头部名称
    pub header_name: String,
    /// 查询参数名称
    pub query_param: String,
    /// 密钥前缀
    pub key_prefix: String,
}

/// API 密钥信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKeyInfo {
    /// 密钥名称
    pub name: String,
    /// 密钥值（哈希存储）
    pub key_hash: String,
    /// 权限列表
    pub permissions: Vec<String>,
    /// 是否启用
    pub enabled: bool,
    /// 创建时间
    pub created_at: String,
    /// 过期时间（可选）
    pub expires_at: Option<String>,
    /// 使用限制
    pub usage_limits: Option<ApiKeyUsageLimits>,
}

/// API 密钥使用限制
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKeyUsageLimits {
    /// 每日请求限制
    pub daily_limit: Option<u32>,
    /// 每月请求限制
    pub monthly_limit: Option<u32>,
    /// 总请求限制
    pub total_limit: Option<u32>,
}

/// JWT 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JwtConfig {
    /// 是否启用
    pub enabled: bool,
    /// 密钥
    pub secret: String,
    /// 算法
    pub algorithm: JwtAlgorithm,
    /// 令牌过期时间（秒）
    pub expiry: u64,
    /// 刷新令牌过期时间（秒）
    pub refresh_expiry: u64,
    /// 发行者
    pub issuer: String,
    /// 受众
    pub audience: String,
}

/// JWT 算法
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum JwtAlgorithm {
    /// HS256
    HS256,
    /// HS384
    HS384,
    /// HS512
    HS512,
    /// RS256
    RS256,
    /// RS384
    RS384,
    /// RS512
    RS512,
}

/// 基础认证配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BasicAuthConfig {
    /// 是否启用
    pub enabled: bool,
    /// 用户列表
    pub users: Vec<BasicAuthUser>,
    /// 领域名称
    pub realm: String,
}

/// 基础认证用户
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BasicAuthUser {
    /// 用户名
    pub username: String,
    /// 密码哈希
    pub password_hash: String,
    /// 权限
    pub permissions: Vec<String>,
    /// 是否启用
    pub enabled: bool,
}

/// OAuth 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthConfig {
    /// OAuth 提供商
    pub provider: OAuthProvider,
    /// 客户端 ID
    pub client_id: String,
    /// 客户端密钥
    pub client_secret: String,
    /// 授权 URL
    pub auth_url: String,
    /// 令牌 URL
    pub token_url: String,
    /// 用户信息 URL
    pub user_info_url: String,
    /// 重定向 URL
    pub redirect_url: String,
    /// 作用域
    pub scopes: Vec<String>,
}

/// OAuth 提供商
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OAuthProvider {
    /// Google
    Google,
    /// GitHub
    Github,
    /// OAuth2.0 通用
    OAuth2,
}

/// 响应格式配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseFormatConfig {
    /// 默认格式
    pub default_format: String,
    /// 支持的格式
    pub supported_formats: Vec<ResponseFormat>,
    /// 是否包含调试信息
    pub include_debug_info: bool,
    /// 是否包含性能指标
    pub include_metrics: bool,
    /// 是否包含请求 ID
    pub include_request_id: bool,
    /// 响应压缩
    pub compression: ResponseCompressionConfig,
    /// 分页配置
    pub pagination: PaginationConfig,
}

/// 响应格式
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ResponseFormat {
    /// JSON 格式
    Json,
    /// XML 格式
    Xml,
    /// CSV 格式
    Csv,
    /// RSS 格式
    Rss,
    /// ATOM 格式
    Atom,
    /// HTML 格式
    Html,
    /// 纯文本格式
    Plain,
}

/// 响应压缩配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseCompressionConfig {
    /// 是否启用
    pub enabled: bool,
    /// 压缩算法
    pub algorithms: Vec<String>,
    /// 压缩阈值（字节）
    pub threshold: usize,
}

/// 分页配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginationConfig {
    /// 默认页面大小
    pub default_page_size: usize,
    /// 最大页面大小
    pub max_page_size: usize,
    /// 页码参数名
    pub page_param: String,
    /// 页面大小参数名
    pub page_size_param: String,
}

/// 路由配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteConfig {
    /// API 基础路径
    pub base_path: String,
    /// 版本策略
    pub versioning: VersioningStrategy,
    /// 自定义路由
    pub custom_routes: Vec<CustomRoute>,
    /// 路由组
    pub route_groups: Vec<RouteGroup>,
}

/// 版本策略
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VersioningStrategy {
    /// URL 路径版本
    UrlPath,
    /// 头部版本
    Header,
    /// 查询参数版本
    QueryParam,
    /// 无版本
    None,
}

/// 自定义路由
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomRoute {
    /// 路由路径
    pub path: String,
    /// HTTP 方法
    pub methods: Vec<HttpMethod>,
    /// 处理器名称
    pub handler: String,
    /// 是否需要认证
    pub auth_required: bool,
    /// 权限要求
    pub permissions: Vec<String>,
    /// 速率限制覆盖
    pub rate_limit_override: Option<EndpointLimit>,
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

/// 路由组
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteGroup {
    /// 组名称
    pub name: String,
    /// 前缀
    pub prefix: String,
    /// 组中间件
    pub middleware: Vec<String>,
    /// 是否需要认证
    pub auth_required: bool,
    /// 权限要求
    pub permissions: Vec<String>,
}

/// 中间件配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MiddlewareConfig {
    /// 启用的中间件列表
    pub enabled: Vec<String>,
    /// 中间件配置
    pub configs: std::collections::HashMap<String, serde_json::Value>,
    /// 执行顺序
    pub order: Vec<String>,
}

/// API 安全配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    /// 是否启用 HTTPS 强制
    pub force_https: bool,
    /// 安全头部
    pub security_headers: SecurityHeadersConfig,
    /// 输入验证
    pub input_validation: InputValidationConfig,
    /// 输出过滤
    pub output_filtering: OutputFilteringConfig,
}

/// 安全头部配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityHeadersConfig {
    /// 是否启用
    pub enabled: bool,
    /// 自定义头部
    pub custom_headers: std::collections::HashMap<String, String>,
}

/// 输入验证配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputValidationConfig {
    /// 最大查询长度
    pub max_query_length: usize,
    /// 允许的字符集
    pub allowed_characters: String,
    /// 是否启用 SQL 注入防护
    pub enable_sql_injection_protection: bool,
    /// 是否启用 XSS 防护
    pub enable_xss_protection: bool,
}

/// 输出过滤配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputFilteringConfig {
    /// 是否启用内容过滤
    pub enable_content_filtering: bool,
    /// 过滤规则
    pub filter_rules: Vec<FilterRule>,
}

/// 过滤规则
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterRule {
    /// 规则名称
    pub name: String,
    /// 规则类型
    pub rule_type: FilterRuleType,
    /// 规则模式
    pub pattern: String,
    /// 是否启用
    pub enabled: bool,
}

/// 过滤规则类型
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FilterRuleType {
    /// 正则表达式
    Regex,
    /// 字符串匹配
    StringMatch,
    /// 词汇列表
    WordList,
}

/// API 文档配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentationConfig {
    /// 是否启用
    pub enabled: bool,
    /// 文档类型
    pub doc_type: DocumentationType,
    /// 文档路径
    pub path: String,
    /// 是否包含示例
    pub include_examples: bool,
    /// 自定义样式
    pub custom_css: Option<String>,
}

/// 文档类型
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DocumentationType {
    /// OpenAPI 3.0
    OpenApi3,
    /// Swagger 2.0
    Swagger2,
    /// 自定义文档
    Custom,
}


impl Default for RouteConfig {
    fn default() -> Self {
        Self {
            base_path: "/api".to_string(),
            versioning: VersioningStrategy::None,
            custom_routes: vec![],
            route_groups: vec![],
        }
    }
}

impl Default for MiddlewareConfig {
    fn default() -> Self {
        Self {
            enabled: vec!["cors".to_string(), "logging".to_string()],
            configs: std::collections::HashMap::new(),
            order: vec!["cors".to_string(), "logging".to_string()],
        }
    }
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            force_https: true,
            security_headers: SecurityHeadersConfig::default(),
            input_validation: InputValidationConfig::default(),
            output_filtering: OutputFilteringConfig::default(),
        }
    }
}

impl Default for SecurityHeadersConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            custom_headers: std::collections::HashMap::new(),
        }
    }
}

impl Default for InputValidationConfig {
    fn default() -> Self {
        Self {
            max_query_length: 1000,
            allowed_characters: String::new(),
            enable_sql_injection_protection: true,
            enable_xss_protection: true,
        }
    }
}

impl Default for OutputFilteringConfig {
    fn default() -> Self {
        Self {
            enable_content_filtering: true,
            filter_rules: vec![],
        }
    }
}

impl Default for DocumentationConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            doc_type: DocumentationType::OpenApi3,
            path: "/docs".to_string(),
            include_examples: true,
            custom_css: None,
        }
    }
}

impl Default for ApiKeyConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            api_keys: vec![],
            header_name: "X-API-Key".to_string(),
            query_param: "api_key".to_string(),
            key_prefix: "sk_".to_string(),
        }
    }
}

impl Default for JwtConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            secret: String::new(),
            algorithm: JwtAlgorithm::HS256,
            expiry: 3600,
            refresh_expiry: 86400,
            issuer: "SeeSea".to_string(),
            audience: "SeeSea".to_string(),
        }
    }
}

impl Default for BasicAuthConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            users: vec![],
            realm: "SeeSea API".to_string(),
        }
    }
}

impl Default for ResponseCompressionConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            algorithms: vec!["gzip".to_string(), "deflate".to_string()],
            threshold: 1024,
        }
    }
}

impl Default for PaginationConfig {
    fn default() -> Self {
        Self {
            default_page_size: 10,
            max_page_size: 100,
            page_param: "page".to_string(),
            page_size_param: "page_size".to_string(),
        }
    }
}
impl Default for ApiConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            version: "v1".to_string(),
            enable_cors: true,
            cors: CorsConfig::default(),
            rate_limit: RateLimitConfig::default(),
            auth: AuthConfig::default(),
            response_format: ResponseFormatConfig::default(),
            routes: RouteConfig::default(),
            middleware: MiddlewareConfig::default(),
            security: SecurityConfig::default(),
            documentation: DocumentationConfig::default(),
            metrics: MetricsConfig::default(),
        }
    }
}

impl ApiConfig {
    /// 验证 API 配置
    pub fn validate(&self) -> ConfigValidationResult {
        let mut result = ConfigValidationResult::success();

        // 验证版本格式
        if self.version.is_empty() {
            result.add_error("API 版本不能为空".to_string());
        }

        // 验证速率限制
        if let Some(rate_limit) = Some(&self.rate_limit) {
            if rate_limit.enabled {
                if rate_limit.requests_per_second == 0
                    && rate_limit.requests_per_minute == 0
                    && rate_limit.requests_per_hour == 0
                    && rate_limit.requests_per_day == 0
                {
                    result.add_error("启用速率限制时必须指定至少一个时间段的限制".to_string());
                }

                if rate_limit.burst_size == 0 {
                    result.add_error("突发请求大小必须大于 0".to_string());
                }
            }
        }

        // 验证认证配置
        if let Some(auth) = Some(&self.auth) {
            if auth.enabled {
                match auth.auth_type {
                    AuthType::ApiKey => {
                        if auth.api_key.api_keys.is_empty() {
                            result.add_error("启用 API 密钥认证时必须指定至少一个密钥".to_string());
                        }
                    }
                    AuthType::Jwt => {
                        if auth.jwt.secret.is_empty() {
                            result.add_error("启用 JWT 认证时必须指定密钥".to_string());
                        }
                    }
                    AuthType::Basic => {
                        if auth.basic_auth.users.is_empty() {
                            result.add_error("启用基础认证时必须指定至少一个用户".to_string());
                        }
                    }
                    AuthType::None => {}
                }
            }
        }

        result
    }

    /// 检查是否需要认证
    pub fn requires_auth(&self) -> bool {
        self.auth.enabled
    }

    /// 获取支持格式的字符串列表
    pub fn get_supported_formats(&self) -> Vec<String> {
        self.response_format
            .supported_formats
            .iter()
            .map(|f| match f {
                ResponseFormat::Json => "json".to_string(),
                ResponseFormat::Xml => "xml".to_string(),
                ResponseFormat::Csv => "csv".to_string(),
                ResponseFormat::Rss => "rss".to_string(),
                ResponseFormat::Atom => "atom".to_string(),
                ResponseFormat::Html => "html".to_string(),
                ResponseFormat::Plain => "plain".to_string(),
            })
            .collect()
    }
}

impl Default for CorsConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            allowed_origins: vec!["*".to_string()],
            allowed_methods: vec![
                "GET".to_string(),
                "POST".to_string(),
                "PUT".to_string(),
                "DELETE".to_string(),
                "OPTIONS".to_string(),
            ],
            allowed_headers: vec!["*".to_string()],
            exposed_headers: vec![],
            allow_credentials: false,
            max_age: 86400, // 24 hours
            allow_wildcard_origin: true,
        }
    }
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            strategy: RateLimitStrategy::SlidingWindow,
            requests_per_second: 10,
            requests_per_minute: 100,
            requests_per_hour: 1000,
            requests_per_day: 10000,
            burst_size: 20,
            user_based_limits: UserBasedLimits::default(),
            endpoint_based_limits: EndpointBasedLimits::default(),
        }
    }
}

impl Default for UserBasedLimits {
    fn default() -> Self {
        Self {
            authenticated_multiplier: 2.0,
            premium_multiplier: 5.0,
            admin_exempt: true,
        }
    }
}

impl Default for EndpointBasedLimits {
    fn default() -> Self {
        Self {
            search_endpoint: Some(EndpointLimit {
                requests_per_second: 5,
                requests_per_minute: 30,
                authenticated_only: false,
            }),
            config_endpoint: Some(EndpointLimit {
                requests_per_second: 1,
                requests_per_minute: 10,
                authenticated_only: true,
            }),
            health_endpoint: Some(EndpointLimit {
                requests_per_second: 10,
                requests_per_minute: 100,
                authenticated_only: false,
            }),
            metrics_endpoint: Some(EndpointLimit {
                requests_per_second: 2,
                requests_per_minute: 20,
                authenticated_only: true,
            }),
        }
    }
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            auth_type: AuthType::None,
            api_key: ApiKeyConfig::default(),
            jwt: JwtConfig::default(),
            basic_auth: BasicAuthConfig::default(),
            oauth: None,
        }
    }
}

impl Default for ResponseFormatConfig {
    fn default() -> Self {
        Self {
            default_format: "json".to_string(),
            supported_formats: vec![
                ResponseFormat::Json,
                ResponseFormat::Xml,
                ResponseFormat::Csv,
            ],
            include_debug_info: false,
            include_metrics: true,
            include_request_id: true,
            compression: ResponseCompressionConfig::default(),
            pagination: PaginationConfig::default(),
        }
    }
}