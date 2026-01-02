use std::fmt;

/// Raming系统错误类型
#[derive(Debug, Clone)]
pub enum RamingError {
    /// 事件系统错误
    EventSystemError(String),
    /// 内存共享错误
    MemoryShareError(String),
    /// 重排序错误
    RerankingError(String),
    /// 处理器错误
    HandlerError(String),
    /// 超时错误
    TimeoutError(String),
}

impl fmt::Display for RamingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RamingError::EventSystemError(msg) => write!(f, "事件系统错误: {}", msg),
            RamingError::MemoryShareError(msg) => write!(f, "内存共享错误: {}", msg),
            RamingError::RerankingError(msg) => write!(f, "重排序错误: {}", msg),
            RamingError::HandlerError(msg) => write!(f, "处理器错误: {}", msg),
            RamingError::TimeoutError(msg) => write!(f, "超时错误: {}", msg),
        }
    }
}

impl std::error::Error for RamingError {}

/// Raming结果类型
pub type RamingResult<T> = std::result::Result<T, RamingError>;
