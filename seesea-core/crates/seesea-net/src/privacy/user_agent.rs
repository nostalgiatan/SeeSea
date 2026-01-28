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

//! User-Agent 管理模块
//!
//! 提供 User-Agent 生成、轮换和隐私保护功能

use crate::PrivacyConfig;
use crate::UaRotationStrategy;
use rand::prelude::*;

/// 获取 User-Agent
///
/// 根据配置返回合适的 User-Agent 字符串
///
/// # 参数
///
/// * `config` - 隐私配置
///
/// # 返回
///
/// User-Agent 字符串
pub fn get_user_agent(config: &PrivacyConfig) -> String {
    match config.user_agent_rotation.rotation_strategy {
        UaRotationStrategy::Random => get_random_user_agent(),
        UaRotationStrategy::RoundRobin => get_realistic_user_agents()[0].clone(),
        UaRotationStrategy::Weighted => get_random_user_agent(),
        UaRotationStrategy::TimeBased => get_random_user_agent(),
        UaRotationStrategy::EngineBased => get_random_user_agent(),
    }
}

/// 获取随机 User-Agent
///
/// 使用 rand crate 提供高质量随机选择
///
/// # 返回
///
/// 随机选择的 User-Agent 字符串
pub fn get_random_user_agent() -> String {
    let agents = get_realistic_user_agents();
    agents.choose(&mut thread_rng()).unwrap().clone()
}

/// 获取真实的 User-Agent 列表
///
/// 返回常见浏览器的 User-Agent 字符串列表
///
/// # 返回
///
/// User-Agent 字符串列表
fn get_realistic_user_agents() -> Vec<String> {
    vec![
        String::from(
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
        ),
        String::from(
            "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
        ),
        String::from(
            "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
        ),
        String::from(
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:121.0) Gecko/20100101 Firefox/121.0",
        ),
        String::from(
            "Mozilla/5.0 (Macintosh; Intel Mac OS X 10.15; rv:121.0) Gecko/20100101 Firefox/121.0",
        ),
    ]
}

/// 应用 User-Agent 到请求
///
/// 根据配置将 User-Agent 添加到 HTTP 请求头
///
/// # 参数
///
/// * `req` - HTTP 请求
/// * `config` - 隐私配置
///
/// # 返回
///
/// 修改后的 HTTP 请求
pub fn apply_user_agent<'a>(
    req: &'a mut reqwest::Request,
    config: &'a PrivacyConfig,
) -> &'a mut reqwest::Request {
    use reqwest::header::{HeaderValue, USER_AGENT};

    let user_agent = get_user_agent(config);
    if let Ok(header_value) = HeaderValue::from_str(&user_agent) {
        req.headers_mut().insert(USER_AGENT, header_value);
    }

    req
}
