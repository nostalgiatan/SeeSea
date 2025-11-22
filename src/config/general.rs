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

//! 通用配置类型定义

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use crate::config::common::EngineLoadingMode;
use crate::config::types::Environment;

/// 区域模式枚举
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum RegionMode {
    /// 全球模式 - 所有引擎
    Global,
    /// 中国模式 - 仅可在中国访问的引擎
    China,
    /// 自定义模式 - 用户自定义引擎列表
    Custom,
}

impl Default for RegionMode {
    fn default() -> Self {
        RegionMode::Global
    }
}

/// 通用配置结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralConfig {
    /// 实例名称
    #[serde(default = "default_instance_name")]
    pub instance_name: String,
    
    /// 是否启用调试模式
    #[serde(default)]
    pub debug: bool,
    
    /// 配置文件目录
    #[serde(default = "default_config_directory")]
    pub config_directory: PathBuf,
    
    /// 数据目录
    #[serde(default = "default_data_directory")]
    pub data_directory: PathBuf,
    
    /// 临时文件目录
    #[serde(default = "default_temp_directory")]
    pub temp_directory: PathBuf,
    
    /// 运行环境
    #[serde(default)]
    pub environment: Environment,
    
    /// 引擎加载模式
    #[serde(default)]
    pub engine_loading_mode: EngineLoadingMode,
    
    /// 区域模式 - 决定加载哪些搜索引擎
    #[serde(default)]
    pub region_mode: RegionMode,
    
    /// 是否启用指标收集
    #[serde(default)]
    pub enable_metrics: bool,
}

fn default_instance_name() -> String {
    "SeeSea".to_string()
}

fn default_config_directory() -> PathBuf {
    PathBuf::from("config")
}

fn default_data_directory() -> PathBuf {
    PathBuf::from("data")
}

fn default_temp_directory() -> PathBuf {
    std::env::temp_dir().join("seesea")
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            instance_name: default_instance_name(),
            debug: false,
            config_directory: default_config_directory(),
            data_directory: default_data_directory(),
            temp_directory: default_temp_directory(),
            environment: Environment::default(),
            engine_loading_mode: EngineLoadingMode::default(),
            region_mode: RegionMode::default(),
            enable_metrics: false,
        }
    }
}

impl Default for EngineLoadingMode {
    fn default() -> Self {
        EngineLoadingMode::Settings
    }
}
