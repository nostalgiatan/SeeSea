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

//! 运行时配置管理
//!
//! 管理可在运行时修改的配置

use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::RwLock;

/// 运行时配置
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RuntimeConfig {
    /// 当前启用的引擎
    pub enabled_engines: HashSet<String>,

    /// 禁用的引擎
    pub disabled_engines: HashSet<String>,
}

impl RuntimeConfig {
    /// 创建新的运行时配置
    pub fn new() -> Self {
        Self::default()
    }

    /// 检查引擎是否启用
    pub fn is_engine_enabled(&self, engine: &str) -> bool {
        !self.disabled_engines.contains(engine)
    }

    /// 启用引擎
    pub fn enable_engine(&mut self, engine: String) -> Result<(), String> {
        self.disabled_engines.remove(&engine);
        self.enabled_engines.insert(engine);
        Ok(())
    }

    /// 禁用引擎
    pub fn disable_engine(&mut self, engine: String) -> Result<(), String> {
        self.enabled_engines.remove(&engine);
        self.disabled_engines.insert(engine);
        Ok(())
    }

    /// 批量启用引擎
    pub fn enable_engines(&mut self, engines: Vec<String>) -> Result<usize, String> {
        let mut count = 0;
        for engine in engines {
            self.enable_engine(engine)?;
            count += 1;
        }
        Ok(count)
    }

    /// 批量禁用引擎
    pub fn disable_engines(&mut self, engines: Vec<String>) -> Result<usize, String> {
        let mut count = 0;
        for engine in engines {
            self.disable_engine(engine)?;
            count += 1;
        }
        Ok(count)
    }

    /// 获取所有启用的引擎
    pub fn get_enabled_engines(&self) -> Vec<String> {
        self.enabled_engines.iter().cloned().collect()
    }

    /// 获取所有禁用的引擎
    pub fn get_disabled_engines(&self) -> Vec<String> {
        self.disabled_engines.iter().cloned().collect()
    }
}

/// 运行时配置管理器
pub struct RuntimeConfigManager {
    config: Arc<RwLock<RuntimeConfig>>,
}

impl RuntimeConfigManager {
    /// 创建新的运行时配置管理器
    pub fn new() -> Self {
        Self {
            config: Arc::new(RwLock::new(RuntimeConfig::new())),
        }
    }

    /// 获取配置克隆
    pub async fn get_config(&self) -> RuntimeConfig {
        self.config.read().await.clone()
    }

    /// 检查引擎是否启用
    pub async fn is_engine_enabled(&self, engine: &str) -> bool {
        self.config.read().await.is_engine_enabled(engine)
    }

    /// 启用引擎
    pub async fn enable_engine(&self, engine: String) -> Result<(), String> {
        let mut config = self.config.write().await;
        config.enable_engine(engine)
    }

    /// 禁用引擎
    pub async fn disable_engine(&self, engine: String) -> Result<(), String> {
        let mut config = self.config.write().await;
        config.disable_engine(engine)
    }

    /// 批量启用引擎
    pub async fn enable_engines(&self, engines: Vec<String>) -> Result<usize, String> {
        let mut config = self.config.write().await;
        config.enable_engines(engines)
    }

    /// 批量禁用引擎
    pub async fn disable_engines(&self, engines: Vec<String>) -> Result<usize, String> {
        let mut config = self.config.write().await;
        config.disable_engines(engines)
    }

    /// 重置配置
    pub async fn reset(&self) {
        let mut config = self.config.write().await;
        *config = RuntimeConfig::new();
    }
}

impl Default for RuntimeConfigManager {
    fn default() -> Self {
        Self::new()
    }
}
