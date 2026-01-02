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

//! 代理配置类型

use crate::common::ProxyType;
use serde::{Deserialize, Serialize};

/// 代理配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyConfig {
    /// 代理类型
    pub proxy_type: ProxyType,
    /// 代理地址（例如: "127.0.0.1:9050"）
    pub address: String,
    /// 认证用户名（可选）
    pub username: Option<String>,
    /// 认证密码（可选）
    pub password: Option<String>,
    /// 是否启用
    #[serde(default)]
    pub enabled: bool,
}

impl Default for ProxyConfig {
    fn default() -> Self {
        Self {
            proxy_type: ProxyType::Http,
            address: String::new(),
            username: None,
            password: None,
            enabled: false,
        }
    }
}
