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

//! DNS配置类型

use serde::{Deserialize, Serialize};

/// DNS 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnsConfig {
    /// 是否启用 DNS over HTTPS (DoH)
    pub doh_enabled: bool,
    /// DoH 服务器列表
    pub doh_servers: Vec<String>,
    /// 是否使用系统 DNS 作为后备
    pub fallback_to_system: bool,
}

impl Default for DnsConfig {
    fn default() -> Self {
        Self {
            doh_enabled: false,
            doh_servers: vec![
                String::from("https://cloudflare-dns.com/dns-query"),
                String::from("https://dns.google/dns-query"),
            ],
            fallback_to_system: true,
        }
    }
}
