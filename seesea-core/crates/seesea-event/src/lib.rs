//! SeeSea通用事件驱动系统
//!
//! 提供通用的事件驱动机制，支持跨模块异步通信

use once_cell::sync::Lazy;
use std::sync::Arc;

pub mod async_events;
pub mod errors;
pub mod events;

// 从errors模块导出
pub use errors::{RamingError, RamingResult};

// 从events模块导出
pub use events::{
    DataEvent, ErrorEvent, Event, EventBuilder, EventBus, EventListener, EventType,
    StringEventOperations,
};

// 从async_events模块导出
pub use async_events::{
    AsyncEvent, AsyncEventBus, AsyncEventHandler, EventPayload, StringAsyncEventOperations,
};

/// 全局事件总线实例
pub static GLOBAL_EVENT_BUS: Lazy<Arc<EventBus>> = Lazy::new(|| Arc::new(EventBus::new()));

/// 全局异步事件总线实例
pub static GLOBAL_ASYNC_EVENT_BUS: Lazy<Arc<AsyncEventBus>> =
    Lazy::new(|| Arc::new(AsyncEventBus::new()));

/// 获取全局事件总线实例
pub fn get_global_event_bus() -> Arc<EventBus> {
    GLOBAL_EVENT_BUS.clone()
}

/// 获取全局异步事件总线实例
pub fn get_global_async_event_bus() -> Arc<AsyncEventBus> {
    GLOBAL_ASYNC_EVENT_BUS.clone()
}
