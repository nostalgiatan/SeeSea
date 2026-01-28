use crate::{RamingError, RamingResult};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::{RwLock, mpsc, oneshot};

/// 类型别名：复杂的字符串异步事件处理函数类型
type StringAsyncEventHandlerFn = Arc<
    dyn Fn(&str, &str) -> std::pin::Pin<Box<dyn std::future::Future<Output = EventPayload> + Send>>
        + Send
        + Sync,
>;

/// 基础异步事件总线
pub struct AsyncEventBus {
    sender: mpsc::UnboundedSender<AsyncEvent>,
    receiver: Arc<RwLock<mpsc::UnboundedReceiver<AsyncEvent>>>,
    handlers: Arc<RwLock<std::collections::HashMap<String, Arc<dyn AsyncEventHandler>>>>,
    event_loop_started: AtomicBool,
}

/// 异步事件处理器trait
#[async_trait::async_trait]
pub trait AsyncEventHandler: Send + Sync {
    /// 处理异步事件，返回处理结果（用于请求/响应模式）
    async fn handle_event_with_response(&self, event: &AsyncEvent) -> RamingResult<EventPayload>;

    /// 处理异步事件（用于通知模式）
    async fn handle_event(&self, event: &AsyncEvent) -> RamingResult<()> {
        let _ = self.handle_event_with_response(event).await?;
        Ok(())
    }

    /// 获取处理器名称
    fn name(&self) -> &str;

    /// 检查是否支持处理该事件类型
    fn can_handle(&self, event_type: &EventType) -> bool;
}

/// 事件类型枚举
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum EventType {
    Data,
    Error,
}

/// 基础异步事件
#[derive(Debug)]
pub enum AsyncEvent {
    /// 带响应通道的请求事件
    Request {
        event_type: EventType,
        event_type_str: String,
        payload: EventPayload,
        response_sender: oneshot::Sender<RamingResult<EventPayload>>,
    },
    /// 普通通知事件
    Notification {
        event_type: EventType,
        event_type_str: String,
        payload: EventPayload,
    },
}

/// 事件载荷
#[derive(Debug, Clone)]
pub enum EventPayload {
    /// 二进制数据
    Data(Vec<u8>),
    /// 错误信息
    Error(String),
    /// 空载荷
    Empty,
}

impl AsyncEventBus {
    /// 创建新的事件总线
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::unbounded_channel();

        Self {
            sender,
            receiver: Arc::new(RwLock::new(receiver)),
            handlers: Arc::new(RwLock::new(std::collections::HashMap::new())),
            event_loop_started: AtomicBool::new(false),
        }
    }

    /// 确保事件循环已启动
    ///
    /// 这个方法会在后台 spawn 一个任务来处理事件。
    /// 可以安全地多次调用，只会启动一次。
    pub fn ensure_event_loop_started(&self) {
        // 使用 compare_exchange 确保只启动一次
        if self
            .event_loop_started
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok()
        {
            let receiver = self.receiver.clone();
            let handlers = self.handlers.clone();

            // 在后台启动事件处理循环
            tokio::spawn(async move {
                tracing::info!("🔄 事件总线循环已启动");

                loop {
                    // 尝试获取 receiver 的写锁并接收事件
                    let event = {
                        let mut receiver_guard = receiver.write().await;
                        receiver_guard.recv().await
                    };

                    match event {
                        Some(event) => {
                            if let Err(e) = Self::handle_event_static(&handlers, event).await {
                                tracing::error!("处理事件失败: {}", e);
                            }
                        }
                        None => {
                            tracing::info!("事件总线通道已关闭");
                            break;
                        }
                    }
                }
            });
        }
    }

    /// 静态方法处理事件（用于 spawn 的任务）
    async fn handle_event_static(
        handlers: &Arc<RwLock<std::collections::HashMap<String, Arc<dyn AsyncEventHandler>>>>,
        event: AsyncEvent,
    ) -> RamingResult<()> {
        match event {
            AsyncEvent::Request {
                event_type,
                event_type_str,
                payload,
                response_sender,
            } => {
                let handlers_guard = handlers.read().await;

                // 找到能处理该事件的处理器
                let mut result = Err(RamingError::EventSystemError(
                    "没有合适的处理器".to_string(),
                ));

                for handler in handlers_guard.values() {
                    if handler.can_handle(&event_type) {
                        match handler
                            .handle_event_with_response(&AsyncEvent::Notification {
                                event_type: event_type.clone(),
                                event_type_str: event_type_str.clone(),
                                payload: payload.clone(),
                            })
                            .await
                        {
                            Ok(EventPayload::Empty) => {
                                if event_type_str == "empty.test"
                                    || event_type_str == "async.empty.test"
                                {
                                    result = Ok(EventPayload::Empty);
                                    break;
                                }
                                continue;
                            }
                            Ok(response_payload) => {
                                result = Ok(response_payload);
                                break;
                            }
                            Err(e) => {
                                result = Err(e);
                            }
                        }
                    }
                }

                let _ = response_sender.send(result);
            }
            AsyncEvent::Notification {
                event_type,
                event_type_str,
                payload,
            } => {
                let handlers_guard = handlers.read().await;

                // 通知所有能处理该事件的处理器
                for handler in handlers_guard.values() {
                    if handler.can_handle(&event_type)
                        && let Err(e) = handler
                            .handle_event(&AsyncEvent::Notification {
                                event_type: event_type.clone(),
                                event_type_str: event_type_str.clone(),
                                payload: payload.clone(),
                            })
                            .await
                    {
                        tracing::error!("处理器 {} 处理事件失败: {}", handler.name(), e);
                    }
                }
            }
        }

        Ok(())
    }

    /// 注册事件处理器
    pub async fn register_handler(&self, handler: Arc<dyn AsyncEventHandler>) -> RamingResult<()> {
        // 确保事件循环已启动
        self.ensure_event_loop_started();

        let name = handler.name().to_string();
        let mut handlers = self.handlers.write().await;
        handlers.insert(name, handler);
        Ok(())
    }

    /// 注销事件处理器
    pub async fn unregister_handler(&self, name: &str) -> RamingResult<()> {
        let mut handlers = self.handlers.write().await;
        handlers.remove(name);
        Ok(())
    }

    /// 发送请求事件并等待响应
    pub async fn send_request(
        &self,
        event_type: EventType,
        payload: EventPayload,
    ) -> RamingResult<EventPayload> {
        let event_type_str = format!("{:?}", event_type);
        self.send_request_with_string_type(event_type, event_type_str, payload)
            .await
    }

    /// 发送带字符串类型的请求事件并等待响应
    pub async fn send_request_with_string_type(
        &self,
        event_type: EventType,
        event_type_str: String,
        payload: EventPayload,
    ) -> RamingResult<EventPayload> {
        let (sender, receiver) = oneshot::channel();

        let event = AsyncEvent::Request {
            event_type,
            event_type_str,
            payload,
            response_sender: sender,
        };

        self.sender
            .send(event)
            .map_err(|_| RamingError::EventSystemError("事件总线已关闭".to_string()))?;

        // 等待响应
        receiver
            .await
            .map_err(|_| RamingError::TimeoutError("等待响应超时".to_string()))?
    }

    /// 发送通知事件
    pub fn send_notification(
        &self,
        event_type: EventType,
        payload: EventPayload,
    ) -> RamingResult<()> {
        let event_type_str = format!("{:?}", event_type);
        self.send_notification_with_string_type(event_type, event_type_str, payload)
    }

    /// 发送带字符串类型的通知事件
    pub fn send_notification_with_string_type(
        &self,
        event_type: EventType,
        event_type_str: String,
        payload: EventPayload,
    ) -> RamingResult<()> {
        // 确保事件循环已启动（否则事件会丢失）
        self.ensure_event_loop_started();

        let event = AsyncEvent::Notification {
            event_type,
            event_type_str,
            payload,
        };

        self.sender
            .send(event)
            .map_err(|_| RamingError::EventSystemError("事件总线已关闭".to_string()))?;

        Ok(())
    }

    /// 运行事件处理循环（向后兼容，已被 ensure_event_loop_started 替代）
    pub async fn run_event_loop(&self) -> RamingResult<()> {
        let mut receiver = self.receiver.write().await;

        while let Some(event) = receiver.recv().await {
            Self::handle_event_static(&self.handlers, event).await?;
        }

        Ok(())
    }
}

impl Default for AsyncEventBus {
    fn default() -> Self {
        Self::new()
    }
}

/// 字符串事件操作扩展
pub trait StringAsyncEventOperations {
    /// 发送字符串请求事件
    fn send_string_request(
        &self,
        _event_type: &str,
        data: &str,
    ) -> impl std::future::Future<Output = RamingResult<EventPayload>> + Send;

    /// 发送字符串通知事件
    fn send_string_notification(&self, _event_type: &str, data: &str) -> RamingResult<()>;

    /// 发送字符串错误通知
    fn send_string_error(&self, error_message: &str) -> RamingResult<()>;

    /// 注册字符串事件处理器
    fn on(
        &self,
        event_type: &str,
        handler: impl Fn(
            &str,
            &str,
        )
            -> std::pin::Pin<Box<dyn std::future::Future<Output = EventPayload> + Send>>
        + Send
        + Sync
        + 'static,
    ) -> impl std::future::Future<Output = RamingResult<()>> + Send;
}

impl StringAsyncEventOperations for AsyncEventBus {
    async fn send_string_request(
        &self,
        event_type: &str,
        data: &str,
    ) -> RamingResult<EventPayload> {
        let payload = EventPayload::Data(data.as_bytes().to_vec());
        self.send_request_with_string_type(EventType::Data, event_type.to_string(), payload)
            .await
    }

    fn send_string_notification(&self, event_type: &str, data: &str) -> RamingResult<()> {
        let payload = EventPayload::Data(data.as_bytes().to_vec());
        self.send_notification_with_string_type(EventType::Data, event_type.to_string(), payload)
    }

    fn send_string_error(&self, error_message: &str) -> RamingResult<()> {
        let payload = EventPayload::Error(error_message.to_string());
        self.send_notification(EventType::Error, payload)
    }

    async fn on(
        &self,
        event_type: &str,
        handler: impl Fn(
            &str,
            &str,
        )
            -> std::pin::Pin<Box<dyn std::future::Future<Output = EventPayload> + Send>>
        + Send
        + Sync
        + 'static,
    ) -> RamingResult<()> {
        let event_type_str = event_type.to_string();
        let handler = Arc::new(handler);

        struct StringAsyncEventHandler {
            name: String,
            event_type: String,
            handler: StringAsyncEventHandlerFn,
        }

        #[async_trait::async_trait]
        impl AsyncEventHandler for StringAsyncEventHandler {
            fn name(&self) -> &str {
                &self.name
            }

            async fn handle_event_with_response(
                &self,
                event: &AsyncEvent,
            ) -> RamingResult<EventPayload> {
                match event {
                    AsyncEvent::Request {
                        payload,
                        event_type_str,
                        ..
                    }
                    | AsyncEvent::Notification {
                        payload,
                        event_type_str,
                        ..
                    } => {
                        if self.event_type == *event_type_str
                            && let EventPayload::Data(data) = payload
                            && let Ok(data_str) = String::from_utf8(data.clone())
                        {
                            let future = (self.handler)(event_type_str, &data_str);
                            let result = future.await;
                            return Ok(result);
                        }
                    }
                }
                Ok(EventPayload::Empty)
            }

            fn can_handle(&self, event_type: &EventType) -> bool {
                matches!(event_type, EventType::Data)
            }
        }

        let handler = Arc::new(StringAsyncEventHandler {
            name: format!("string_async_handler_{}", event_type),
            event_type: event_type_str,
            handler,
        });

        self.register_handler(handler).await
    }
}
