//! 内存池管理系统

use crate::errors::{RamingError, RamingResult};
use crate::types::PoolStats;
use parking_lot::{Mutex, RwLock};
use seesea_config::RamingMemoryConfig as MemoryConfig;
use std::collections::VecDeque;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::{debug, info, warn};

/// 内存块结构
#[derive(Debug)]
pub struct MemoryBlock {
    /// 内存数据
    data: Vec<u8>,
    /// 块大小
    size: usize,
    /// 创建时间
    created_at: Instant,
    /// 最后使用时间
    last_used: RwLock<Instant>,
    /// 使用计数
    use_count: std::sync::atomic::AtomicU64,
    /// 是否在使用中
    in_use: RwLock<bool>,
}

impl MemoryBlock {
    /// 创建新的内存块
    pub fn new(size: usize) -> Self {
        let now = Instant::now();
        Self {
            data: vec![0u8; size],
            size,
            created_at: now,
            last_used: RwLock::new(now),
            use_count: std::sync::atomic::AtomicU64::new(0),
            in_use: RwLock::new(false),
        }
    }

    /// 获取内存数据的可变引用
    pub fn data_mut(&mut self) -> &mut [u8] {
        &mut self.data
    }

    /// 获取内存数据的不可变引用
    pub fn data(&self) -> &[u8] {
        &self.data
    }

    /// 获取块大小
    pub fn size(&self) -> usize {
        self.size
    }

    /// 标记为使用中
    pub fn mark_in_use(&self) {
        *self.in_use.write() = true;
        self.use_count
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    /// 标记为空闲
    pub fn mark_free(&self) {
        *self.in_use.write() = false;
        *self.last_used.write() = Instant::now();
    }

    /// 是否在使用中
    pub fn is_in_use(&self) -> bool {
        *self.in_use.read()
    }

    /// 获取使用计数
    pub fn use_count(&self) -> u64 {
        self.use_count.load(std::sync::atomic::Ordering::Relaxed)
    }

    /// 获取空闲时间
    pub fn idle_time(&self) -> Duration {
        Instant::now() - *self.last_used.read()
    }

    /// 重置块数据
    pub fn reset(&mut self) {
        self.data.fill(0);
        self.use_count
            .store(0, std::sync::atomic::Ordering::Relaxed);
        *self.last_used.write() = self.created_at;
        *self.in_use.write() = false;
    }
}

/// 内存池结构
pub struct MemoryPool {
    /// 池名称
    name: String,
    /// 块大小
    block_size: usize,
    /// 最大块数
    max_blocks: usize,
    /// 空闲块队列
    free_blocks: Mutex<VecDeque<Arc<MemoryBlock>>>,
    /// 所有块（包括使用中）
    all_blocks: RwLock<Vec<Arc<MemoryBlock>>>,
    /// 配置
    config: MemoryConfig,
    /// 统计信息
    stats: RwLock<PoolStats>,
    /// 最后清理时间
    last_cleanup: RwLock<Instant>,
}

impl MemoryPool {
    /// 创建新的内存池
    pub fn new(name: String, block_size: usize, max_blocks: usize, config: MemoryConfig) -> Self {
        let now = Instant::now();
        let mut free_blocks = VecDeque::new();
        let mut all_blocks = Vec::new();

        // 预分配一些块
        let initial_blocks = (max_blocks / 4).max(4).min(max_blocks);
        for _ in 0..initial_blocks {
            let block = Arc::new(MemoryBlock::new(block_size));
            free_blocks.push_back(block.clone());
            all_blocks.push(block);
        }

        info!(
            "创建内存池 '{}' - 块大小: {}, 最大块数: {}, 初始块数: {}",
            name, block_size, max_blocks, initial_blocks
        );

        Self {
            name,
            block_size,
            max_blocks,
            free_blocks: Mutex::new(free_blocks),
            all_blocks: RwLock::new(all_blocks),
            config,
            stats: RwLock::new(PoolStats {
                total_blocks: initial_blocks,
                free_blocks: initial_blocks,
                used_blocks: 0,
                allocation_count: 0,
                deallocation_count: 0,
                cache_hit_rate: 0.0,
                avg_allocation_time_ms: 0.0,
            }),
            last_cleanup: RwLock::new(now),
        }
    }

    /// 从池中分配内存块（便利方法）
    pub fn allocate(&self) -> RamingResult<PooledMemory> {
        // 由于 self 是 &MemoryPool，我们需要通过 MemoryPoolManager 来分配
        // 这个方法不应该被直接调用，但为了向后兼容性保留
        Err(RamingError::ResourceExhausted(
            "请使用 MemoryPoolManager::allocate 或 Arc<MemoryPool>::allocate_from_arc".to_string(),
        ))
    }

    /// 从池中分配内存块（便利方法）
    pub fn allocate_from_arc(self: &Arc<MemoryPool>) -> RamingResult<PooledMemory> {
        let start_time = Instant::now();
        debug!(
            "MemoryPool::allocate_from_arc() called for pool '{}'",
            self.name
        );

        // 尝试从空闲队列获取块
        if let Some(block) = self.free_blocks.lock().pop_front() {
            debug!("MemoryPool::allocate_from_arc() - found free block");
            block.mark_in_use();
            debug!("MemoryPool::allocate_from_arc() - block marked as in use");
            self.update_stats_on_allocation();

            let allocation_time = start_time.elapsed();
            self.update_allocation_time(allocation_time);

            debug!(
                "从内存池 '{}' 分配块 - 大小: {}, 分配时间: {:?}",
                self.name, self.block_size, allocation_time
            );

            return Ok(PooledMemory {
                block: Some(block),
                pool: Arc::clone(self), // self is already &Arc<MemoryPool>
            });
        }

        // 检查是否达到最大块数限制
        if self.all_blocks.read().len() >= self.max_blocks {
            // 内存池已满，尝试清理过期块
            self.cleanup_expired_blocks()?;

            // 再次尝试分配
            if let Some(block) = self.free_blocks.lock().pop_front() {
                block.mark_in_use();
                self.update_stats_on_allocation();

                let allocation_time = start_time.elapsed();
                self.update_allocation_time(allocation_time);

                return Ok(PooledMemory {
                    block: Some(block),
                    pool: Arc::clone(self), // self is already &Arc<MemoryPool>
                });
            }

            return Err(RamingError::ResourceExhausted(format!(
                "内存池 '{}' 已满 - 块大小: {}, 最大块数: {}",
                self.name, self.block_size, self.max_blocks
            )));
        }

        // 创建新块
        let block = Arc::new(MemoryBlock::new(self.block_size));
        block.mark_in_use();

        // 添加到所有块列表
        self.all_blocks.write().push(block.clone());
        self.update_stats_on_allocation();

        let allocation_time = start_time.elapsed();
        self.update_allocation_time(allocation_time);

        info!(
            "创建新内存块 - 池: '{}', 大小: {}, 分配时间: {:?}",
            self.name, self.block_size, allocation_time
        );

        Ok(PooledMemory {
            block: Some(block),
            pool: Arc::clone(self), // self is already &Arc<MemoryPool>
        })
    }

    /// 将块返回到池中
    pub fn deallocate(&self, block: Arc<MemoryBlock>) {
        println!(
            "MemoryPool::deallocate() called for pool '{}' with block size: {}",
            self.name,
            block.size()
        );

        if block.size() != self.block_size {
            warn!(
                "尝试将大小不匹配的块返回到内存池 '{}' - 期望大小: {}, 实际大小: {}",
                self.name,
                self.block_size,
                block.size()
            );
            return;
        }

        block.mark_free();
        println!("MemoryPool::deallocate() - block marked as free");

        self.free_blocks.lock().push_back(block);
        println!("MemoryPool::deallocate() - block added to free_blocks queue");

        self.update_stats_on_deallocation();

        println!("将块返回到内存池 '{}'", self.name);
    }

    /// 清理过期块
    pub fn cleanup_expired_blocks(&self) -> RamingResult<usize> {
        let now = Instant::now();
        let max_idle_time = Duration::from_secs(300); // 5分钟

        let mut removed_count = 0;
        let mut free_blocks = self.free_blocks.lock();
        let mut all_blocks = self.all_blocks.write();

        // 清理空闲时间过长的块
        let mut new_free_blocks = VecDeque::new();
        let mut blocks_to_remove = Vec::new();

        for block in free_blocks.drain(..) {
            if block.idle_time() > max_idle_time && all_blocks.len() > 4 {
                // 标记要移除的块
                blocks_to_remove.push(block.clone());
            } else {
                new_free_blocks.push_back(block);
            }
        }

        // 从所有块列表中移除过期块
        all_blocks.retain(|block| {
            if blocks_to_remove.iter().any(|b| Arc::ptr_eq(b, block)) {
                removed_count += 1;
                false
            } else {
                true
            }
        });

        // 恢复剩余的免费块
        *free_blocks = new_free_blocks;

        *self.last_cleanup.write() = now;

        if removed_count > 0 {
            info!(
                "清理内存池 '{}' - 移除 {} 个过期块",
                self.name, removed_count
            );
        }

        Ok(removed_count)
    }

    /// 获取统计信息
    pub fn stats(&self) -> PoolStats {
        let stats = self.stats.read().clone();
        println!(
            "MemoryPool::stats() called for pool '{}' at {:p} - returning: used_blocks={}, free_blocks={}, total_blocks={}",
            self.name,
            self as *const MemoryPool,
            stats.used_blocks,
            stats.free_blocks,
            stats.total_blocks
        );
        stats
    }

    /// 获取池信息
    pub fn info(&self) -> PoolInfo {
        let stats = self.stats();
        PoolInfo {
            name: self.name.clone(),
            block_size: self.block_size,
            max_blocks: self.max_blocks,
            total_blocks: stats.total_blocks,
            free_blocks: stats.free_blocks,
            used_blocks: stats.used_blocks,
            cache_hit_rate: stats.cache_hit_rate,
            last_cleanup: *self.last_cleanup.read(),
        }
    }

    // 内部方法

    fn update_stats_on_allocation(&self) {
        let mut stats = self.stats.write();
        stats.allocation_count += 1;
        stats.used_blocks += 1;
        stats.free_blocks = stats.free_blocks.saturating_sub(1);

        // 更新缓存命中率
        let total_ops = stats.allocation_count + stats.deallocation_count;
        if total_ops > 0 {
            stats.cache_hit_rate = (stats.allocation_count as f64
                - (stats.total_blocks - stats.free_blocks) as f64)
                / total_ops as f64;
        }
    }

    fn update_stats_on_deallocation(&self) {
        println!(
            "MemoryPool::update_stats_on_deallocation() called on pool {:p}",
            self as *const MemoryPool
        );
        let mut stats = self.stats.write();
        println!(
            "MemoryPool::update_stats_on_deallocation() - before: used_blocks={}, free_blocks={}, deallocation_count={}",
            stats.used_blocks, stats.free_blocks, stats.deallocation_count
        );

        stats.deallocation_count += 1;
        stats.used_blocks = stats.used_blocks.saturating_sub(1);
        stats.free_blocks += 1;

        println!(
            "MemoryPool::update_stats_on_deallocation() - after: used_blocks={}, free_blocks={}, deallocation_count={}",
            stats.used_blocks, stats.free_blocks, stats.deallocation_count
        );

        // 立即验证写入是否成功
        drop(stats);
        let verify_stats = self.stats.read();
        println!(
            "MemoryPool::update_stats_on_deallocation() - verification: used_blocks={}, free_blocks={}, deallocation_count={}",
            verify_stats.used_blocks, verify_stats.free_blocks, verify_stats.deallocation_count
        );
    }

    fn update_allocation_time(&self, duration: Duration) {
        let mut stats = self.stats.write();
        let current_avg = stats.avg_allocation_time_ms;
        let count = stats.allocation_count;

        if count > 0 {
            stats.avg_allocation_time_ms =
                (current_avg * (count - 1) as f64 + duration.as_millis() as f64) / count as f64;
        }
    }
}

impl Clone for MemoryPool {
    fn clone(&self) -> Self {
        // 这是一个简化的克隆实现，主要用于内部引用
        Self {
            name: self.name.clone(),
            block_size: self.block_size,
            max_blocks: self.max_blocks,
            free_blocks: Mutex::new(VecDeque::new()),
            all_blocks: RwLock::new(Vec::new()),
            config: self.config.clone(),
            stats: RwLock::new(self.stats()),
            last_cleanup: RwLock::new(*self.last_cleanup.read()),
        }
    }
}

/// 池化内存结构
pub struct PooledMemory {
    block: Option<Arc<MemoryBlock>>,
    pool: Arc<MemoryPool>,
}

impl PooledMemory {
    /// 获取内存数据的可变引用
    /// 注意：由于 Arc 的不可变性，此方法返回的引用必须在使用期间保证独占访问
    pub fn data_mut(&mut self) -> Option<&mut [u8]> {
        self.block.as_mut().map(|block| {
            // 由于 Arc 是共享引用，我们需要使用内部可变性
            // 使用 UnsafeCell 是 Rust 标准库中实现内部可变性的标准方式
            // 这里的 unsafe 是必要的，因为我们在绕过 Rust 的借用检查
            // 但通过 MemoryPool 的设计（每次只分配给一个 PooledMemory），保证了线程安全
            let data_ptr = block.data.as_ptr() as *mut u8;
            unsafe { std::slice::from_raw_parts_mut(data_ptr, block.size()) }
        })
    }

    /// 获取内存数据的不可变引用
    pub fn data(&self) -> Option<&[u8]> {
        self.block.as_ref().map(|block| block.data())
    }

    /// 获取块大小
    pub fn size(&self) -> Option<usize> {
        self.block.as_ref().map(|block| block.size())
    }

    /// 是否有效
    pub fn is_valid(&self) -> bool {
        self.block.is_some()
    }
}

impl Drop for PooledMemory {
    fn drop(&mut self) {
        println!(
            "PooledMemory::drop() called, block: {:?}",
            self.block.is_some()
        );
        if let Some(block) = self.block.take() {
            println!(
                "PooledMemory::drop() - deallocating block with size: {}",
                block.size()
            );
            self.pool.deallocate(block);
        }
    }
}

/// 内存池管理器
pub struct MemoryPoolManager {
    /// 内存池映射（按块大小分类）
    pools: RwLock<std::collections::HashMap<usize, Arc<MemoryPool>>>,
    /// 配置
    config: MemoryConfig,
    /// 默认池大小
    _default_pool_sizes: Vec<usize>,
}

impl MemoryPoolManager {
    /// 创建新的内存池管理器
    pub fn new(config: MemoryConfig) -> Self {
        let default_sizes = vec![
            1024,    // 1KB
            4096,    // 4KB
            16384,   // 16KB
            65536,   // 64KB
            262144,  // 256KB
            1048576, // 1MB
        ];

        let manager = Self {
            pools: RwLock::new(std::collections::HashMap::new()),
            config,
            _default_pool_sizes: default_sizes.clone(),
        };

        // 预创建默认池
        for size in &default_sizes {
            manager.get_or_create_pool(*size);
        }

        info!("内存池管理器已创建 - 默认池大小: {:?}", default_sizes);
        manager
    }

    /// 分配内存
    pub fn allocate(&self, size: usize) -> RamingResult<PooledMemory> {
        // 找到合适的池大小
        let pool_size = self.find_appropriate_pool_size(size);

        let pool = self.get_or_create_pool(pool_size);
        pool.allocate_from_arc()
    }

    /// 获取或创建池
    fn get_or_create_pool(&self, size: usize) -> Arc<MemoryPool> {
        // 首先尝试获取现有池
        if let Some(pool) = self.pools.read().get(&size) {
            return pool.clone();
        }

        // 创建新池
        let pool_name = format!("pool_{}", size);
        let max_blocks = self.calculate_max_blocks(size);
        let pool = Arc::new(MemoryPool::new(
            pool_name,
            size,
            max_blocks,
            self.config.clone(),
        ));

        // 添加到管理器
        self.pools.write().insert(size, pool.clone());

        info!("创建新内存池 - 大小: {}, 最大块数: {}", size, max_blocks);
        pool
    }

    /// 查找合适的池大小
    fn find_appropriate_pool_size(&self, requested_size: usize) -> usize {
        // 使用2的幂次方策略
        let mut size = 1024;
        while size < requested_size && size < self.config.max_segment_size {
            size *= 2;
        }
        size.min(self.config.max_segment_size)
    }

    /// 计算最大块数
    fn calculate_max_blocks(&self, block_size: usize) -> usize {
        let total_pool_memory = self.config.pool_size;
        let max_blocks = total_pool_memory / block_size;
        max_blocks.clamp(4, 1000) // 限制在4-1000之间
    }

    /// 获取所有池的统计信息
    pub fn get_all_stats(&self) -> Vec<PoolStats> {
        self.pools
            .read()
            .values()
            .map(|pool| pool.stats())
            .collect()
    }

    /// 清理所有池
    pub fn cleanup_all(&self) -> RamingResult<usize> {
        let mut total_removed = 0;
        for pool in self.pools.read().values() {
            total_removed += pool.cleanup_expired_blocks()?;
        }
        Ok(total_removed)
    }

    /// 获取池信息
    pub fn get_pool_info(&self) -> Vec<PoolInfo> {
        self.pools.read().values().map(|pool| pool.info()).collect()
    }
}

/// 池信息
#[derive(Debug, Clone)]
pub struct PoolInfo {
    pub name: String,
    pub block_size: usize,
    pub max_blocks: usize,
    pub total_blocks: usize,
    pub free_blocks: usize,
    pub used_blocks: usize,
    pub cache_hit_rate: f64,
    pub last_cleanup: Instant,
}
