// 模块名称: tf::error
// 职责范围: 统一的错误处理机制，定义3xxx系列错误码
// 期望实现计划:
// - 定义清晰的错误类型枚举
// - 为每个错误分配唯一的3xxx错误码
// - 实现与PyO3兼容的错误转换
// - 提供详细的错误信息
// 已实现功能:
// - 完整的错误类型定义
// - 3xxx错误码分配
// - PyO3错误转换实现
// 使用依赖:
// - pyo3: Python绑定
// - thiserror: 自动实现Error trait
// 主要接口:
// - TFError: 主错误类型枚举
// - PyResult<T>: 类型别名，简化错误处理
// 注意事项:
// - 所有错误码必须以3开头
// - 错误信息应包含足够的上下文
// - 确保与Python异常类型正确映射

use pyo3::exceptions::{PyKeyError, PyRuntimeError, PyValueError};
use pyo3::prelude::*;
use thiserror::Error;

/// 统一的错误类型，包含3xxx错误码
#[derive(Error, Debug)]
pub enum TFError {
    /// 向量维度不匹配 (错误码: 3000)
    #[error("3000: Vector dimension mismatch. Expected {expected}, got {actual}")]
    VectorDimensionMismatch { expected: usize, actual: usize },

    /// 锁定错误 (错误码: 3001)
    #[error("3001: Lock error")]
    LockError,

    /// 写入锁定错误 (错误码: 3002)
    #[error("3002: Write lock error")]
    WriteLockError,

    /// 向量存储错误 (错误码: 3003)
    #[error("3003: Vector store error: {0}")]
    VecStoreError(#[from] vecstore::VecStoreError),

    /// 文件系统错误 (错误码: 3004)
    #[error("3004: File system error: {0}")]
    FileSystemError(#[from] std::io::Error),

    /// 文档未找到 (错误码: 3005)
    #[error("3005: Document not found: {0}")]
    DocumentNotFound(String),

    /// Python回调错误 (错误码: 3006)
    #[error("3006: Python callback error: {0}")]
    PythonCallbackError(#[from] PyErr),

    /// JSON序列化错误 (错误码: 3007)
    #[error("3007: JSON serialization error: {0}")]
    JsonError(#[from] serde_json::Error),

    /// 无效参数 (错误码: 3008)
    #[error("3008: Invalid parameter: {0}")]
    InvalidParameter(String),

    /// 初始化错误 (错误码: 3009)
    #[error("3009: Initialization error: {0}")]
    InitializationError(String),

    /// 通用错误 (错误码: 3010)
    #[error("3010: Generic error: {0}")]
    GenericError(#[from] anyhow::Error),
}

/// 类型别名，简化自定义错误的使用
pub type TFResult<T> = Result<T, TFError>;

// 实现从TFError到PyErr的转换
impl From<TFError> for PyErr {
    fn from(err: TFError) -> Self {
        match err {
            TFError::VectorDimensionMismatch { .. } => PyValueError::new_err(err.to_string()),
            TFError::DocumentNotFound(_) => PyKeyError::new_err(err.to_string()),
            TFError::InvalidParameter(_) => PyValueError::new_err(err.to_string()),
            _ => PyRuntimeError::new_err(err.to_string()),
        }
    }
}
