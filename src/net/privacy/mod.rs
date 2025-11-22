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

//! 隐私保护模块
//!
//! 提供请求头伪造、指纹对抗、User-Agent 轮换等隐私保护功能

pub mod headers;
pub mod fingerprint;
pub mod tor;
pub mod user_agent;
pub mod manager;

#[cfg(test)]
mod integration_tests;

pub use headers::configure_privacy;
pub use user_agent::{UserAgentGenerator, get_random_user_agent};
pub use fingerprint::FingerprintProtector;
pub use manager::{PrivacyManager, PrivacyLevel, PrivacyStats};
