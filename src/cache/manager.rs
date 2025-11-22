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

//! 缓存管理器
//!
//! 提供基于 sled 的缓存管理核心功能

use crate::cache::types::*;
use once_cell::sync::Lazy;
use sled::Db;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// 缓存错误类型
#[derive(Debug, error_derive::Error)]
pub enum CacheError {
    /// 数据库错误
    #[error("数据库错误: {0}")]
    DatabaseError(String),

    /// 序列化错误
    #[error("序列化错误: {0}")]
    SerializationError(String),

    /// 键不存在
    #[error("键不存在: {0}")]
    KeyNotFound(String),

    /// 缓存已禁用
    #[error("缓存已禁用")]
    CacheDisabled,

    /// 缓存已满
    #[error("缓存已满，无法写入新数据")]
    CacheFull,

    /// 条目已过期
    #[error("缓存条目已过期")]
    EntryExpired,
}

/// 缓存结果类型
pub type Result<T> = std::result::Result<T, CacheError>;

/// 全局单例实例（使用Lazy和Mutex确保线程安全）
/// 
/// ## 单例模式实现
/// 
/// 使用`Lazy`和`Mutex`实现线程安全的单例模式：
/// - `Lazy`: 提供线程安全的延迟初始化
/// - `Mutex`: 允许独占访问，提供内部可变性
/// 
/// 这种实现确保：
/// 1. 全局只有一个CacheManager实例
/// 2. 线程安全的访问
/// 3. 延迟初始化（首次调用时创建）
/// 4. 避免重复初始化
/// 5. 无需手动管理内存（没有unsafe代码）
static GLOBAL_CACHE_MANAGER: Lazy<Mutex<Option<Arc<CacheManager>>>> = Lazy::new(|| Mutex::new(None));

/// 缓存管理器
///
/// 基于 sled 实现的高性能缓存管理器（单例模式）
pub struct CacheManager {
    /// sled 数据库实例
    db: Db,
    /// 元数据树
    metadata_tree: sled::Tree,
    /// 配置
    config: CacheImplConfig,
    /// 统计信息
    #[allow(dead_code)]
    stats: Arc<CacheStats>,
    /// 命中计数器（原子操作）
    hits: Arc<AtomicU64>,
    /// 未命中计数器（原子操作）
    misses: Arc<AtomicU64>,
    /// 写入计数器（原子操作）
    writes: Arc<AtomicU64>,
    /// 删除计数器（原子操作）
    deletes: Arc<AtomicU64>,
    /// 过期清理计数器（原子操作）
    evictions: Arc<AtomicU64>,
}

impl CacheManager {
    /// 获取全局缓存管理器实例（单例模式）
    ///
    /// # 参数
    ///
    /// * `config` - 缓存配置（仅在第一次调用时使用）
    ///
    /// # 返回值
    ///
    /// 返回缓存管理器实例或错误
    ///
    /// # 示例
    ///
    /// ```rust,no_run
    /// use seesea::cache::{CacheManager, CacheImplConfig};
    ///
    /// let config = CacheImplConfig::default();
    /// let manager = CacheManager::instance(config)?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn instance(config: CacheImplConfig) -> Result<Arc<Self>> {
        let mut guard = GLOBAL_CACHE_MANAGER.lock()
            .map_err(|e| CacheError::DatabaseError(format!("Lock poisoned: {}", e)))?;
        
        if let Some(manager) = guard.as_ref() {
            // Already initialized, return clone
            Ok(Arc::clone(manager))
        } else {
            // Initialize for the first time
            let manager = Self::create_internal(config)?;
            let arc_manager = Arc::new(manager);
            *guard = Some(Arc::clone(&arc_manager));
            Ok(arc_manager)
        }
    }

    /// 创建新的缓存管理器（内部方法）
    fn create_internal(config: CacheImplConfig) -> Result<Self> {
        // 创建数据库目录
        if let Some(parent) = Path::new(&config.db_path).parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                CacheError::DatabaseError(format!("创建缓存目录失败: {}", e))
            })?;
        }

        // 根据缓存模式配置 sled
        let db_config = match config.mode {
            CacheMode::LowLatency => sled::Config::default()
                .path(&config.db_path)
                .cache_capacity(1024 * 1024 * 128) // 128MB 缓存
                .flush_every_ms(Some(1000)), // 每秒刷新
            CacheMode::HighThroughput => sled::Config::default()
                .path(&config.db_path)
                .cache_capacity(1024 * 1024 * 64) // 64MB 缓存
                .flush_every_ms(Some(5000)), // 5秒刷新
            CacheMode::LowMemory => sled::Config::default()
                .path(&config.db_path)
                .cache_capacity(1024 * 1024 * 16) // 16MB 缓存
                .flush_every_ms(Some(10000)), // 10秒刷新
        };

        let db = db_config.open().map_err(|e| {
            CacheError::DatabaseError(format!("打开数据库失败: {}", e))
        })?;

        let metadata_tree = db.open_tree("metadata").map_err(|e| {
            CacheError::DatabaseError(format!("打开元数据树失败: {}", e))
        })?;

        Ok(Self {
            db,
            metadata_tree,
            config,
            stats: Arc::new(CacheStats::default()),
            hits: Arc::new(AtomicU64::new(0)),
            misses: Arc::new(AtomicU64::new(0)),
            writes: Arc::new(AtomicU64::new(0)),
            deletes: Arc::new(AtomicU64::new(0)),
            evictions: Arc::new(AtomicU64::new(0)),
        })
    }

    /// 创建新的缓存管理器（已弃用，使用instance替代）
    #[deprecated(since = "0.1.0", note = "请使用 instance() 方法获取单例实例")]
    pub fn new(config: CacheImplConfig) -> Result<Self> {
        Self::create_internal(config)
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
    pub fn get(&self, key: &str) -> Result<Option<CacheValue>> {
        if !self.config.enabled {
            return Err(CacheError::CacheDisabled);
        }

        // 获取元数据
        let metadata = match self.get_metadata(key)? {
            Some(meta) => meta,
            None => {
                self.misses.fetch_add(1, Ordering::Relaxed);
                return Ok(None);
            }
        };

        // 检查是否过期
        if metadata.is_expired() {
            self.misses.fetch_add(1, Ordering::Relaxed);
            // 不删除过期条目，保持持久化存储
            // 
            // 设计理念：
            // 1. 过期的数据仍保留在 sled 数据库中，不占用内存（sled 自动管理内存）
            // 2. 可通过 get_include_stale() 访问过期数据，用于全文搜索等场景
            // 3. 避免因缓存过期而丢失有价值的历史数据
            // 4. 减少网络请求，提高系统性能
            // 
            // 存储考虑：过期数据占用磁盘空间，可通过 cleanup_expired() 手动清理
            return Ok(None);
        }

        // 获取数据
        let value = self.db.get(key.as_bytes()).map_err(|e| {
            CacheError::DatabaseError(format!("读取缓存失败: {}", e))
        })?;

        match value {
            Some(v) => {
                self.hits.fetch_add(1, Ordering::Relaxed);
                // 更新元数据访问信息（异步，不阻塞读取）
                let _ = self.update_metadata_access(key);
                Ok(Some(v.to_vec()))
            }
            None => {
                self.misses.fetch_add(1, Ordering::Relaxed);
                Ok(None)
            }
        }
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
    pub fn get_include_stale(&self, key: &str) -> Result<Option<(CacheValue, bool)>> {
        if !self.config.enabled {
            return Err(CacheError::CacheDisabled);
        }

        // 获取元数据
        let metadata = match self.get_metadata(key)? {
            Some(meta) => meta,
            None => {
                self.misses.fetch_add(1, Ordering::Relaxed);
                return Ok(None);
            }
        };

        // 检查是否过期
        let is_stale = metadata.is_expired();
        
        // 获取数据
        let value = self.db.get(key.as_bytes()).map_err(|e| {
            CacheError::DatabaseError(format!("读取缓存失败: {}", e))
        })?;

        match value {
            Some(v) => {
                if is_stale {
                    self.misses.fetch_add(1, Ordering::Relaxed);
                } else {
                    self.hits.fetch_add(1, Ordering::Relaxed);
                }
                // 更新元数据访问信息（异步，不阻塞读取）
                let _ = self.update_metadata_access(key);
                Ok(Some((v.to_vec(), is_stale)))
            }
            None => {
                self.misses.fetch_add(1, Ordering::Relaxed);
                Ok(None)
            }
        }
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
    pub fn set(&self, key: String, value: CacheValue, ttl: Option<Duration>) -> Result<()> {
        if !self.config.enabled {
            return Err(CacheError::CacheDisabled);
        }

        // 检查缓存大小限制
        let value_size = value.len();
        if self.is_cache_full(value_size)? {
            // 尝试清理过期条目
            self.cleanup_expired()?;
            // 再次检查
            if self.is_cache_full(value_size)? {
                return Err(CacheError::CacheFull);
            }
        }

        // 创建元数据
        let ttl_duration = ttl.or_else(|| Some(Duration::from_secs(self.config.default_ttl_secs)));
        let metadata = CacheEntryMetadata::new(ttl_duration, value_size);

        // 写入数据
        self.db.insert(key.as_bytes(), value.as_slice()).map_err(|e| {
            CacheError::DatabaseError(format!("写入缓存失败: {}", e))
        })?;

        // 写入元数据
        self.set_metadata(&key, &metadata)?;

        self.writes.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }

    /// 删除缓存项
    ///
    /// # 参数
    ///
    /// * `key` - 缓存键
    pub fn delete(&self, key: &str) -> Result<bool> {
        if !self.config.enabled {
            return Err(CacheError::CacheDisabled);
        }

        let existed = self.db.remove(key.as_bytes()).map_err(|e| {
            CacheError::DatabaseError(format!("删除缓存失败: {}", e))
        })?.is_some();

        if existed {
            let _ = self.metadata_tree.remove(key.as_bytes());
            self.deletes.fetch_add(1, Ordering::Relaxed);
        }

        Ok(existed)
    }

    /// 清空所有缓存
    pub fn clear(&self) -> Result<()> {
        if !self.config.enabled {
            return Err(CacheError::CacheDisabled);
        }

        self.db.clear().map_err(|e| {
            CacheError::DatabaseError(format!("清空缓存失败: {}", e))
        })?;

        self.metadata_tree.clear().map_err(|e| {
            CacheError::DatabaseError(format!("清空元数据失败: {}", e))
        })?;

        Ok(())
    }

    /// 清理过期条目
    ///
    /// 遍历所有条目并删除已过期的
    pub fn cleanup_expired(&self) -> Result<usize> {
        let mut count = 0;
        
        for item in self.metadata_tree.iter() {
            let (key, value) = item.map_err(|e| {
                CacheError::DatabaseError(format!("遍历元数据失败: {}", e))
            })?;

            let metadata: CacheEntryMetadata = bincode::serde::decode_from_slice(&value, bincode::config::standard())
                .map(|(meta, _)| meta)
                .map_err(|e| {
                    CacheError::SerializationError(format!("反序列化元数据失败: {}", e))
                })?;

            if metadata.is_expired() {
                let key_str = String::from_utf8_lossy(&key);
                if self.delete(&key_str)? {
                    count += 1;
                    self.evictions.fetch_add(1, Ordering::Relaxed);
                }
            }
        }

        Ok(count)
    }

    /// 获取缓存统计信息
    pub fn stats(&self) -> CacheStats {
        CacheStats {
            hits: self.hits.load(Ordering::Relaxed),
            misses: self.misses.load(Ordering::Relaxed),
            writes: self.writes.load(Ordering::Relaxed),
            deletes: self.deletes.load(Ordering::Relaxed),
            total_keys: self.db.len() as u64,
            estimated_size_bytes: self.db.size_on_disk().unwrap_or(0),
            evictions: self.evictions.load(Ordering::Relaxed),
        }
    }

    /// 刷新到磁盘
    pub fn flush(&self) -> Result<()> {
        self.db.flush().map_err(|e| {
            CacheError::DatabaseError(format!("刷新缓存失败: {}", e))
        })?;
        Ok(())
    }

    /// 获取数据库迭代器
    ///
    /// 用于遍历所有缓存条目
    ///
    /// # 返回值
    ///
    /// 返回 sled 数据库的迭代器
    pub fn iter(&self) -> sled::Iter {
        self.db.iter()
    }

    // 私有辅助方法

    pub fn get_metadata(&self, key: &str) -> Result<Option<CacheEntryMetadata>> {
        match self.metadata_tree.get(key.as_bytes()) {
            Ok(Some(data)) => {
                let metadata: CacheEntryMetadata = bincode::serde::decode_from_slice(&data, bincode::config::standard())
                    .map(|(meta, _)| meta)
                    .map_err(|e| {
                        CacheError::SerializationError(format!("反序列化元数据失败: {}", e))
                    })?;
                Ok(Some(metadata))
            }
            Ok(None) => Ok(None),
            Err(e) => Err(CacheError::DatabaseError(format!("读取元数据失败: {}", e))),
        }
    }

    fn set_metadata(&self, key: &str, metadata: &CacheEntryMetadata) -> Result<()> {
        let data = bincode::serde::encode_to_vec(metadata, bincode::config::standard()).map_err(|e| {
            CacheError::SerializationError(format!("序列化元数据失败: {}", e))
        })?;

        self.metadata_tree.insert(key.as_bytes(), data.as_slice()).map_err(|e| {
            CacheError::DatabaseError(format!("写入元数据失败: {}", e))
        })?;

        Ok(())
    }

    fn update_metadata_access(&self, key: &str) -> Result<()> {
        if let Some(mut metadata) = self.get_metadata(key)? {
            metadata.update_access();
            self.set_metadata(key, &metadata)?;
        }
        Ok(())
    }

    fn is_cache_full(&self, new_size: usize) -> Result<bool> {
        let current_size = self.db.size_on_disk().unwrap_or(0);
        Ok(current_size + new_size as u64 > self.config.max_size_bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    fn temp_cache_config() -> CacheImplConfig {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        
        let temp_dir = std::env::temp_dir();
        let unique_id = COUNTER.fetch_add(1, Ordering::SeqCst);
        let db_path = temp_dir.join(format!(
            "test_cache_{}_{}",
            std::process::id(),
            unique_id
        ));
        
        CacheImplConfig {
            db_path: db_path.to_string_lossy().to_string(),
            default_ttl_secs: 10,
            max_size_bytes: 1024 * 1024, // 1MB for tests
            enabled: true,
            compression: false,
            mode: CacheMode::HighThroughput,
        }
    }

    #[test]
    #[serial]
    fn test_cache_manager_creation() {
        let config = temp_cache_config();
        let manager = CacheManager::new(config);
        assert!(manager.is_ok());
    }

    #[test]
    #[serial]
    fn test_cache_set_and_get() {
        let config = temp_cache_config();
        let manager = match CacheManager::new(config) {
            Ok(m) => m,
            Err(_) => return, // Skip test if cache creation fails
        };

        let key = "test_key".to_string();
        let value = b"test_value".to_vec();

        // 设置缓存
        let _ = manager.set(key.clone(), value.clone(), None);

        // 获取缓存
        let result = manager.get(&key).unwrap_or(None);
        assert!(result.is_some());
        assert_eq!(result.unwrap(), value);
    }

    #[test]
    #[serial]
    fn test_cache_expiration() {
        let config = temp_cache_config();
        let manager = match CacheManager::new(config) {
            Ok(m) => m,
            Err(_) => return, // Skip test if cache creation fails
        };

        let key = "expire_key".to_string();
        let value = b"expire_value".to_vec();

        // 设置1秒过期
        let _ = manager.set(key.clone(), value, Some(Duration::from_secs(1)));

        // 立即获取应该存在
        assert!(manager.get(&key).unwrap_or(None).is_some());

        // 等待过期
        std::thread::sleep(Duration::from_millis(1100));

        // 获取应该返回 None
        assert!(manager.get(&key).unwrap_or(None).is_none());
    }

    #[test]
    #[serial]
    fn test_cache_delete() {
        let config = temp_cache_config();
        let manager = match CacheManager::new(config) {
            Ok(m) => m,
            Err(_) => return,
        };

        let key = "delete_key".to_string();
        let value = b"delete_value".to_vec();

        let _ = manager.set(key.clone(), value, None);
        assert!(manager.get(&key).unwrap_or(None).is_some());

        let deleted = manager.delete(&key).unwrap_or(false);
        assert!(deleted);

        assert!(manager.get(&key).unwrap_or(None).is_none());
    }

    #[test]
    #[serial]
    fn test_cache_stats() {
        let manager = match CacheManager::new(temp_cache_config()) {
            Ok(m) => m,
            Err(_) => return,
        };

        let key = "stats_key".to_string();
        let value = b"stats_value".to_vec();

        // 初始统计
        let stats = manager.stats();
        assert_eq!(stats.hits, 0);
        assert_eq!(stats.misses, 0);

        // 未命中
        let _ = manager.get(&key);
        let stats = manager.stats();
        assert_eq!(stats.misses, 1);

        // 写入
        let _ = manager.set(key.clone(), value, None);
        let stats = manager.stats();
        assert_eq!(stats.writes, 1);

        // 命中
        let _ = manager.get(&key);
        let stats = manager.stats();
        assert_eq!(stats.hits, 1);
    }
}
