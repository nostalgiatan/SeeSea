//! 搜索引擎注册表模块

use std::collections::HashMap;
use std::sync::Arc;

// 引入各个引擎实现
pub mod duckduckgo;

pub use duckduckgo::DuckDuckGoEngine;

/// 搜索引擎注册表
/// 
/// 用于管理和注册搜索引擎实例
pub struct EngineRegistry {
    engines: HashMap<String, Arc<dyn crate::derive::SearchEngine>>,
}

impl EngineRegistry {
    /// 创建新的引擎注册表
    pub fn new() -> Self {
        Self {
            engines: HashMap::new(),
        }
    }

    /// 注册搜索引擎
    pub fn register(&mut self, name: String, engine: Arc<dyn crate::derive::SearchEngine>) {
        self.engines.insert(name, engine);
    }

    /// 获取搜索引擎
    pub fn get(&self, name: &str) -> Option<Arc<dyn crate::derive::SearchEngine>> {
        self.engines.get(name).cloned()
    }

    /// 列出所有引擎名称
    pub fn list(&self) -> Vec<String> {
        self.engines.keys().cloned().collect()
    }

    /// 检查是否包含指定引擎
    pub fn contains(&self, name: &str) -> bool {
        self.engines.contains_key(name)
    }

    /// 移除引擎
    pub fn remove(&mut self, name: &str) -> Option<Arc<dyn crate::derive::SearchEngine>> {
        self.engines.remove(name)
    }

    /// 清空所有引擎
    pub fn clear(&mut self) {
        self.engines.clear();
    }
}

impl Default for EngineRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_creation() {
        let registry = EngineRegistry::new();
        assert_eq!(registry.list().len(), 0);
    }

    #[test]
    fn test_registry_contains() {
        let mut registry = EngineRegistry::new();
        assert!(!registry.contains("test"));
    }

    #[test]
    fn test_registry_default() {
        let registry = EngineRegistry::default();
        assert_eq!(registry.list().len(), 0);
    }
}
