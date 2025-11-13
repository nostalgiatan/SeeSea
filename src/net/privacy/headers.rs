//! 请求头伪造模块
//!
//! 提供请求头的伪造和混淆功能

use crate::net::types::PrivacyConfig;
use reqwest::ClientBuilder;

/// 配置隐私保护
///
/// # 参数
///
/// * `builder` - reqwest ClientBuilder
/// * `config` - 隐私配置
///
/// # 返回
///
/// 配置好隐私保护的 ClientBuilder
pub fn configure_privacy(builder: ClientBuilder, config: &PrivacyConfig) -> ClientBuilder {
    let mut builder = builder;

    // 配置 User-Agent
    if config.user_agent_strategy != crate::net::types::UserAgentStrategy::Fixed {
        let user_agent = super::user_agent::get_user_agent(config);
        builder = builder.user_agent(user_agent);
    } else if let Some(ref custom_ua) = config.custom_user_agent {
        builder = builder.user_agent(custom_ua);
    }

    // 禁用自动 gzip 以避免指纹特征（如果配置了移除指纹）
    if config.remove_fingerprints {
        builder = builder.gzip(false).brotli(false).deflate(false);
    }

    builder
}

/// 生成伪造的请求头
///
/// # 参数
///
/// * `url` - 目标 URL
/// * `config` - 隐私配置
///
/// # 返回
///
/// 请求头键值对列表
pub fn generate_fake_headers(url: &str, config: &PrivacyConfig) -> Vec<(String, String)> {
    let mut headers = Vec::new();

    // 添加常见的浏览器请求头
    headers.push(("Accept".to_string(), "text/html,application/xhtml+xml,application/xml;q=0.9,image/webp,*/*;q=0.8".to_string()));
    headers.push(("Accept-Language".to_string(), "en-US,en;q=0.9".to_string()));
    headers.push(("Accept-Encoding".to_string(), "gzip, deflate, br".to_string()));
    headers.push(("DNT".to_string(), "1".to_string()));
    headers.push(("Connection".to_string(), "keep-alive".to_string()));
    headers.push(("Upgrade-Insecure-Requests".to_string(), "1".to_string()));

    // 伪造 Referer
    if config.fake_referer {
        if let Some(referer) = generate_fake_referer(url) {
            headers.push(("Referer".to_string(), referer));
        }
    }

    // 添加 Sec-Fetch 头（现代浏览器特征）
    if config.fake_headers {
        headers.push(("Sec-Fetch-Dest".to_string(), "document".to_string()));
        headers.push(("Sec-Fetch-Mode".to_string(), "navigate".to_string()));
        headers.push(("Sec-Fetch-Site".to_string(), "none".to_string()));
        headers.push(("Sec-Fetch-User".to_string(), "?1".to_string()));
    }

    headers
}

/// 生成伪造的 Referer
///
/// # 参数
///
/// * `url` - 目标 URL
///
/// # 返回
///
/// 伪造的 Referer URL
fn generate_fake_referer(url: &str) -> Option<String> {
    // 从 URL 中提取域名作为 Referer
    if let Ok(parsed_url) = url::Url::parse(url) {
        if let Some(host) = parsed_url.host_str() {
            return Some(format!("https://{}/", host));
        }
    }
    None
}

/// 移除指纹特征的请求头
///
/// # 返回
///
/// 需要移除的请求头名称列表
pub fn get_fingerprint_headers() -> Vec<String> {
    vec![
        "X-Requested-With".to_string(),
        "X-Forwarded-For".to_string(),
        "Via".to_string(),
        "X-Real-IP".to_string(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::net::types::{PrivacyConfig, UserAgentStrategy};

    #[test]
    fn test_generate_fake_headers() {
        let config = PrivacyConfig::default();
        let headers = generate_fake_headers("https://example.com/search", &config);
        assert!(!headers.is_empty());
        assert!(headers.iter().any(|(k, _)| k == "Accept"));
    }

    #[test]
    fn test_generate_fake_headers_with_referer() {
        let mut config = PrivacyConfig::default();
        config.fake_referer = true;
        let headers = generate_fake_headers("https://example.com/search", &config);
        assert!(headers.iter().any(|(k, _)| k == "Referer"));
    }

    #[test]
    fn test_generate_fake_referer() {
        let referer = generate_fake_referer("https://example.com/search?q=test");
        assert_eq!(referer, Some("https://example.com/".to_string()));
    }

    #[test]
    fn test_get_fingerprint_headers() {
        let headers = get_fingerprint_headers();
        assert!(headers.contains(&"X-Requested-With".to_string()));
    }

    #[test]
    fn test_configure_privacy() {
        let config = PrivacyConfig {
            user_agent_strategy: UserAgentStrategy::Realistic,
            custom_user_agent: None,
            fake_headers: true,
            fake_referer: true,
            remove_fingerprints: true,
        };
        let builder = ClientBuilder::new();
        let _builder = configure_privacy(builder, &config);
        // 只测试不会 panic
    }
}
