//! Raming系统错误处理

use thiserror::Error;

/// Raming系统的主要错误类型
#[derive(Error, Debug, Clone)]
pub enum RamingError {
    /// 内存分配失败
    #[error("内存分配失败: {0}")]
    MemoryAllocationError(String),

    /// 共享内存段已存在
    #[error("共享内存段已存在: {0}")]
    MemorySegmentExists(String),

    /// 共享内存段不存在
    #[error("共享内存段不存在: {0}")]
    MemorySegmentNotFound(String),

    /// 内存访问冲突
    #[error("内存访问冲突: {0}")]
    MemoryAccessViolation(String),

    /// 事件绑定错误
    #[error("事件绑定错误: {0}")]
    EventBindingError(String),

    /// 事件系统错误
    #[error("事件系统错误: {0}")]
    EventSystemError(String),

    /// 事件处理错误
    #[error("事件处理错误: {0}")]
    EventHandlingError(String),

    /// 绑定错误
    #[error("绑定错误: {0}")]
    BindingError(String),

    /// 绑定类型不匹配
    #[error("绑定类型不匹配: 期望 {expected}, 实际 {actual}")]
    BindingTypeMismatch { expected: String, actual: String },

    /// 序列化错误
    #[error("序列化错误: {0}")]
    SerializationError(String),

    /// 反序列化错误
    #[error("反序列化错误: {0}")]
    DeserializationError(String),

    /// 权限错误
    #[error("权限错误: {0}")]
    PermissionError(String),

    /// 系统资源不足
    #[error("系统资源不足: {0}")]
    ResourceExhausted(String),

    /// 跨平台兼容性错误
    #[error("跨平台兼容性错误: {0}")]
    PlatformCompatibilityError(String),

    /// 配置错误
    #[error("配置错误: {0}")]
    ConfigurationError(String),

    /// 内部错误
    #[error("内部错误: {0}")]
    InternalError(String),
}

/// Raming系统的结果类型
pub type RamingResult<T> = Result<T, RamingError>;

impl RamingError {
    /// 创建内存分配错误
    pub fn memory_allocation(details: impl Into<String>) -> Self {
        RamingError::MemoryAllocationError(details.into())
    }

    /// 创建内存段已存在错误
    pub fn segment_exists(name: impl Into<String>) -> Self {
        RamingError::MemorySegmentExists(name.into())
    }

    /// 创建内存段不存在错误
    pub fn segment_not_found(name: impl Into<String>) -> Self {
        RamingError::MemorySegmentNotFound(name.into())
    }

    /// 创建事件绑定错误
    pub fn event_binding(details: impl Into<String>) -> Self {
        RamingError::EventBindingError(details.into())
    }

    /// 创建绑定错误
    pub fn binding_error(details: impl Into<String>) -> Self {
        RamingError::BindingError(details.into())
    }

    /// 创建事件系统错误
    pub fn event_system_error(details: impl Into<String>) -> Self {
        RamingError::EventSystemError(details.into())
    }

    /// 创建事件处理错误
    pub fn event_handling_error(details: impl Into<String>) -> Self {
        RamingError::EventHandlingError(details.into())
    }

    /// 检查是否是内存相关错误
    pub fn is_memory_error(&self) -> bool {
        matches!(
            self,
            RamingError::MemoryAllocationError(_)
                | RamingError::MemorySegmentExists(_)
                | RamingError::MemorySegmentNotFound(_)
                | RamingError::MemoryAccessViolation(_)
        )
    }

    /// 检查是否是事件相关错误
    pub fn is_event_error(&self) -> bool {
        matches!(
            self,
            RamingError::EventBindingError(_)
                | RamingError::EventHandlingError(_)
                | RamingError::BindingError(_)
        )
    }
}

/// 从其他错误类型转换
impl From<bincode::error::EncodeError> for RamingError {
    fn from(error: bincode::error::EncodeError) -> Self {
        RamingError::SerializationError(error.to_string())
    }
}

impl From<bincode::error::DecodeError> for RamingError {
    fn from(error: bincode::error::DecodeError) -> Self {
        RamingError::DeserializationError(error.to_string())
    }
}

impl From<std::io::Error> for RamingError {
    fn from(error: std::io::Error) -> Self {
        RamingError::InternalError(error.to_string())
    }
}

impl From<seesea_event::RamingError> for RamingError {
    fn from(error: seesea_event::RamingError) -> Self {
        RamingError::EventHandlingError(error.to_string())
    }
}

impl From<serde_json::Error> for RamingError {
    fn from(error: serde_json::Error) -> Self {
        RamingError::SerializationError(error.to_string())
    }
}
