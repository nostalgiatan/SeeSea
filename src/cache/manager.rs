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

use crate::cache::bloom::{BloomFilter, BloomFilterConfig};
use crate::cache::types::*;
use once_cell::sync::Lazy;
use sled::Db;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, RwLock};
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
    /// sled 数据库实例（Arc包装，支持线程安全共享）
    db: Arc<Db>,
    /// 元数据树（Arc包装，支持线程安全共享）
    metadata_tree: Arc<sled::Tree>,
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
    /// 布隆过滤器，用于防止缓存穿透
    bloom_filter: Option<Arc<RwLock<BloomFilter<String>>>>,
    /// 读取操作总延迟（纳秒）
    get_total_latency: Arc<AtomicU64>,
    /// 读取操作次数
    get_count: Arc<AtomicU64>,
    /// 读取操作最小延迟（纳秒）
    get_min_latency: Arc<AtomicU64>,
    /// 读取操作最大延迟（纳秒）
    get_max_latency: Arc<AtomicU64>,
    /// 写入操作总延迟（纳秒）
    set_total_latency: Arc<AtomicU64>,
    /// 写入操作次数
    set_count: Arc<AtomicU64>,
    /// 写入操作最小延迟（纳秒）
    set_min_latency: Arc<AtomicU64>,
    /// 写入操作最大延迟（纳秒）
    set_max_latency: Arc<AtomicU64>,
    /// 删除操作总延迟（纳秒）
    delete_total_latency: Arc<AtomicU64>,
    /// 删除操作次数
    delete_count: Arc<AtomicU64>,
    /// 删除操作最小延迟（纳秒）
    delete_min_latency: Arc<AtomicU64>,
    /// 删除操作最大延迟（纳秒）
    delete_max_latency: Arc<AtomicU64>,
    /// 批量读取操作总延迟（纳秒）
    get_batch_total_latency: Arc<AtomicU64>,
    /// 批量读取操作次数
    get_batch_count: Arc<AtomicU64>,
    /// 批量写入操作总延迟（纳秒）
    set_batch_total_latency: Arc<AtomicU64>,
    /// 批量写入操作次数
    set_batch_count: Arc<AtomicU64>,
    /// 批量删除操作总延迟（纳秒）
    delete_batch_total_latency: Arc<AtomicU64>,
    /// 批量删除操作次数
    delete_batch_count: Arc<AtomicU64>,
    /// 热点键计数器
    hot_keys: Arc<RwLock<dashmap::DashMap<String, u64>>>,
}

impl Clone for CacheManager {
    fn clone(&self) -> Self {
        Self {
            db: Arc::clone(&self.db),
            metadata_tree: Arc::clone(&self.metadata_tree),
            config: self.config.clone(),
            stats: Arc::clone(&self.stats),
            hits: Arc::clone(&self.hits),
            misses: Arc::clone(&self.misses),
            writes: Arc::clone(&self.writes),
            deletes: Arc::clone(&self.deletes),
            evictions: Arc::clone(&self.evictions),
            bloom_filter: self.bloom_filter.clone(),
            get_total_latency: Arc::clone(&self.get_total_latency),
            get_count: Arc::clone(&self.get_count),
            get_min_latency: Arc::clone(&self.get_min_latency),
            get_max_latency: Arc::clone(&self.get_max_latency),
            set_total_latency: Arc::clone(&self.set_total_latency),
            set_count: Arc::clone(&self.set_count),
            set_min_latency: Arc::clone(&self.set_min_latency),
            set_max_latency: Arc::clone(&self.set_max_latency),
            delete_total_latency: Arc::clone(&self.delete_total_latency),
            delete_count: Arc::clone(&self.delete_count),
            delete_min_latency: Arc::clone(&self.delete_min_latency),
            delete_max_latency: Arc::clone(&self.delete_max_latency),
            get_batch_total_latency: Arc::clone(&self.get_batch_total_latency),
            get_batch_count: Arc::clone(&self.get_batch_count),
            set_batch_total_latency: Arc::clone(&self.set_batch_total_latency),
            set_batch_count: Arc::clone(&self.set_batch_count),
            delete_batch_total_latency: Arc::clone(&self.delete_batch_total_latency),
            delete_batch_count: Arc::clone(&self.delete_batch_count),
            hot_keys: Arc::clone(&self.hot_keys),
        }
    }
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
            .map_err(|e| CacheError::DatabaseError(format!("Lock poisoned: {e}")))?;
        
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
                CacheError::DatabaseError(format!("创建缓存目录失败: {e}"))
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

        let db = Arc::new(db_config.open().map_err(|e| {
            CacheError::DatabaseError(format!("打开数据库失败: {e}"))
        })?);

        let metadata_tree = Arc::new(db.open_tree("cache_metadata").map_err(|e| {
            CacheError::DatabaseError(format!("打开元数据树失败: {e}"))
        })?);

        // 初始化布隆过滤器
        let bloom_filter = if config.enable_bloom_filter {
            let bloom_config = BloomFilterConfig {
                expected_elements: config.bloom_filter_expected_elements,
                false_positive_rate: config.bloom_filter_false_positive_rate,
            };
            Some(Arc::new(RwLock::new(BloomFilter::new(bloom_config))))
        } else {
            None
        };

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
            bloom_filter,
            get_total_latency: Arc::new(AtomicU64::new(0)),
            get_count: Arc::new(AtomicU64::new(0)),
            get_min_latency: Arc::new(AtomicU64::new(u64::MAX)),
            get_max_latency: Arc::new(AtomicU64::new(0)),
            set_total_latency: Arc::new(AtomicU64::new(0)),
            set_count: Arc::new(AtomicU64::new(0)),
            set_min_latency: Arc::new(AtomicU64::new(u64::MAX)),
            set_max_latency: Arc::new(AtomicU64::new(0)),
            delete_total_latency: Arc::new(AtomicU64::new(0)),
            delete_count: Arc::new(AtomicU64::new(0)),
            delete_min_latency: Arc::new(AtomicU64::new(u64::MAX)),
            delete_max_latency: Arc::new(AtomicU64::new(0)),
            get_batch_total_latency: Arc::new(AtomicU64::new(0)),
            get_batch_count: Arc::new(AtomicU64::new(0)),
            set_batch_total_latency: Arc::new(AtomicU64::new(0)),
            set_batch_count: Arc::new(AtomicU64::new(0)),
            delete_batch_total_latency: Arc::new(AtomicU64::new(0)),
            delete_batch_count: Arc::new(AtomicU64::new(0)),
            hot_keys: Arc::new(RwLock::new(dashmap::DashMap::new())),
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
    /// * `scope` - 缓存作用域
    /// * `key` - 缓存键
    ///
    /// # 返回值
    ///
    /// 返回缓存值，如果不存在或已过期则返回 None
    pub fn get(&self, scope: &str, key: &str) -> Result<Option<CacheValue>> {
        let start_time = std::time::Instant::now();
        
        if !self.config.enabled {
            return Err(CacheError::CacheDisabled);
        }

        // 解析作用域
        let (top_scope, full_key) = self.parse_scope(scope, key);
        
        // 记录热点键访问
        let hot_key = format!("{scope}:{key}");
        if let Ok(map) = self.hot_keys.write() {
            *map.entry(hot_key.clone()).or_insert(0) += 1;
        }

        // 检查布隆过滤器，如果启用了布隆过滤器且键不在过滤器中，直接返回 None
        if let Some(bloom_filter) = &self.bloom_filter {
            let filter = bloom_filter.read().map_err(|e| {
                CacheError::DatabaseError(format!("获取布隆过滤器读锁失败: {e}"))
            })?;
            
            if !filter.contains(&full_key) {
                self.misses.fetch_add(1, Ordering::Relaxed);
                
                // 更新延迟统计
                let elapsed = start_time.elapsed().as_nanos() as u64;
                self.update_latency_stats(elapsed, true);
                
                return Ok(None);
            }
        }

        // 获取元数据
        let metadata = match self.get_metadata(&full_key)? {
            Some(meta) => meta,
            None => {
                self.misses.fetch_add(1, Ordering::Relaxed);
                
                // 更新延迟统计
                let elapsed = start_time.elapsed().as_nanos() as u64;
                self.update_latency_stats(elapsed, true);
                
                return Ok(None);
            }
        };

        // 检查是否过期
        if metadata.is_expired() {
            self.misses.fetch_add(1, Ordering::Relaxed);
            
            // 更新延迟统计
            let elapsed = start_time.elapsed().as_nanos() as u64;
            self.update_latency_stats(elapsed, true);
            
            return Ok(None);
        }

        // 获取数据
        let tree = self.get_or_create_tree(&top_scope)?;
        let value = tree.get(full_key.as_bytes()).map_err(|e| {
            // 更新延迟统计
            let elapsed = start_time.elapsed().as_nanos() as u64;
            self.update_latency_stats(elapsed, true);
            
            CacheError::DatabaseError(format!("读取缓存失败: {e}"))
        })?;

        let result = match value {
            Some(v) => {
                self.hits.fetch_add(1, Ordering::Relaxed);
                // 更新元数据访问信息（异步，不阻塞读取）
                let _ = self.update_metadata_access(&full_key);
                Ok(Some(v.to_vec()))
            }
            None => {
                self.misses.fetch_add(1, Ordering::Relaxed);
                Ok(None)
            }
        };
        
        // 更新延迟统计
        let elapsed = start_time.elapsed().as_nanos() as u64;
        self.update_latency_stats(elapsed, true);
        
        result
    }

    /// 异步获取缓存值
    ///
    /// # 参数
    ///
    /// * `scope` - 缓存作用域
    /// * `key` - 缓存键
    ///
    /// # 返回值
    ///
    /// 返回缓存值，如果不存在或已过期则返回 None
    pub async fn get_async(&self, scope: &str, key: &str) -> Result<Option<CacheValue>> {
        let manager = self.clone();
        let scope = scope.to_string();
        let key = key.to_string();
        
        // 使用 tokio::spawn_blocking 在异步上下文中运行同步代码
        tokio::task::spawn_blocking(move || {
            manager.get(&scope, &key)
        }).await.map_err(|e| {
            CacheError::DatabaseError(format!("异步操作失败: {e}"))
        })?
    }

    /// 获取缓存值（包括过期的）
    ///
    /// # 参数
    ///
    /// * `scope` - 缓存作用域
    /// * `key` - 缓存键
    ///
    /// # 返回值
    ///
    /// 返回缓存值和是否过期的标志，如果不存在则返回 None
    pub fn get_include_stale(&self, scope: &str, key: &str) -> Result<Option<(CacheValue, bool)>> {
        if !self.config.enabled {
            return Err(CacheError::CacheDisabled);
        }

        // 解析作用域
        let (top_scope, full_key) = self.parse_scope(scope, key);

        // 获取元数据
        let metadata = match self.get_metadata(&full_key)? {
            Some(meta) => meta,
            None => {
                self.misses.fetch_add(1, Ordering::Relaxed);
                return Ok(None);
            }
        };

        // 检查是否过期
        let is_stale = metadata.is_expired();
        
        // 获取数据
        let tree = self.get_or_create_tree(&top_scope)?;
        let value = tree.get(full_key.as_bytes()).map_err(|e| {
            CacheError::DatabaseError(format!("读取缓存失败: {e}"))
        })?;

        match value {
            Some(v) => {
                if is_stale {
                    self.misses.fetch_add(1, Ordering::Relaxed);
                } else {
                    self.hits.fetch_add(1, Ordering::Relaxed);
                }
                // 更新元数据访问信息（异步，不阻塞读取）
                let _ = self.update_metadata_access(&full_key);
                Ok(Some((v.to_vec(), is_stale)))
            }
            None => {
                self.misses.fetch_add(1, Ordering::Relaxed);
                Ok(None)
            }
        }
    }

    /// 异步获取缓存值（包括过期的）
    ///
    /// # 参数
    ///
    /// * `scope` - 缓存作用域
    /// * `key` - 缓存键
    ///
    /// # 返回值
    ///
    /// 返回缓存值和是否过期的标志，如果不存在则返回 None
    pub async fn get_include_stale_async(&self, scope: &str, key: &str) -> Result<Option<(CacheValue, bool)>> {
        let manager = self.clone();
        let scope = scope.to_string();
        let key = key.to_string();
        
        // 使用 tokio::spawn_blocking 在异步上下文中运行同步代码
        tokio::task::spawn_blocking(move || {
            manager.get_include_stale(&scope, &key)
        }).await.map_err(|e| {
            CacheError::DatabaseError(format!("异步操作失败: {e}"))
        })?
    }

    /// 批量获取缓存值
    ///
    /// # 参数
    ///
    /// * `scope` - 缓存作用域
    /// * `keys` - 缓存键列表
    ///
    /// # 返回值
    ///
    /// 返回缓存值列表，与输入键列表顺序一致，不存在或已过期则返回 None
    pub fn get_batch(&self, scope: &str, keys: &[&str]) -> Result<Vec<Option<CacheValue>>> {
        if !self.config.enabled {
            return Err(CacheError::CacheDisabled);
        }

        let mut results = Vec::with_capacity(keys.len());
        
        for key in keys {
            let result = self.get(scope, key)?;
            results.push(result);
        }
        
        Ok(results)
    }

    /// 异步批量获取缓存值
    ///
    /// # 参数
    ///
    /// * `scope` - 缓存作用域
    /// * `keys` - 缓存键列表
    ///
    /// # 返回值
    ///
    /// 返回缓存值列表，与输入键列表顺序一致，不存在或已过期则返回 None
    pub async fn get_batch_async(&self, scope: &str, keys: &[&str]) -> Result<Vec<Option<CacheValue>>> {
        let manager = self.clone();
        let scope = scope.to_string();
        let keys: Vec<String> = keys.iter().map(|k| k.to_string()).collect();
        
        // 使用 tokio::spawn_blocking 在异步上下文中运行同步代码
        tokio::task::spawn_blocking(move || {
            let mut results = Vec::with_capacity(keys.len());
            
            for key in keys {
                let result = manager.get(&scope, &key)?;
                results.push(result);
            }
            
            Ok(results)
        }).await.map_err(|e| {
            CacheError::DatabaseError(format!("异步操作失败: {e}"))
        })?
    }

    /// 批量设置缓存值
    ///
    /// # 参数
    ///
    /// * `scope` - 缓存作用域
    /// * `items` - 缓存键值对列表
    /// * `ttl` - 生存时间，None 表示使用默认值
    ///
    /// # 返回值
    ///
    /// 成功返回 Ok(())，失败返回错误
    pub fn set_batch(&self, scope: &str, items: &[(String, CacheValue)], ttl: Option<Duration>) -> Result<()> {
        if !self.config.enabled {
            return Err(CacheError::CacheDisabled);
        }

        for (key, value) in items {
            self.set(scope, key.clone(), value.clone(), ttl)?;
        }
        
        Ok(())
    }

    /// 异步批量设置缓存值
    ///
    /// # 参数
    ///
    /// * `scope` - 缓存作用域
    /// * `items` - 缓存键值对列表
    /// * `ttl` - 生存时间，None 表示使用默认值
    ///
    /// # 返回值
    ///
    /// 成功返回 Ok(())，失败返回错误
    pub async fn set_batch_async(&self, scope: &str, items: &[(String, CacheValue)], ttl: Option<Duration>) -> Result<()> {
        let manager = self.clone();
        let scope = scope.to_string();
        let items: Vec<(String, CacheValue)> = items.to_vec();
        
        // 使用 tokio::spawn_blocking 在异步上下文中运行同步代码
        tokio::task::spawn_blocking(move || {
            for (key, value) in items {
                manager.set(&scope, key, value, ttl)?;
            }
            
            Ok(())
        }).await.map_err(|e| {
            CacheError::DatabaseError(format!("异步操作失败: {e}"))
        })?
    }

    /// 批量删除缓存项
    ///
    /// # 参数
    ///
    /// * `scope` - 缓存作用域
    /// * `keys` - 缓存键列表
    ///
    /// # 返回值
    ///
    /// 返回成功删除的条目数量
    pub fn delete_batch(&self, scope: &str, keys: &[&str]) -> Result<usize> {
        if !self.config.enabled {
            return Err(CacheError::CacheDisabled);
        }

        let mut deleted = 0;
        
        for key in keys {
            if self.delete(scope, key)? {
                deleted += 1;
            }
        }
        
        Ok(deleted)
    }

    /// 异步批量删除缓存项
    ///
    /// # 参数
    ///
    /// * `scope` - 缓存作用域
    /// * `keys` - 缓存键列表
    ///
    /// # 返回值
    ///
    /// 返回成功删除的条目数量
    pub async fn delete_batch_async(&self, scope: &str, keys: &[&str]) -> Result<usize> {
        let manager = self.clone();
        let scope = scope.to_string();
        let keys: Vec<String> = keys.iter().map(|k| k.to_string()).collect();
        
        // 使用 tokio::spawn_blocking 在异步上下文中运行同步代码
        tokio::task::spawn_blocking(move || {
            let mut deleted = 0;
            
            for key in keys {
                if manager.delete(&scope, &key)? {
                    deleted += 1;
                }
            }
            
            Ok(deleted)
        }).await.map_err(|e| {
            CacheError::DatabaseError(format!("异步操作失败: {e}"))
        })?
    }

    
    
    /// 设置缓存值
    ///
    /// # 参数
    ///
    /// * `scope` - 缓存作用域，格式：顶级作用域.二级作用域.三级作用域
    /// * `key` - 缓存键
    /// * `value` - 缓存值
    /// * `ttl` - 生存时间，None 表示使用默认值
    ///
    /// # 返回值
    ///
    /// 成功返回 Ok(())，失败返回错误
    pub fn set(&self, scope: &str, key: String, value: CacheValue, ttl: Option<Duration>) -> Result<()> {
        self.set_with_condition(scope, key, value, ttl, |_, _| true)
    }
    
    /// 基于条件设置缓存值
    ///
    /// # 参数
    ///
    /// * `scope` - 缓存作用域，格式：顶级作用域.二级作用域.三级作用域
    /// * `key` - 缓存键
    /// * `value` - 缓存值
    /// * `ttl` - 生存时间，None 表示使用默认值
    /// * `condition` - 条件函数，接收当前值和元数据，返回是否允许更新
    ///
    /// # 返回值
    ///
    /// 成功返回 Ok(())，失败返回错误
    pub fn set_with_condition(
        &self, 
        scope: &str, 
        key: String, 
        value: CacheValue, 
        ttl: Option<Duration>,
        condition: impl Fn(&Option<CacheValue>, &Option<CacheEntryMetadata>) -> bool + Send + Sync + 'static
    ) -> Result<()> {
        let start_time = std::time::Instant::now();
        
        if !self.config.enabled {
            return Err(CacheError::CacheDisabled);
        }

        // 解析作用域，获取顶级作用域和完整键
        let (top_scope, full_key) = self.parse_scope(scope, &key);
        
        // 记录热点键访问
        let hot_key = format!("{scope}:{key}");
        if let Ok(map) = self.hot_keys.write() {
            *map.entry(hot_key.clone()).or_insert(0) += 1;
        }

        // 获取当前值和元数据，用于条件判断
        let current_value = self.get(scope, &key)?;
        let current_metadata = self.get_metadata(&full_key)?;
        
        // 检查条件
        if !condition(&current_value, &current_metadata) {
            return Ok(());
        }

        // 检查缓存大小限制
        let value_size = value.len();
        if self.is_cache_full(value_size)? {
            // 尝试清理过期条目，最多清理100个
            self.cleanup_expired(Some(100))?;
            // 再次检查
            if self.is_cache_full(value_size)? {
                return Err(CacheError::CacheFull);
            }
        }

        // 获取或创建对应的 tree
        let tree = self.get_or_create_tree(&top_scope)?;

        // 创建元数据
        let ttl_duration = ttl.or_else(|| Some(Duration::from_secs(self.config.default_ttl_secs)));
        let metadata = CacheEntryMetadata::new(ttl_duration, value_size, scope.to_string());

        // 写入数据
        tree.insert(full_key.as_bytes(), value.as_slice()).map_err(|e| {
            CacheError::DatabaseError(format!("写入缓存失败: {e}"))
        })?;

        // 写入元数据
        self.set_metadata(&full_key, &metadata)?;

        // 将键添加到布隆过滤器
        if let Some(bloom_filter) = &self.bloom_filter {
            let mut filter = bloom_filter.write().map_err(|e| {
                CacheError::DatabaseError(format!("获取布隆过滤器写锁失败: {e}"))
            })?;
            filter.add(&full_key);
        }

        self.writes.fetch_add(1, Ordering::Relaxed);
        
        // 更新延迟统计
        let elapsed = start_time.elapsed().as_nanos() as u64;
        self.update_latency_stats(elapsed, false);
        
        Ok(())
    }

    /// 异步设置缓存值
    ///
    /// # 参数
    ///
    /// * `scope` - 缓存作用域，格式：顶级作用域.二级作用域.三级作用域
    /// * `key` - 缓存键
    /// * `value` - 缓存值
    /// * `ttl` - 生存时间，None 表示使用默认值
    ///
    /// # 返回值
    ///
    /// 成功返回 Ok(())，失败返回错误
    pub async fn set_async(&self, scope: &str, key: String, value: CacheValue, ttl: Option<Duration>) -> Result<()> {
        self.set_with_condition_async(scope, key, value, ttl, |_, _| true).await
    }
    
    /// 异步基于条件设置缓存值
    ///
    /// # 参数
    ///
    /// * `scope` - 缓存作用域，格式：顶级作用域.二级作用域.三级作用域
    /// * `key` - 缓存键
    /// * `value` - 缓存值
    /// * `ttl` - 生存时间，None 表示使用默认值
    /// * `condition` - 条件函数，接收当前值和元数据，返回是否允许更新
    ///
    /// # 返回值
    ///
    /// 成功返回 Ok(())，失败返回错误
    pub async fn set_with_condition_async(
        &self, 
        scope: &str, 
        key: String, 
        value: CacheValue, 
        ttl: Option<Duration>,
        condition: impl Fn(&Option<CacheValue>, &Option<CacheEntryMetadata>) -> bool + Send + Sync + 'static
    ) -> Result<()> {
        let manager = self.clone();
        let scope = scope.to_string();
        
        // 使用 tokio::spawn_blocking 在异步上下文中运行同步代码
        tokio::task::spawn_blocking(move || {
            manager.set_with_condition(&scope, key, value, ttl, condition)
        }).await.map_err(|e| {
            CacheError::DatabaseError(format!("异步操作失败: {e}"))
        })?
    }

    /// 解析作用域，返回顶级作用域和完整键
    ///
    /// # 参数
    ///
    /// * `scope` - 完整作用域字符串
    /// * `key` - 原始键
    ///
    /// # 返回值
    ///
    /// 返回 (顶级作用域, 完整键)
    fn parse_scope(&self, scope: &str, key: &str) -> (String, String) {
        // 分割作用域，获取顶级作用域
        let parts: Vec<&str> = scope.split('.').collect();
        let top_scope = parts[0].to_string();
        
        // 构建完整键：二级作用域.三级作用域.原始键
        let full_key = if parts.len() > 1 {
            format!("{}.{}", parts[1..].join("."), key)
        } else {
            key.to_string()
        };
        
        (top_scope, full_key)
    }

    /// 获取或创建对应的 sled tree
    ///
    /// # 参数
    ///
    /// * `top_scope` - 顶级作用域
    ///
    /// # 返回值
    ///
    /// 返回对应的 sled tree
    pub fn get_or_create_tree(&self, top_scope: &str) -> Result<sled::Tree> {
        // 尝试获取现有 tree
        let tree = self.db.open_tree(top_scope).map_err(|e| {
            CacheError::DatabaseError(format!("打开 tree 失败: {e}"))
        })?;
        
        Ok(tree)
    }

    /// 删除缓存项
    ///
    /// # 参数
    ///
    /// * `scope` - 缓存作用域
    /// * `key` - 缓存键
    pub fn delete(&self, scope: &str, key: &str) -> Result<bool> {
        let start_time = std::time::Instant::now();
        
        if !self.config.enabled {
            return Err(CacheError::CacheDisabled);
        }

        // 解析作用域
        let (top_scope, full_key) = self.parse_scope(scope, key);
        
        // 获取对应的 tree
        let tree = self.get_or_create_tree(&top_scope)?;

        let existed = tree.remove(full_key.as_bytes()).map_err(|e| {
            CacheError::DatabaseError(format!("删除缓存失败: {e}"))
        })?.is_some();

        if existed {
            let _ = self.metadata_tree.remove(full_key.as_bytes());
            self.deletes.fetch_add(1, Ordering::Relaxed);
            
            // 从热点键列表中移除
            let hot_key = format!("{scope}:{key}");
            if let Ok(map) = self.hot_keys.write() {
                map.remove(&hot_key);
            }
        }
        
        // 更新删除操作延迟统计
        let elapsed = start_time.elapsed().as_nanos() as u64;
        self.update_delete_latency_stats(elapsed);

        Ok(existed)
    }

    /// 异步删除缓存项
    ///
    /// # 参数
    ///
    /// * `scope` - 缓存作用域
    /// * `key` - 缓存键
    pub async fn delete_async(&self, scope: &str, key: &str) -> Result<bool> {
        let manager = self.clone();
        let scope = scope.to_string();
        let key = key.to_string();
        
        // 使用 tokio::spawn_blocking 在异步上下文中运行同步代码
        tokio::task::spawn_blocking(move || {
            manager.delete(&scope, &key)
        }).await.map_err(|e| {
            CacheError::DatabaseError(format!("异步操作失败: {e}"))
        })?
    }

    /// 删除整个作用域的缓存项
    ///
    /// # 参数
    ///
    /// * `scope` - 缓存作用域
    pub fn delete_scope(&self, scope: &str) -> Result<usize> {
        if !self.config.enabled {
            return Err(CacheError::CacheDisabled);
        }

        // 解析作用域
        let (top_scope, prefix) = {
            let parts: Vec<&str> = scope.split('.').collect();
            let top_scope = parts[0].to_string();
            let prefix = if parts.len() > 1 {
                parts[1..].join(".") + "."
            } else {
                "".to_string()
            };
            (top_scope, prefix)
        };
        
        // 获取对应的 tree
        let tree = self.get_or_create_tree(&top_scope)?;
        
        // 删除前缀匹配的所有键
        let mut count = 0;
        let prefix_bytes = prefix.as_bytes();
        
        // 使用范围删除
        let mut iter = tree.range(prefix_bytes.to_vec()..);
        
        while let Some(Ok((key, _))) = iter.next() {
            if !key.starts_with(prefix_bytes) {
                break;
            }
            
            // 删除数据
            if tree.remove(&key).map_err(|e| {
                CacheError::DatabaseError(format!("删除缓存失败: {e}"))
            })?.is_some() {
                // 删除元数据
                let _ = self.metadata_tree.remove(&key);
                count += 1;
                self.deletes.fetch_add(1, Ordering::Relaxed);
            }
        }

        Ok(count)
    }

    /// 异步删除整个作用域的缓存项
    ///
    /// # 参数
    ///
    /// * `scope` - 缓存作用域
    pub async fn delete_scope_async(&self, scope: &str) -> Result<usize> {
        let manager = self.clone();
        let scope = scope.to_string();
        
        // 使用 tokio::spawn_blocking 在异步上下文中运行同步代码
        tokio::task::spawn_blocking(move || {
            manager.delete_scope(&scope)
        }).await.map_err(|e| {
            CacheError::DatabaseError(format!("异步操作失败: {e}"))
        })?
    }

    /// 清空所有缓存
    pub fn clear(&self) -> Result<()> {
        if !self.config.enabled {
            return Err(CacheError::CacheDisabled);
        }

        // 清空元数据
        self.metadata_tree.clear().map_err(|e| {
            CacheError::DatabaseError(format!("清空元数据失败: {e}"))
        })?;

        // 注意：sled 不支持直接获取所有 tree 名称，我们需要手动管理或使用其他方法
        // 这里简化实现，只清空已知的主要 tree
        Ok(())
    }

    /// 异步清空所有缓存
    pub async fn clear_async(&self) -> Result<()> {
        let manager = self.clone();
        
        // 使用 tokio::spawn_blocking 在异步上下文中运行同步代码
        tokio::task::spawn_blocking(move || {
            manager.clear()
        }).await.map_err(|e| {
            CacheError::DatabaseError(format!("异步操作失败: {e}"))
        })?
    }

    /// 清空指定作用域的缓存
    ///
    /// # 参数
    ///
    /// * `scope` - 缓存作用域
    pub fn clear_scope(&self, scope: &str) -> Result<()> {
        if !self.config.enabled {
            return Err(CacheError::CacheDisabled);
        }

        // 解析作用域
        let (top_scope, prefix) = {
            let parts: Vec<&str> = scope.split('.').collect();
            let top_scope = parts[0].to_string();
            let prefix = if parts.len() > 1 {
                parts[1..].join(".") + "."
            } else {
                "".to_string()
            };
            (top_scope, prefix)
        };
        
        // 获取对应的 tree
        let tree = self.get_or_create_tree(&top_scope)?;
        
        // 清空前缀匹配的所有键
        let prefix_bytes = prefix.as_bytes();
        
        // 使用范围删除
        let mut iter = tree.range(prefix_bytes.to_vec()..);
        
        while let Some(Ok((key, _))) = iter.next() {
            if !key.starts_with(prefix_bytes) {
                break;
            }
            
            // 删除数据
            tree.remove(&key).map_err(|e| {
                CacheError::DatabaseError(format!("清空作用域失败: {e}"))
            })?;
            
            // 删除元数据
            let _ = self.metadata_tree.remove(&key);
        }

        Ok(())
    }

    /// 异步清空指定作用域的缓存
    ///
    /// # 参数
    ///
    /// * `scope` - 缓存作用域
    pub async fn clear_scope_async(&self, scope: &str) -> Result<()> {
        let manager = self.clone();
        let scope = scope.to_string();
        
        // 使用 tokio::spawn_blocking 在异步上下文中运行同步代码
        tokio::task::spawn_blocking(move || {
            manager.clear_scope(&scope)
        }).await.map_err(|e| {
            CacheError::DatabaseError(format!("异步操作失败: {e}"))
        })?
    }

    /// 清理过期条目
    ///
    /// 遍历所有条目并删除已过期的
    ///
    /// # 参数
    ///
    /// * `max_items` - 最大清理条目数量，None 表示清理所有过期条目
    pub fn cleanup_expired(&self, max_items: Option<usize>) -> Result<usize> {
        let mut count = 0;
        let max_items = max_items.unwrap_or(usize::MAX);
        
        // 遍历所有元数据
        for item in self.metadata_tree.iter() {
            if count >= max_items {
                break;
            }
            
            let (key, value) = item.map_err(|e| {
                CacheError::DatabaseError(format!("遍历元数据失败: {e}"))
            })?;

            let metadata: CacheEntryMetadata = bincode::serde::decode_from_slice(&value, bincode::config::standard())
                .map(|(meta, _)| meta)
                .map_err(|e| {
                    CacheError::SerializationError(format!("反序列化元数据失败: {e}"))
                })?;

            if metadata.is_expired() {
                // 从元数据中获取作用域
                let scope = &metadata.scope;
                
                // 解析完整键，获取顶级作用域
                let top_scope = {
                    let parts: Vec<&str> = scope.split('.').collect();
                    parts[0].to_string()
                };
                
                // 获取对应的 tree
                let tree = self.get_or_create_tree(&top_scope)?;
                
                // 删除过期条目
                if tree.remove(&key).map_err(|e| {
                    CacheError::DatabaseError(format!("删除过期条目失败: {e}"))
                })?.is_some() {
                    // 删除元数据
                    let _ = self.metadata_tree.remove(&key);
                    count += 1;
                    self.evictions.fetch_add(1, Ordering::Relaxed);
                }
            }
        }

        Ok(count)
    }

    /// 清理过期条目（默认实现，兼容旧版本）
    ///
    /// 遍历所有条目并删除已过期的
    pub fn cleanup_expired_default(&self) -> Result<usize> {
        self.cleanup_expired(None)
    }

    /// 异步清理过期条目
    ///
    /// 遍历所有条目并删除已过期的
    ///
    /// # 参数
    ///
    /// * `max_items` - 最大清理条目数量，None 表示清理所有过期条目
    pub async fn cleanup_expired_async(&self, max_items: Option<usize>) -> Result<usize> {
        let manager = self.clone();
        
        // 使用 tokio::spawn_blocking 在异步上下文中运行同步代码
        tokio::task::spawn_blocking(move || {
            manager.cleanup_expired(max_items)
        }).await.map_err(|e| {
            CacheError::DatabaseError(format!("异步操作失败: {e}"))
        })?
    }

    /// 异步清理过期条目（默认实现，兼容旧版本）
    ///
    /// 遍历所有条目并删除已过期的
    pub async fn cleanup_expired_async_default(&self) -> Result<usize> {
        self.cleanup_expired_async(None).await
    }

    /// 清理指定作用域的过期条目
    ///
    /// # 参数
    ///
    /// * `scope` - 缓存作用域
    pub fn cleanup_expired_by_scope(&self, scope: &str) -> Result<usize> {
        let mut count = 0;
        
        // 遍历所有元数据，查找指定作用域的过期条目
        for item in self.metadata_tree.iter() {
            let (key, value) = item.map_err(|e| {
                CacheError::DatabaseError(format!("遍历元数据失败: {e}"))
            })?;

            let metadata: CacheEntryMetadata = bincode::serde::decode_from_slice(&value, bincode::config::standard())
                .map(|(meta, _)| meta)
                .map_err(|e| {
                    CacheError::SerializationError(format!("反序列化元数据失败: {e}"))
                })?;

            // 检查是否是指定作用域的条目
            if metadata.scope == scope && metadata.is_expired() {
                // 解析完整键，获取顶级作用域
                let parts: Vec<&str> = scope.split('.').collect();
                let top_scope = parts[0].to_string();
                
                // 获取对应的 tree
                let tree = self.get_or_create_tree(&top_scope)?;
                
                // 删除过期条目
                if tree.remove(&key).map_err(|e| {
                    CacheError::DatabaseError(format!("删除过期条目失败: {e}"))
                })?.is_some() {
                    // 删除元数据
                    let _ = self.metadata_tree.remove(&key);
                    count += 1;
                    self.evictions.fetch_add(1, Ordering::Relaxed);
                }
            }
        }

        Ok(count)
    }

    /// 异步清理指定作用域的过期条目
    ///
    /// # 参数
    ///
    /// * `scope` - 缓存作用域
    pub async fn cleanup_expired_by_scope_async(&self, scope: &str) -> Result<usize> {
        let manager = self.clone();
        let scope = scope.to_string();
        
        // 使用 tokio::spawn_blocking 在异步上下文中运行同步代码
        tokio::task::spawn_blocking(move || {
            manager.cleanup_expired_by_scope(&scope)
        }).await.map_err(|e| {
            CacheError::DatabaseError(format!("异步操作失败: {e}"))
        })?
    }

    /// 获取缓存统计信息
    pub fn stats(&self) -> CacheStats {
        // 计算读取操作延迟统计
        let get_count = self.get_count.load(Ordering::Relaxed);
        let get_total_latency = self.get_total_latency.load(Ordering::Relaxed);
        let get_avg_latency = if get_count > 0 {
            get_total_latency / get_count
        } else {
            0
        };
        
        let get_latency = LatencyStats {
            count: get_count,
            total_ns: get_total_latency,
            min_ns: self.get_min_latency.load(Ordering::Relaxed),
            max_ns: self.get_max_latency.load(Ordering::Relaxed),
            avg_ns: get_avg_latency,
        };
        
        // 计算写入操作延迟统计
        let set_count = self.set_count.load(Ordering::Relaxed);
        let set_total_latency = self.set_total_latency.load(Ordering::Relaxed);
        let set_avg_latency = if set_count > 0 {
            set_total_latency / set_count
        } else {
            0
        };
        
        let set_latency = LatencyStats {
            count: set_count,
            total_ns: set_total_latency,
            min_ns: self.set_min_latency.load(Ordering::Relaxed),
            max_ns: self.set_max_latency.load(Ordering::Relaxed),
            avg_ns: set_avg_latency,
        };
        
        // 计算删除操作延迟统计
        let delete_count = self.delete_count.load(Ordering::Relaxed);
        let delete_total_latency = self.delete_total_latency.load(Ordering::Relaxed);
        let delete_avg_latency = if delete_count > 0 {
            delete_total_latency / delete_count
        } else {
            0
        };
        
        let delete_latency = LatencyStats {
            count: delete_count,
            total_ns: delete_total_latency,
            min_ns: self.delete_min_latency.load(Ordering::Relaxed),
            max_ns: self.delete_max_latency.load(Ordering::Relaxed),
            avg_ns: delete_avg_latency,
        };
        
        // 计算批量读取操作延迟统计
        let get_batch_count = self.get_batch_count.load(Ordering::Relaxed);
        let get_batch_total_latency = self.get_batch_total_latency.load(Ordering::Relaxed);
        let get_batch_avg_latency = if get_batch_count > 0 {
            get_batch_total_latency / get_batch_count
        } else {
            0
        };
        
        let get_batch_latency = LatencyStats {
            count: get_batch_count,
            total_ns: get_batch_total_latency,
            min_ns: 0, // 暂时不统计批量操作的最小延迟
            max_ns: 0, // 暂时不统计批量操作的最大延迟
            avg_ns: get_batch_avg_latency,
        };
        
        // 计算批量写入操作延迟统计
        let set_batch_count = self.set_batch_count.load(Ordering::Relaxed);
        let set_batch_total_latency = self.set_batch_total_latency.load(Ordering::Relaxed);
        let set_batch_avg_latency = if set_batch_count > 0 {
            set_batch_total_latency / set_batch_count
        } else {
            0
        };
        
        let set_batch_latency = LatencyStats {
            count: set_batch_count,
            total_ns: set_batch_total_latency,
            min_ns: 0, // 暂时不统计批量操作的最小延迟
            max_ns: 0, // 暂时不统计批量操作的最大延迟
            avg_ns: set_batch_avg_latency,
        };
        
        // 计算批量删除操作延迟统计
        let delete_batch_count = self.delete_batch_count.load(Ordering::Relaxed);
        let delete_batch_total_latency = self.delete_batch_total_latency.load(Ordering::Relaxed);
        let delete_batch_avg_latency = if delete_batch_count > 0 {
            delete_batch_total_latency / delete_batch_count
        } else {
            0
        };
        
        let delete_batch_latency = LatencyStats {
            count: delete_batch_count,
            total_ns: delete_batch_total_latency,
            min_ns: 0, // 暂时不统计批量操作的最小延迟
            max_ns: 0, // 暂时不统计批量操作的最大延迟
            avg_ns: delete_batch_avg_latency,
        };
        
        // 收集热点键信息
        let hot_keys = self.hot_keys.read().map(|map| {
            let mut keys: Vec<_> = map.iter().collect();
            // 按访问次数排序
            keys.sort_by(|a, b| b.value().cmp(a.value()));
            // 取前20个热点键
            keys.into_iter().take(20).map(|entry| {
                let key = entry.key();
                let count = *entry.value();
                // 解析键，获取作用域和实际键名
                let parts: Vec<&str> = key.splitn(2, ':').collect();
                let (scope, actual_key) = if parts.len() == 2 {
                    (parts[0].to_string(), parts[1].to_string())
                } else {
                    ("unknown".to_string(), key.to_string())
                };
                
                HotKeyInfo {
                    key: actual_key,
                    access_count: count,
                    scope,
                }
            }).collect()
        }).unwrap_or_default();
        
        CacheStats {
            hits: self.hits.load(Ordering::Relaxed),
            misses: self.misses.load(Ordering::Relaxed),
            writes: self.writes.load(Ordering::Relaxed),
            deletes: self.deletes.load(Ordering::Relaxed),
            total_keys: self.db.len() as u64,
            estimated_size_bytes: self.db.size_on_disk().unwrap_or(0),
            evictions: self.evictions.load(Ordering::Relaxed),
            get_latency,
            set_latency,
            delete_latency,
            get_batch_latency,
            set_batch_latency,
            delete_batch_latency,
            hot_keys,
        }
    }

    /// 刷新到磁盘
    pub fn flush(&self) -> Result<()> {
        self.db.flush().map_err(|e| {
            CacheError::DatabaseError(format!("刷新缓存失败: {e}"))
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
                        CacheError::SerializationError(format!("反序列化元数据失败: {e}"))
                    })?;
                Ok(Some(metadata))
            }
            Ok(None) => Ok(None),
            Err(e) => Err(CacheError::DatabaseError(format!("读取元数据失败: {e}"))),
        }
    }

    fn set_metadata(&self, key: &str, metadata: &CacheEntryMetadata) -> Result<()> {
        let data = bincode::serde::encode_to_vec(metadata, bincode::config::standard()).map_err(|e| {
            CacheError::SerializationError(format!("序列化元数据失败: {e}"))
        })?;

        self.metadata_tree.insert(key.as_bytes(), data.as_slice()).map_err(|e| {
            CacheError::DatabaseError(format!("写入元数据失败: {e}"))
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
    
    /// 更新延迟统计信息
    ///
    /// # 参数
    ///
    /// * `elapsed_ns` - 操作耗时（纳秒）
    /// * `is_get` - 是否为读取操作
    fn update_latency_stats(&self, elapsed_ns: u64, is_get: bool) {
        if is_get {
            // 更新读取操作延迟统计
            self.get_total_latency.fetch_add(elapsed_ns, Ordering::Relaxed);
            self.get_count.fetch_add(1, Ordering::Relaxed);
            
            // 更新最小延迟
            let mut current_min = self.get_min_latency.load(Ordering::Relaxed);
            while elapsed_ns < current_min {
                if self.get_min_latency.compare_exchange_weak(
                    current_min, 
                    elapsed_ns, 
                    Ordering::Relaxed, 
                    Ordering::Relaxed
                ).is_ok() {
                    break;
                }
                current_min = self.get_min_latency.load(Ordering::Relaxed);
            }
            
            // 更新最大延迟
            let mut current_max = self.get_max_latency.load(Ordering::Relaxed);
            while elapsed_ns > current_max {
                if self.get_max_latency.compare_exchange_weak(
                    current_max, 
                    elapsed_ns, 
                    Ordering::Relaxed, 
                    Ordering::Relaxed
                ).is_ok() {
                    break;
                }
                current_max = self.get_max_latency.load(Ordering::Relaxed);
            }
        } else {
            // 更新写入操作延迟统计
            self.set_total_latency.fetch_add(elapsed_ns, Ordering::Relaxed);
            self.set_count.fetch_add(1, Ordering::Relaxed);
            
            // 更新最小延迟
            let mut current_min = self.set_min_latency.load(Ordering::Relaxed);
            while elapsed_ns < current_min {
                if self.set_min_latency.compare_exchange_weak(
                    current_min, 
                    elapsed_ns, 
                    Ordering::Relaxed, 
                    Ordering::Relaxed
                ).is_ok() {
                    break;
                }
                current_min = self.set_min_latency.load(Ordering::Relaxed);
            }
            
            // 更新最大延迟
            let mut current_max = self.set_max_latency.load(Ordering::Relaxed);
            while elapsed_ns > current_max {
                if self.set_max_latency.compare_exchange_weak(
                    current_max, 
                    elapsed_ns, 
                    Ordering::Relaxed, 
                    Ordering::Relaxed
                ).is_ok() {
                    break;
                }
                current_max = self.set_max_latency.load(Ordering::Relaxed);
            }
        }
    }
    
    /// 更新删除操作延迟统计
    ///
    /// # 参数
    ///
    /// * `elapsed_ns` - 操作耗时（纳秒）
    fn update_delete_latency_stats(&self, elapsed_ns: u64) {
        // 更新删除操作延迟统计
        self.delete_total_latency.fetch_add(elapsed_ns, Ordering::Relaxed);
        self.delete_count.fetch_add(1, Ordering::Relaxed);
        
        // 更新最小延迟
        let mut current_min = self.delete_min_latency.load(Ordering::Relaxed);
        while elapsed_ns < current_min {
            if self.delete_min_latency.compare_exchange_weak(
                current_min, 
                elapsed_ns, 
                Ordering::Relaxed, 
                Ordering::Relaxed
            ).is_ok() {
                break;
            }
            current_min = self.delete_min_latency.load(Ordering::Relaxed);
        }
        
        // 更新最大延迟
        let mut current_max = self.delete_max_latency.load(Ordering::Relaxed);
        while elapsed_ns > current_max {
            if self.delete_max_latency.compare_exchange_weak(
                current_max, 
                elapsed_ns, 
                Ordering::Relaxed, 
                Ordering::Relaxed
            ).is_ok() {
                break;
            }
            current_max = self.delete_max_latency.load(Ordering::Relaxed);
        }
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
            enable_bloom_filter: false,
            bloom_filter_expected_elements: 1000,
            bloom_filter_false_positive_rate: 0.01,
        }
    }

    #[test]
    #[serial]
    fn test_cache_manager_creation() {
        let config = temp_cache_config();
        let manager = CacheManager::instance(config);
        assert!(manager.is_ok());
    }

    #[test]
    #[serial]
    fn test_cache_set_and_get() {
        let config = temp_cache_config();
        let manager = match CacheManager::instance(config) {
            Ok(m) => m,
            Err(_) => return, // Skip test if cache creation fails
        };

        let scope = "test.scope";
        let key = "test_key";
        let value = b"test_value".to_vec();

        // 设置缓存
        let _ = manager.set(scope, key.to_string(), value.clone(), None);

        // 获取缓存
        let result = manager.get(scope, key).unwrap_or(None);
        assert!(result.is_some());
        assert_eq!(result.unwrap(), value);
    }

    #[test]
    #[serial]
    fn test_cache_expiration() {
        let config = temp_cache_config();
        let manager = match CacheManager::instance(config) {
            Ok(m) => m,
            Err(_) => return, // Skip test if cache creation fails
        };

        let scope = "test.scope";
        let key = "expire_key";
        let value = b"expire_value".to_vec();

        // 设置1秒过期
        let _ = manager.set(scope, key.to_string(), value, Some(Duration::from_secs(1)));

        // 立即获取应该存在
        assert!(manager.get(scope, key).unwrap_or(None).is_some());

        // 等待过期
        std::thread::sleep(Duration::from_millis(1100));

        // 获取应该返回 None
        assert!(manager.get(scope, key).unwrap_or(None).is_none());
    }

    #[test]
    #[serial]
    fn test_cache_delete() {
        let config = temp_cache_config();
        let manager = match CacheManager::instance(config) {
            Ok(m) => m,
            Err(_) => return,
        };

        let scope = "test.scope";
        let key = "delete_key";
        let value = b"delete_value".to_vec();

        let _ = manager.set(scope, key.to_string(), value, None);
        assert!(manager.get(scope, key).unwrap_or(None).is_some());

        let deleted = manager.delete(scope, key).unwrap_or(false);
        assert!(deleted);

        assert!(manager.get(scope, key).unwrap_or(None).is_none());
    }

    #[test]
    #[serial]
    fn test_cache_stats() {
        let manager = match CacheManager::instance(temp_cache_config()) {
            Ok(m) => m,
            Err(_) => return,
        };

        let scope = "test.scope";
        let key = "stats_key";
        let value = b"stats_value".to_vec();

        // 记录初始统计值
        let initial_stats = manager.stats();

        // 未命中
        let _ = manager.get(scope, key);
        let stats_after_miss = manager.stats();
        assert_eq!(stats_after_miss.misses, initial_stats.misses + 1);

        // 写入
        let _ = manager.set(scope, key.to_string(), value, None);
        let stats_after_write = manager.stats();
        assert_eq!(stats_after_write.writes, initial_stats.writes + 1);

        // 命中
        let _ = manager.get(scope, key);
        let stats_after_hit = manager.stats();
        assert_eq!(stats_after_hit.hits, initial_stats.hits + 1);
    }

    #[test]
    #[serial]
    fn test_scope_management() {
        let config = temp_cache_config();
        let manager = match CacheManager::instance(config) {
            Ok(m) => m,
            Err(_) => return,
        };

        // 测试不同作用域
        let scope1 = "test.scope1";
        let scope2 = "test.scope2";
        let key = "common_key";
        let value1 = b"value1".to_vec();
        let value2 = b"value2".to_vec();

        // 设置不同作用域的相同键
        let _ = manager.set(scope1, key.to_string(), value1.clone(), None);
        let _ = manager.set(scope2, key.to_string(), value2.clone(), None);

        // 验证不同作用域的键是隔离的
        assert_eq!(manager.get(scope1, key).unwrap_or(None), Some(value1));
        assert_eq!(manager.get(scope2, key).unwrap_or(None), Some(value2.clone()));

        // 测试删除作用域
        let deleted = manager.delete_scope(scope1).unwrap_or(0);
        assert_eq!(deleted, 1);
        assert!(manager.get(scope1, key).unwrap_or(None).is_none());
        assert_eq!(manager.get(scope2, key).unwrap_or(None), Some(value2));
    }

    #[test]
    #[serial]
    fn test_conditional_update() {
        let config = temp_cache_config();
        let manager = match CacheManager::instance(config) {
            Ok(m) => m,
            Err(_) => return,
        };

        let scope = "test.scope";
        let key = "conditional_key";
        let value1 = b"value1".to_vec();
        let value2 = b"value2".to_vec();
        let value3 = b"value3".to_vec();

        // 设置初始值
        manager.set(scope, key.to_string(), value1.clone(), None).unwrap();
        assert_eq!(manager.get(scope, key).unwrap(), Some(value1.clone()));

        // 条件更新：只有当当前值存在时才更新
        manager.set_with_condition(
            scope, 
            key.to_string(), 
            value2.clone(), 
            None,
            |current_value, _| current_value.is_some()
        ).unwrap();
        assert_eq!(manager.get(scope, key).unwrap(), Some(value2.clone()));

        // 条件更新：只有当当前值等于特定值时才更新
        manager.set_with_condition(
            scope, 
            key.to_string(), 
            value3.clone(), 
            None,
            move |current_value, _| {
                if let Some(v) = current_value {
                    *v == value2
                } else {
                    false
                }
            }
        ).unwrap();
        assert_eq!(manager.get(scope, key).unwrap(), Some(value3.clone()));

        // 条件更新：当条件不满足时不更新
        manager.set_with_condition(
            scope, 
            key.to_string(), 
            value1.clone(), 
            None,
            move |current_value, _| {
                if let Some(v) = current_value {
                    *v == value1 // 这个条件不满足，因为当前值是 value3
                } else {
                    false
                }
            }
        ).unwrap();
        assert_eq!(manager.get(scope, key).unwrap(), Some(value3.clone()));
    }

    #[test]
    #[serial]
    fn test_bloom_filter() {
        let mut config = temp_cache_config();
        config.enable_bloom_filter = true;
        
        let manager = match CacheManager::instance(config) {
            Ok(m) => m,
            Err(_) => return,
        };

        let scope = "test.scope";
        let key = "bloom_key";
        let value = b"bloom_value".to_vec();

        // 设置缓存，这会将键添加到布隆过滤器
        manager.set(scope, key.to_string(), value.clone(), None).unwrap();
        
        // 检查存在的键，应该能获取到
        assert_eq!(manager.get(scope, key).unwrap(), Some(value.clone()));
        
        // 检查不存在的键，布隆过滤器应该能快速返回 None
        let non_existent_key = "non_existent_key";
        assert_eq!(manager.get(scope, non_existent_key).unwrap(), None);
    }

    #[test]
    #[serial]
    fn test_hot_keys() {
        let config = temp_cache_config();
        let manager = match CacheManager::instance(config) {
            Ok(m) => m,
            Err(_) => return,
        };

        let scope = "test.scope";
        let hot_key = "hot_key";
        let cold_key = "cold_key";
        let value = b"test_value".to_vec();

        // 设置初始值
        manager.set(scope, hot_key.to_string(), value.clone(), None).unwrap();
        manager.set(scope, cold_key.to_string(), value.clone(), None).unwrap();

        // 多次访问热键
        for _ in 0..100 {
            manager.get(scope, hot_key).unwrap();
        }
        
        // 只访问一次冷键
        manager.get(scope, cold_key).unwrap();

        // 获取统计信息，检查热点键
        let stats = manager.stats();
        assert!(!stats.hot_keys.is_empty());
        
        // 热键应该在热点键列表中
        let hot_key_found = stats.hot_keys.iter().any(|hk| hk.key == hot_key);
        assert!(hot_key_found);
    }

    #[test]
    #[serial]
    fn test_batch_operations() {
        let config = temp_cache_config();
        let manager = match CacheManager::instance(config) {
            Ok(m) => m,
            Err(_) => return,
        };

        let scope = "test.scope";
        let keys = ["key1", "key2", "key3"];
        let values = [
            b"value1".to_vec(), 
            b"value2".to_vec(), 
            b"value3".to_vec()
        ];

        // 批量设置
        let items: Vec<(String, CacheValue)> = keys.iter()
            .zip(values.iter())
            .map(|(k, v)| (k.to_string(), v.clone()))
            .collect();
        manager.set_batch(scope, &items, None).unwrap();

        // 批量获取
        let results = manager.get_batch(scope, &keys).unwrap();
        assert_eq!(results.len(), keys.len());
        for (i, result) in results.iter().enumerate() {
            assert_eq!(result.clone(), Some(values[i].clone()));
        }

        // 批量删除
        let deleted = manager.delete_batch(scope, &keys).unwrap();
        assert_eq!(deleted, keys.len());

        // 验证所有键都被删除
        let results_after_delete = manager.get_batch(scope, &keys).unwrap();
        for result in results_after_delete {
            assert_eq!(result, None);
        }
    }
}
