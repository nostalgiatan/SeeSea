//! 共享内存管理系统

use crate::errors::{RamingError, RamingResult};
use crate::pool::{MemoryPoolManager, PooledMemory};
use crate::types::{MemorySegmentInfo, MemoryStats};
use chrono::Utc;
use dashmap::DashMap;
use memmap2::{Mmap, MmapOptions};
use parking_lot::RwLock;
use seesea_config::RamingMemoryConfig as MemoryConfig;
use std::fs::OpenOptions;
use std::path::Path;
use std::sync::Arc;
use tracing::info;
use uuid::Uuid;

/// 内存段结构
pub struct MemorySegment {
    /// 段ID
    id: Uuid,
    /// 段名称
    name: String,
    /// 内存数据
    data: Arc<RwLock<Vec<u8>>>,
    /// 池化内存（可选）
    _pooled_memory: Option<PooledMemory>,
    /// 内存映射（可选）
    mmap: Option<Mmap>,
    /// 创建时间
    created_at: chrono::DateTime<Utc>,
    /// 最后访问时间
    last_accessed: Arc<RwLock<chrono::DateTime<Utc>>>,
    /// 访问计数
    access_count: Arc<std::sync::atomic::AtomicU64>,
    /// 引用计数
    ref_count: Arc<std::sync::atomic::AtomicU32>,
    /// 是否只读
    read_only: bool,
    /// 配置
    _config: MemoryConfig,
}

impl MemorySegment {
    /// 创建新的内存段
    pub fn new(name: String, size: usize, config: MemoryConfig) -> RamingResult<Self> {
        if size > config.max_segment_size {
            return Err(RamingError::memory_allocation(format!(
                "段大小 {} 超过最大限制 {}",
                size, config.max_segment_size
            )));
        }

        let now = Utc::now();
        let data = vec![0u8; size];

        Ok(Self {
            id: Uuid::new_v4(),
            name,
            data: Arc::new(RwLock::new(data)),
            _pooled_memory: None,
            mmap: None,
            created_at: now,
            last_accessed: Arc::new(RwLock::new(now)),
            access_count: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            ref_count: Arc::new(std::sync::atomic::AtomicU32::new(1)),
            read_only: false,
            _config: config,
        })
    }

    /// 从内存池创建新的内存段
    pub fn from_pool(
        name: String,
        pooled_memory: PooledMemory,
        config: MemoryConfig,
    ) -> RamingResult<Self> {
        let block_size = pooled_memory
            .size()
            .ok_or_else(|| RamingError::InternalError("池化内存无效".to_string()))?;

        if block_size > config.max_segment_size {
            return Err(RamingError::memory_allocation(format!(
                "段大小 {} 超过最大限制 {}",
                block_size, config.max_segment_size
            )));
        }

        let now = Utc::now();
        let data = pooled_memory.data().unwrap_or(&[]).to_vec();

        Ok(Self {
            id: Uuid::new_v4(),
            name,
            data: Arc::new(RwLock::new(data)),
            _pooled_memory: Some(pooled_memory),
            mmap: None,
            created_at: now,
            last_accessed: Arc::new(RwLock::new(now)),
            access_count: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            ref_count: Arc::new(std::sync::atomic::AtomicU32::new(1)),
            read_only: false,
            _config: config,
        })
    }

    /// 从文件创建内存映射段
    pub fn from_file(name: String, file_path: &Path, config: MemoryConfig) -> RamingResult<Self> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(file_path)
            .map_err(|e| RamingError::InternalError(format!("无法打开文件: {}", e)))?;

        let mmap = unsafe {
            MmapOptions::new()
                .map(&file)
                .map_err(|e| RamingError::InternalError(format!("内存映射失败: {}", e)))?
        };

        let now = Utc::now();

        Ok(Self {
            id: Uuid::new_v4(),
            name,
            data: Arc::new(RwLock::new(Vec::new())), // 内存映射模式下不使用此字段
            mmap: Some(mmap),
            _pooled_memory: None,
            created_at: now,
            last_accessed: Arc::new(RwLock::new(now)),
            access_count: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            ref_count: Arc::new(std::sync::atomic::AtomicU32::new(1)),
            read_only: false,
            _config: config,
        })
    }

    /// 读取数据
    pub fn read(&self, offset: usize, length: usize) -> RamingResult<Vec<u8>> {
        self.update_access_time();
        self.access_count
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        if let Some(mmap) = &self.mmap {
            if offset + length > mmap.len() {
                return Err(RamingError::MemoryAccessViolation(format!(
                    "读取范围超出边界: offset={}, length={}, size={}",
                    offset,
                    length,
                    mmap.len()
                )));
            }
            Ok(mmap[offset..offset + length].to_vec())
        } else {
            let data = self.data.read();
            if offset + length > data.len() {
                return Err(RamingError::MemoryAccessViolation(format!(
                    "读取范围超出边界: offset={}, length={}, size={}",
                    offset,
                    length,
                    data.len()
                )));
            }
            Ok(data[offset..offset + length].to_vec())
        }
    }

    /// 写入数据
    pub fn write(&self, offset: usize, data: &[u8]) -> RamingResult<()> {
        if self.read_only {
            return Err(RamingError::PermissionError("内存段是只读的".to_string()));
        }

        self.update_access_time();
        self.access_count
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        if let Some(_mmap) = &self.mmap {
            // 内存映射写入需要转换为可变映射
            return Err(RamingError::InternalError(
                "内存映射段为只读模式，无法直接写入。请使用可变映射或复制到内存段进行修改"
                    .to_string(),
            ));
        } else {
            let mut vec_data = self.data.write();
            if offset + data.len() > vec_data.len() {
                return Err(RamingError::MemoryAccessViolation(format!(
                    "写入范围超出边界: offset={}, length={}, size={}",
                    offset,
                    data.len(),
                    vec_data.len()
                )));
            }
            vec_data[offset..offset + data.len()].copy_from_slice(data);
        }
        Ok(())
    }

    /// 锁定内存段
    pub async fn lock(&self, offset: usize, length: usize) -> RamingResult<()> {
        self.update_access_time();

        let size = if let Some(mmap) = &self.mmap {
            mmap.len()
        } else {
            self.data.read().len()
        };

        if offset + length > size {
            return Err(RamingError::MemoryAccessViolation(format!(
                "锁定范围超出边界: offset={}, length={}, size={}",
                offset, length, size
            )));
        }

        Ok(())
    }

    /// 解锁内存段
    pub async fn unlock(&self, offset: usize, length: usize) -> RamingResult<()> {
        self.update_access_time();

        let size = if let Some(mmap) = &self.mmap {
            mmap.len()
        } else {
            self.data.read().len()
        };

        if offset + length > size {
            return Err(RamingError::MemoryAccessViolation(format!(
                "解锁范围超出边界: offset={}, length={}, size={}",
                offset, length, size
            )));
        }

        Ok(())
    }

    /// 同步内存段到磁盘
    pub async fn sync(&self) -> RamingResult<()> {
        self.update_access_time();

        if let Some(_mmap) = &self.mmap {
            // 对于 memmap2，内存映射会自动同步
            // 如果需要强制同步，可以调用系统级的 sync
            tracing::debug!("内存映射段已自动同步");
            Ok(())
        } else {
            // 普通内存段无需同步
            Ok(())
        }
    }

    /// 获取段信息
    pub fn info(&self) -> MemorySegmentInfo {
        let size = if let Some(mmap) = &self.mmap {
            mmap.len()
        } else {
            self.data.read().len()
        };

        MemorySegmentInfo {
            id: self.id,
            name: self.name.clone(),
            size,
            created_at: self.created_at,
            last_accessed: *self.last_accessed.read(),
            access_count: self.access_count.load(std::sync::atomic::Ordering::Relaxed),
            read_only: self.read_only,
            ref_count: self.ref_count.load(std::sync::atomic::Ordering::Relaxed),
        }
    }

    /// 增加引用计数
    pub fn ref_inc(&self) {
        self.ref_count
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    /// 减少引用计数
    pub fn ref_dec(&self) -> u32 {
        self.ref_count
            .fetch_sub(1, std::sync::atomic::Ordering::Relaxed)
            - 1
    }

    /// 更新访问时间
    fn update_access_time(&self) {
        *self.last_accessed.write() = Utc::now();
    }

    /// 获取名称
    pub fn name(&self) -> &str {
        &self.name
    }

    /// 获取大小
    pub fn size(&self) -> usize {
        if let Some(mmap) = &self.mmap {
            mmap.len()
        } else {
            self.data.read().len()
        }
    }

    /// 检查是否有效
    pub fn is_valid(&self) -> bool {
        self.ref_count.load(std::sync::atomic::Ordering::Relaxed) > 0
    }
}

/// 共享内存管理器
pub struct SharedMemory {
    /// 内存段映射
    segments: Arc<DashMap<String, Arc<MemorySegment>>>,
    /// 内存池管理器
    pool_manager: Arc<MemoryPoolManager>,
    /// 配置
    config: MemoryConfig,
    /// 统计信息
    stats: Arc<RwLock<MemoryStats>>,
}

impl SharedMemory {
    /// 创建新的共享内存管理器
    pub fn new(config: MemoryConfig) -> RamingResult<Self> {
        let pool_manager = Arc::new(MemoryPoolManager::new(config.clone()));

        Ok(Self {
            segments: Arc::new(DashMap::new()),
            pool_manager,
            config,
            stats: Arc::new(RwLock::new(MemoryStats {
                total_memory: 0,
                active_segments: 0,
                total_segments: 0,
                cache_hit_rate: 0.0,
                avg_access_time_ms: 0.0,
                pool_usage: 0,
                total_allocated: 0,
            })),
        })
    }

    /// 创建内存段
    pub fn create_segment(&self, name: String, size: usize) -> RamingResult<Arc<MemorySegment>> {
        if self.segments.len() >= self.config.max_segments {
            return Err(RamingError::ResourceExhausted(format!(
                "内存段数量达到最大限制: {}",
                self.config.max_segments
            )));
        }

        if self.segments.contains_key(&name) {
            return Err(RamingError::segment_exists(name));
        }

        let segment = Arc::new(MemorySegment::new(name.clone(), size, self.config.clone())?);
        self.segments.insert(name, segment.clone());

        self.update_stats();
        Ok(segment)
    }

    /// 从内存池创建内存段
    pub fn create_segment_from_pool(
        &self,
        name: String,
        size: usize,
    ) -> RamingResult<Arc<MemorySegment>> {
        if self.segments.len() >= self.config.max_segments {
            return Err(RamingError::ResourceExhausted(format!(
                "内存段数量达到最大限制: {}",
                self.config.max_segments
            )));
        }

        if self.segments.contains_key(&name) {
            return Err(RamingError::segment_exists(name));
        }

        // 从内存池分配
        let pooled_memory = self.pool_manager.allocate(size)?;
        let segment = Arc::new(MemorySegment::from_pool(
            name.clone(),
            pooled_memory,
            self.config.clone(),
        )?);
        self.segments.insert(name, segment.clone());

        self.update_stats();
        Ok(segment)
    }

    /// 获取内存段（返回Option以兼容测试）
    pub fn get_segment(&self, name: &str) -> Option<Arc<MemorySegment>> {
        self.segments.get(name).map(|segment| segment.clone())
    }

    /// 获取内存段（返回Result）
    pub fn get_segment_result(&self, name: &str) -> RamingResult<Arc<MemorySegment>> {
        self.segments
            .get(name)
            .map(|segment| segment.clone())
            .ok_or_else(|| RamingError::segment_not_found(name))
    }

    /// 删除内存段
    pub fn delete_segment(&self, name: &str) -> RamingResult<()> {
        if let Some((_, segment)) = self.segments.remove(name) {
            if segment.ref_dec() == 0 {
                self.update_stats();
                Ok(())
            } else {
                // 如果还有其他引用，重新插入
                self.segments.insert(name.to_string(), segment);
                Err(RamingError::InternalError("内存段仍在被引用".to_string()))
            }
        } else {
            Err(RamingError::segment_not_found(name))
        }
    }

    /// 列出所有内存段
    pub fn list_segments(&self) -> Vec<MemorySegmentInfo> {
        self.segments
            .iter()
            .map(|entry| entry.value().info())
            .collect()
    }

    /// 获取统计信息
    pub fn stats(&self) -> MemoryStats {
        self.stats.read().clone()
    }

    /// 启动共享内存管理器
    pub fn start(&self) -> RamingResult<()> {
        info!("共享内存管理器已启动");
        Ok(())
    }

    /// 停止共享内存管理器
    pub fn stop(&self) -> RamingResult<()> {
        info!("共享内存管理器已停止");
        Ok(())
    }

    /// 健康检查
    pub fn health_check(&self) -> RamingResult<crate::manager::HealthStatus> {
        Ok(crate::manager::HealthStatus::healthy())
    }

    /// 获取统计信息
    pub fn get_stats(&self) -> MemoryStats {
        self.stats.read().clone()
    }

    /// 清理过期内存段
    pub fn cleanup(&self) -> RamingResult<usize> {
        let now = Utc::now();
        let mut removed_count = 0;
        let mut to_remove = Vec::new();

        for entry in self.segments.iter() {
            let info = entry.value().info();
            let elapsed = now.signed_duration_since(info.last_accessed).num_seconds();

            if elapsed > self.config.cleanup_interval.as_secs() as i64 {
                to_remove.push(entry.key().clone());
            }
        }

        for name in to_remove {
            if self.delete_segment(&name).is_ok() {
                removed_count += 1;
            }
        }

        self.update_stats();
        Ok(removed_count)
    }

    /// 更新统计信息
    fn update_stats(&self) {
        let mut stats = self.stats.write();
        stats.total_segments = self.segments.len();
        stats.active_segments = stats.total_segments;

        let mut total_memory = 0;
        let mut total_access_count = 0u64;

        for entry in self.segments.iter() {
            let info = entry.value().info();
            total_memory += info.size;
            total_access_count += info.access_count;
        }

        stats.total_memory = total_memory;
        stats.avg_access_time_ms = if stats.active_segments > 0 {
            total_access_count as f64 / stats.active_segments as f64
        } else {
            0.0
        };

        // 更新内存池使用情况
        let pool_stats = self.pool_manager.get_all_stats();
        stats.pool_usage = pool_stats.iter().map(|s| s.used_blocks).sum();
    }

    /// 获取内存池统计信息
    pub fn get_pool_stats(&self) -> Vec<crate::types::PoolStats> {
        self.pool_manager.get_all_stats()
    }

    /// 获取内存池信息
    pub fn get_pool_info(&self) -> Vec<crate::pool::PoolInfo> {
        self.pool_manager.get_pool_info()
    }

    /// 清理内存池
    pub fn cleanup_pools(&self) -> RamingResult<usize> {
        self.pool_manager.cleanup_all()
    }
}
