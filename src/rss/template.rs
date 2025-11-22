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

//! RSS template management
//!
//! 提供 RSS 模板加载和管理功能

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};

/// RSS 模板元数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RssTemplateMeta {
    pub name: String,
    pub description: String,
    pub language: Option<String>,
    pub provider: Option<String>,
    pub version: Option<String>,
    pub persistent: bool,
    pub auto_update: bool,
    pub update_interval: u64,
}

/// RSS 模板
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RssTemplate {
    pub meta: RssTemplateMeta,
    pub feeds: HashMap<String, String>,
}

/// RSS 模板管理器
pub struct RssTemplateManager {
    template_dir: PathBuf,
}

impl RssTemplateManager {
    /// 创建新的模板管理器
    pub fn new<P: AsRef<Path>>(template_dir: P) -> Self {
        Self {
            template_dir: template_dir.as_ref().to_path_buf(),
        }
    }

    /// 列出所有可用的模板
    pub fn list_templates(&self) -> Result<Vec<String>, Box<dyn std::error::Error + Send + Sync>> {
        let mut templates = Vec::new();

        if !self.template_dir.exists() {
            return Ok(templates);
        }

        for entry in fs::read_dir(&self.template_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("see") {
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    // 移除 .rss 部分
                    if let Some(name) = stem.strip_suffix(".rss") {
                        templates.push(name.to_string());
                    }
                }
            }
        }

        Ok(templates)
    }

    /// 加载模板
    pub fn load_template(&self, name: &str) -> Result<RssTemplate, Box<dyn std::error::Error + Send + Sync>> {
        let template_path = self.template_dir.join(format!("{}.rss.see", name));

        if !template_path.exists() {
            return Err(format!("Template '{}' not found", name).into());
        }

        let content = fs::read_to_string(template_path)?;
        self.parse_template(&content)
    }

    /// 解析模板内容 (TOML 格式)
    fn parse_template(&self, content: &str) -> Result<RssTemplate, Box<dyn std::error::Error + Send + Sync>> {
        #[derive(Deserialize)]
        struct RawTemplate {
            meta: RawMeta,
            feeds: HashMap<String, String>,
        }

        #[derive(Deserialize)]
        struct RawMeta {
            name: String,
            description: String,
            language: Option<String>,
            provider: Option<String>,
            version: Option<String>,
            #[serde(default = "default_true")]
            persistent: bool,
            #[serde(default = "default_true")]
            auto_update: bool,
            #[serde(default = "default_update_interval")]
            update_interval: u64,
        }

        fn default_true() -> bool { true }
        fn default_update_interval() -> u64 { 3600 }

        let raw: RawTemplate = toml::from_str(content)?;

        Ok(RssTemplate {
            meta: RssTemplateMeta {
                name: raw.meta.name,
                description: raw.meta.description,
                language: raw.meta.language,
                provider: raw.meta.provider,
                version: raw.meta.version,
                persistent: raw.meta.persistent,
                auto_update: raw.meta.auto_update,
                update_interval: raw.meta.update_interval,
            },
            feeds: raw.feeds,
        })
    }

    /// 获取模板信息
    pub fn get_template_info(&self, name: &str) -> Result<RssTemplateMeta, Box<dyn std::error::Error + Send + Sync>> {
        let template = self.load_template(name)?;
        Ok(template.meta)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_template_manager_creation() {
        let manager = RssTemplateManager::new("rss/template");
        assert!(true);
    }
}
