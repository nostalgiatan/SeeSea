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

//! RSS template management
//!
//! 提供 RSS 模板加载和管理功能

use crate::types::RssResult;
use seesea_errors::{io as io_errors, parse as parse_errors, validation as validation_errors};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

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
    pub fn new<P: AsRef<Path>>(template_dir: P) -> RssResult<Self> {
        let dir_path = template_dir.as_ref().to_path_buf();

        // 验证目录路径
        if dir_path.to_string_lossy().trim().is_empty() {
            return Err(validation_errors::empty_field("模板目录路径").into());
        }

        // 如果目录不存在，尝试创建它
        if !dir_path.exists() {
            match fs::create_dir_all(&dir_path) {
                Ok(_) => {
                    eprintln!("模板目录不存在，已创建: {:?}", dir_path);
                }
                Err(e) => {
                    return Err(io_errors::directory_create_failed(
                        &dir_path.to_string_lossy(),
                        &format!("创建模板目录失败: {}", e),
                    )
                    .into());
                }
            }
        }

        // 验证是否为目录
        if dir_path.exists() && !dir_path.is_dir() {
            return Err(validation_errors::validation_error("指定路径不是目录").into());
        }

        Ok(Self {
            template_dir: dir_path,
        })
    }

    /// 列出所有可用的模板
    pub fn list_templates(&self) -> RssResult<Vec<String>> {
        let mut templates = Vec::new();

        if !self.template_dir.exists() {
            return Ok(templates);
        }

        // 读取目录内容
        let entries = fs::read_dir(&self.template_dir)
            .map_err(|e| io_errors::io_error(format!("读取模板目录失败: {}", e)))?;

        for entry in entries {
            let entry = entry.map_err(|e| io_errors::io_error(format!("读取目录项失败: {}", e)))?;
            let path = entry.path();

            // 检查文件扩展名
            if path.extension().and_then(|s| s.to_str()) == Some("see")
                && let Some(stem) = path.file_stem().and_then(|s| s.to_str())
            {
                // 移除 .rss 部分
                if let Some(name) = stem.strip_suffix(".rss")
                    && !name.trim().is_empty()
                {
                    templates.push(name.to_string());
                }
            }
        }

        // 按名称排序
        templates.sort();

        Ok(templates)
    }

    /// 加载模板
    pub fn load_template(&self, name: &str) -> RssResult<RssTemplate> {
        // 验证模板名称
        if name.trim().is_empty() {
            return Err(validation_errors::empty_field("模板名称").into());
        }

        // 验证名称格式（防止路径遍历攻击）
        if name.contains("..") || name.contains("/") || name.contains("\\") {
            return Err(validation_errors::validation_error("模板名称包含非法字符").into());
        }

        let template_path = self.template_dir.join(format!("{name}.rss.see"));

        // 检查文件是否存在
        if !template_path.exists() {
            return Err(io_errors::file_not_found(&format!("模板 '{}' 不存在", name)).into());
        }

        // 检查是否为文件
        if !template_path.is_file() {
            return Err(validation_errors::validation_error("模板路径不是文件").into());
        }

        // 读取文件内容
        let content = fs::read_to_string(&template_path).map_err(|e| {
            io_errors::file_read_failed(
                template_path.to_str().unwrap_or("unknown"),
                &format!("读取模板文件失败: {}", e),
            )
        })?;

        // 验证内容不为空
        if content.trim().is_empty() {
            return Err(parse_errors::invalid_format("模板文件为空").into());
        }

        self.parse_template(&content)
    }

    /// 解析模板内容 (TOML 格式)
    fn parse_template(&self, content: &str) -> RssResult<RssTemplate> {
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

        fn default_true() -> bool {
            true
        }
        fn default_update_interval() -> u64 {
            3600
        }

        // 解析 TOML 内容
        let raw: RawTemplate = toml::from_str(content)
            .map_err(|e| parse_errors::parse_error(format!("TOML解析错误: {}", e)))?;

        // 验证必需的元数据字段
        if raw.meta.name.trim().is_empty() {
            return Err(validation_errors::empty_field("模板名称").into());
        }

        if raw.meta.description.trim().is_empty() {
            return Err(validation_errors::empty_field("模板描述").into());
        }

        // 验证feeds字段
        if raw.feeds.is_empty() {
            return Err(validation_errors::validation_error("模板必须包含至少一个feed").into());
        }

        // 验证所有feed URL
        for (feed_name, feed_url) in &raw.feeds {
            if feed_name.trim().is_empty() {
                return Err(validation_errors::empty_field("feed名称").into());
            }
            if feed_url.trim().is_empty() {
                return Err(
                    validation_errors::empty_field(&format!("feed '{}' 的URL", feed_name)).into(),
                );
            }
            if !feed_url.starts_with("http://") && !feed_url.starts_with("https://") {
                return Err(
                    validation_errors::invalid_url(&format!("feed '{}' 的URL", feed_name)).into(),
                );
            }
        }

        // 验证更新间隔
        if raw.meta.update_interval == 0 {
            return Err(validation_errors::validation_error("更新间隔必须大于0").into());
        }

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
    pub fn get_template_info(&self, name: &str) -> RssResult<RssTemplateMeta> {
        let template = self.load_template(name)?;
        Ok(template.meta)
    }

    /// 保存模板
    pub fn save_template(&self, name: &str, template: &RssTemplate) -> RssResult<()> {
        // 验证模板名称
        if name.trim().is_empty() {
            return Err(validation_errors::empty_field("模板名称").into());
        }

        // 验证名称格式
        if name.contains("..") || name.contains("/") || name.contains("\\") {
            return Err(validation_errors::validation_error("模板名称包含非法字符").into());
        }

        // 验证模板内容
        if template.meta.name.trim().is_empty() {
            return Err(validation_errors::empty_field("模板元数据名称").into());
        }

        if template.feeds.is_empty() {
            return Err(validation_errors::validation_error("模板必须包含至少一个feed").into());
        }

        let template_path = self.template_dir.join(format!("{name}.rss.see"));

        // 序列化为 TOML
        let content = toml::to_string_pretty(template)
            .map_err(|e| parse_errors::parse_error(format!("TOML序列化错误: {}", e)))?;

        // 写入文件
        fs::write(&template_path, content).map_err(|e| {
            io_errors::file_write_failed(
                template_path.to_str().unwrap_or("unknown"),
                &format!("写入模板文件失败: {}", e),
            )
        })?;

        Ok(())
    }

    /// 删除模板
    pub fn delete_template(&self, name: &str) -> RssResult<()> {
        // 验证模板名称
        if name.trim().is_empty() {
            return Err(validation_errors::empty_field("模板名称").into());
        }

        // 验证名称格式
        if name.contains("..") || name.contains("/") || name.contains("\\") {
            return Err(validation_errors::validation_error("模板名称包含非法字符").into());
        }

        let template_path = self.template_dir.join(format!("{name}.rss.see"));

        // 检查文件是否存在
        if !template_path.exists() {
            return Err(io_errors::file_not_found(&format!("模板 '{}' 不存在", name)).into());
        }

        // 删除文件
        fs::remove_file(&template_path)
            .map_err(|e| io_errors::io_error(format!("删除模板文件失败: {}", e)))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_template_manager_creation() {
        let result = RssTemplateManager::new("rss/template");
        assert!(result.is_ok());
    }

    #[test]
    fn test_invalid_template_name() {
        let manager = RssTemplateManager::new("test_templates").unwrap();

        // 测试空名称
        let result = manager.load_template("");
        assert!(result.is_err());

        // 测试包含非法字符的名称
        let result = manager.load_template("../test");
        assert!(result.is_err());

        let result = manager.load_template("test/file");
        assert!(result.is_err());
    }
}
