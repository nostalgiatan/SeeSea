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

//! 配置模块的核心类型定义

use serde::{Deserialize, Serialize};

/// 运行环境
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Environment {
    /// 开发环境
    Development,
    /// 测试环境
    Testing,
    /// 预发布环境
    Staging,
    /// 生产环境
    Production,
}

impl Default for Environment {
    fn default() -> Self {
        Self::Development
    }
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ConfigFormat {
    /// TOML 格式
    Toml,
    /// JSON 格式
    Json,
    /// YAML 格式
    Yaml,
}

impl Default for ConfigFormat {
    fn default() -> Self {
        Self::Toml
    }
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