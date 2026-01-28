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

//! RSS Feed 配置模块

use serde::{Deserialize, Serialize};

/// RSS Feed 配置
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RssConfig {
    /// 是否启用 RSS 功能
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    /// RSS 模板目录
    #[serde(default = "default_template_dir")]
    pub template_dir: String,
    /// 配置文件路径
    #[serde(default = "default_config_path")]
    pub config_path: String,
    /// 默认更新间隔（秒）
    #[serde(default = "default_update_interval")]
    pub default_update_interval: u64,
    /// 最大保留项目数
    #[serde(default = "default_max_items")]
    pub max_items_per_feed: usize,
    /// 是否启用自动更新
    #[serde(default = "default_auto_update")]
    pub auto_update: bool,
    /// 启动时更新持久化 RSS
    #[serde(default = "default_update_on_startup")]
    pub update_on_startup: bool,
    /// 持久化 RSS Feeds
    #[serde(default)]
    pub persistent_feeds: Vec<PersistentFeed>,
}

/// 持久化 RSS Feed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistentFeed {
    /// Feed 名称
    pub name: String,
    /// Feed URL
    pub url: String,
    /// 更新间隔（秒）
    #[serde(default = "default_update_interval")]
    pub update_interval: u64,
    /// 是否启用
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

fn default_enabled() -> bool {
    true
}

fn default_template_dir() -> String {
    "rss/template".to_string()
}

fn default_config_path() -> String {
    ".seesea/rss_config.toml".to_string()
}

fn default_update_interval() -> u64 {
    3600 // 1 hour
}

fn default_max_items() -> usize {
    1000
}

fn default_auto_update() -> bool {
    true
}

fn default_update_on_startup() -> bool {
    true
}
