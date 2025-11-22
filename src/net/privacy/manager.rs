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

//! 隐私保护管理器
//!
//! 统一管理所有隐私保护功能的协调器

use crate::net::types::{PrivacyConfig, TlsConfig, DohConfig};
use super::fingerprint::FingerprintProtector;
use super::user_agent::UserAgentGenerator;
use super::headers::generate_fake_headers;
use std::sync::Arc;
use tokio::sync::RwLock;

/// 隐私保护管理器
///
/// 协调所有隐私保护功能，包括：
/// - User-Agent 轮换
/// - 请求头伪造
/// - TLS 指纹混淆
/// - DNS over HTTPS
/// - Tor 代理
pub struct PrivacyManager {
    /// 隐私配置
    config: Arc<RwLock<PrivacyConfig>>,
    /// TLS 配置
    tls_config: Arc<RwLock<TlsConfig>>,
    /// DoH 配置
    doh_config: Arc<RwLock<DohConfig>>,
    /// 指纹保护器
    fingerprint_protector: Arc<FingerprintProtector>,
    /// User-Agent 生成器（预留用于未来功能）
    #[allow(dead_code)]
    ua_generator: Arc<UserAgentGenerator>,
}

impl PrivacyManager {
    /// 创建新的隐私管理器
    pub fn new(
        privacy_config: PrivacyConfig,
        tls_config: TlsConfig,
        doh_config: DohConfig,
    ) -> Self {
        let fingerprint_protector = Arc::new(
            FingerprintProtector::new(tls_config.fingerprint_level)
        );
        let ua_generator = Arc::new(UserAgentGenerator::new());

        Self {
            config: Arc::new(RwLock::new(privacy_config)),
            tls_config: Arc::new(RwLock::new(tls_config)),
            doh_config: Arc::new(RwLock::new(doh_config)),
            fingerprint_protector,
            ua_generator,
        }
    }

    /// 获取随机 User-Agent
    pub async fn get_user_agent(&self) -> String {
        let config = self.config.read().await;
        super::user_agent::get_user_agent(&config)
    }

    /// 生成隐私保护的请求头
    pub async fn get_privacy_headers(&self, url: &str) -> Vec<(String, String)> {
        let config = self.config.read().await;
        generate_fake_headers(url, &config)
    }

    /// 获取 TLS 参数
    pub async fn get_tls_params(&self) -> super::fingerprint::ObfuscatedTlsParams {
        self.fingerprint_protector.get_obfuscated_params()
    }

    /// 更新隐私配置
    pub async fn update_privacy_config(&self, new_config: PrivacyConfig) {
        let mut config = self.config.write().await;
        *config = new_config;
    }

    /// 更新 TLS 配置
    pub async fn update_tls_config(&self, new_config: TlsConfig) {
        let mut config = self.tls_config.write().await;
        *config = new_config;
    }

    /// 检查是否启用 DoH
    pub async fn is_doh_enabled(&self) -> bool {
        let config = self.doh_config.read().await;
        config.enabled
    }

    /// 获取 DoH 服务器列表
    pub async fn get_doh_servers(&self) -> Vec<String> {
        let config = self.doh_config.read().await;
        config.servers.clone()
    }

    /// 获取隐私保护级别评估
    pub async fn get_privacy_level(&self) -> PrivacyLevel {
        let privacy_config = self.config.read().await;
        let tls_config = self.tls_config.read().await;
        let doh_config = self.doh_config.read().await;

        let mut score = 0;

        // 基础隐私功能 (20分)
        if privacy_config.fake_headers { score += 5; }
        if privacy_config.fake_referer { score += 5; }
        if privacy_config.remove_fingerprints { score += 10; }

        // User-Agent 策略 (20分)
        score += match privacy_config.user_agent_strategy {
            crate::net::types::UserAgentStrategy::Fixed => 0,
            crate::net::types::UserAgentStrategy::Realistic => 10,
            crate::net::types::UserAgentStrategy::Random => 15,
            crate::net::types::UserAgentStrategy::Custom => 20,
        };

        // TLS 指纹混淆 (30分)
        score += match tls_config.fingerprint_level {
            crate::net::types::TlsFingerprintLevel::None => 0,
            crate::net::types::TlsFingerprintLevel::Basic => 10,
            crate::net::types::TlsFingerprintLevel::Advanced => 20,
            crate::net::types::TlsFingerprintLevel::Full => 30,
        };

        // DoH (15分)
        if doh_config.enabled {
            score += 15;
        }

        // 证书验证 (15分 - 禁用验证可能需要，但有风险)
        if !tls_config.verify_certificates {
            score += 5; // 部分分数，因为这是双刃剑
        } else {
            score += 10; // 保持验证更安全
        }

        match score {
            0..=30 => PrivacyLevel::Low,
            31..=60 => PrivacyLevel::Medium,
            61..=85 => PrivacyLevel::High,
            _ => PrivacyLevel::Maximum,
        }
    }

    /// 获取隐私保护统计信息
    pub async fn get_stats(&self) -> PrivacyStats {
        PrivacyStats {
            privacy_level: self.get_privacy_level().await,
            fake_headers_enabled: self.config.read().await.fake_headers,
            fingerprint_protection: self.tls_config.read().await.fingerprint_level,
            doh_enabled: self.is_doh_enabled().await,
            user_agent_strategy: self.config.read().await.user_agent_strategy,
        }
    }
}

/// 隐私保护级别
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrivacyLevel {
    /// 低级别保护
    Low,
    /// 中级别保护
    Medium,
    /// 高级别保护
    High,
    /// 最大保护
    Maximum,
}

impl std::fmt::Display for PrivacyLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PrivacyLevel::Low => write!(f, "低"),
            PrivacyLevel::Medium => write!(f, "中"),
            PrivacyLevel::High => write!(f, "高"),
            PrivacyLevel::Maximum => write!(f, "最大"),
        }
    }
}

/// 隐私统计信息
#[derive(Debug, Clone)]
pub struct PrivacyStats {
    /// 隐私级别
    pub privacy_level: PrivacyLevel,
    /// 是否启用伪造请求头
    pub fake_headers_enabled: bool,
    /// 指纹保护级别
    pub fingerprint_protection: crate::net::types::TlsFingerprintLevel,
    /// 是否启用 DoH
    pub doh_enabled: bool,
    /// User-Agent 策略
    pub user_agent_strategy: crate::net::types::UserAgentStrategy,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::net::types::{UserAgentStrategy, TlsFingerprintLevel};

    #[tokio::test]
    async fn test_privacy_manager_creation() {
        let privacy_config = PrivacyConfig::default();
        let tls_config = TlsConfig::default();
        let doh_config = DohConfig::default();

        let manager = PrivacyManager::new(privacy_config, tls_config, doh_config);
        
        let ua = manager.get_user_agent().await;
        assert!(!ua.is_empty());
    }

    #[tokio::test]
    async fn test_privacy_level_calculation() {
        let mut privacy_config = PrivacyConfig::default();
        privacy_config.fake_headers = true;
        privacy_config.fake_referer = true;
        privacy_config.remove_fingerprints = true;
        privacy_config.user_agent_strategy = UserAgentStrategy::Random;

        let mut tls_config = TlsConfig::default();
        tls_config.fingerprint_level = TlsFingerprintLevel::Full;

        let mut doh_config = DohConfig::default();
        doh_config.enabled = true;

        let manager = PrivacyManager::new(privacy_config, tls_config, doh_config);
        
        let level = manager.get_privacy_level().await;
        assert!(matches!(level, PrivacyLevel::High | PrivacyLevel::Maximum));
    }

    #[tokio::test]
    async fn test_get_stats() {
        let manager = PrivacyManager::new(
            PrivacyConfig::default(),
            TlsConfig::default(),
            DohConfig::default(),
        );

        let stats = manager.get_stats().await;
        assert!(stats.fake_headers_enabled);
    }
}
