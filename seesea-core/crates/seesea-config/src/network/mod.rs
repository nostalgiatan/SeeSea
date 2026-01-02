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

//! 网络配置模块
//!
//! 提供网络相关的配置类型，包括代理、TLS、DNS、连接池等配置

mod dns;
mod pool;
mod proxy;
mod tls;
mod types;

pub use dns::*;
pub use pool::*;
pub use proxy::ProxyConfig;
pub use tls::*;
pub use types::*;
