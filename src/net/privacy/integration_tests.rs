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
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

//! 隐私保护集成测试

#[cfg(test)]
mod tests {
    use crate::net::config::{
        DnsConfig, PrivacyConfig, TlsConfig, TlsFingerprintLevel, UserAgentStrategy,
    };
    use crate::net::privacy::{PrivacyLevel, PrivacyManager};

    #[tokio::test]
    async fn test_privacy_manager_integration() {
        let mut privacy_config = PrivacyConfig::default();
        privacy_config.fake_headers = true;
        privacy_config.fake_referer = true;
        privacy_config.remove_fingerprints = true;
        privacy_config.user_agent_strategy = UserAgentStrategy::Random;

        let mut tls_config = TlsConfig::default();
        tls_config.fingerprint_level = TlsFingerprintLevel::Advanced;

        let mut dns_config = DnsConfig::default();
        dns_config.doh_enabled = true;

        let manager = PrivacyManager::new(privacy_config, tls_config, dns_config);

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

        let mut dns_config = DnsConfig::default();
        dns_config.doh_enabled = false;

        let manager = PrivacyManager::new(privacy_config, tls_config, dns_config);

        let level = manager.get_privacy_level().await;
        assert_eq!(level, PrivacyLevel::Low);
    }

    #[tokio::test]
    async fn test_privacy_stats() {
        let manager = PrivacyManager::new(
            PrivacyConfig::default(),
            TlsConfig::default(),
            DnsConfig::default(),
        );

        let stats = manager.get_stats().await;
        assert!(stats.fake_headers_enabled);
        assert!(matches!(
            stats.privacy_level,
            PrivacyLevel::Medium | PrivacyLevel::High
        ));
    }

    #[tokio::test]
    async fn test_privacy_config_update() {
        let manager = PrivacyManager::new(
            PrivacyConfig::default(),
            TlsConfig::default(),
            DnsConfig::default(),
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

        let manager =
            PrivacyManager::new(PrivacyConfig::default(), tls_config, DnsConfig::default());

        let params = manager.get_tls_params().await;
        assert!(!params.cipher_suites.is_empty());
        assert!(!params.supported_versions.is_empty());
    }
}
