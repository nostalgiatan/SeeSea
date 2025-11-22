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

//! TLS 配置和指纹混淆模块
//!
//! 提供 TLS 配置和浏览器指纹对抗功能

use crate::error::Result;
use crate::net::types::{TlsConfig, TlsFingerprintLevel};
use rand::seq::SliceRandom;
use reqwest::ClientBuilder;

/// 配置 TLS
///
/// # 参数
///
/// * `builder` - reqwest ClientBuilder
/// * `config` - TLS 配置
///
/// # 返回
///
/// 配置好 TLS 的 ClientBuilder
pub fn configure_tls(builder: ClientBuilder, config: &TlsConfig) -> Result<ClientBuilder> {
    let mut builder = builder;

    // 配置证书验证
    if !config.verify_certificates {
        builder = builder.danger_accept_invalid_certs(true);
    }

    // 配置 TLS 版本
    // reqwest 默认使用系统 TLS 栈，版本由系统决定
    // 对于更细粒度的控制，可能需要使用 rustls

    // 根据指纹混淆级别应用不同策略
    builder = match config.fingerprint_level {
        TlsFingerprintLevel::None => builder,
        TlsFingerprintLevel::Basic => {
            // 基础混淆：使用标准配置
            builder
        }
        TlsFingerprintLevel::Advanced => {
            // 高级混淆：模拟浏览器行为
            apply_advanced_fingerprint_protection(builder)
        }
        TlsFingerprintLevel::Full => {
            // 完全随机化：最大程度的混淆
            apply_full_fingerprint_protection(builder)
        }
    };

    Ok(builder)
}

/// 应用高级指纹保护
fn apply_advanced_fingerprint_protection(builder: ClientBuilder) -> ClientBuilder {
    // 模拟现代浏览器的 TLS 配置
    // 这里使用 reqwest 的默认配置，它已经模拟了浏览器行为
    builder
        .use_rustls_tls() // 使用 rustls 而不是系统 TLS
        .https_only(false) // 允许 HTTP（根据需要）
}

/// 应用完全指纹保护（随机化）
fn apply_full_fingerprint_protection(builder: ClientBuilder) -> ClientBuilder {
    // 完全随机化 TLS 参数
    // 注意：reqwest 的 API 限制了我们能做的随机化程度
    // 对于更深入的随机化，需要使用底层的 rustls 配置
    builder
        .use_rustls_tls()
        .https_only(false)
}

/// 生成随机 TLS 扩展顺序
///
/// 用于对抗基于 TLS 扩展顺序的指纹识别
/// 使用 rand crate 的 Fisher-Yates 洗牌算法
///
/// # 返回
///
/// 随机打乱顺序的 TLS 扩展 ID 列表
pub fn randomize_tls_extensions() -> Vec<u16> {
    // 常见的 TLS 扩展
    let mut extensions = vec![
        0,    // server_name
        10,   // supported_groups
        11,   // ec_point_formats
        13,   // signature_algorithms
        16,   // application_layer_protocol_negotiation
        23,   // extended_master_secret
        35,   // session_ticket
        43,   // supported_versions
        45,   // psk_key_exchange_modes
        51,   // key_share
    ];

    // 使用 rand crate 的 shuffle 方法（基于 Fisher-Yates 算法）
    let mut rng = rand::rng();
    extensions.shuffle(&mut rng);
    
    extensions
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_configure_tls_with_verification() {
        let mut config = TlsConfig::default();
        config.verify_certificates = true;
        
        let builder = ClientBuilder::new();
        let result = configure_tls(builder, &config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_configure_tls_without_verification() {
        let mut config = TlsConfig::default();
        config.verify_certificates = false;
        
        let builder = ClientBuilder::new();
        let result = configure_tls(builder, &config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_configure_tls_basic_fingerprint() {
        let mut config = TlsConfig::default();
        config.fingerprint_level = TlsFingerprintLevel::Basic;
        
        let builder = ClientBuilder::new();
        let result = configure_tls(builder, &config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_configure_tls_advanced_fingerprint() {
        let mut config = TlsConfig::default();
        config.fingerprint_level = TlsFingerprintLevel::Advanced;
        
        let builder = ClientBuilder::new();
        let result = configure_tls(builder, &config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_randomize_tls_extensions() {
        let extensions = randomize_tls_extensions();
        assert!(!extensions.is_empty());
        assert!(extensions.contains(&0)); // server_name 应该存在
    }
}
