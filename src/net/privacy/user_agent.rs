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

//! User-Agent 轮换模块
//!
//! 提供真实的浏览器 User-Agent 生成和轮换功能

use crate::net::types::{PrivacyConfig, UserAgentStrategy};
use rand::prelude::*;

/// User-Agent 生成器
pub struct UserAgentGenerator {
    /// 预定义的真实浏览器 User-Agent 列表
    user_agents: Vec<String>,
    /// 当前索引
    current_index: usize,
}

impl UserAgentGenerator {
    /// 创建新的 User-Agent 生成器
    pub fn new() -> Self {
        Self {
            user_agents: get_realistic_user_agents(),
            current_index: 0,
        }
    }

    /// 获取下一个 User-Agent（轮换）
    pub fn next(&mut self) -> &str {
        let ua = &self.user_agents[self.current_index];
        self.current_index = (self.current_index + 1) % self.user_agents.len();
        ua
    }

    /// 获取随机 User-Agent
    ///
    /// 使用 rand crate 提供高质量随机选择
    ///
    /// # 返回
    ///
    /// 随机选择的 User-Agent 字符串引用
    pub fn random(&self) -> &str {
        let mut rng = rand::rng();
        self.user_agents
            .choose(&mut rng)
            .map(|s| s.as_str())
            .unwrap_or("Mozilla/5.0")
    }
}

impl Default for UserAgentGenerator {
    fn default() -> Self {
        Self::new()
    }
}

/// 根据配置获取 User-Agent
///
/// # 参数
///
/// * `config` - 隐私配置
///
/// # 返回
///
/// User-Agent 字符串
pub fn get_user_agent(config: &PrivacyConfig) -> String {
    match config.user_agent_strategy {
        UserAgentStrategy::Fixed => {
            config.custom_user_agent.clone()
                .unwrap_or_else(|| String::from("Mozilla/5.0"))
        }
        UserAgentStrategy::Random => {
            get_random_user_agent()
        }
        UserAgentStrategy::Realistic => {
            get_realistic_user_agents()[0].clone()
        }
        UserAgentStrategy::Custom => {
            config.custom_user_agent.clone()
                .unwrap_or_else(|| get_random_user_agent())
        }
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
    let mut rng = rand::rng();
    
    agents
        .choose(&mut rng)
        .cloned()
        .unwrap_or_else(|| agents[0].clone())
}

/// 获取真实的浏览器 User-Agent 列表
///
/// # 返回
///
/// User-Agent 字符串向量
fn get_realistic_user_agents() -> Vec<String> {
    vec![
        // Chrome on Windows
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36".to_string(),
        
        // Firefox on Windows
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:121.0) Gecko/20100101 Firefox/121.0".to_string(),
        
        // Edge on Windows
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36 Edg/120.0.0.0".to_string(),
        
        // Chrome on macOS
        "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36".to_string(),
        
        // Safari on macOS
        "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.2 Safari/605.1.15".to_string(),
        
        // Firefox on macOS
        "Mozilla/5.0 (Macintosh; Intel Mac OS X 10.15; rv:121.0) Gecko/20100101 Firefox/121.0".to_string(),
        
        // Chrome on Linux
        "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36".to_string(),
        
        // Firefox on Linux
        "Mozilla/5.0 (X11; Linux x86_64; rv:121.0) Gecko/20100101 Firefox/121.0".to_string(),
        
        // Chrome on Android
        "Mozilla/5.0 (Linux; Android 13) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.6099.144 Mobile Safari/537.36".to_string(),
        
        // Safari on iOS
        "Mozilla/5.0 (iPhone; CPU iPhone OS 17_2 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.2 Mobile/15E148 Safari/604.1".to_string(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_agent_generator_new() {
        let generator = UserAgentGenerator::new();
        assert!(!generator.user_agents.is_empty());
    }

    #[test]
    fn test_user_agent_generator_next() {
        let mut generator = UserAgentGenerator::new();
        let first = generator.next().to_string();
        let second = generator.next().to_string();
        // 应该轮换
        assert_ne!(first, second);
    }

    #[test]
    fn test_user_agent_generator_random() {
        let generator = UserAgentGenerator::new();
        let ua = generator.random();
        assert!(!ua.is_empty());
        assert!(ua.contains("Mozilla"));
    }

    #[test]
    fn test_get_random_user_agent() {
        let ua = get_random_user_agent();
        assert!(!ua.is_empty());
        assert!(ua.contains("Mozilla"));
    }

    #[test]
    fn test_get_user_agent_realistic() {
        let config = PrivacyConfig {
            user_agent_strategy: UserAgentStrategy::Realistic,
            custom_user_agent: None,
            fake_headers: false,
            fake_referer: false,
            remove_fingerprints: false,
        };
        let ua = get_user_agent(&config);
        assert!(!ua.is_empty());
    }

    #[test]
    fn test_get_user_agent_custom() {
        let config = PrivacyConfig {
            user_agent_strategy: UserAgentStrategy::Custom,
            custom_user_agent: Some("MyCustomUA/1.0".to_string()),
            fake_headers: false,
            fake_referer: false,
            remove_fingerprints: false,
        };
        let ua = get_user_agent(&config);
        assert_eq!(ua, "MyCustomUA/1.0");
    }

    #[test]
    fn test_realistic_user_agents_count() {
        let agents = get_realistic_user_agents();
        assert!(agents.len() >= 10); // 至少有 10 个真实的 UA
    }
}
