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

//! 隐私保护集成测试

#[cfg(test)]
mod tests {
    use crate::net::privacy::{PrivacyManager, PrivacyLevel};
    use crate::net::types::{PrivacyConfig, TlsConfig, DohConfig, UserAgentStrategy, TlsFingerprintLevel};

    #[tokio::test]
    async fn test_privacy_manager_integration() {
        let mut privacy_config = PrivacyConfig::default();
        privacy_config.fake_headers = true;
        privacy_config.fake_referer = true;
        privacy_config.remove_fingerprints = true;
        privacy_config.user_agent_strategy = UserAgentStrategy::Random;

        let mut tls_config = TlsConfig::default();
        tls_config.fingerprint_level = TlsFingerprintLevel::Advanced;

        let mut doh_config = DohConfig::default();
        doh_config.enabled = true;

        let manager = PrivacyManager::new(privacy_config, tls_config, doh_config);

        // Test User-Agent generation
        let ua = manager.get_user_agent().await;
        assert!(!ua.is_empty());
        assert!(ua.contains("Mozilla"));

        // Test privacy headers
        let headers = manager.get_privacy_headers("https://example.com").await;
        assert!(!headers.is_empty());
        assert!(headers.iter().any(|(k, _)| k == "Accept"));

        // Test DoH
        assert!(manager.is_doh_enabled().await);
        let servers = manager.get_doh_servers().await;
        assert!(!servers.is_empty());

        // Test privacy level
        let level = manager.get_privacy_level().await;
        assert!(matches!(level, PrivacyLevel::High | PrivacyLevel::Maximum));
    }

    #[tokio::test]
    async fn test_privacy_level_low() {
        let mut privacy_config = PrivacyConfig::default();
        privacy_config.fake_headers = false;
        privacy_config.fake_referer = false;
        privacy_config.remove_fingerprints = false;
        privacy_config.user_agent_strategy = UserAgentStrategy::Fixed;

        let mut tls_config = TlsConfig::default();
        tls_config.fingerprint_level = TlsFingerprintLevel::None;

        let mut doh_config = DohConfig::default();
        doh_config.enabled = false;

        let manager = PrivacyManager::new(privacy_config, tls_config, doh_config);

        let level = manager.get_privacy_level().await;
        assert_eq!(level, PrivacyLevel::Low);
    }

    #[tokio::test]
    async fn test_privacy_stats() {
        let manager = PrivacyManager::new(
            PrivacyConfig::default(),
            TlsConfig::default(),
            DohConfig::default(),
        );

        let stats = manager.get_stats().await;
        assert!(stats.fake_headers_enabled);
        assert!(matches!(stats.privacy_level, PrivacyLevel::Medium | PrivacyLevel::High));
    }

    #[tokio::test]
    async fn test_privacy_config_update() {
        let manager = PrivacyManager::new(
            PrivacyConfig::default(),
            TlsConfig::default(),
            DohConfig::default(),
        );

        // Update privacy config
        let mut new_config = PrivacyConfig::default();
        new_config.fake_headers = false;
        manager.update_privacy_config(new_config).await;

        let stats = manager.get_stats().await;
        assert!(!stats.fake_headers_enabled);
    }

    #[tokio::test]
    async fn test_tls_params() {
        let mut tls_config = TlsConfig::default();
        tls_config.fingerprint_level = TlsFingerprintLevel::Full;

        let manager = PrivacyManager::new(
            PrivacyConfig::default(),
            tls_config,
            DohConfig::default(),
        );

        let params = manager.get_tls_params().await;
        assert!(!params.cipher_suites.is_empty());
        assert!(!params.supported_versions.is_empty());
    }
}
