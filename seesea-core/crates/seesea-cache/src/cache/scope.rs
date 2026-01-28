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

//! 作用域缓存访问器
//!
//! 提供对指定作用域缓存的通用访问

use crate::cache::manager::CacheManager;
use seesea_errors::Result;
use std::sync::Arc;
use std::time::Duration;

/// 通用作用域缓存访问器
///
/// 用于访问指定作用域的缓存
pub struct ScopeCache {
    /// 缓存管理器
    manager: Arc<CacheManager>,
    /// 作用域名称
    scope: String,
}

impl ScopeCache {
    /// 创建新的作用域缓存访问器
    pub fn new(manager: Arc<CacheManager>, scope: String) -> Self {
        Self { manager, scope }
    }

    /// 获取作用域名称
    pub fn scope(&self) -> &str {
        &self.scope
    }

    /// 获取缓存管理器
    pub fn manager(&self) -> &CacheManager {
        &self.manager
    }

    /// 设置缓存项
    pub fn set(&self, key: String, value: Vec<u8>, ttl: Option<Duration>) -> Result<()> {
        self.manager
            .set(&self.scope, key, value, ttl)
            .map_err(|e| e.into())
    }

    /// 获取缓存项
    pub fn get(&self, key: &str) -> Result<Option<Vec<u8>>> {
        self.manager.get(&self.scope, key).map_err(|e| e.into())
    }

    /// 删除缓存项
    pub fn delete(&self, key: &str) -> Result<bool> {
        self.manager.delete(&self.scope, key).map_err(|e| e.into())
    }

    /// 清空作用域缓存
    pub fn clear(&self) -> Result<()> {
        self.manager.clear_scope(&self.scope).map_err(|e| e.into())
    }

    /// 获取作用域缓存大小
    pub fn len(&self) -> Result<usize> {
        self.manager.scope_size(&self.scope).map_err(|e| e.into())
    }

    /// 检查作用域缓存是否为空
    pub fn is_empty(&self) -> Result<bool> {
        let size = self.len()?;
        Ok(size == 0)
    }

    /// 获取所有键
    pub fn keys(&self) -> Result<Vec<String>> {
        self.manager.scope_keys(&self.scope).map_err(|e| e.into())
    }

    /// 检查键是否存在
    pub fn contains_key(&self, key: &str) -> Result<bool> {
        self.manager
            .contains_key(&self.scope, key)
            .map_err(|e| e.into())
    }

    /// 批量设置缓存项
    pub fn set_batch(&self, items: Vec<(String, Vec<u8>, Option<Duration>)>) -> Result<()> {
        // Extract TTL from first item if available, convert items to expected format
        let ttl = items.first().and_then(|&(_, _, ttl)| ttl);
        let items: Vec<(String, Vec<u8>)> = items.into_iter().map(|(k, v, _)| (k, v)).collect();
        self.manager
            .set_batch(&self.scope, &items, ttl)
            .map_err(|e| e.into())
    }

    /// 批量获取缓存项
    pub fn get_batch(&self, keys: &[String]) -> Result<Vec<Option<Vec<u8>>>> {
        let key_refs: Vec<&str> = keys.iter().map(|s| s.as_str()).collect();
        self.manager
            .get_batch(&self.scope, &key_refs)
            .map_err(|e| e.into())
    }

    /// 批量删除缓存项
    pub fn delete_batch(&self, keys: &[String]) -> Result<u64> {
        let key_refs: Vec<&str> = keys.iter().map(|s| s.as_str()).collect();
        let deleted: usize = self
            .manager
            .delete_batch(&self.scope, &key_refs)
            .map_err(seesea_errors::ErrorInfo::from)?;
        Ok(deleted as u64)
    }

    /// 获取缓存统计信息
    pub fn stats(&self) -> Result<ScopeCacheStats> {
        let stats = self.manager.stats();
        // For now, return basic scope stats - this could be enhanced to filter by scope
        Ok(ScopeCacheStats {
            total_hits: stats.total_hits,
            total_misses: stats.total_misses,
            total_inserts: stats.total_inserts,
            total_deletes: stats.total_deletes,
            total_evictions: stats.total_evictions,
            total_updates: stats.total_updates,
            current_size: stats.current_size,
        })
    }

    /// 清理过期缓存项
    pub fn cleanup_expired(&self) -> Result<u64> {
        let deleted: usize = self
            .manager
            .cleanup_expired_by_scope(&self.scope)
            .map_err(seesea_errors::ErrorInfo::from)?;
        Ok(deleted as u64)
    }
}

/// 作用域缓存统计信息
#[derive(Debug, Clone)]
pub struct ScopeCacheStats {
    /// 总命中数
    pub total_hits: u64,
    /// 总未命中数
    pub total_misses: u64,
    /// 总驱逐数
    pub total_evictions: u64,
    /// 总插入数
    pub total_inserts: u64,
    /// 总更新数
    pub total_updates: u64,
    /// 总删除数
    pub total_deletes: u64,
    /// 当前大小
    pub current_size: usize,
}

#[cfg(test)]
mod tests {
    use crate::cache::on::CacheInterface;
    use crate::cache::types::{CacheImplConfig, CacheMode};
    use std::time::Duration;

    #[test]
    fn test_scope_cache() {
        let temp_dir = std::env::temp_dir();
        let db_path = temp_dir.join(format!("test_scope_cache_{}", std::process::id()));

        let config = CacheImplConfig {
            db_path: db_path.to_string_lossy().to_string(),
            secondary_path: None,
            is_secondary: false,
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
            secondary_path: None,
            is_secondary: false,
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
        let scope_cache = interface.scope("test.expiration");

        // 测试过期
        let key = "expire_key";
        let value = b"expire_value".to_vec();

        scope_cache
            .set(key.to_string(), value, Some(Duration::from_millis(100)))
            .expect("设置缓存失败");

        // 立即获取应该存在
        let result = scope_cache.get(key).expect("获取缓存失败");
        assert!(result.is_some());

        // 等待过期
        std::thread::sleep(Duration::from_millis(150));

        // 过期后获取应该不存在
        let result = scope_cache.get(key).expect("获取缓存失败");
        assert!(result.is_none());
    }
}
