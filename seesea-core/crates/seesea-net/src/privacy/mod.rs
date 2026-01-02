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

//! 隐私保护模块
//!
//! 提供请求头伪造、指纹对抗、User-Agent 轮换等隐私保护功能

pub mod fingerprint;
pub mod headers;
pub mod manager;
pub mod tor;
pub mod user_agent;

#[cfg(test)]
mod integration_tests;

pub use fingerprint::FingerprintProtector;
pub use headers::configure_privacy;
pub use manager::{PrivacyLevel, PrivacyManager, PrivacyStats};
pub use user_agent::get_random_user_agent;
