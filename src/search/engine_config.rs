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

//! 搜索引擎配置管理
//!
//! 统一管理所有搜索引擎的配置

use serde::{Deserialize, Serialize};

/// 引擎模式
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EngineMode {
    /// 全局模式
    Global,
    /// 自定义模式（用户指定引擎）
    Custom(Vec<String>),
}

impl Default for EngineMode {
    fn default() -> Self {
        EngineMode::Global
    }
}

/// 搜索引擎配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineListConfig {
    /// 全局模式引擎列表
    pub global_engines: Vec<String>,
    /// 所有可用引擎列表
    pub all_available_engines: Vec<String>,
}

impl Default for EngineListConfig {
    fn default() -> Self {
        #[cfg(not(feature = "python"))]
        let all_engines = vec![
            "yandex".to_string(),
            "bing".to_string(),
            "baidu".to_string(),
            "so".to_string(),
            "sogou".to_string(),
            "bilibili".to_string(),
            "unsplash".to_string(),
            "bing_images".to_string(),
            "sogou_videos".to_string(),
        ];

        #[cfg(feature = "python")]
        let all_engines = vec![
            "yandex".to_string(),
            "bing".to_string(),
            "baidu".to_string(),
            "so".to_string(),
            "sogou".to_string(),
            "bilibili".to_string(),
            "unsplash".to_string(),
            "bing_images".to_string(),
            "sogou_videos".to_string(),
            "xinhua".to_string(),
            // "quark".to_string(),  // Commented out: quark engine disabled
        ];

        #[cfg(not(feature = "python"))]
        let global_engines = vec![
            "yandex".to_string(),
            "bing".to_string(),
            "baidu".to_string(),
            "so".to_string(),
            "sogou".to_string(),
            "bilibili".to_string(),
            "unsplash".to_string(),
            "bing_images".to_string(),
            "sogou_videos".to_string(),
        ];

        #[cfg(feature = "python")]
        let global_engines = vec![
            "yandex".to_string(),
            "bing".to_string(),
            "baidu".to_string(),
            "so".to_string(),
            "sogou".to_string(),
            "bilibili".to_string(),
            "unsplash".to_string(),
            "bing_images".to_string(),
            "sogou_videos".to_string(),
            "xinhua".to_string(),
            // "quark".to_string(),  // Commented out: quark engine disabled
        ];

        Self {
            global_engines,
            all_available_engines: all_engines,
        }
    }
}

impl EngineListConfig {
    /// 根据模式获取引擎列表
    pub fn get_engines_for_mode(&self, mode: &EngineMode) -> Vec<String> {
        match mode {
            EngineMode::Global => {
                // 恢复使用所有引擎
                self.global_engines.clone()
            }
            EngineMode::Custom(engines) => {
                // 验证自定义引擎是否在可用列表中
                engines.iter()
                    .filter(|engine| self.all_available_engines.contains(engine))
                    .cloned()
                    .collect()
            }
        }
    }

    /// 验证引擎是否可用
    pub fn is_engine_available(&self, engine: &str) -> bool {
        self.all_available_engines.contains(&engine.to_string())
    }

    /// 添加全局引擎
    pub fn add_global_engine(&mut self, engine: String) -> Result<(), String> {
        if !self.is_engine_available(&engine) {
            return Err(format!("Engine '{}' is not available", engine));
        }
        if !self.global_engines.contains(&engine) {
            self.global_engines.push(engine);
        }
        Ok(())
    }

    /// 移除全局引擎
    pub fn remove_global_engine(&mut self, engine: &str) {
        self.global_engines.retain(|e| e != engine);
    }

    /// 获取默认引擎列表
    pub fn get_default_engines() -> Vec<String> {
        let config = EngineListConfig::default();
        config.global_engines
    }

    /// 验证引擎列表
    pub fn validate_engines(&self, engines: &[String]) -> Result<(), String> {
        for engine in engines {
            if !self.is_engine_available(engine) {
                return Err(format!("Engine '{}' is not available. Available engines: {:?}",
                    engine, self.all_available_engines));
            }
        }
        Ok(())
    }

    /// 过滤可用引擎
    pub fn filter_available_engines(&self, engines: &[String]) -> Vec<String> {
        engines.iter()
            .filter(|engine| self.is_engine_available(engine))
            .cloned()
            .collect()
    }
}

// 全局引擎配置实例
lazy_static::lazy_static! {
    pub static ref ENGINE_CONFIG: EngineListConfig = EngineListConfig::default();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_engine_list_config_default() {
        let config = EngineListConfig::default();
        assert!(!config.global_engines.is_empty());
        assert!(!config.all_available_engines.is_empty());
        assert!(config.global_engines.len() <= config.all_available_engines.len());
    }

    #[test]
    fn test_get_engines_for_mode() {
        let config = EngineListConfig::default();

        let global_engines = config.get_engines_for_mode(&EngineMode::Global);
        assert_eq!(global_engines, config.global_engines);

        let custom_engines = config.get_engines_for_mode(&EngineMode::Custom(
            vec!["yandex".to_string(), "baidu".to_string()]
        ));
        assert_eq!(custom_engines, vec!["yandex", "baidu"]);
    }

    #[test]
    fn test_is_engine_available() {
        let config = EngineListConfig::default();
        assert!(config.is_engine_available("yandex"));
        assert!(config.is_engine_available("baidu"));
        assert!(!config.is_engine_available("nonexistent"));
    }

    #[test]
    fn test_add_remove_engine() {
        let mut config = EngineListConfig::default();
        let initial_count = config.global_engines.len();

        // 添加已存在的引擎
        config.add_global_engine("yandex".to_string()).unwrap();
        assert_eq!(config.global_engines.len(), initial_count);

        // 添加不存在的引擎
        let result = config.add_global_engine("nonexistent".to_string());
        assert!(result.is_err());

        // 移除引擎
        config.remove_global_engine("yandex");
        assert_eq!(config.global_engines.len(), initial_count - 1);
        assert!(!config.global_engines.contains(&"yandex".to_string()));
    }

    #[test]
    fn test_validate_engines() {
        let config = EngineListConfig::default();

        // 验证有效引擎
        let valid_engines = vec!["yandex".to_string(), "baidu".to_string()];
        assert!(config.validate_engines(&valid_engines).is_ok());

        // 验证无效引擎
        let invalid_engines = vec!["yandex".to_string(), "nonexistent".to_string()];
        assert!(config.validate_engines(&invalid_engines).is_err());
    }

    #[test]
    fn test_filter_available_engines() {
        let config = EngineListConfig::default();

        let engines = vec!["yandex".to_string(), "nonexistent".to_string(), "baidu".to_string()];
        let filtered = config.filter_available_engines(&engines);

        assert_eq!(filtered.len(), 2);
        assert!(filtered.contains(&"yandex".to_string()));
        assert!(filtered.contains(&"baidu".to_string()));
        assert!(!filtered.contains(&"nonexistent".to_string()));
    }
}
