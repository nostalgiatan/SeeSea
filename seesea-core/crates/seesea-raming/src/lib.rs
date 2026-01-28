//! SeeSea Raming - 共享内存和事件绑定系统
//!
//! 提供高性能的共享内存管理、事件绑定和跨进程通信功能

pub mod bindings;
pub mod errors;
pub mod events;
pub mod manager;
pub mod memory;
pub mod pool;
pub mod types;

pub use bindings::{Binding, BindingManager, BindingStatus, ExecutionStatus};
pub use errors::{RamingError, RamingResult};
pub use events::{
    EventHandler, RamingEventBinding, RamingEventBus, RamingEventData, RamingEventListener,
    RamingEventType,
};
pub use manager::{ManagerConfig, RamingManager};
pub use memory::{MemorySegment, SharedMemory};
pub use pool::{MemoryPool, MemoryPoolManager, PoolInfo, PooledMemory};
pub use types::{BindingConfig, BindingType, EventConfig, MemoryConfig, PoolStats};

use tracing::{debug, info};

/// 初始化raming系统
pub fn init() -> RamingResult<()> {
    info!("初始化SeeSea Raming共享内存系统");

    // 初始化内存管理器
    let manager = RamingManager::new(Default::default())?;

    // 设置全局管理器
    manager.init_global()?;

    debug!("Raming系统初始化完成");
    Ok(())
}

/// 获取全局raming管理器
pub fn global_manager() -> RamingResult<RamingManager> {
    RamingManager::global()
}
