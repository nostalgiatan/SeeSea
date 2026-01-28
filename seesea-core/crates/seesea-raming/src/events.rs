//! 事件绑定和分发系统 - 基于seesea-event集成
//!
//! 提供高性能的事件处理机制，支持内存共享和跨进程通信

use crate::errors::{RamingError, RamingResult};
use crate::memory::SharedMemory;
use crate::types::{BindingType, EventConfig, EventStats};
use dashmap::DashMap;
use parking_lot::RwLock;
use seesea_event::{
    AsyncEventBus, DataEvent, Event, EventBus, EventListener, EventType as BaseEventType,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::{debug, info, warn};
use uuid::Uuid;

/// Raming事件类型 - 扩展基础事件类型
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RamingEventType {
    /// 内存共享事件
    MemoryShare,
    /// 内存更新事件
    MemoryUpdate,
    /// 内存删除事件
    MemoryDelete,
    /// 绑定创建事件
    BindingCreated,
    /// 绑定删除事件
    BindingDeleted,
    /// 引擎注册事件
    EngineRegister,
    /// 搜索请求事件
    SearchRequest,
    /// 搜索响应事件
    SearchResponse,
    /// 系统事件
    System,
    /// 自定义事件
    Custom(String),
}

impl std::fmt::Display for RamingEventType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RamingEventType::MemoryShare => write!(f, "MemoryShare"),
            RamingEventType::MemoryUpdate => write!(f, "MemoryUpdate"),
            RamingEventType::MemoryDelete => write!(f, "MemoryDelete"),
            RamingEventType::BindingCreated => write!(f, "BindingCreated"),
            RamingEventType::BindingDeleted => write!(f, "BindingDeleted"),
            RamingEventType::EngineRegister => write!(f, "EngineRegister"),
            RamingEventType::SearchRequest => write!(f, "SearchRequest"),
            RamingEventType::SearchResponse => write!(f, "SearchResponse"),
            RamingEventType::System => write!(f, "System"),
            RamingEventType::Custom(s) => write!(f, "{}", s),
        }
    }
}

impl RamingEventType {
    /// 转换为基础事件类型
    pub fn to_base_type(&self) -> BaseEventType {
        match self {
            RamingEventType::MemoryShare => BaseEventType::Data,
            RamingEventType::MemoryUpdate => BaseEventType::Data,
            RamingEventType::MemoryDelete => BaseEventType::Data,
            RamingEventType::BindingCreated => BaseEventType::Data,
            RamingEventType::BindingDeleted => BaseEventType::Data,
            RamingEventType::EngineRegister => BaseEventType::Data,
            RamingEventType::SearchRequest => BaseEventType::Data,
            RamingEventType::SearchResponse => BaseEventType::Data,
            RamingEventType::System => BaseEventType::Data,
            RamingEventType::Custom(_) => BaseEventType::Data,
        }
    }

    /// 获取事件类型名称
    pub fn name(&self) -> String {
        match self {
            RamingEventType::MemoryShare => "memory_share".to_string(),
            RamingEventType::MemoryUpdate => "memory_update".to_string(),
            RamingEventType::MemoryDelete => "memory_delete".to_string(),
            RamingEventType::BindingCreated => "binding_created".to_string(),
            RamingEventType::BindingDeleted => "binding_deleted".to_string(),
            RamingEventType::EngineRegister => "engine_register".to_string(),
            RamingEventType::SearchRequest => "search_request".to_string(),
            RamingEventType::SearchResponse => "search_response".to_string(),
            RamingEventType::System => "system".to_string(),
            RamingEventType::Custom(s) => format!("custom_{}", s),
        }
    }
}

/// 事件优先级
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum EventPriority {
    /// 低优先级
    Low = 0,
    /// 普通优先级
    Normal = 1,
    /// 高优先级
    High = 2,
    /// 紧急优先级
    Critical = 3,
}

/// Raming事件数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RamingEventData {
    /// 事件ID
    pub id: Uuid,
    /// 事件类型
    pub event_type: RamingEventType,
    /// 事件优先级
    pub priority: EventPriority,
    /// 事件源
    pub source: String,
    /// 事件目标
    pub target: Option<String>,
    /// 事件负载
    pub payload: serde_json::Value,
    /// 时间戳
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// 重试次数
    pub retry_count: u32,
    /// 最大重试次数
    pub max_retries: u32,
}

impl RamingEventData {
    /// 创建新事件
    pub fn new(event_type: RamingEventType, source: String, payload: serde_json::Value) -> Self {
        Self {
            id: Uuid::new_v4(),
            event_type,
            priority: EventPriority::Normal,
            source,
            target: None,
            payload,
            timestamp: chrono::Utc::now(),
            retry_count: 0,
            max_retries: 3,
        }
    }

    /// 设置优先级
    pub fn with_priority(mut self, priority: EventPriority) -> Self {
        self.priority = priority;
        self
    }

    /// 设置目标
    pub fn with_target(mut self, target: String) -> Self {
        self.target = Some(target);
        self
    }

    /// 设置最大重试次数
    pub fn with_max_retries(mut self, max_retries: u32) -> Self {
        self.max_retries = max_retries;
        self
    }

    /// 转换为JSON字符串
    pub fn to_json(&self) -> RamingResult<String> {
        serde_json::to_string(self)
            .map_err(|e| RamingError::EventHandlingError(format!("事件序列化失败: {}", e)))
    }

    /// 从JSON字符串解析
    pub fn from_json(json: &str) -> RamingResult<Self> {
        serde_json::from_str(json)
            .map_err(|e| RamingError::EventHandlingError(format!("事件反序列化失败: {}", e)))
    }

    /// 转换为DataEvent
    pub fn to_data_event(&self) -> DataEvent {
        let payload_bytes = self.to_json().unwrap_or_default().into_bytes();

        DataEvent {
            event_type: self.event_type.name().into(),
            payload: payload_bytes.into(),
        }
    }

    /// 从DataEvent解析
    pub fn from_data_event(data_event: &DataEvent) -> RamingResult<Self> {
        let json_str = String::from_utf8_lossy(&data_event.payload);
        Self::from_json(&json_str)
    }
}

/// Raming事件监听器trait - 扩展基础事件监听器
#[async_trait::async_trait]
pub trait RamingEventListener: Send + Sync + 'static {
    /// 处理Raming事件
    async fn handle_raming_event(&self, event: RamingEventData) -> RamingResult<()>;

    /// 获取处理器名称
    fn name(&self) -> &str;

    /// 获取支持的绑定类型
    fn supported_binding_types(&self) -> Vec<BindingType>;

    /// 检查是否支持该事件类型
    fn supports_event_type(&self, event_type: &RamingEventType) -> bool;
}

/// 事件处理器trait - 兼容接口
#[async_trait::async_trait]
pub trait EventHandler: RamingEventListener {}

/// 为所有RamingEventListener实现EventHandler
impl<T: RamingEventListener> EventHandler for T {}

/// 适配器：将RamingEventListener转换为EventListener
pub struct RamingEventAdapter {
    inner: Arc<dyn RamingEventListener>,
}

impl RamingEventAdapter {
    pub fn new(inner: Arc<dyn RamingEventListener>) -> Self {
        Self { inner }
    }
}

#[async_trait::async_trait]
impl EventListener for RamingEventAdapter {
    async fn on_event(&self, event: &Event) -> seesea_event::RamingResult<()> {
        match event {
            Event::Data(data_event) => {
                if let Ok(raming_event) = RamingEventData::from_data_event(data_event)
                    && self.inner.supports_event_type(&raming_event.event_type)
                {
                    self.inner
                        .handle_raming_event(raming_event)
                        .await
                        .map_err(|e| seesea_event::RamingError::EventSystemError(e.to_string()))?;
                }
                Ok(())
            }
            Event::Error(_) => Ok(()),
        }
    }

    fn name(&self) -> &str {
        self.inner.name()
    }

    fn interested_events(&self) -> &[BaseEventType] {
        &[BaseEventType::Data]
    }
}

/// 适配器：将AsyncEventHandler转换为RamingEventListener
pub struct AsyncEventHandlerAdapter {
    inner: Arc<dyn seesea_event::AsyncEventHandler>,
}

impl AsyncEventHandlerAdapter {
    pub fn new(inner: Arc<dyn seesea_event::AsyncEventHandler>) -> Self {
        Self { inner }
    }
}

#[async_trait::async_trait]
impl RamingEventListener for AsyncEventHandlerAdapter {
    async fn handle_raming_event(&self, event: RamingEventData) -> RamingResult<()> {
        let payload_bytes = serde_json::to_vec(&event.payload)
            .map_err(|e| RamingError::SerializationError(format!("序列化失败: {}", e)))?;
        let data_event = seesea_event::async_events::AsyncEvent::Notification {
            event_type: seesea_event::async_events::EventType::Data,
            event_type_str: event.event_type.to_string(),
            payload: seesea_event::async_events::EventPayload::Data(payload_bytes),
        };
        self.inner
            .handle_event(&data_event)
            .await
            .map_err(|e| RamingError::EventHandlingError(e.to_string()))
    }

    fn name(&self) -> &str {
        "AsyncEventHandlerAdapter"
    }

    fn supported_binding_types(&self) -> Vec<BindingType> {
        vec![BindingType::EventListener]
    }

    fn supports_event_type(&self, _event_type: &RamingEventType) -> bool {
        true
    }
}

/// Raming事件总线 - 基于seesea-event的封装
pub struct RamingEventBus {
    /// 基础事件总线
    base_bus: Arc<EventBus>,
    /// 异步事件总线
    _async_bus: Arc<AsyncEventBus>,
    /// 事件绑定映射
    bindings: Arc<DashMap<String, Arc<RamingEventBinding>>>,
    /// 事件监听器映射
    listeners: Arc<DashMap<RamingEventType, Vec<Arc<dyn RamingEventListener>>>>,
    /// 配置
    _config: EventConfig,
    /// 运行状态
    running: Arc<RwLock<bool>>,
    /// 共享内存引用
    shared_memory: Option<Arc<SharedMemory>>,
}

impl RamingEventBus {
    /// 创建新事件总线
    pub fn new(config: EventConfig) -> RamingResult<Self> {
        let base_bus = Arc::new(EventBus::new());
        let async_bus = Arc::new(AsyncEventBus::new());

        Ok(Self {
            base_bus,
            _async_bus: async_bus,
            bindings: Arc::new(DashMap::new()),
            listeners: Arc::new(DashMap::new()),
            _config: config,
            running: Arc::new(RwLock::new(false)),
            shared_memory: None,
        })
    }

    /// 设置共享内存
    pub fn set_shared_memory(&mut self, shared_memory: Arc<SharedMemory>) {
        self.shared_memory = Some(shared_memory);
    }

    /// 启动事件总线
    pub async fn start(&self) -> RamingResult<()> {
        if *self.running.read() {
            return Err(RamingError::event_handling_error(
                "事件总线已在运行".to_string(),
            ));
        }

        *self.running.write() = true;
        info!("Raming事件总线已启动");

        // 启动基础事件总线的处理循环
        let base_bus = self.base_bus.clone();
        let running = self.running.clone();
        tokio::spawn(async move {
            while *running.read() {
                if let Err(e) = base_bus.run().await {
                    warn!("事件总线运行错误: {}", e);
                    if !*running.read() {
                        break;
                    }
                    tokio::time::sleep(Duration::from_millis(100)).await;
                }
            }
        });

        Ok(())
    }

    /// 停止事件总线
    pub async fn stop(&self) -> RamingResult<()> {
        if !*self.running.read() {
            return Err(RamingError::event_handling_error(
                "事件总线未运行".to_string(),
            ));
        }

        *self.running.write() = false;
        info!("Raming事件总线已停止");
        Ok(())
    }

    /// 发布Raming事件
    pub async fn publish(&self, event: RamingEventData) -> RamingResult<()> {
        if !*self.running.read() {
            return Err(RamingError::event_handling_error(
                "事件总线未运行".to_string(),
            ));
        }

        // 转换为基础事件并发布
        let data_event = event.to_data_event();
        let base_event = Event::Data(data_event);

        self.base_bus.publish(base_event)?;

        info!("已发布Raming事件: {:?}", event.event_type);
        Ok(())
    }

    /// 订阅Raming事件
    pub async fn subscribe(
        &self,
        event_type: RamingEventType,
        listener: Arc<dyn RamingEventListener>,
    ) -> RamingResult<()> {
        // 创建适配器并订阅基础事件
        let adapter = Arc::new(RamingEventAdapter::new(listener.clone()));

        // 订阅到基础事件总线
        self.base_bus.subscribe(adapter).await?;

        // 存储监听器引用
        let mut listeners_list = self.listeners.entry(event_type.clone()).or_default();
        listeners_list.push(listener);

        info!("已订阅Raming事件类型: {:?}", event_type);
        Ok(())
    }

    /// 取消订阅Raming事件
    pub fn unsubscribe(
        &self,
        event_type: &RamingEventType,
        listener_name: &str,
    ) -> RamingResult<()> {
        if let Some(mut listeners_list) = self.listeners.get_mut(event_type) {
            listeners_list.retain(|l| l.name() != listener_name);
            info!(
                "已取消订阅Raming事件类型: {:?}, 监听器: {}",
                event_type, listener_name
            );
        }
        Ok(())
    }

    /// 创建事件绑定（异步版本）
    pub async fn create_binding(
        &self,
        name: String,
        binding_type: BindingType,
        event_type: RamingEventType,
        listener: Arc<dyn RamingEventListener>,
    ) -> RamingResult<()> {
        if self.bindings.contains_key(&name) {
            return Err(RamingError::event_system_error(format!(
                "绑定 {} 已存在",
                name
            )));
        }

        let binding = Arc::new(RamingEventBinding::new(
            name.clone(),
            binding_type,
            event_type.clone(),
            listener.clone(),
            EventConfig::default(),
        ));
        self.bindings.insert(name.clone(), binding);

        // 订阅监听器到事件总线
        self.subscribe(event_type, listener).await?;

        info!("已创建Raming事件绑定: {}, 类型: {:?}", name, binding_type);
        Ok(())
    }

    /// 删除事件绑定
    pub fn remove_binding(&self, name: &str) -> RamingResult<()> {
        if self.bindings.remove(name).is_some() {
            info!("已删除Raming事件绑定: {}", name);
            Ok(())
        } else {
            Err(RamingError::event_system_error(format!(
                "绑定 {} 不存在",
                name
            )))
        }
    }

    /// 获取绑定列表
    pub fn get_bindings(&self) -> Vec<String> {
        self.bindings
            .iter()
            .map(|entry| entry.key().clone())
            .collect()
    }

    /// 检查是否运行中
    pub fn is_running(&self) -> bool {
        *self.running.read()
    }

    /// 健康检查
    pub fn health_check(&self) -> RamingResult<crate::manager::HealthStatus> {
        if *self.running.read() {
            Ok(crate::manager::HealthStatus::healthy())
        } else {
            Ok(crate::manager::HealthStatus::unhealthy(vec![
                "事件总线未运行".to_string(),
            ]))
        }
    }

    /// 获取统计信息
    pub fn get_stats(&self) -> EventStats {
        EventStats {
            total_events: 0,
            active_listeners: self.listeners.len(),
            processing_rate: 0.0,
            avg_processing_time_ms: 0.0,
            queue_length: 0,
            dropped_events: 0,
            published_events: 0,
            successful_events: 0,
            failed_events: 0,
            timed_out_events: 0,
            total_processing_time: std::time::Duration::from_secs(0),
        }
    }
}

/// Raming事件绑定
pub struct RamingEventBinding {
    /// 绑定ID
    pub id: Uuid,
    /// 绑定名称
    pub name: String,
    /// 绑定类型
    pub binding_type: BindingType,
    /// 事件类型
    pub event_type: RamingEventType,
    /// 事件监听器
    pub listener: Arc<dyn RamingEventListener>,
    /// 绑定配置
    pub config: EventConfig,
    /// 是否启用
    pub enabled: Arc<RwLock<bool>>,
    /// 统计信息
    pub stats: Arc<RwLock<BindingStats>>,
    /// 创建时间
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl RamingEventBinding {
    /// 创建新绑定
    pub fn new(
        name: String,
        binding_type: BindingType,
        event_type: RamingEventType,
        listener: Arc<dyn RamingEventListener>,
        config: EventConfig,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            name,
            binding_type,
            event_type,
            listener,
            config,
            enabled: Arc::new(RwLock::new(true)),
            stats: Arc::new(RwLock::new(BindingStats::default())),
            created_at: chrono::Utc::now(),
        }
    }

    /// 启用绑定
    pub fn enable(&self) {
        *self.enabled.write() = true;
    }

    /// 禁用绑定
    pub fn disable(&self) {
        *self.enabled.write() = false;
    }

    /// 检查是否启用
    pub fn is_enabled(&self) -> bool {
        *self.enabled.read()
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
        stats.last_executed = Some(Instant::now());
    }
}

/// 绑定统计信息
#[derive(Debug, Default)]
pub struct BindingStats {
    /// 成功次数
    pub success_count: u64,
    /// 错误次数
    pub error_count: u64,
    /// 总执行次数
    pub total_count: u64,
    /// 总执行时间
    pub total_duration: Duration,
    /// 最后执行时间
    pub last_executed: Option<Instant>,
}

impl BindingStats {
    /// 获取平均执行时间
    pub fn average_duration(&self) -> Duration {
        if self.total_count > 0 {
            self.total_duration / self.total_count as u32
        } else {
            Duration::from_secs(0)
        }
    }

    /// 获取成功率
    pub fn success_rate(&self) -> f64 {
        if self.total_count > 0 {
            self.success_count as f64 / self.total_count as f64
        } else {
            0.0
        }
    }
}

/// 内存共享事件监听器
pub struct MemoryShareListener {
    _shared_memory: Arc<SharedMemory>,
}

impl MemoryShareListener {
    /// 创建新监听器
    pub fn new(shared_memory: Arc<SharedMemory>) -> Self {
        Self {
            _shared_memory: shared_memory,
        }
    }
}

#[async_trait::async_trait]
impl RamingEventListener for MemoryShareListener {
    async fn handle_raming_event(&self, event: RamingEventData) -> RamingResult<()> {
        debug!("处理内存共享事件: {:?}", event);

        // 根据事件负载处理内存共享
        if let Some(segment_name) = event.payload.get("segment_name").and_then(|v| v.as_str()) {
            info!("共享内存段: {}", segment_name);
            // 这里可以添加具体的内存共享逻辑
        }

        Ok(())
    }

    fn name(&self) -> &str {
        "MemoryShareListener"
    }

    fn supported_binding_types(&self) -> Vec<BindingType> {
        vec![BindingType::MemoryShare, BindingType::Bidirectional]
    }

    fn supports_event_type(&self, event_type: &RamingEventType) -> bool {
        matches!(event_type, RamingEventType::MemoryShare)
    }
}

/// 通用事件监听器
pub struct GenericEventListener {
    name: String,
}

impl GenericEventListener {
    /// 创建新监听器
    pub fn new(name: String) -> Self {
        Self { name }
    }
}

#[async_trait::async_trait]
impl RamingEventListener for GenericEventListener {
    async fn handle_raming_event(&self, event: RamingEventData) -> RamingResult<()> {
        info!(
            "通用事件监听器 {} 收到事件: {:?}",
            self.name, event.event_type
        );

        // 这里可以添加具体的事件监听逻辑
        debug!("事件详情: {:?}", event);

        Ok(())
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn supported_binding_types(&self) -> Vec<BindingType> {
        vec![BindingType::EventListener, BindingType::Subscriber]
    }

    fn supports_event_type(&self, _event_type: &RamingEventType) -> bool {
        true // 监听所有事件类型
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_raming_event_data_creation() {
        let event = RamingEventData::new(
            RamingEventType::MemoryShare,
            "test_source".to_string(),
            serde_json::json!({"key": "value"}),
        );

        assert_eq!(event.event_type, RamingEventType::MemoryShare);
        assert_eq!(event.source, "test_source");
        assert_eq!(event.priority, EventPriority::Normal);
        assert_eq!(event.retry_count, 0);
    }

    #[tokio::test]
    async fn test_raming_event_data_serialization() {
        let event = RamingEventData::new(
            RamingEventType::MemoryShare,
            "test_source".to_string(),
            serde_json::json!({"key": "value"}),
        )
        .with_priority(EventPriority::High)
        .with_target("test_target".to_string());

        let json = event.to_json().unwrap();
        let deserialized = RamingEventData::from_json(&json).unwrap();

        assert_eq!(deserialized.event_type, event.event_type);
        assert_eq!(deserialized.source, event.source);
        assert_eq!(deserialized.priority, event.priority);
        assert_eq!(deserialized.target, event.target);
    }

    #[tokio::test]
    async fn test_raming_event_binding() {
        let listener = Arc::new(GenericEventListener::new("test_listener".to_string()));
        let binding = RamingEventBinding::new(
            "test_binding".to_string(),
            BindingType::EventListener,
            RamingEventType::MemoryShare,
            listener,
            EventConfig::default(),
        );

        assert_eq!(binding.name, "test_binding");
        assert_eq!(binding.binding_type, BindingType::EventListener);
        assert!(binding.is_enabled());
    }

    #[tokio::test]
    async fn test_raming_event_bus_lifecycle() {
        let config = EventConfig::default();
        let event_bus = RamingEventBus::new(config).unwrap();
        assert!(!event_bus.is_running());

        event_bus.start().await.unwrap();
        assert!(event_bus.is_running());

        event_bus.stop().await.unwrap();
        assert!(!event_bus.is_running());
    }
}
