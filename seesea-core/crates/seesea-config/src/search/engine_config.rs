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

//! 搜索引擎配置管理
//!
//! 统一管理所有搜索引擎的配置

use serde::{Deserialize, Serialize};

/// 引擎模式
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum EngineMode {
    /// 全局模式
    #[default]
    Global,
    /// 自定义模式（用户指定引擎）
    Custom(Vec<String>),
    /// 快速模式：仅使用快速引擎
    Fast,
    /// 深网模式：仅使用深网引擎
    DeepWeb,
}

/// 搜索引擎配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineListConfig {
    /// 全局模式引擎列表
    pub global_engines: Vec<String>,
    /// 所有可用引擎列表
    pub all_available_engines: Vec<String>,
    /// 快速引擎列表
    pub fast_engines: Vec<String>,
    /// 深网引擎列表
    pub deepweb_engines: Vec<String>,
}

impl Default for EngineListConfig {
    fn default() -> Self {
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
        ];

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
        ];

        let fast_engines = all_engines
            .iter()
            .filter(|&engine| engine != "xinhua")
            .cloned()
            .collect();

        let deepweb_engines = vec!["xinhua".to_string()];

        Self {
            global_engines,
            all_available_engines: all_engines,
            fast_engines,
            deepweb_engines,
        }
    }
}

impl EngineListConfig {
    pub fn get_engines_for_mode(&self, mode: &EngineMode) -> Vec<String> {
        match mode {
            EngineMode::Global => self.global_engines.clone(),
            EngineMode::Custom(engines) => engines
                .iter()
                .filter(|engine| self.all_available_engines.contains(engine))
                .cloned()
                .collect(),
            EngineMode::Fast => self.fast_engines.clone(),
            EngineMode::DeepWeb => self.deepweb_engines.clone(),
        }
    }

    pub fn is_engine_available(&self, engine: &str) -> bool {
        self.all_available_engines.contains(&engine.to_string())
    }

    pub fn add_global_engine(&mut self, engine: String) -> Result<(), String> {
        if !self.is_engine_available(&engine) {
            return Err(format!("Engine '{engine}' is not available"));
        }
        if !self.global_engines.contains(&engine) {
            self.global_engines.push(engine);
        }
        Ok(())
    }

    pub fn remove_global_engine(&mut self, engine: &str) {
        self.global_engines.retain(|e| e != engine);
    }

    pub fn get_default_engines() -> Vec<String> {
        let config = EngineListConfig::default();
        config.global_engines
    }

    pub fn validate_engines(&self, engines: &[String]) -> Result<(), String> {
        for engine in engines {
            if !self.is_engine_available(engine) {
                return Err(format!(
                    "Engine '{}' is not available. Available engines: {:?}",
                    engine, self.all_available_engines
                ));
            }
        }
        Ok(())
    }

    pub fn filter_available_engines(&self, engines: &[String]) -> Vec<String> {
        engines
            .iter()
            .filter(|engine| self.is_engine_available(engine))
            .cloned()
            .collect()
    }
}
