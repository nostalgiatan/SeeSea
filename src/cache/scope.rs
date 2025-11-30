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

//! 作用域缓存访问器
//!
//! 提供对指定作用域缓存的通用访问

use crate::cache::manager::{CacheManager, Result};
use std::sync::Arc;
use std::time::Duration;

/// 通用作用域缓存访问器
///
/// 用于访问指定作用域的缓存
pub struct ScopeCache {
    /// 缓存管理器
    manager: Arc<CacheManager>,
    /// 作用域
    scope: String,
}

impl ScopeCache {
    /// 创建新的作用域缓存访问器
    ///
    /// # 参数
    ///
    /// * `manager` - 缓存管理器
    /// * `scope` - 缓存作用域
    pub fn new(manager: Arc<CacheManager>, scope: String) -> Self {
        Self { manager, scope }
    }

    /// 获取缓存值
    ///
    /// # 参数
    ///
    /// * `key` - 缓存键
    ///
    /// # 返回值
    ///
    /// 返回缓存值，如果不存在或已过期则返回 None
    pub fn get(&self, key: &str) -> Result<Option<Vec<u8>>> {
        self.manager.get(&self.scope, key)
    }

    /// 获取缓存值（包括过期的）
    ///
    /// # 参数
    ///
    /// * `key` - 缓存键
    ///
    /// # 返回值
    ///
    /// 返回缓存值和是否过期的标志，如果不存在则返回 None
    pub fn get_include_stale(&self, key: &str) -> Result<Option<(Vec<u8>, bool)>> {
        self.manager.get_include_stale(&self.scope, key)
    }

    /// 设置缓存值
    ///
    /// # 参数
    ///
    /// * `key` - 缓存键
    /// * `value` - 缓存值
    /// * `ttl` - 生存时间，None 表示使用默认值
    ///
    /// # 返回值
    ///
    /// 成功返回 Ok(())，失败返回错误
    pub fn set(&self, key: String, value: Vec<u8>, ttl: Option<Duration>) -> Result<()> {
        self.manager.set(&self.scope, key, value, ttl)
    }

    /// 删除缓存项
    ///
    /// # 参数
    ///
    /// * `key` - 缓存键
    pub fn delete(&self, key: &str) -> Result<bool> {
        self.manager.delete(&self.scope, key)
    }

    /// 清空当前作用域的缓存
    pub fn clear(&self) -> Result<()> {
        self.manager.clear_scope(&self.scope)
    }

    /// 清理当前作用域的过期条目
    pub fn cleanup_expired(&self) -> Result<usize> {
        self.manager.cleanup_expired_by_scope(&self.scope)
    }

    /// 获取当前作用域
    pub fn scope(&self) -> &str {
        &self.scope
    }
}

#[cfg(test)]
mod tests {

    use crate::cache::{CacheImplConfig, CacheInterface, CacheMode};
    use std::time::Duration;

    #[test]
    fn test_scope_cache() {
        let temp_dir = std::env::temp_dir();
        let db_path = temp_dir.join(format!("test_scope_cache_{}", std::process::id()));

        let config = CacheImplConfig {
            db_path: db_path.to_string_lossy().to_string(),
            default_ttl_secs: 10,
            max_size_bytes: 1024 * 1024,
            enabled: true,
            compression: false,
            mode: CacheMode::HighThroughput,
            enable_bloom_filter: false,
            bloom_filter_expected_elements: 1000,
            bloom_filter_false_positive_rate: 0.01,
        };

        let interface = CacheInterface::new(config).expect("创建缓存接口失败");
        let scope_cache = interface.scope("test.scope");

        // 测试设置和获取
        let key = "test_key";
        let value = b"test_value".to_vec();

        scope_cache
            .set(key.to_string(), value.clone(), None)
            .expect("设置缓存失败");
        let result = scope_cache.get(key).expect("获取缓存失败");
        assert!(result.is_some());
        assert_eq!(result.unwrap(), value);

        // 测试删除
        let deleted = scope_cache.delete(key).expect("删除缓存失败");
        assert!(deleted);
        let result = scope_cache.get(key).expect("获取缓存失败");
        assert!(result.is_none());
    }

    #[test]
    fn test_scope_cache_expiration() {
        let temp_dir = std::env::temp_dir();
        let db_path = temp_dir.join(format!(
            "test_scope_cache_expiration_{}",
            std::process::id()
        ));

        let config = CacheImplConfig {
            db_path: db_path.to_string_lossy().to_string(),
            default_ttl_secs: 10,
            max_size_bytes: 1024 * 1024,
            enabled: true,
            compression: false,
            mode: CacheMode::HighThroughput,
            enable_bloom_filter: false,
            bloom_filter_expected_elements: 1000,
            bloom_filter_false_positive_rate: 0.01,
        };

        let interface = CacheInterface::new(config).expect("创建缓存接口失败");
        let scope_cache = interface.scope("test.scope.expiration");

        // 设置1秒过期
        let key = "expire_key";
        let value = b"expire_value".to_vec();
        scope_cache
            .set(key.to_string(), value, Some(Duration::from_secs(1)))
            .expect("设置缓存失败");

        // 立即获取应该存在
        let result = scope_cache.get(key).expect("获取缓存失败");
        assert!(result.is_some());

        // 等待过期
        std::thread::sleep(Duration::from_millis(1100));

        // 获取应该返回 None
        let result = scope_cache.get(key).expect("获取缓存失败");
        assert!(result.is_none());

        // 使用 get_include_stale 应该能获取到过期数据
        let result = scope_cache
            .get_include_stale(key)
            .expect("获取过期缓存失败");
        assert!(result.is_some());
        assert!(result.unwrap().1);
    }
}
