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

//! TLS配置类型

use serde::{Deserialize, Serialize};

/// TLS 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TlsConfig {
    /// 是否验证证书
    #[serde(default = "default_verify_certificates")]
    pub verify_certificates: bool,
    /// 允许的 TLS 版本
    #[serde(default)]
    pub allowed_versions: Vec<TlsVersion>,
    /// 客户端证书路径（可选）
    pub client_cert_path: Option<String>,
    /// 客户端私钥路径（可选）
    pub client_key_path: Option<String>,
    /// CA 证书路径（可选）
    pub ca_cert_path: Option<String>,
}

impl Default for TlsConfig {
    fn default() -> Self {
        Self {
            verify_certificates: default_verify_certificates(),
            allowed_versions: vec![TlsVersion::Tls12, TlsVersion::Tls13],
            client_cert_path: None,
            client_key_path: None,
            ca_cert_path: None,
        }
    }
}

fn default_verify_certificates() -> bool {
    true
}

/// TLS 版本枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TlsVersion {
    /// TLS 1.2
    #[serde(rename = "TLSv1_2")]
    Tls12,
    /// TLS 1.3
    #[serde(rename = "TLSv1_3")]
    Tls13,
}
