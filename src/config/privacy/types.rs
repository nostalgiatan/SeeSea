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

//! 隐私保护配置类型定义

use crate::config::common::{ConfigValidationResult, FingerprintLevel, ProxyType, TimingStrategy};
use serde::{Deserialize, Serialize};

/// 隐私保护配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrivacyConfig {
    /// User-Agent 轮换
    pub user_agent_rotation: UserAgentRotationConfig,
    /// 代理链配置
    pub proxy_chain: Vec<ProxyConfig>,
    /// 是否启用 Tor
    pub enable_tor: bool,
    /// Tor 配置
    pub tor_config: TorConfig,
    /// TLS 指纹保护
    pub fingerprint_protection: FingerprintProtectionConfig,
    /// 请求时序随机化
    pub request_timing: TimingConfig,
    /// DNS 配置
    pub dns_config: DnsConfig,
    /// 请求头配置
    pub headers: HeaderConfig,
    /// Cookie 处理
    pub cookie_handling: CookieConfig,
}

/// User-Agent 轮换配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserAgentRotationConfig {
    /// 是否启用
    pub enabled: bool,
    /// 轮换策略
    pub rotation_strategy: UaRotationStrategy,
    /// 自定义 User-Agent 列表
    pub custom_user_agents: Vec<String>,
    /// 轮换间隔（请求数）
    pub rotation_interval: usize,
    /// 是否包含移动端 UA
    pub include_mobile: bool,
    /// 是否按浏览器类型分组
    pub group_by_browser: bool,
}

/// User-Agent 轮换策略
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UaRotationStrategy {
    /// 随机选择
    Random,
    /// 轮询选择
    RoundRobin,
    /// 按权重选择
    Weighted,
    /// 基于时间选择
    TimeBased,
    /// 基于引擎选择
    EngineBased,
}

/// 代理配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyConfig {
    /// 代理类型
    pub proxy_type: ProxyType,
    /// 代理地址
    pub address: String,
    /// 端口号
    pub port: u16,
    /// 用户名（可选）
    pub username: Option<String>,
    /// 密码（可选）
    pub password: Option<String>,
    /// 是否启用
    pub enabled: bool,
    /// 代理权重（用于负载均衡）
    pub weight: f32,
    /// 超时时间（秒）
    pub timeout: u64,
    /// 重试次数
    pub retry_count: u32,
    /// 支持的域名（白名单，空则支持所有）
    pub allowed_domains: Vec<String>,
    /// 排除的域名（黑名单）
    pub blocked_domains: Vec<String>,
}

/// Tor 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TorConfig {
    /// 是否启用
    pub enabled: bool,
    /// SOCKS 端口
    pub socks_port: u16,
    /// 控制端口
    pub control_port: Option<u16>,
    /// 控制密码（可选）
    pub control_password: Option<String>,
    /// 节点国家代码
    pub exit_nodes: Option<Vec<String>>,
    /// 排除的国家代码
    pub exclude_nodes: Option<Vec<String>>,
    /// 是否启用严格节点选择
    pub strict_nodes: bool,
    /// 回路构建超时（秒）
    pub circuit_build_timeout: u64,
    /// 最大失败重试次数
    pub max_retries: u32,
}

/// TLS 指纹保护配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FingerprintProtectionConfig {
    /// 保护级别
    pub protection_level: FingerprintLevel,
    /// 是否随机化 TLS 扩展
    pub randomize_extensions: bool,
    /// 是否随机化密码套件
    pub randomize_cipher_suites: bool,
    /// 是否模拟常见浏览器
    pub emulate_browsers: bool,
    /// 自定义 TLS 配置
    pub custom_tls_config: Option<TlsFingerprintConfig>,
}

/// TLS 指纹配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TlsFingerprintConfig {
    /// 目标浏览器类型
    pub target_browser: String,
    /// 浏览器版本
    pub version: String,
    /// 操作系统
    pub os: String,
    /// 自定义 ALPN 协议
    pub alpn_protocols: Vec<String>,
}

/// 时序随机化配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimingConfig {
    /// 时序策略
    pub timing_strategy: TimingStrategy,
    /// 最小延迟（毫秒）
    pub min_delay: u64,
    /// 最大延迟（毫秒）
    pub max_delay: u64,
    /// 是否基于请求大小调整延迟
    pub size_based_delay: bool,
    /// 是否基于引擎调整延迟
    pub engine_based_delay: bool,
}

/// DNS 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnsConfig {
    /// 是否启用 DoH
    pub enabled: bool,
    /// DoH 服务器列表
    pub servers: Vec<DnsServer>,
    /// 超时时间（毫秒）
    pub timeout: u64,
    /// 重试次数
    pub retry_count: u32,
    /// 是否启用 DNS 缓存
    pub enable_cache: bool,
    /// 缓存过期时间（秒）
    pub cache_ttl: u64,
}

/// DNS 服务器配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnsServer {
    /// 服务器名称
    pub name: String,
    /// 服务器 URL
    pub url: String,
    /// 是否启用
    pub enabled: bool,
    /// 权重
    pub weight: f32,
    /// 支持的查询类型
    pub supported_types: Vec<DnsRecordType>,
}

/// DNS 记录类型
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum DnsRecordType {
    /// A 记录
    A,
    /// AAAA 记录
    AAAA,
    /// CNAME 记录
    CNAME,
    /// MX 记录
    MX,
    /// TXT 记录
    TXT,
    /// NS 记录
    NS,
}

/// 请求头配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeaderConfig {
    /// 是否移除隐私敏感头
    pub remove_privacy_headers: bool,
    /// 是否标准化 Accept 头
    pub normalize_accept: bool,
    /// 是否随机化其他头
    pub randomize_headers: bool,
    /// 自定义请求头
    pub custom_headers: Vec<CustomHeader>,
    /// 移除的头列表
    pub remove_headers: Vec<String>,
}

/// 自定义请求头
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomHeader {
    /// 头名称
    pub name: String,
    /// 头值
    pub value: String,
    /// 是否启用
    pub enabled: bool,
    /// 应用条件（可选）
    pub condition: Option<HeaderCondition>,
}

/// 头应用条件
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HeaderCondition {
    /// 始终应用
    Always,
    /// 仅应用到特定引擎
    Engine(String),
    /// 仅应用到特定域名
    Domain(String),
    /// 仅应用到特定请求类型
    RequestType(String),
}

/// Cookie 处理配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CookieConfig {
    /// 是否接受 Cookie
    pub accept_cookies: bool,
    /// 是否发送 Cookie
    pub send_cookies: bool,
    /// Cookie 过滤策略
    pub filter_policy: CookieFilterPolicy,
    /// 会话 Cookie 是否持久化
    pub persist_session_cookies: bool,
    /// 第三方 Cookie 策略
    pub third_party_policy: ThirdPartyCookiePolicy,
}

/// Cookie 过滤策略
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CookieFilterPolicy {
    /// 允许所有 Cookie
    AllowAll,
    /// 仅允许会话 Cookie
    SessionOnly,
    /// 仅允许第一方 Cookie
    FirstPartyOnly,
    /// 基于域名白名单
    Whitelist(Vec<String>),
    /// 基于域名黑名单
    Blacklist(Vec<String>),
    /// 完全禁用 Cookie
    Disabled,
}

/// 第三方 Cookie 策略
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ThirdPartyCookiePolicy {
    /// 允许所有第三方 Cookie
    AllowAll,
    /// 阻止所有第三方 Cookie
    BlockAll,
    /// 仅允许访问过的第三方
    BlockUnvisited,
    /// 基于隐私级别
    PrivacyBased,
}

impl Default for PrivacyConfig {
    fn default() -> Self {
        Self {
            user_agent_rotation: UserAgentRotationConfig::default(),
            proxy_chain: Vec::new(),
            enable_tor: false,
            tor_config: TorConfig::default(),
            fingerprint_protection: FingerprintProtectionConfig::default(),
            request_timing: TimingConfig::default(),
            dns_config: DnsConfig::default(),
            headers: HeaderConfig::default(),
            cookie_handling: CookieConfig::default(),
        }
    }
}

impl PrivacyConfig {
    /// 验证隐私配置
    pub fn validate(&self) -> ConfigValidationResult {
        let mut result = ConfigValidationResult::success();

        // 验证代理配置
        for (i, proxy) in self.proxy_chain.iter().enumerate() {
            if proxy.enabled && proxy.address.is_empty() {
                result.add_error(format!("代理 {} 地址不能为空", i + 1));
            }

            // u16 最大值为 65535，所以只需检查 0
            if proxy.port == 0 {
                result.add_error(format!("代理 {} 端口无效", i + 1));
            }

            if proxy.timeout == 0 {
                result.add_error(format!("代理 {} 超时时间必须大于 0", i + 1));
            }
        }

        // 验证 Tor 配置
        if self.enable_tor {
            // u16 最大值为 65535，所以只需检查 0
            if self.tor_config.socks_port == 0 {
                result.add_error("Tor SOCKS 端口无效".to_string());
            }

            if let Some(control_port) = self.tor_config.control_port {
                // u16 最大值为 65535，所以只需检查 0
                if control_port == 0 {
                    result.add_error("Tor 控制端口无效".to_string());
                }
            }
        }

        // 验证时序配置
        if self.request_timing.min_delay >= self.request_timing.max_delay {
            result.add_error("最小延迟不能大于等于最大延迟".to_string());
        }

        // 验证 DNS 配置
        if self.dns_config.enabled {
            if self.dns_config.servers.is_empty() {
                result.add_error("启用 DNS 时必须指定至少一个服务器".to_string());
            }

            if self.dns_config.timeout == 0 {
                result.add_error("DNS 超时时间必须大于 0".to_string());
            }
        }

        result
    }
}

impl Default for UserAgentRotationConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            rotation_strategy: UaRotationStrategy::Random,
            custom_user_agents: Vec::new(),
            rotation_interval: 10,
            include_mobile: false,
            group_by_browser: true,
        }
    }
}

impl Default for TorConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            socks_port: 9050,
            control_port: Some(9051),
            control_password: None,
            exit_nodes: None,
            exclude_nodes: None,
            strict_nodes: false,
            circuit_build_timeout: 60,
            max_retries: 3,
        }
    }
}

impl Default for FingerprintProtectionConfig {
    fn default() -> Self {
        Self {
            protection_level: FingerprintLevel::Basic,
            randomize_extensions: true,
            randomize_cipher_suites: true,
            emulate_browsers: true,
            custom_tls_config: None,
        }
    }
}

impl Default for TimingConfig {
    fn default() -> Self {
        Self {
            timing_strategy: TimingStrategy::Light,
            min_delay: 100, // 100ms
            max_delay: 2000, // 2s
            size_based_delay: true,
            engine_based_delay: true,
        }
    }
}

impl Default for DnsConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            servers: vec![
                // 国际 DNS 服务商
                DnsServer {
                    name: "Cloudflare".to_string(),
                    url: "https://cloudflare-dns.com/dns-query".to_string(),
                    enabled: true,
                    weight: 1.0,
                    supported_types: vec![
                        DnsRecordType::A,
                        DnsRecordType::AAAA,
                        DnsRecordType::CNAME,
                    ],
                },
                DnsServer {
                    name: "Google".to_string(),
                    url: "https://dns.google/dns-query".to_string(),
                    enabled: true,
                    weight: 1.0,
                    supported_types: vec![
                        DnsRecordType::A,
                        DnsRecordType::AAAA,
                        DnsRecordType::CNAME,
                    ],
                },

                // 国内 DNS 服务商
                DnsServer {
                    name: "阿里云".to_string(),
                    url: "https://dns.alidns.com/dns-query".to_string(),
                    enabled: true,
                    weight: 1.2, // 给国内服务稍高权重
                    supported_types: vec![
                        DnsRecordType::A,
                        DnsRecordType::AAAA,
                        DnsRecordType::CNAME,
                    ],
                },
                DnsServer {
                    name: "腾讯 DNSPod".to_string(),
                    url: "https://doh.pub/dns-query".to_string(),
                    enabled: true,
                    weight: 1.2,
                    supported_types: vec![
                        DnsRecordType::A,
                        DnsRecordType::AAAA,
                        DnsRecordType::CNAME,
                    ],
                },
                DnsServer {
                    name: "360 DoH".to_string(),
                    url: "https://doh.360.cn/dns-query".to_string(),
                    enabled: true,
                    weight: 1.1,
                    supported_types: vec![
                        DnsRecordType::A,
                        DnsRecordType::AAAA,
                        DnsRecordType::CNAME,
                    ],
                },
            ],
            timeout: 5000, // 5 seconds
            retry_count: 2,
            enable_cache: true,
            cache_ttl: 300, // 5 minutes
        }
    }
}

impl Default for HeaderConfig {
    fn default() -> Self {
        Self {
            remove_privacy_headers: true,
            normalize_accept: true,
            randomize_headers: false,
            custom_headers: Vec::new(),
            remove_headers: vec![
                "DNT".to_string(),
                "X-Forwarded-For".to_string(),
                "X-Real-IP".to_string(),
                "Via".to_string(),
            ],
        }
    }
}

impl Default for CookieConfig {
    fn default() -> Self {
        Self {
            accept_cookies: false,
            send_cookies: false,
            filter_policy: CookieFilterPolicy::Disabled,
            persist_session_cookies: false,
            third_party_policy: ThirdPartyCookiePolicy::BlockAll,
        }
    }
}