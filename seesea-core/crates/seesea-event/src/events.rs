//! 通用事件系统
//! 提供基础的事件类型和监听器机制

use crate::{RamingError, RamingResult};
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc};

/// 类型别名：字符串事件处理函数类型
type StringEventHandlerFn = Arc<dyn Fn(&str, &str) + Send + Sync>;

/// 基础事件类型
#[derive(Debug, Clone)]
pub enum Event {
    /// 通用数据事件
    Data(DataEvent),
    /// 错误事件
    Error(ErrorEvent),
}

/// 数据事件 - 通用数据载体
#[derive(Debug, Clone)]
pub struct DataEvent {
    pub event_type: Arc<str>,
    pub payload: Arc<[u8]>,
}

/// 错误事件
#[derive(Debug, Clone)]
pub struct ErrorEvent {
    pub error_type: Arc<str>,
    pub message: Arc<str>,
}

/// 事件监听器trait
#[async_trait]
pub trait EventListener: Send + Sync {
    /// 处理事件
    async fn on_event(&self, event: &Event) -> RamingResult<()>;

    /// 获取监听器名称
    fn name(&self) -> &str;

    /// 获取感兴趣的事件类型
    fn interested_events(&self) -> &[EventType];
}

/// 事件类型枚举
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum EventType {
    Data,
    Error,
    All,
}

/// 通用事件总线
pub struct EventBus {
    sender: mpsc::UnboundedSender<Arc<Event>>,
    receiver: Arc<RwLock<mpsc::UnboundedReceiver<Arc<Event>>>>,
    listeners: Arc<RwLock<std::collections::HashMap<String, Arc<dyn EventListener>>>>,
    event_listeners: Arc<RwLock<std::collections::HashMap<EventType, Vec<String>>>>,
}

impl EventBus {
    /// 创建新的事件总线
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::unbounded_channel();

        Self {
            sender,
            receiver: Arc::new(RwLock::new(receiver)),
            listeners: Arc::new(RwLock::new(std::collections::HashMap::new())),
            event_listeners: Arc::new(RwLock::new(std::collections::HashMap::new())),
        }
    }

    /// 发布事件
    pub fn publish(&self, event: Event) -> RamingResult<()> {
        let event = Arc::new(event);
        self.sender
            .send(event)
            .map_err(|_| RamingError::EventSystemError("事件总线已关闭".to_string()))?;
        Ok(())
    }

    /// 同步处理单个事件（用于测试）
    pub async fn process_event_sync(&self, event: Event) -> RamingResult<()> {
        self.dispatch_event(Arc::new(event)).await
    }

    /// 注册监听器
    pub async fn subscribe(&self, listener: Arc<dyn EventListener>) -> RamingResult<()> {
        let name = listener.name().to_string();
        let interested_events: Vec<EventType> = listener.interested_events().to_vec();

        // 注册监听器
        let mut listeners = self.listeners.write().await;
        listeners.insert(name.clone(), listener);
        drop(listeners);

        // 注册事件类型映射
        let mut event_listeners = self.event_listeners.write().await;
        for event_type in interested_events {
            event_listeners
                .entry(event_type)
                .or_insert_with(Vec::new)
                .push(name.clone());
        }

        Ok(())
    }

    /// 取消注册监听器
    pub async fn unsubscribe(&self, name: &str) -> RamingResult<()> {
        let mut listeners = self.listeners.write().await;
        listeners.remove(name);

        let mut event_listeners = self.event_listeners.write().await;
        for listeners_list in event_listeners.values_mut() {
            listeners_list.retain(|n| n != name);
        }

        Ok(())
    }

    /// 运行事件循环
    pub async fn run(&self) -> RamingResult<()> {
        let mut receiver = self.receiver.write().await;

        while let Some(event) = receiver.recv().await {
            self.dispatch_event(event).await?;
        }

        Ok(())
    }

    /// 分发事件到对应的监听器
    async fn dispatch_event(&self, event: Arc<Event>) -> RamingResult<()> {
        let event_type = EventType::from(&event);
        let listeners = self.listeners.read().await;
        let event_listeners = self.event_listeners.read().await;

        // 获取对该事件感兴趣的监听器
        let interested_listeners = event_listeners
            .get(&event_type)
            .into_iter()
            .flatten()
            .chain(event_listeners.get(&EventType::All).into_iter().flatten());

        // 分发事件
        for listener_name in interested_listeners {
            if let Some(listener) = listeners.get(listener_name)
                && let Err(e) = listener.on_event(&event).await
            {
                // 记录错误但不中断其他监听器
                tracing::error!("监听器 {} 处理事件失败: {}", listener_name, e);
            }
        }

        Ok(())
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}

impl EventType {
    /// 从事件获取事件类型
    pub fn from(event: &Event) -> EventType {
        match event {
            Event::Data(_) => EventType::Data,
            Event::Error(_) => EventType::Error,
        }
    }
}

/// 事件构建器
pub struct EventBuilder;

impl EventBuilder {
    /// 创建数据事件
    pub fn data_event(event_type: impl Into<Arc<str>>, payload: Vec<u8>) -> Event {
        Event::Data(DataEvent {
            event_type: event_type.into(),
            payload: Arc::from(payload.into_boxed_slice()),
        })
    }

    /// 创建错误事件
    pub fn error_event(error_type: impl Into<Arc<str>>, message: impl Into<Arc<str>>) -> Event {
        Event::Error(ErrorEvent {
            error_type: error_type.into(),
            message: message.into(),
        })
    }

    /// 创建字符串数据事件
    pub fn string_event(event_type: impl Into<Arc<str>>, data: impl Into<String>) -> Event {
        let data = data.into();
        Event::Data(DataEvent {
            event_type: event_type.into(),
            payload: Arc::from(data.into_bytes().into_boxed_slice()),
        })
    }
}

/// 字符串事件操作扩展
pub trait StringEventOperations {
    /// 发布字符串事件
    fn publish_string(&self, event_type: &str, data: &str) -> RamingResult<()>;

    /// 发布字符串错误事件
    fn publish_string_error(&self, error_type: &str, message: &str) -> RamingResult<()>;

    /// 注册字符串事件监听器
    fn on(
        &self,
        event_type: &str,
        handler: impl Fn(&str, &str) + Send + Sync + 'static,
    ) -> RamingResult<()>;
}

impl StringEventOperations for EventBus {
    fn publish_string(&self, event_type: &str, data: &str) -> RamingResult<()> {
        let event = EventBuilder::string_event(event_type, data);
        self.publish(event)
    }

    fn publish_string_error(&self, error_type: &str, message: &str) -> RamingResult<()> {
        let event = EventBuilder::error_event(error_type, message);
        self.publish(event)
    }

    fn on(
        &self,
        event_type: &str,
        handler: impl Fn(&str, &str) + Send + Sync + 'static,
    ) -> RamingResult<()> {
        let event_type_str = event_type.to_string();
        let handler = Arc::new(handler);

        struct StringEventListener {
            name: String,
            event_type: String,
            handler: StringEventHandlerFn,
        }

        #[async_trait]
        impl EventListener for StringEventListener {
            fn name(&self) -> &str {
                &self.name
            }

            fn interested_events(&self) -> &[EventType] {
                &[EventType::Data]
            }

            async fn on_event(&self, event: &Event) -> RamingResult<()> {
                if let Event::Data(data_event) = event
                    && data_event.event_type.as_ref() == self.event_type
                    && let Ok(data_str) = String::from_utf8(data_event.payload.to_vec())
                {
                    (self.handler)(data_event.event_type.as_ref(), &data_str);
                }
                Ok(())
            }
        }

        let listener = Arc::new(StringEventListener {
            name: format!("string_listener_{}", event_type),
            event_type: event_type_str,
            handler,
        });

        // Use a blocking approach to register the listener
        let runtime = tokio::runtime::Runtime::new().map_err(|e| {
            RamingError::EventSystemError(format!("Failed to create runtime: {}", e))
        })?;

        runtime.block_on(async { self.subscribe(listener).await })
    }
}
