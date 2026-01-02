use error::ErrorCategory;
use seesea_errors::*;
use std::error::Error as StdError;

/// 测试错误信息的基本创建和属性
#[test]
fn test_error_info_creation() {
    let error = ErrorInfo::new(1001, "测试错误消息".to_string())
        .with_category(ErrorCategory::Validation)
        .with_severity(ErrorSeverity::Error);

    assert_eq!(error.code(), 1001);
    assert_eq!(error.message(), "测试错误消息");
    assert_eq!(error.category(), ErrorCategory::Validation);
    assert_eq!(error.severity(), ErrorSeverity::Error);
    assert!(error.context().is_empty());
    assert!(error.source().is_none());
}

/// 测试错误信息创建（带上下文）
#[test]
fn test_error_info_with_context() {
    let error = ErrorInfo::new(1002, "测试错误消息".to_string())
        .with_category(ErrorCategory::Network)
        .with_severity(ErrorSeverity::Warning)
        .with_context("测试上下文".to_string());

    assert_eq!(error.code(), 1002);
    assert_eq!(error.message(), "测试错误消息");
    assert_eq!(error.category(), ErrorCategory::Network);
    assert_eq!(error.severity(), ErrorSeverity::Warning);
    assert_eq!(error.context(), &["测试上下文".to_string()]);
    assert!(error.source().is_none());
}

/// 测试错误信息创建（带源错误）
#[test]
fn test_error_info_with_source() {
    let source_error = ErrorInfo::new(2001, "源错误消息".to_string())
        .with_category(ErrorCategory::Io)
        .with_severity(ErrorSeverity::Error);

    let error = ErrorInfo::with_source(1003, "外层错误消息".to_string(), source_error)
        .with_category(ErrorCategory::Network)
        .with_severity(ErrorSeverity::Error);

    assert_eq!(error.code(), 1003);
    assert_eq!(error.message(), "外层错误消息");
    assert_eq!(error.category(), ErrorCategory::Network);
    assert_eq!(error.severity(), ErrorSeverity::Error);
    assert!(error.source().is_some());
}

/// 测试错误严重程度级别
#[test]
fn test_error_severity_levels() {
    let debug_error =
        ErrorInfo::new(3001, "调试错误".to_string()).with_severity(ErrorSeverity::Debug);
    let info_error =
        ErrorInfo::new(3002, "信息错误".to_string()).with_severity(ErrorSeverity::Info);
    let warning_error =
        ErrorInfo::new(3003, "警告错误".to_string()).with_severity(ErrorSeverity::Warning);
    let error_error =
        ErrorInfo::new(3004, "错误级别错误".to_string()).with_severity(ErrorSeverity::Error);
    let critical_error =
        ErrorInfo::new(3005, "严重错误".to_string()).with_severity(ErrorSeverity::Critical);

    assert_eq!(debug_error.severity(), ErrorSeverity::Debug);
    assert_eq!(info_error.severity(), ErrorSeverity::Info);
    assert_eq!(warning_error.severity(), ErrorSeverity::Warning);
    assert_eq!(error_error.severity(), ErrorSeverity::Error);
    assert_eq!(critical_error.severity(), ErrorSeverity::Critical);

    // 测试严重程度比较
    assert!(ErrorSeverity::Debug < ErrorSeverity::Info);
    assert!(ErrorSeverity::Info < ErrorSeverity::Warning);
    assert!(ErrorSeverity::Warning < ErrorSeverity::Error);
    assert!(ErrorSeverity::Error < ErrorSeverity::Critical);
}

/// 测试错误类别
#[test]
fn test_error_categories() {
    let io_error = ErrorInfo::new(4001, "IO错误".to_string()).with_category(ErrorCategory::Io);
    let network_error =
        ErrorInfo::new(4002, "网络错误".to_string()).with_category(ErrorCategory::Network);
    let search_error =
        ErrorInfo::new(4003, "搜索错误".to_string()).with_category(ErrorCategory::Search);
    let parse_error =
        ErrorInfo::new(4004, "解析错误".to_string()).with_category(ErrorCategory::Parse);
    let validation_error =
        ErrorInfo::new(4005, "验证错误".to_string()).with_category(ErrorCategory::Validation);
    let permission_error =
        ErrorInfo::new(4006, "权限错误".to_string()).with_category(ErrorCategory::Permission);
    let configuration_error =
        ErrorInfo::new(4007, "配置错误".to_string()).with_category(ErrorCategory::Configuration);
    let database_error =
        ErrorInfo::new(4008, "数据库错误".to_string()).with_category(ErrorCategory::Database);
    let business_error =
        ErrorInfo::new(4009, "业务逻辑错误".to_string()).with_category(ErrorCategory::Business);
    let system_error =
        ErrorInfo::new(4010, "系统错误".to_string()).with_category(ErrorCategory::System);
    let other_error =
        ErrorInfo::new(4011, "其他错误".to_string()).with_category(ErrorCategory::Other);

    assert_eq!(io_error.category(), ErrorCategory::Io);
    assert_eq!(network_error.category(), ErrorCategory::Network);
    assert_eq!(search_error.category(), ErrorCategory::Search);
    assert_eq!(parse_error.category(), ErrorCategory::Parse);
    assert_eq!(validation_error.category(), ErrorCategory::Validation);
    assert_eq!(permission_error.category(), ErrorCategory::Permission);
    assert_eq!(configuration_error.category(), ErrorCategory::Configuration);
    assert_eq!(database_error.category(), ErrorCategory::Database);
    assert_eq!(business_error.category(), ErrorCategory::Business);
    assert_eq!(system_error.category(), ErrorCategory::System);
    assert_eq!(other_error.category(), ErrorCategory::Other);
}

/// 测试错误严重程度的判断方法
#[test]
fn test_error_severity_judgment() {
    let critical_error =
        ErrorInfo::new(5001, "严重错误".to_string()).with_severity(ErrorSeverity::Critical);
    let error_level_error =
        ErrorInfo::new(5002, "错误级别错误".to_string()).with_severity(ErrorSeverity::Error);
    let warning_error =
        ErrorInfo::new(5003, "警告错误".to_string()).with_severity(ErrorSeverity::Warning);

    assert!(critical_error.is_critical());
    assert!(!error_level_error.is_critical());
    assert!(!warning_error.is_critical());

    assert!(!critical_error.is_warning());
    assert!(!error_level_error.is_warning());
    assert!(warning_error.is_warning());
}

/// 测试错误链（多个错误组合）
#[test]
fn test_error_chain() {
    let base_error = ErrorInfo::new(6001, "基础错误".to_string())
        .with_category(ErrorCategory::Io)
        .with_severity(ErrorSeverity::Error);

    let middle_error = ErrorInfo::with_source(6002, "中间层错误".to_string(), base_error)
        .with_category(ErrorCategory::Network)
        .with_severity(ErrorSeverity::Error);

    let top_error = ErrorInfo::with_source(6003, "顶层错误".to_string(), middle_error)
        .with_category(ErrorCategory::Business)
        .with_severity(ErrorSeverity::Error);

    // 验证顶层错误
    assert_eq!(top_error.code(), 6003);
    assert_eq!(top_error.message(), "顶层错误");
    assert_eq!(top_error.category(), ErrorCategory::Business);
    assert!(top_error.source().is_some());
}

/// 测试错误的错误转换
#[test]
fn test_error_conversion() {
    let error = ErrorInfo::new(8001, "测试错误转换".to_string())
        .with_category(ErrorCategory::Validation)
        .with_severity(ErrorSeverity::Error);

    // 转换为标准错误
    let std_error: Box<dyn StdError> = Box::new(error.clone());
    assert_eq!(
        std_error.to_string(),
        "[错误][验证错误][错误码: 8001] 测试错误转换"
    );

    // 验证错误源
    assert!(std_error.source().is_none());
}

/// 测试错误的显示格式
#[test]
fn test_error_display() {
    let error = ErrorInfo::new(9001, "测试显示格式".to_string())
        .with_category(ErrorCategory::Network)
        .with_severity(ErrorSeverity::Warning)
        .with_context("显示测试上下文".to_string());

    let display_string = format!("{}", error);
    assert!(display_string.contains("测试显示格式"));
    assert!(display_string.contains("警告"));
    assert!(display_string.contains("网络错误"));
    assert!(display_string.contains("9001"));
    assert!(display_string.contains("显示测试上下文"));
}

/// 测试错误的调试格式
#[test]
fn test_error_debug() {
    let error = ErrorInfo::new(10001, "测试调试格式".to_string())
        .with_category(ErrorCategory::System)
        .with_severity(ErrorSeverity::Critical);

    let debug_string = format!("{:?}", error);
    assert!(debug_string.contains("ErrorInfo"));
    assert!(debug_string.contains("10001"));
}

/// 测试错误的比较
#[test]
fn test_error_equality() {
    let error1 = ErrorInfo::new(11001, "相同错误".to_string())
        .with_category(ErrorCategory::Validation)
        .with_severity(ErrorSeverity::Error);

    let error2 = ErrorInfo::new(11001, "相同错误".to_string())
        .with_category(ErrorCategory::Validation)
        .with_severity(ErrorSeverity::Error);

    let error3 = ErrorInfo::new(11002, "不同错误".to_string())
        .with_category(ErrorCategory::Network)
        .with_severity(ErrorSeverity::Warning);

    assert_eq!(error1.code(), error2.code());
    assert_eq!(error1.category(), error2.category());
    assert_eq!(error1.severity(), error2.severity());

    assert_ne!(error1.code(), error3.code());
    assert_ne!(error1.category(), error3.category());
    assert_ne!(error1.severity(), error3.severity());
}

/// 测试错误的克隆
#[test]
fn test_error_clone() {
    let original = ErrorInfo::new(12001, "原始错误".to_string())
        .with_category(ErrorCategory::Database)
        .with_severity(ErrorSeverity::Error)
        .with_context("原始上下文".to_string());

    let cloned = original.clone();

    assert_eq!(cloned.code(), original.code());
    assert_eq!(cloned.message(), original.message());
    assert_eq!(cloned.category(), original.category());
    assert_eq!(cloned.severity(), original.severity());
    assert_eq!(cloned.context(), original.context());
}
