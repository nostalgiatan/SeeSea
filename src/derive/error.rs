//! 搜索引擎派生模块的错误处理
//!
//! 本模块定义了搜索引擎抽象骨架中使用的所有错误类型，
//! 使用项目内部的 error 模块进行错误处理，确保零依赖和最佳性能。

use std::fmt;

/// 搜索引擎错误类型
///
/// 定义了搜索引擎操作中可能出现的所有错误情况
#[derive(Debug)]
pub enum DeriveError {
    /// 验证错误
    ///
    /// 当查询参数验证失败时返回此错误
    Validation {
        /// 错误详细信息
        message: String,
        /// 字段名（如果有）
        field: Option<String>,
    },

    /// 网络错误
    ///
    /// 当网络请求失败时返回此错误
    Network {
        /// 错误详细信息
        message: String,
        /// HTTP 状态码（如果有）
        status_code: Option<u16>,
    },

    /// 解析错误
    ///
    /// 当响应解析失败时返回此错误
    Parse {
        /// 错误详细信息
        message: String,
        /// 解析的内容类型
        content_type: Option<String>,
    },

    /// 超时错误
    ///
    /// 当请求超时时返回此错误
    Timeout {
        /// 超时时长（秒）
        duration_secs: u64,
    },

    /// 配置错误
    ///
    /// 当引擎配置无效时返回此错误
    Configuration {
        /// 错误详细信息
        message: String,
    },

    /// 缓存错误
    ///
    /// 当缓存操作失败时返回此错误
    Cache {
        /// 错误详细信息
        message: String,
        /// 操作类型（读/写）
        operation: CacheOperation,
    },

    /// 速率限制错误
    ///
    /// 当触发速率限制时返回此错误
    RateLimit {
        /// 重试等待时间（秒）
        retry_after_secs: u64,
    },

    /// 引擎不可用错误
    ///
    /// 当搜索引擎不可用时返回此错误
    EngineUnavailable {
        /// 引擎名称
        engine_name: String,
        /// 原因
        reason: String,
    },

    /// 内部错误
    ///
    /// 未预期的内部错误
    Internal {
        /// 错误详细信息
        message: String,
    },
}

/// 缓存操作类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheOperation {
    /// 读取操作
    Read,
    /// 写入操作
    Write,
    /// 删除操作
    Delete,
}

impl fmt::Display for CacheOperation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CacheOperation::Read => write!(f, "读取"),
            CacheOperation::Write => write!(f, "写入"),
            CacheOperation::Delete => write!(f, "删除"),
        }
    }
}

impl fmt::Display for DeriveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DeriveError::Validation { message, field } => {
                if let Some(field_name) = field {
                    write!(f, "验证错误[字段: {}]: {}", field_name, message)
                } else {
                    write!(f, "验证错误: {}", message)
                }
            }
            DeriveError::Network { message, status_code } => {
                if let Some(code) = status_code {
                    write!(f, "网络错误[状态码: {}]: {}", code, message)
                } else {
                    write!(f, "网络错误: {}", message)
                }
            }
            DeriveError::Parse { message, content_type } => {
                if let Some(ct) = content_type {
                    write!(f, "解析错误[类型: {}]: {}", ct, message)
                } else {
                    write!(f, "解析错误: {}", message)
                }
            }
            DeriveError::Timeout { duration_secs } => {
                write!(f, "请求超时: {}秒后无响应", duration_secs)
            }
            DeriveError::Configuration { message } => {
                write!(f, "配置错误: {}", message)
            }
            DeriveError::Cache { message, operation } => {
                write!(f, "缓存{}错误: {}", operation, message)
            }
            DeriveError::RateLimit { retry_after_secs } => {
                write!(f, "速率限制: 请在{}秒后重试", retry_after_secs)
            }
            DeriveError::EngineUnavailable { engine_name, reason } => {
                write!(f, "搜索引擎 '{}' 不可用: {}", engine_name, reason)
            }
            DeriveError::Internal { message } => {
                write!(f, "内部错误: {}", message)
            }
        }
    }
}

impl std::error::Error for DeriveError {}

/// DeriveError 的结果类型别名
pub type Result<T> = std::result::Result<T, DeriveError>;

/// 从 reqwest::Error 转换为 DeriveError
impl From<reqwest::Error> for DeriveError {
    fn from(err: reqwest::Error) -> Self {
        if err.is_timeout() {
            DeriveError::Timeout {
                duration_secs: 30, // 默认超时时间
            }
        } else if err.is_status() {
            let status_code = err.status().map(|s| s.as_u16());
            DeriveError::Network {
                message: err.to_string(),
                status_code,
            }
        } else {
            DeriveError::Network {
                message: err.to_string(),
                status_code: None,
            }
        }
    }
}

/// 从 serde_json::Error 转换为 DeriveError
impl From<serde_json::Error> for DeriveError {
    fn from(err: serde_json::Error) -> Self {
        DeriveError::Parse {
            message: err.to_string(),
            content_type: Some("application/json".to_string()),
        }
    }
}

/// 从 url::ParseError 转换为 DeriveError
impl From<url::ParseError> for DeriveError {
    fn from(err: url::ParseError) -> Self {
        DeriveError::Validation {
            message: format!("无效的 URL: {}", err),
            field: Some("url".to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = DeriveError::Validation {
            message: "查询不能为空".to_string(),
            field: Some("query".to_string()),
        };
        assert_eq!(err.to_string(), "验证错误[字段: query]: 查询不能为空");

        let err = DeriveError::Network {
            message: "连接被拒绝".to_string(),
            status_code: Some(403),
        };
        assert_eq!(err.to_string(), "网络错误[状态码: 403]: 连接被拒绝");

        let err = DeriveError::Timeout {
            duration_secs: 30,
        };
        assert_eq!(err.to_string(), "请求超时: 30秒后无响应");

        let err = DeriveError::RateLimit {
            retry_after_secs: 60,
        };
        assert_eq!(err.to_string(), "速率限制: 请在60秒后重试");
    }

    #[test]
    fn test_cache_operation_display() {
        assert_eq!(CacheOperation::Read.to_string(), "读取");
        assert_eq!(CacheOperation::Write.to_string(), "写入");
        assert_eq!(CacheOperation::Delete.to_string(), "删除");
    }

    #[test]
    fn test_error_conversion_from_serde_json() {
        let json_err = serde_json::from_str::<serde_json::Value>("invalid json")
            .err()
            .expect("应该产生错误");
        let derive_err: DeriveError = json_err.into();
        
        match derive_err {
            DeriveError::Parse { content_type, .. } => {
                assert_eq!(content_type, Some("application/json".to_string()));
            }
            _ => panic!("预期 Parse 错误"),
        }
    }

    #[test]
    fn test_error_conversion_from_url() {
        let url_err = url::Url::parse("not a url")
            .err()
            .expect("应该产生错误");
        let derive_err: DeriveError = url_err.into();
        
        match derive_err {
            DeriveError::Validation { field, .. } => {
                assert_eq!(field, Some("url".to_string()));
            }
            _ => panic!("预期 Validation 错误"),
        }
    }
}
