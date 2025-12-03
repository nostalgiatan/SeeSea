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

//! 配置模块的核心类型定义
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

use serde::{Deserialize, Serialize};

/// 运行环境
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Environment {
    /// 开发环境
    #[default]
    Development,
    /// 测试环境
    Testing,
    /// 预发布环境
    Staging,
    /// 生产环境
    Production,
}

impl std::fmt::Display for Environment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Environment::Development => write!(f, "development"),
            Environment::Testing => write!(f, "testing"),
            Environment::Staging => write!(f, "staging"),
            Environment::Production => write!(f, "production"),
        }
    }
}

/// 应用元数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplicationMeta {
    /// 应用名称
    pub name: String,
    /// 应用版本
    pub version: String,
    /// 应用描述
    pub description: String,
    /// 作者信息
    pub author: String,
    /// 主页 URL
    pub homepage: Option<String>,
    /// 许可证
    pub license: Option<String>,
    /// 仓库地址
    pub repository: Option<String>,
}

impl Default for ApplicationMeta {
    fn default() -> Self {
        Self {
            name: "SeeSea".to_string(),
            version: "0.1.0".to_string(),
            description: "隐私保护的 Rust 元搜索引擎".to_string(),
            author: "SeeSea Team".to_string(),
            homepage: Some("https://seesea.example.com".to_string()),
            license: Some("MIT".to_string()),
            repository: Some("https://github.com/seesea/seesea".to_string()),
        }
    }
}

/// 配置文件格式
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ConfigFormat {
    /// TOML 格式
    #[default]
    Toml,
    /// JSON 格式
    Json,
    /// YAML 格式
    Yaml,
}

/// 配置文件元数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigMetadata {
    /// 配置版本
    pub version: String,
    /// 创建时间
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
    /// 更新时间
    pub updated_at: Option<chrono::DateTime<chrono::Utc>>,
    /// 配置格式
    pub format: ConfigFormat,
    /// 备注
    pub notes: Option<String>,
}

impl Default for ConfigMetadata {
    fn default() -> Self {
        Self {
            version: "1.0".to_string(),
            created_at: Some(chrono::Utc::now()),
            updated_at: Some(chrono::Utc::now()),
            format: ConfigFormat::Toml,
            notes: None,
        }
    }
}
