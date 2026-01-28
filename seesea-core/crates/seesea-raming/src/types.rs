//! Raming系统类型定义

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub use seesea_config::raming::{
    BindingConfig, BindingStats, BindingType, EventConfig, EventPriority, EventStats, MemoryAccess,
    MemoryConfig, MemoryStats, PoolStats,
};

/// 内存段信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemorySegmentInfo {
    /// 段ID
    pub id: Uuid,
    /// 段名称
    pub name: String,
    /// 段大小
    pub size: usize,
    /// 创建时间
    pub created_at: DateTime<Utc>,
    /// 最后访问时间
    pub last_accessed: DateTime<Utc>,
    /// 访问次数
    pub access_count: u64,
    /// 是否只读
    pub read_only: bool,
    /// 引用计数
    pub ref_count: u32,
}

/// 事件信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventInfo {
    /// 事件ID
    pub id: Uuid,
    /// 事件名称
    pub name: String,
    /// 事件类型
    pub event_type: String,
    /// 创建时间
    pub created_at: DateTime<Utc>,
    /// 最后触发时间
    pub last_triggered: Option<DateTime<Utc>>,
    /// 触发次数
    pub trigger_count: u64,
    /// 监听器数量
    pub listener_count: usize,
}

/// 内存段信息
#[derive(Debug, Clone)]
pub struct SharedData<T> {
    /// 数据ID
    pub id: Uuid,
    /// 数据版本
    pub version: u64,
    /// 创建时间
    pub created_at: DateTime<Utc>,
    /// 最后修改时间
    pub modified_at: DateTime<Utc>,
    /// 实际数据
    pub data: T,
    /// 元数据
    pub metadata: std::collections::HashMap<String, String>,
}

impl<T: Serialize + for<'de> Deserialize<'de>> SharedData<T> {
    /// 创建新的共享数据
    pub fn new(data: T) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            version: 1,
            created_at: now,
            modified_at: now,
            data,
            metadata: std::collections::HashMap::new(),
        }
    }

    /// 更新数据
    pub fn update(&mut self, new_data: T) {
        self.data = new_data;
        self.version += 1;
        self.modified_at = Utc::now();
    }

    /// 添加元数据
    pub fn add_metadata(&mut self, key: String, value: String) {
        self.metadata.insert(key, value);
    }
}
