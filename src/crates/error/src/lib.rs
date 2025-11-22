// Copyright 2025 nostalgiatan
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! # 错误处理框架
//!
//! 这是一个零依赖的强大错误处理框架，提供自定义属性装饰器和自动宏支持。
//! 
//! ## 特性
//! 
//! - **零外部依赖**: 不依赖 `thiserror` 或 `anyhow` 等外部库
//! - **自定义装饰器**: 通过 `#[derive(Error)]` 自动为错误枚举实现错误处理能力
//! - **无隐式转换**: 所有转换都是显式的，确保最高性能
//! - **完整的错误信息**: 支持错误码、错误消息、错误源等
//! - **错误链**: 支持错误的嵌套和追踪
//!
//! ## 使用示例
//!
//! ```rust
//! use error::{Error, ErrorKind};
//!
//! #[derive(Debug, Error)]
//! enum MyError {
//!     #[error("IO错误: {0}")]
//!     Io(String),
//!     
//!     #[error("解析错误: {msg}")]
//!     Parse { msg: String },
//!     
//!     #[error("未知错误")]
//!     Unknown,
//! }
//! ```

pub use error_derive::Error;

use std::fmt;

/// 错误严重程度
///
/// 定义错误的严重程度级别，用于错误分类和处理策略
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ErrorSeverity {
    /// 调试级别：开发调试时的信息
    Debug = 0,
    /// 信息级别：普通信息性错误
    Info = 1,
    /// 警告级别：需要注意但不影响继续运行
    Warning = 2,
    /// 错误级别：需要处理的错误
    Error = 3,
    /// 严重级别：严重错误，可能导致程序崩溃
    Critical = 4,
}

impl fmt::Display for ErrorSeverity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ErrorSeverity::Debug => write!(f, "调试"),
            ErrorSeverity::Info => write!(f, "信息"),
            ErrorSeverity::Warning => write!(f, "警告"),
            ErrorSeverity::Error => write!(f, "错误"),
            ErrorSeverity::Critical => write!(f, "严重"),
        }
    }
}

/// 错误类别
///
/// 定义错误的类别，便于错误分类和统计
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCategory {
    /// IO 错误
    Io,
    /// 网络错误
    Network,
    /// 搜索错误
    Search,
    /// 解析错误
    Parse,
    /// 验证错误
    Validation,
    /// 权限错误
    Permission,
    /// 配置错误
    Configuration,
    /// 数据库错误
    Database,
    /// 业务逻辑错误
    Business,
    /// 系统错误
    System,
    /// 其他错误
    Other,
}

impl fmt::Display for ErrorCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ErrorCategory::Io => write!(f, "IO错误"),
            ErrorCategory::Network => write!(f, "网络错误"),
            ErrorCategory::Search => write!(f, "搜索错误"),
            ErrorCategory::Parse => write!(f, "解析错误"),
            ErrorCategory::Validation => write!(f, "验证错误"),
            ErrorCategory::Permission => write!(f, "权限错误"),
            ErrorCategory::Configuration => write!(f, "配置错误"),
            ErrorCategory::Database => write!(f, "数据库错误"),
            ErrorCategory::Business => write!(f, "业务逻辑错误"),
            ErrorCategory::System => write!(f, "系统错误"),
            ErrorCategory::Other => write!(f, "其他错误"),
        }
    }
}

/// 错误类型的核心trait
/// 
/// 所有自定义错误类型都应该实现此trait，以提供统一的错误处理接口。
pub trait ErrorKind: fmt::Debug + fmt::Display {
    /// 获取错误码
    /// 
    /// 每个错误变体都应该有一个唯一的错误码，便于错误分类和处理。
    fn error_code(&self) -> u32;
    
    /// 获取错误消息
    /// 
    /// 返回人类可读的错误描述信息。
    fn error_message(&self) -> String;
    
    /// 获取错误源
    /// 
    /// 如果当前错误是由另一个错误引起的，返回源错误的引用。
    fn source(&self) -> Option<&dyn ErrorKind> {
        None
    }
}

/// 错误信息结构体
/// 
/// 封装了错误的详细信息，包括错误码、消息、源错误、上下文、严重程度和类别等。
#[derive(Debug)]
pub struct ErrorInfo {
    /// 错误码
    code: u32,
    /// 错误消息
    message: String,
    /// 源错误（可选）
    source: Option<Box<dyn ErrorKind>>,
    /// 错误上下文（用于添加额外信息）
    context: Vec<String>,
    /// 错误严重程度
    severity: ErrorSeverity,
    /// 错误类别
    category: ErrorCategory,
}

impl ErrorInfo {
    /// 创建一个新的错误信息
    /// 
    /// # 参数
    /// 
    /// * `code` - 错误码
    /// * `message` - 错误消息
    /// 
    /// # 示例
    /// 
    /// ```rust
    /// use error::ErrorInfo;
    /// 
    /// let error = ErrorInfo::new(404, "未找到资源".to_string());
    /// ```
    pub fn new(code: u32, message: String) -> Self {
        Self {
            code,
            message,
            source: None,
            context: Vec::new(),
            severity: ErrorSeverity::Error,
            category: ErrorCategory::Other,
        }
    }
    
    /// 创建一个带有源错误的错误信息
    /// 
    /// # 参数
    /// 
    /// * `code` - 错误码
    /// * `message` - 错误消息
    /// * `source` - 源错误
    /// 
    /// # 示例
    /// 
    /// ```rust
    /// use error::ErrorInfo;
    /// 
    /// let source = ErrorInfo::new(500, "内部错误".to_string());
    /// let error = ErrorInfo::with_source(404, "请求失败".to_string(), source);
    /// ```
    pub fn with_source<E: ErrorKind + 'static>(code: u32, message: String, source: E) -> Self {
        Self {
            code,
            message,
            source: Some(Box::new(source)),
            context: Vec::new(),
            severity: ErrorSeverity::Error,
            category: ErrorCategory::Other,
        }
    }
    
    /// 添加错误上下文
    ///
    /// 用于在错误传播过程中添加额外的上下文信息。
    ///
    /// # 示例
    ///
    /// ```rust
    /// use error::ErrorInfo;
    ///
    /// let mut error = ErrorInfo::new(404, "文件未找到".to_string());
    /// error.add_context("尝试读取配置文件: /etc/app/config.toml".to_string());
    /// ```
    pub fn add_context(&mut self, context: String) {
        self.context.push(context);
    }
    
    /// 链式添加错误上下文
    ///
    /// 返回 Self 以支持链式调用。
    ///
    /// # 示例
    ///
    /// ```rust
    /// use error::ErrorInfo;
    ///
    /// let error = ErrorInfo::new(404, "文件未找到".to_string())
    ///     .with_context("尝试读取配置文件".to_string())
    ///     .with_context("在初始化模块时".to_string());
    /// ```
    pub fn with_context(mut self, context: String) -> Self {
        self.context.push(context);
        self
    }
    
    /// 设置错误严重程度
    ///
    /// # 示例
    ///
    /// ```rust
    /// use error::{ErrorInfo, ErrorSeverity};
    ///
    /// let error = ErrorInfo::new(500, "内部服务器错误".to_string())
    ///     .with_severity(ErrorSeverity::Critical);
    /// ```
    pub fn with_severity(mut self, severity: ErrorSeverity) -> Self {
        self.severity = severity;
        self
    }
    
    /// 设置错误类别
    ///
    /// # 示例
    ///
    /// ```rust
    /// use error::{ErrorInfo, ErrorCategory};
    ///
    /// let error = ErrorInfo::new(403, "访问被拒绝".to_string())
    ///     .with_category(ErrorCategory::Permission);
    /// ```
    pub fn with_category(mut self, category: ErrorCategory) -> Self {
        self.category = category;
        self
    }
    
    /// 获取错误码
    pub fn code(&self) -> u32 {
        self.code
    }
    
    /// 获取错误消息
    pub fn message(&self) -> &str {
        &self.message
    }
    
    /// 获取源错误
    pub fn source(&self) -> Option<&dyn ErrorKind> {
        self.source.as_ref().map(|e| e.as_ref())
    }
    
    /// 获取错误上下文
    pub fn context(&self) -> &[String] {
        &self.context
    }
    
    /// 获取错误严重程度
    pub fn severity(&self) -> ErrorSeverity {
        self.severity
    }
    
    /// 获取错误类别
    pub fn category(&self) -> ErrorCategory {
        self.category
    }
    
    /// 判断是否为严重错误
    pub fn is_critical(&self) -> bool {
        self.severity == ErrorSeverity::Critical
    }
    
    /// 判断是否为警告
    pub fn is_warning(&self) -> bool {
        self.severity == ErrorSeverity::Warning
    }
}

impl fmt::Display for ErrorInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}][{}][错误码: {}] {}", 
               self.severity, self.category, self.code, self.message)?;
        
        // 显示上下文信息
        for ctx in &self.context {
            write!(f, "\n  上下文: {}", ctx)?;
        }
        
        // 显示源错误
        if let Some(source) = &self.source {
            write!(f, "\n  由以下错误引起: {}", source)?;
        }
        Ok(())
    }
}

impl ErrorKind for ErrorInfo {
    fn error_code(&self) -> u32 {
        self.code
    }
    
    fn error_message(&self) -> String {
        self.message.clone()
    }
    
    fn source(&self) -> Option<&dyn ErrorKind> {
        self.source.as_ref().map(|e| e.as_ref())
    }
}

/// 结果类型别名
/// 
/// 使用 `ErrorInfo` 作为错误类型的 Result 别名，简化函数签名。
/// 
/// # 示例
/// 
/// ```rust
/// use error::Result;
/// 
/// fn do_something() -> Result<i32> {
///     Ok(42)
/// }
/// ```
pub type Result<T> = std::result::Result<T, ErrorInfo>;

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_error_info_new() {
        let error = ErrorInfo::new(404, "未找到资源".to_string());
        assert_eq!(error.code(), 404);
        assert_eq!(error.message(), "未找到资源");
        assert!(error.source().is_none());
    }
    
    #[test]
    fn test_error_info_with_source() {
        let source = ErrorInfo::new(500, "内部错误".to_string());
        let error = ErrorInfo::with_source(404, "请求失败".to_string(), source);
        
        assert_eq!(error.code(), 404);
        assert_eq!(error.message(), "请求失败");
        assert!(error.source().is_some());
        
        if let Some(src) = error.source() {
            assert_eq!(src.error_code(), 500);
            assert_eq!(src.error_message(), "内部错误");
        }
    }
    
    #[test]
    fn test_error_info_display() {
        let error = ErrorInfo::new(404, "未找到资源".to_string());
        let display = format!("{}", error);
        assert!(display.contains("404"));
        assert!(display.contains("未找到资源"));
    }
    
    #[test]
    fn test_error_info_display_with_source() {
        let source = ErrorInfo::new(500, "内部错误".to_string());
        let error = ErrorInfo::with_source(404, "请求失败".to_string(), source);
        let display = format!("{}", error);
        assert!(display.contains("404"));
        assert!(display.contains("请求失败"));
        assert!(display.contains("由以下错误引起"));
        assert!(display.contains("内部错误"));
    }
    
    #[test]
    fn test_error_kind_trait() {
        let error = ErrorInfo::new(404, "未找到资源".to_string());
        assert_eq!(error.error_code(), 404);
        assert_eq!(error.error_message(), "未找到资源");
    }
    
    #[test]
    fn test_result_type() {
        let success: Result<i32> = Ok(42);
        assert!(success.is_ok());
        if let Ok(val) = success {
            assert_eq!(val, 42);
        }
        
        let failure: Result<i32> = Err(ErrorInfo::new(500, "错误".to_string()));
        assert!(failure.is_err());
    }
    
    #[test]
    fn test_error_context() {
        let mut error = ErrorInfo::new(404, "文件未找到".to_string());
        error.add_context("尝试读取配置文件".to_string());
        error.add_context("在初始化模块时".to_string());
        
        assert_eq!(error.context().len(), 2);
        assert_eq!(error.context()[0], "尝试读取配置文件");
        assert_eq!(error.context()[1], "在初始化模块时");
        
        let display = format!("{}", error);
        assert!(display.contains("上下文"));
        assert!(display.contains("尝试读取配置文件"));
        assert!(display.contains("在初始化模块时"));
    }
    
    #[test]
    fn test_error_context_chain() {
        let error = ErrorInfo::new(404, "文件未找到".to_string())
            .with_context("尝试读取配置文件".to_string())
            .with_context("在初始化模块时".to_string());
        
        assert_eq!(error.context().len(), 2);
    }
    
    #[test]
    fn test_error_severity() {
        let error = ErrorInfo::new(500, "内部错误".to_string())
            .with_severity(ErrorSeverity::Critical);
        
        assert_eq!(error.severity(), ErrorSeverity::Critical);
        assert!(error.is_critical());
        assert!(!error.is_warning());
        
        let warning = ErrorInfo::new(200, "注意".to_string())
            .with_severity(ErrorSeverity::Warning);
        
        assert!(warning.is_warning());
        assert!(!warning.is_critical());
    }
    
    #[test]
    fn test_error_category() {
        let error = ErrorInfo::new(403, "访问被拒绝".to_string())
            .with_category(ErrorCategory::Permission);
        
        assert_eq!(error.category(), ErrorCategory::Permission);
        
        let display = format!("{}", error);
        assert!(display.contains("权限错误"));
    }
    
    #[test]
    fn test_error_severity_ordering() {
        assert!(ErrorSeverity::Critical > ErrorSeverity::Error);
        assert!(ErrorSeverity::Error > ErrorSeverity::Warning);
        assert!(ErrorSeverity::Warning > ErrorSeverity::Info);
        assert!(ErrorSeverity::Info > ErrorSeverity::Debug);
    }
    
    #[test]
    fn test_error_severity_display() {
        assert_eq!(format!("{}", ErrorSeverity::Debug), "调试");
        assert_eq!(format!("{}", ErrorSeverity::Info), "信息");
        assert_eq!(format!("{}", ErrorSeverity::Warning), "警告");
        assert_eq!(format!("{}", ErrorSeverity::Error), "错误");
        assert_eq!(format!("{}", ErrorSeverity::Critical), "严重");
    }
    
    #[test]
    fn test_error_category_display() {
        assert_eq!(format!("{}", ErrorCategory::Io), "IO错误");
        assert_eq!(format!("{}", ErrorCategory::Network), "网络错误");
        assert_eq!(format!("{}", ErrorCategory::Parse), "解析错误");
        assert_eq!(format!("{}", ErrorCategory::Permission), "权限错误");
    }
    
    #[test]
    fn test_complex_error_with_all_features() {
        let source = ErrorInfo::new(1001, "底层IO错误".to_string())
            .with_severity(ErrorSeverity::Error)
            .with_category(ErrorCategory::Io);
        
        let error = ErrorInfo::with_source(2001, "读取文件失败".to_string(), source)
            .with_context("处理配置文件".to_string())
            .with_context("应用启动阶段".to_string())
            .with_severity(ErrorSeverity::Critical)
            .with_category(ErrorCategory::Configuration);
        
        assert_eq!(error.code(), 2001);
        assert_eq!(error.message(), "读取文件失败");
        assert_eq!(error.context().len(), 2);
        assert_eq!(error.severity(), ErrorSeverity::Critical);
        assert_eq!(error.category(), ErrorCategory::Configuration);
        assert!(error.source().is_some());
        
        let display = format!("{}", error);
        assert!(display.contains("严重"));
        assert!(display.contains("配置错误"));
        assert!(display.contains("上下文"));
        assert!(display.contains("由以下错误引起"));
    }
}
