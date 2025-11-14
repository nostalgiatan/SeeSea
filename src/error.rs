//! 错误处理模块
//!
//! 提供便利的错误类型和辅助函数

pub use error::{ErrorInfo, ErrorKind, ErrorCategory, ErrorSeverity};

/// Result 类型别名
pub type Result<T> = std::result::Result<T, ErrorInfo>;

/// Error 类型别名
pub type Error = ErrorInfo;

/// 创建网络错误
pub fn network_error(message: impl Into<String>) -> ErrorInfo {
    ErrorInfo::new(1000, message.into())
        .with_category(ErrorCategory::Network)
}

/// 创建搜索错误
pub fn search_error(message: impl Into<String>) -> ErrorInfo {
    ErrorInfo::new(2000, message.into())
        .with_category(ErrorCategory::Search)
}
