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
//! 统一管理所有搜索引擎的配置，支持按类型过滤

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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
    /// 全局模式引擎列表（混合所有类型）
    pub global_engines: Vec<String>,
    /// 所有可用引擎列表
    pub all_available_engines: Vec<String>,
    /// 快速引擎列表
    pub fast_engines: Vec<String>,
    /// 深网引擎列表
    pub deepweb_engines: Vec<String>,
    /// 引擎类型映射
    #[serde(skip)]
    engine_type_map: HashMap<String, Vec<String>>,
}

impl Default for EngineListConfig {
    fn default() -> Self {
        // 定义所有引擎及其类型
        let mut engine_type_map = HashMap::new();

        // 通用搜索引擎
        engine_type_map.insert(
            "general".to_string(),
            vec![
                "yandex".to_string(),
                "bing".to_string(),
                "baidu".to_string(),
                "so".to_string(),
                "sogou".to_string(),
                "xinhua".to_string(),
            ],
        );

        // 图片搜索引擎
        engine_type_map.insert(
            "image".to_string(),
            vec![
                "unsplash".to_string(),
                "bing_images".to_string(),
                "sogou_images".to_string(),
            ],
        );

        // 视频搜索引擎
        engine_type_map.insert(
            "video".to_string(),
            vec![
                "bilibili".to_string(),
                "bing_videos".to_string(),
                "sogou_videos".to_string(),
            ],
        );

        // 新闻搜索引擎
        engine_type_map.insert("news".to_string(), vec!["bing_news".to_string()]);

        // 社交搜索引擎
        engine_type_map.insert("social".to_string(), vec!["sogou_wechat".to_string()]);

        let all_engines: Vec<String> = engine_type_map
            .values()
            .flat_map(|v| v.iter().cloned())
            .collect();

        let global_engines = vec![
            "yandex".to_string(),
            "bing".to_string(),
            "baidu".to_string(),
            "so".to_string(),
            "sogou".to_string(),
            "bilibili".to_string(),
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
            engine_type_map,
        }
    }
}

impl EngineListConfig {
    /// 根据模式获取引擎列表
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

    /// 根据引擎类型获取引擎列表
    pub fn get_engines_for_type(&self, engine_type: &str) -> Vec<String> {
        self.engine_type_map
            .get(engine_type)
            .cloned()
            .unwrap_or_default()
    }

    /// 获取所有支持的引擎类型
    pub fn get_supported_types(&self) -> Vec<String> {
        self.engine_type_map.keys().cloned().collect()
    }

    /// 检查引擎是否可用
    pub fn is_engine_available(&self, engine: &str) -> bool {
        self.all_available_engines.contains(&engine.to_string())
    }

    /// 添加全局引擎
    pub fn add_global_engine(&mut self, engine: String) -> Result<(), String> {
        if !self.is_engine_available(&engine) {
            return Err(format!("Engine '{engine}' is not available"));
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
                return Err(format!(
                    "Engine '{}' is not available. Available engines: {:?}",
                    engine, self.all_available_engines
                ));
            }
        }
        Ok(())
    }

    /// 过滤可用引擎
    pub fn filter_available_engines(&self, engines: &[String]) -> Vec<String> {
        engines
            .iter()
            .filter(|engine| self.is_engine_available(engine))
            .cloned()
            .collect()
    }
}
