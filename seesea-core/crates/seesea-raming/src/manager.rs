//! Raming系统管理器
//!
//! 提供统一的接口来管理共享内存和事件系统

use crate::bindings::BindingManager;
use crate::errors::{RamingError, RamingResult};
use crate::events::{AsyncEventHandlerAdapter, RamingEventBus, RamingEventData, RamingEventType};
use crate::memory::SharedMemory;
use crate::types::{BindingConfig, BindingType, EventConfig, EventStats, MemoryStats};
use once_cell::sync::OnceCell;
use parking_lot::RwLock;
use seesea_config::RamingMemoryConfig as MemoryConfig;
use std::sync::Arc;
use tracing::{info, warn};

/// 全局Raming管理器实例
static GLOBAL_MANAGER: OnceCell<RamingManager> = OnceCell::new();

/// Raming系统管理器
pub struct RamingManager {
    /// 共享内存管理器
    shared_memory: Arc<SharedMemory>,
    /// 事件总线
    event_bus: Arc<RamingEventBus>,
    /// 绑定管理器
    binding_manager: Arc<BindingManager>,
    /// 内存配置
    memory_config: MemoryConfig,
    /// 事件配置
    event_config: EventConfig,
    /// 绑定配置
    binding_config: BindingConfig,
    /// 运行状态
    running: Arc<RwLock<bool>>,
    /// 统计信息
    stats: Arc<RwLock<ManagerStats>>,
}

impl RamingManager {
    /// 创建新的管理器
    pub fn new(config: ManagerConfig) -> RamingResult<Self> {
        let memory_config = config.memory_config.unwrap_or_default();
        let event_config = config.event_config.unwrap_or_default();
        let binding_config = config.binding_config.unwrap_or_default();

        // 创建共享内存管理器
        let shared_memory = Arc::new(SharedMemory::new(memory_config.clone())?);

        // 创建事件总线
        let event_bus = Arc::new(RamingEventBus::new(event_config.clone())?);

        // 创建绑定管理器
        let binding_manager = Arc::new(BindingManager::new(binding_config.clone())?);

        let mut manager = Self {
            shared_memory,
            event_bus,
            binding_manager,
            memory_config,
            event_config,
            binding_config,
            running: Arc::new(RwLock::new(false)),
            stats: Arc::new(RwLock::new(ManagerStats::default())),
        };

        // 设置事件总线的共享内存引用
        if let Some(event_bus_mut) = Arc::get_mut(&mut manager.event_bus) {
            event_bus_mut.set_shared_memory(manager.shared_memory.clone());
        }

        // 设置绑定管理器的引用
        if let Some(binding_manager_mut) = Arc::get_mut(&mut manager.binding_manager) {
            binding_manager_mut.set_shared_memory(manager.shared_memory.clone());
            binding_manager_mut.set_event_bus(manager.event_bus.clone());
        }

        Ok(manager)
    }

    /// 初始化全局管理器
    pub fn init_global(&self) -> RamingResult<()> {
        GLOBAL_MANAGER
            .set(self.clone())
            .map_err(|_| RamingError::MemoryAllocationError("全局管理器已初始化".to_string()))?;

        info!("全局Raming管理器已初始化");
        Ok(())
    }

    /// 获取全局管理器
    pub fn global() -> RamingResult<RamingManager> {
        GLOBAL_MANAGER
            .get()
            .cloned()
            .ok_or_else(|| RamingError::MemorySegmentNotFound("全局管理器未初始化".to_string()))
    }

    /// 启动管理器
    pub async fn start(&self) -> RamingResult<()> {
        if *self.running.read() {
            return Err(RamingError::EventHandlingError(
                "管理器已在运行".to_string(),
            ));
        }

        info!("正在启动Raming管理器...");

        // 启动共享内存管理器
        self.shared_memory.start()?;

        // 启动事件总线
        self.event_bus.start().await?;

        // 启动绑定管理器
        self.binding_manager.start()?;

        *self.running.write() = true;

        info!("Raming管理器已启动");

        // 发布系统启动事件
        let event = RamingEventData::new(
            RamingEventType::System,
            "RamingManager".to_string(),
            serde_json::json!({
                "action": "started",
                "timestamp": chrono::Utc::now().to_rfc3339(),
            }),
        );

        if let Err(e) = self.event_bus.publish(event).await {
            warn!("发布启动事件失败: {}", e);
        }

        Ok(())
    }

    /// 停止管理器
    pub async fn stop(&self) -> RamingResult<()> {
        if !*self.running.read() {
            return Err(RamingError::EventHandlingError("管理器未运行".to_string()));
        }

        info!("正在停止Raming管理器...");

        // 发布系统停止事件
        let event = RamingEventData::new(
            RamingEventType::System,
            "RamingManager".to_string(),
            serde_json::json!({
                "action": "stopping",
                "timestamp": chrono::Utc::now().to_rfc3339(),
            }),
        );

        if let Err(e) = self.event_bus.publish(event).await {
            warn!("发布停止事件失败: {}", e);
        }

        // 停止绑定管理器
        self.binding_manager.stop()?;

        // 停止事件总线
        self.event_bus.stop().await?;

        // 停止共享内存管理器
        self.shared_memory.stop()?;

        *self.running.write() = false;

        info!("Raming管理器已停止");
        Ok(())
    }

    /// 获取共享内存管理器
    pub fn shared_memory(&self) -> &Arc<SharedMemory> {
        &self.shared_memory
    }

    /// 获取事件总线
    pub fn event_bus(&self) -> &Arc<RamingEventBus> {
        &self.event_bus
    }

    /// 获取绑定管理器
    pub fn binding_manager(&self) -> &Arc<BindingManager> {
        &self.binding_manager
    }

    /// 获取内存统计信息
    pub fn get_memory_stats(&self) -> MemoryStats {
        self.shared_memory.get_stats()
    }

    /// 获取事件统计信息
    pub fn get_event_stats(&self) -> EventStats {
        self.event_bus.get_stats()
    }

    /// 获取管理器统计信息
    pub fn get_manager_stats(&self) -> ManagerStats {
        self.stats.read().clone()
    }

    /// 获取运行状态
    pub fn is_running(&self) -> bool {
        *self.running.read()
    }

    /// 获取内存配置
    pub fn memory_config(&self) -> &MemoryConfig {
        &self.memory_config
    }

    /// 获取事件配置
    pub fn event_config(&self) -> &EventConfig {
        &self.event_config
    }

    /// 获取绑定配置
    pub fn binding_config(&self) -> &BindingConfig {
        &self.binding_config
    }

    /// 健康检查
    pub fn health_check(&self) -> RamingResult<HealthStatus> {
        let memory_health = self.shared_memory.health_check()?;
        let event_health = self.event_bus.health_check()?;
        let binding_health = self.binding_manager.health_check()?;

        let overall_health =
            if memory_health.is_healthy && event_health.is_healthy && binding_health.is_healthy {
                HealthStatus::healthy()
            } else {
                HealthStatus::unhealthy(vec![
                    format!("内存: {}", memory_health.message),
                    format!("事件: {}", event_health.message),
                    format!("绑定: {}", binding_health.message),
                ])
            };

        Ok(overall_health)
    }

    /// 创建内存段
    pub async fn create_memory_segment(
        &self,
        name: String,
        size: usize,
        read_only: bool,
    ) -> RamingResult<()> {
        self.shared_memory.create_segment(name.clone(), size)?;

        // 发布内存创建事件
        let event = RamingEventData::new(
            RamingEventType::MemoryShare,
            "RamingManager".to_string(),
            serde_json::json!({
                "action": "segment_created",
                "segment_name": name,
                "size": size,
                "read_only": read_only,
            }),
        );

        self.event_bus.publish(event).await?;
        Ok(())
    }

    /// 删除内存段
    pub async fn delete_memory_segment(&self, name: &str) -> RamingResult<()> {
        self.shared_memory.delete_segment(name)?;

        // 发布内存删除事件
        let event = RamingEventData::new(
            RamingEventType::MemoryDelete,
            "RamingManager".to_string(),
            serde_json::json!({
                "action": "segment_deleted",
                "segment_name": name,
            }),
        );

        self.event_bus.publish(event).await?;
        Ok(())
    }

    /// 创建事件绑定
    pub async fn create_event_binding(
        &self,
        name: String,
        binding_type: BindingType,
        event_type: RamingEventType,
        handler: Arc<dyn seesea_event::AsyncEventHandler>,
    ) -> RamingResult<()> {
        self.binding_manager
            .create_binding(name.clone(), binding_type)?;

        let adapter: Arc<dyn crate::events::RamingEventListener> =
            Arc::new(AsyncEventHandlerAdapter::new(handler));
        self.event_bus
            .create_binding(name.clone(), binding_type, event_type, adapter)
            .await?;

        // 发布绑定创建事件
        let event = RamingEventData::new(
            RamingEventType::BindingCreated,
            "RamingManager".to_string(),
            serde_json::json!({
                "action": "binding_created",
                "binding_name": name,
                "binding_type": format!("{:?}", binding_type),
            }),
        );

        self.event_bus.publish(event).await?;
        Ok(())
    }

    /// 删除事件绑定
    pub async fn delete_event_binding(&self, name: &str) -> RamingResult<()> {
        self.binding_manager.delete_binding(name)?;
        self.event_bus.remove_binding(name)?;

        // 发布绑定删除事件
        let event = RamingEventData::new(
            RamingEventType::BindingDeleted,
            "RamingManager".to_string(),
            serde_json::json!({
                "action": "binding_deleted",
                "binding_name": name,
            }),
        );

        self.event_bus.publish(event).await?;
        Ok(())
    }
}

/// 管理器配置
#[derive(Debug, Clone)]
pub struct ManagerConfig {
    /// 内存配置
    pub memory_config: Option<MemoryConfig>,
    /// 事件配置
    pub event_config: Option<EventConfig>,
    /// 绑定配置
    pub binding_config: Option<BindingConfig>,
}

impl Default for ManagerConfig {
    fn default() -> Self {
        Self {
            memory_config: Some(MemoryConfig::default()),
            event_config: Some(EventConfig::default()),
            binding_config: Some(BindingConfig::default()),
        }
    }
}

/// 管理器统计信息
#[derive(Debug, Clone, Default)]
pub struct ManagerStats {
    /// 启动时间
    pub start_time: Option<chrono::DateTime<chrono::Utc>>,
    /// 运行时长（秒）
    pub uptime_seconds: u64,
    /// 内存操作次数
    pub memory_operations: u64,
    /// 事件处理次数
    pub event_operations: u64,
    /// 绑定操作次数
    pub binding_operations: u64,
    /// 错误次数
    pub error_count: u64,
}

/// 健康状态
#[derive(Debug, Clone)]
pub struct HealthStatus {
    /// 是否健康
    pub is_healthy: bool,
    /// 状态消息
    pub message: String,
    /// 详细信息
    pub details: Vec<String>,
}

impl HealthStatus {
    /// 创建健康状态
    pub fn healthy() -> Self {
        Self {
            is_healthy: true,
            message: "系统运行正常".to_string(),
            details: vec![],
        }
    }

    /// 创建不健康状态
    pub fn unhealthy(details: Vec<String>) -> Self {
        Self {
            is_healthy: false,
            message: "系统存在问题".to_string(),
            details,
        }
    }
}

// 实现Clone trait
impl Clone for RamingManager {
    fn clone(&self) -> Self {
        Self {
            shared_memory: self.shared_memory.clone(),
            event_bus: self.event_bus.clone(),
            binding_manager: self.binding_manager.clone(),
            memory_config: self.memory_config.clone(),
            event_config: self.event_config.clone(),
            binding_config: self.binding_config.clone(),
            running: self.running.clone(),
            stats: self.stats.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_manager_creation() {
        let config = ManagerConfig::default();
        let manager = RamingManager::new(config).unwrap();

        assert!(!manager.is_running());
        assert_eq!(manager.memory_config().max_segment_size, 1024 * 1024 * 100);
        assert_eq!(manager.event_config().queue_size, 10000);
    }

    #[tokio::test]
    async fn test_manager_lifecycle() {
        let config = ManagerConfig::default();
        let manager = RamingManager::new(config).unwrap();

        assert!(!manager.is_running());

        manager.start().await.unwrap();
        assert!(manager.is_running());

        manager.stop().await.unwrap();
        assert!(!manager.is_running());
    }

    #[tokio::test]
    async fn test_global_manager() {
        let config = ManagerConfig::default();
        let manager = RamingManager::new(config).unwrap();

        // 初始化全局管理器
        manager.init_global().unwrap();

        // 获取全局管理器
        let global_manager = RamingManager::global().unwrap();
        assert!(!global_manager.is_running());
    }
}
