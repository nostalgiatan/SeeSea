//! 绑定管理器 - 管理内存和事件的绑定关系
//!
//! 提供统一的接口来创建、管理和删除内存段与事件之间的绑定关系

use crate::errors::{RamingError, RamingResult};
use crate::events::{RamingEventBus, RamingEventListener, RamingEventType};
use crate::memory::{MemorySegment, SharedMemory};
use crate::types::{BindingConfig, BindingStats, BindingType};
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tracing::{info, warn};
use uuid::Uuid;

/// 绑定状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BindingStatus {
    /// 已创建
    Created,
    /// 未激活
    Inactive,
    /// 已激活
    Active,
    /// 已暂停
    Paused,
    /// 已删除
    Deleted,
    /// 错误状态
    Error,
}

/// 绑定配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BindingDefinition {
    /// 绑定ID
    pub id: Uuid,
    /// 绑定名称
    pub name: String,
    /// 绑定类型
    pub binding_type: BindingType,
    /// 内存段名称（如果适用）
    pub memory_segment: Option<String>,
    /// 事件类型（如果适用）
    pub event_type: Option<RamingEventType>,
    /// 处理器配置
    pub handler_config: serde_json::Value,
    /// 是否启用
    pub enabled: bool,
    /// 自动激活
    pub auto_activate: bool,
    /// 重试配置
    pub retry_config: RetryConfig,
    /// 超时配置
    pub timeout_config: TimeoutConfig,
    /// 元数据
    pub metadata: serde_json::Value,
    /// 创建时间
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// 更新时间
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// 重试配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryConfig {
    /// 最大重试次数
    pub max_retries: u32,
    /// 重试间隔（毫秒）
    pub retry_interval_ms: u64,
    /// 指数退避
    pub exponential_backoff: bool,
    /// 最大重试间隔（毫秒）
    pub max_retry_interval_ms: u64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            retry_interval_ms: 1000,
            exponential_backoff: true,
            max_retry_interval_ms: 60000,
        }
    }
}

/// 超时配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeoutConfig {
    /// 连接超时（毫秒）
    pub connect_timeout_ms: u64,
    /// 读取超时（毫秒）
    pub read_timeout_ms: u64,
    /// 写入超时（毫秒）
    pub write_timeout_ms: u64,
    /// 总超时（毫秒）
    pub total_timeout_ms: u64,
}

impl Default for TimeoutConfig {
    fn default() -> Self {
        Self {
            connect_timeout_ms: 5000,
            read_timeout_ms: 10000,
            write_timeout_ms: 5000,
            total_timeout_ms: 30000,
        }
    }
}

/// 绑定实例
pub struct Binding {
    /// 绑定定义
    pub definition: Arc<RwLock<BindingDefinition>>,
    /// 绑定状态
    pub status: Arc<RwLock<BindingStatus>>,
    /// 统计信息
    pub stats: Arc<RwLock<BindingStats>>,
    /// 最后执行时间
    pub last_executed: Arc<RwLock<Option<DateTime<Utc>>>>,
    /// 事件监听器（如果适用）
    pub event_listener: Option<Arc<dyn RamingEventListener>>,
    /// 内存段引用（如果适用）
    pub memory_segment: Option<Arc<MemorySegment>>,
    /// 绑定名称（用于测试）
    pub name: String,
    /// 绑定类型（用于测试）
    pub binding_type: BindingType,
    /// 事件类型（用于测试）
    pub event_type: Option<RamingEventType>,
}

impl Binding {
    /// 创建新绑定
    pub fn new(definition: BindingDefinition) -> Self {
        let name = definition.name.clone();
        let binding_type = definition.binding_type;
        let event_type = definition.event_type.clone();

        Self {
            definition: Arc::new(RwLock::new(definition)),
            status: Arc::new(RwLock::new(BindingStatus::Created)),
            stats: Arc::new(RwLock::new(BindingStats::default())),
            last_executed: Arc::new(RwLock::new(None)),
            event_listener: None,
            memory_segment: None,
            name,
            binding_type,
            event_type,
        }
    }

    /// 激活绑定
    pub fn activate(&self) -> RamingResult<()> {
        *self.status.write() = BindingStatus::Active;
        info!("绑定已激活: {}", self.definition.read().name);
        Ok(())
    }

    /// 暂停绑定
    pub fn pause(&self) -> RamingResult<()> {
        *self.status.write() = BindingStatus::Paused;
        info!("绑定已暂停: {}", self.definition.read().name);
        Ok(())
    }

    /// 删除绑定
    pub fn delete(&self) -> RamingResult<()> {
        *self.status.write() = BindingStatus::Deleted;
        info!("绑定已删除: {}", self.definition.read().name);
        Ok(())
    }

    /// 启用绑定
    pub fn enable(&self) {
        *self.status.write() = BindingStatus::Active;
    }

    /// 禁用绑定
    pub fn disable(&self) {
        *self.status.write() = BindingStatus::Inactive;
    }

    /// 检查是否启用
    pub fn is_enabled(&self) -> bool {
        self.definition.read().enabled && *self.status.read() == BindingStatus::Active
    }

    /// 更新统计信息
    pub fn update_stats(&self, success: bool, duration: Duration) {
        let mut stats = self.stats.write();
        if success {
            stats.success_count += 1;
        } else {
            stats.error_count += 1;
        }
        stats.total_count += 1;
        stats.total_duration += duration;
        *self.last_executed.write() = Some(Utc::now());
    }

    /// 获取执行状态
    pub fn get_execution_status(&self) -> ExecutionStatus {
        let stats = self.stats.read();
        let last_executed = *self.last_executed.read();

        ExecutionStatus {
            total_executions: stats.total_count,
            successful_executions: stats.success_count,
            failed_executions: stats.error_count,
            success_rate: if stats.total_count > 0 {
                stats.success_count as f64 / stats.total_count as f64
            } else {
                0.0
            },
            average_duration_ms: if stats.total_count > 0 {
                stats.total_duration.as_millis() as f64 / stats.total_count as f64
            } else {
                0.0
            },
            last_executed,
            current_status: *self.status.read(),
        }
    }
}

/// 执行状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionStatus {
    /// 总执行次数
    pub total_executions: u64,
    /// 成功执行次数
    pub successful_executions: u64,
    /// 失败执行次数
    pub failed_executions: u64,
    /// 成功率
    pub success_rate: f64,
    /// 平均执行时间（毫秒）
    pub average_duration_ms: f64,
    /// 最后执行时间
    pub last_executed: Option<DateTime<Utc>>,
    /// 当前状态
    pub current_status: BindingStatus,
}

/// 绑定管理器
pub struct BindingManager {
    /// 绑定配置
    _config: BindingConfig,
    /// 绑定映射
    bindings: Arc<DashMap<String, Arc<Binding>>>,
    /// 绑定定义映射
    definitions: Arc<DashMap<Uuid, BindingDefinition>>,
    /// 共享内存引用
    shared_memory: Option<Arc<SharedMemory>>,
    /// 事件总线引用
    event_bus: Option<Arc<RamingEventBus>>,
    /// 运行状态
    running: Arc<RwLock<bool>>,
    /// 统计信息
    stats: Arc<RwLock<ManagerStats>>,
}

/// 管理器统计信息
#[derive(Debug, Clone, Default)]
pub struct ManagerStats {
    /// 总绑定数量
    pub total_bindings: usize,
    /// 活跃绑定数量
    pub active_bindings: usize,
    /// 创建操作次数
    pub create_operations: u64,
    /// 删除操作次数
    pub delete_operations: u64,
    /// 激活操作次数
    pub activate_operations: u64,
    /// 暂停操作次数
    pub pause_operations: u64,
    /// 错误次数
    pub error_count: u64,
}

impl BindingManager {
    /// 创建新绑定管理器
    pub fn new(config: BindingConfig) -> RamingResult<Self> {
        Ok(Self {
            _config: config,
            bindings: Arc::new(DashMap::new()),
            definitions: Arc::new(DashMap::new()),
            shared_memory: None,
            event_bus: None,
            running: Arc::new(RwLock::new(false)),
            stats: Arc::new(RwLock::new(ManagerStats::default())),
        })
    }

    /// 设置共享内存引用
    pub fn set_shared_memory(&mut self, shared_memory: Arc<SharedMemory>) {
        self.shared_memory = Some(shared_memory);
    }

    /// 设置事件总线引用
    pub fn set_event_bus(&mut self, event_bus: Arc<RamingEventBus>) {
        self.event_bus = Some(event_bus);
    }

    /// 启动绑定管理器
    pub fn start(&self) -> RamingResult<()> {
        if *self.running.read() {
            return Err(RamingError::EventHandlingError(
                "绑定管理器已在运行".to_string(),
            ));
        }

        *self.running.write() = true;
        info!("绑定管理器已启动");
        Ok(())
    }

    /// 停止绑定管理器
    pub fn stop(&self) -> RamingResult<()> {
        if !*self.running.read() {
            return Err(RamingError::EventHandlingError(
                "绑定管理器未运行".to_string(),
            ));
        }

        // 暂停所有绑定
        for binding in self.bindings.iter() {
            if let Err(e) = binding.value().pause() {
                warn!("暂停绑定失败 {}: {}", binding.key(), e);
            }
        }

        *self.running.write() = false;
        info!("绑定管理器已停止");
        Ok(())
    }

    /// 创建绑定
    pub fn create_binding(
        &self,
        name: String,
        binding_type: BindingType,
    ) -> RamingResult<Arc<Binding>> {
        if !*self.running.read() {
            return Err(RamingError::EventHandlingError(
                "绑定管理器未运行".to_string(),
            ));
        }

        // 检查名称是否已存在
        if self.bindings.contains_key(&name) {
            return Err(RamingError::BindingError(format!(
                "绑定名称已存在: {}",
                name
            )));
        }

        // 创建绑定定义
        let definition = BindingDefinition {
            id: Uuid::new_v4(),
            name: name.clone(),
            binding_type,
            memory_segment: None,
            event_type: None,
            handler_config: serde_json::json!({}),
            enabled: true,
            auto_activate: true,
            retry_config: RetryConfig::default(),
            timeout_config: TimeoutConfig::default(),
            metadata: serde_json::json!({}),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        // 创建绑定实例
        let binding = Arc::new(Binding::new(definition.clone()));

        // 存储绑定定义
        self.definitions.insert(definition.id, definition);

        // 存储绑定实例
        self.bindings.insert(name.clone(), binding.clone());

        // 更新统计信息
        let mut stats = self.stats.write();
        stats.total_bindings = self.bindings.len();
        stats.create_operations += 1;

        info!("已创建绑定: {}, 类型: {:?}", name, binding_type);

        // 自动激活
        let should_activate = {
            let definition = binding.definition.read();
            definition.auto_activate
        }; // 读锁在此处自动释放

        if should_activate {
            binding.activate()?;
        }

        Ok(binding)
    }

    /// 删除绑定
    pub fn delete_binding(&self, name: &str) -> RamingResult<()> {
        if !*self.running.read() {
            return Err(RamingError::EventHandlingError(
                "绑定管理器未运行".to_string(),
            ));
        }

        // 获取绑定
        let binding = self
            .bindings
            .get(name)
            .ok_or_else(|| RamingError::BindingError(format!("绑定不存在: {}", name)))?
            .clone();

        // 删除绑定
        binding.delete()?;
        self.bindings.remove(name);

        // 更新统计信息
        let mut stats = self.stats.write();
        stats.total_bindings = self.bindings.len();
        stats.delete_operations += 1;

        info!("已删除绑定: {}", name);
        Ok(())
    }

    /// 激活绑定
    pub fn activate_binding(&self, name: &str) -> RamingResult<()> {
        let binding = self.get_binding(name)?;
        binding.activate()?;

        // 更新统计信息
        let mut stats = self.stats.write();
        stats.activate_operations += 1;

        Ok(())
    }

    /// 暂停绑定
    pub fn pause_binding(&self, name: &str) -> RamingResult<()> {
        let binding = self.get_binding(name)?;
        binding.pause()?;

        // 更新统计信息
        let mut stats = self.stats.write();
        stats.pause_operations += 1;

        Ok(())
    }

    /// 获取绑定
    pub fn get_binding(&self, name: &str) -> RamingResult<Arc<Binding>> {
        self.bindings
            .get(name)
            .map(|entry| entry.clone())
            .ok_or_else(|| RamingError::BindingError(format!("绑定不存在: {}", name)))
    }

    /// 获取所有绑定
    pub fn get_all_bindings(&self) -> Vec<Arc<Binding>> {
        self.bindings.iter().map(|entry| entry.clone()).collect()
    }

    /// 获取绑定状态
    pub fn get_binding_status(&self, name: &str) -> RamingResult<BindingStatus> {
        let binding = self.get_binding(name)?;
        let status = *binding.status.read();
        Ok(status)
    }

    /// 获取绑定统计信息
    pub fn get_binding_stats(&self, name: &str) -> RamingResult<BindingStats> {
        let binding = self.get_binding(name)?;
        let stats = binding.stats.read().clone();
        Ok(stats)
    }

    /// 获取执行状态
    pub fn get_execution_status(&self, name: &str) -> RamingResult<ExecutionStatus> {
        let binding = self.get_binding(name)?;
        Ok(binding.get_execution_status())
    }

    /// 获取管理器统计信息
    pub fn get_manager_stats(&self) -> ManagerStats {
        self.stats.read().clone()
    }

    /// 检查是否运行中
    pub fn is_running(&self) -> bool {
        *self.running.read()
    }

    /// 健康检查
    pub fn health_check(&self) -> RamingResult<crate::manager::HealthStatus> {
        if *self.running.read() {
            let stats = self.stats.read();
            let active_bindings = self
                .bindings
                .iter()
                .filter(|entry| entry.value().is_enabled())
                .count();

            Ok(crate::manager::HealthStatus {
                is_healthy: true,
                message: format!(
                    "绑定管理器运行正常，总绑定数: {}，活跃绑定数: {}",
                    stats.total_bindings, active_bindings
                ),
                details: vec![],
            })
        } else {
            Ok(crate::manager::HealthStatus::unhealthy(vec![
                "绑定管理器未运行".to_string(),
            ]))
        }
    }

    /// 更新绑定配置
    pub fn update_binding_config(&self, name: &str, config: serde_json::Value) -> RamingResult<()> {
        let binding = self.get_binding(name)?;

        // 这里可以添加配置验证逻辑
        let mut definition = binding.definition.write();
        definition.handler_config = config;
        definition.updated_at = chrono::Utc::now();

        info!("已更新绑定配置: {}", name);
        Ok(())
    }

    /// 获取绑定（返回Option以兼容测试）
    pub fn get_binding_option(&self, name: &str) -> Option<Arc<Binding>> {
        self.bindings.get(name).map(|entry| entry.clone())
    }

    /// 获取所有绑定名称（返回HashMap以兼容测试）
    pub fn get_bindings(&self) -> std::collections::HashMap<String, Arc<Binding>> {
        self.bindings
            .iter()
            .map(|entry| (entry.key().clone(), entry.value().clone()))
            .collect()
    }

    /// 异步创建绑定
    pub async fn create_binding_async(
        &self,
        name: String,
        binding_type: BindingType,
        event_type: RamingEventType,
        _listener: Arc<dyn RamingEventListener>,
    ) -> RamingResult<Arc<Binding>> {
        if !*self.running.read() {
            return Err(RamingError::EventHandlingError(
                "绑定管理器未运行".to_string(),
            ));
        }

        if self.bindings.contains_key(&name) {
            return Err(RamingError::BindingError(format!(
                "绑定名称已存在: {}",
                name
            )));
        }

        let definition = BindingDefinition {
            id: Uuid::new_v4(),
            name: name.clone(),
            binding_type,
            memory_segment: None,
            event_type: Some(event_type),
            handler_config: serde_json::json!({}),
            enabled: true,
            auto_activate: false,
            retry_config: RetryConfig::default(),
            timeout_config: TimeoutConfig::default(),
            metadata: serde_json::json!({}),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        let binding = Arc::new(Binding::new(definition.clone()));

        self.definitions.insert(definition.id, definition);
        self.bindings.insert(name.clone(), binding.clone());

        let mut stats = self.stats.write();
        stats.total_bindings = self.bindings.len();
        stats.create_operations += 1;

        info!("已创建绑定: {}, 类型: {:?}", name, binding_type);

        Ok(binding)
    }

    /// 异步删除绑定
    pub async fn delete_binding_async(&self, name: &str) -> RamingResult<()> {
        if !*self.running.read() {
            return Err(RamingError::EventHandlingError(
                "绑定管理器未运行".to_string(),
            ));
        }

        let binding = self
            .bindings
            .get(name)
            .ok_or_else(|| RamingError::BindingError(format!("绑定不存在: {}", name)))?
            .clone();

        binding.delete()?;
        self.bindings.remove(name);

        let mut stats = self.stats.write();
        stats.total_bindings = self.bindings.len();
        stats.delete_operations += 1;

        info!("已删除绑定: {}", name);
        Ok(())
    }

    /// 异步处理事件
    pub async fn process_event(
        &self,
        event_data: &crate::events::RamingEventData,
    ) -> RamingResult<()> {
        if !*self.running.read() {
            return Err(RamingError::EventHandlingError(
                "绑定管理器未运行".to_string(),
            ));
        }

        for binding in self.bindings.iter() {
            if let Some(listener) = &binding.value().event_listener
                && let Err(e) = listener.handle_raming_event(event_data.clone()).await
            {
                warn!("事件处理失败 {}: {}", binding.key(), e);
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_binding_creation() {
        let config = BindingConfig::default();
        let manager = BindingManager::new(config).unwrap();

        manager.start().unwrap();

        let binding = manager
            .create_binding("test_binding".to_string(), BindingType::EventListener)
            .unwrap();

        let definition = binding.definition.read();
        assert_eq!(definition.name, "test_binding");
        assert_eq!(definition.binding_type, BindingType::EventListener);
        assert!(binding.is_enabled());

        manager.stop().unwrap();
    }

    #[test]
    fn test_binding_lifecycle() {
        let config = BindingConfig::default();
        let manager = BindingManager::new(config).unwrap();

        manager.start().unwrap();

        let binding_name = "lifecycle_test";
        manager
            .create_binding(binding_name.to_string(), BindingType::EventListener)
            .unwrap();

        // 测试暂停
        manager.pause_binding(binding_name).unwrap();
        assert_eq!(
            manager.get_binding_status(binding_name).unwrap(),
            BindingStatus::Paused
        );

        // 测试激活
        manager.activate_binding(binding_name).unwrap();
        assert_eq!(
            manager.get_binding_status(binding_name).unwrap(),
            BindingStatus::Active
        );

        // 测试删除
        manager.delete_binding(binding_name).unwrap();
        assert!(manager.get_binding(binding_name).is_err());

        manager.stop().unwrap();
    }

    #[test]
    fn test_binding_stats() {
        let config = BindingConfig::default();
        let manager = BindingManager::new(config).unwrap();

        manager.start().unwrap();

        let binding_name = "stats_test";
        let binding = manager
            .create_binding(binding_name.to_string(), BindingType::EventListener)
            .unwrap();

        // 模拟执行统计
        binding.update_stats(true, Duration::from_millis(100));
        binding.update_stats(false, Duration::from_millis(200));
        binding.update_stats(true, Duration::from_millis(150));

        let stats = manager.get_binding_stats(binding_name).unwrap();
        assert_eq!(stats.total_count, 3);
        assert_eq!(stats.success_count, 2);
        assert_eq!(stats.error_count, 1);

        let execution_status = manager.get_execution_status(binding_name).unwrap();
        assert_eq!(execution_status.total_executions, 3);
        assert_eq!(execution_status.successful_executions, 2);
        assert_eq!(execution_status.failed_executions, 1);
        assert_eq!(execution_status.success_rate, 2.0 / 3.0);

        manager.stop().unwrap();
    }

    #[test]
    fn test_manager_stats() {
        let config = BindingConfig::default();
        let manager = BindingManager::new(config).unwrap();

        manager.start().unwrap();

        // 创建多个绑定
        for i in 0..5 {
            manager
                .create_binding(format!("binding_{}", i), BindingType::EventListener)
                .unwrap();
        }

        let stats = manager.get_manager_stats();
        assert_eq!(stats.total_bindings, 5);
        assert_eq!(stats.create_operations, 5);

        // 删除一个绑定
        manager.delete_binding("binding_0").unwrap();

        let stats = manager.get_manager_stats();
        assert_eq!(stats.total_bindings, 4);
        assert_eq!(stats.delete_operations, 1);

        manager.stop().unwrap();
    }
}
