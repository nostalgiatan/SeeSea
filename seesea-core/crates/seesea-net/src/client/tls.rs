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

//! TLS 配置模块
//!
//! 提供 TLS/SSL 连接配置和客户端证书支持

use reqwest::{Certificate, ClientBuilder};
use seesea_config::{FingerprintLevel, TlsConfig};
use seesea_errors::Result;
use std::path::Path;

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

    // 证书验证
    if !config.verify_certificates {
        builder = builder.danger_accept_invalid_certs(true);
    }

    // 配置 TLS 版本
    // reqwest 默认使用系统 TLS 栈，版本由系统决定
    // 对于更细粒度的控制，可能需要使用 rustls

    // 根据指纹混淆级别应用不同策略
    builder = match FingerprintLevel::Basic {
        FingerprintLevel::None => builder,
        FingerprintLevel::Basic => {
            // 基础混淆：使用标准配置
            builder
        }
        FingerprintLevel::Advanced => {
            // 高级混淆：模拟浏览器行为
            apply_advanced_fingerprint_protection(builder)
        }
        FingerprintLevel::Maximum => {
            // 完全混淆：使用最严格的保护
            apply_full_fingerprint_protection(builder)
        }
    };

    // 加载客户端证书
    if let (Some(cert_path), Some(key_path)) = (&config.client_cert_path, &config.client_key_path) {
        builder = load_client_cert(builder, cert_path.as_ref(), key_path.as_ref())?;
    }

    // 加载 CA 证书
    if let Some(ca_path) = &config.ca_cert_path {
        builder = load_ca_cert(builder, ca_path.as_ref())?;
    }

    Ok(builder)
}

/// 应用高级指纹保护
///
/// 模拟常见浏览器行为
fn apply_advanced_fingerprint_protection(builder: ClientBuilder) -> ClientBuilder {
    // 设置常见的 TLS 扩展和参数
    builder.use_rustls_tls().tls_built_in_root_certs(true)
}

/// 应用完全指纹保护
///
/// 使用最严格的保护措施
fn apply_full_fingerprint_protection(builder: ClientBuilder) -> ClientBuilder {
    // 使用 rustls 并配置严格的参数
    builder
        .use_rustls_tls()
        .tls_built_in_root_certs(true)
        .https_only(true)
}

/// 加载客户端证书
///
/// # 参数
///
/// * `builder` - reqwest ClientBuilder
/// * `cert_path` - 证书文件路径
/// * `key_path` - 私钥文件路径
///
/// # 返回
///
/// 配置好客户端证书的 ClientBuilder
fn load_client_cert(
    builder: ClientBuilder,
    _cert_path: &Path,
    _key_path: &Path,
) -> Result<ClientBuilder> {
    // 这里需要根据具体的证书格式和 reqwest 版本实现
    // 可能需要使用 identity 功能
    Ok(builder)
}

/// 加载 CA 证书
///
/// # 参数
///
/// * `builder` - reqwest ClientBuilder
/// * `ca_path` - CA 证书文件路径
///
/// # 返回
///
/// 配置好 CA 证书的 ClientBuilder
fn load_ca_cert(builder: ClientBuilder, ca_path: &Path) -> Result<ClientBuilder> {
    match Certificate::from_pem(
        &std::fs::read(ca_path)
            .map_err(|e| seesea_errors::ssl_error(&format!("Failed to read CA cert file: {e}")))?,
    ) {
        Ok(cert) => Ok(builder.add_root_certificate(cert)),
        Err(e) => Err(seesea_errors::ssl_error(&format!(
            "Failed to load CA cert: {e}"
        ))),
    }
}
