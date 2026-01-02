use error::ErrorCategory;
use seesea_errors::*;
use std::error::Error as StdError;

/// 测试空字段错误创建
#[test]
fn test_empty_field_error_creation() {
    let error = empty_field("username");

    assert_eq!(error.category(), ErrorCategory::Validation);
    assert_eq!(error.severity(), ErrorSeverity::Error);
    assert_eq!(error.code(), 4001); // EMPTY_FIELD
    assert!(error.message().contains("字段 'username' 不能为空"));
    assert!(error.context().is_empty());
}

/// 测试字段太短错误
#[test]
fn test_field_too_short_error() {
    let error = field_too_short("password", 8, 5);

    assert_eq!(error.category(), ErrorCategory::Validation);
    assert_eq!(error.severity(), ErrorSeverity::Error);
    assert_eq!(error.code(), 4002); // FIELD_TOO_SHORT
    assert!(error.message().contains("字段 'password' 太短"));
    assert!(error.message().contains("要求至少 8 个字符"));
    assert!(error.message().contains("实际 5 个字符"));
    assert!(error.context().is_empty());
}

/// 测试字段太长错误
#[test]
fn test_field_too_long_error() {
    let error = field_too_long("username", 20, 25);

    assert_eq!(error.category(), ErrorCategory::Validation);
    assert_eq!(error.severity(), ErrorSeverity::Error);
    assert_eq!(error.code(), 4003); // FIELD_TOO_LONG
    assert!(error.message().contains("字段 'username' 太长"));
    assert!(error.message().contains("要求最多 20 个字符"));
    assert!(error.message().contains("实际 25 个字符"));
    assert!(error.context().is_empty());
}

/// 测试无效邮箱错误
#[test]
fn test_invalid_email_error() {
    let error = invalid_email("invalid-email");

    assert_eq!(error.category(), ErrorCategory::Validation);
    assert_eq!(error.severity(), ErrorSeverity::Error);
    assert_eq!(error.code(), 4004); // INVALID_EMAIL
    assert!(error.message().contains("无效的邮箱地址"));
    assert!(error.message().contains("invalid-email"));
    assert!(error.context().is_empty());
}

/// 测试无效URL错误
#[test]
fn test_invalid_url_error() {
    let error = invalid_url("not-a-url");

    assert_eq!(error.category(), ErrorCategory::Validation);
    assert_eq!(error.severity(), ErrorSeverity::Error);
    assert_eq!(error.code(), 4005); // INVALID_URL
    assert!(error.message().contains("无效的URL地址"));
    assert!(error.message().contains("not-a-url"));
    assert!(error.context().is_empty());
}

/// 测试无效日期错误
#[test]
fn test_invalid_date_error() {
    let error = invalid_date("2023-13-45");

    assert_eq!(error.category(), ErrorCategory::Validation);
    assert_eq!(error.severity(), ErrorSeverity::Error);
    assert_eq!(error.code(), 4006); // INVALID_DATE
    assert!(error.message().contains("无效的日期格式"));
    assert!(error.message().contains("2023-13-45"));
    assert!(error.context().is_empty());
}

/// 测试无效数字错误
#[test]
fn test_invalid_number_error() {
    let error = invalid_number("abc", "age");

    assert_eq!(error.category(), ErrorCategory::Validation);
    assert_eq!(error.severity(), ErrorSeverity::Error);
    assert_eq!(error.code(), 4007); // INVALID_NUMBER
    assert!(error.message().contains("字段 'age' 不是有效的数字"));
    assert!(error.message().contains("abc"));
    assert!(error.context().is_empty());
}

/// 测试无效枚举值错误
#[test]
fn test_invalid_enum_value_error() {
    let allowed_values = vec!["admin", "user", "guest"];
    let error = invalid_enum_value("role", "superuser", &allowed_values);

    assert_eq!(error.category(), ErrorCategory::Validation);
    assert_eq!(error.severity(), ErrorSeverity::Error);
    assert_eq!(error.code(), 4008); // INVALID_ENUM_VALUE
    assert!(error.message().contains("字段 'role' 值 'superuser' 无效"));
    assert!(error.message().contains("admin, user, guest"));
    assert!(error.context().is_empty());
}

/// 测试重复值错误
#[test]
fn test_duplicate_value_error() {
    let error = duplicate_value("username", "johndoe");

    assert_eq!(error.category(), ErrorCategory::Validation);
    assert_eq!(error.severity(), ErrorSeverity::Error);
    assert_eq!(error.code(), 4009); // DUPLICATE_VALUE
    assert!(
        error
            .message()
            .contains("字段 'username' 值 'johndoe' 已存在")
    );
    assert!(error.context().is_empty());
}

/// 测试不支持的参数错误
#[test]
fn test_unsupported_parameter_error() {
    let error = unsupported_parameter("invalid_param");

    assert_eq!(error.category(), ErrorCategory::Validation);
    assert_eq!(error.severity(), ErrorSeverity::Error);
    assert_eq!(error.code(), 4010); // UNSUPPORTED_PARAMETER
    assert!(error.message().contains("不支持的参数"));
    assert!(error.message().contains("invalid_param"));
    assert!(error.context().is_empty());
}

/// 测试通用验证错误创建函数
#[test]
fn test_validation_error_creation() {
    let error = validation_error("自定义验证错误消息");

    assert_eq!(error.category(), ErrorCategory::Validation);
    assert_eq!(error.severity(), ErrorSeverity::Error);
    assert_eq!(error.code(), 4000); // VALIDATION_ERROR_BASE
    assert_eq!(error.message(), "自定义验证错误消息");
    assert!(error.context().is_empty());
}

/// 测试验证错误链（多个验证错误组合）
#[test]
fn test_validation_error_chain() {
    let empty_field_error = empty_field("email");
    let invalid_email_error = invalid_email("not-an-email");
    let field_too_short_error = field_too_short("username", 3, 2);

    // 验证所有错误都是验证类别
    assert_eq!(empty_field_error.category(), ErrorCategory::Validation);
    assert_eq!(invalid_email_error.category(), ErrorCategory::Validation);
    assert_eq!(field_too_short_error.category(), ErrorCategory::Validation);

    // 验证错误代码不同
    assert_eq!(empty_field_error.code(), 4001); // EMPTY_FIELD
    assert_eq!(invalid_email_error.code(), 4004); // INVALID_EMAIL
    assert_eq!(field_too_short_error.code(), 4002); // FIELD_TOO_SHORT

    // 验证严重级别
    assert_eq!(empty_field_error.severity(), ErrorSeverity::Error);
    assert_eq!(invalid_email_error.severity(), ErrorSeverity::Error);
    assert_eq!(field_too_short_error.severity(), ErrorSeverity::Error);
}

/// 测试验证错误的序列化（已移除 - ErrorInfo不支持序列化）
#[test]
fn test_validation_error_serialization_placeholder() {
    // ErrorInfo不支持serde序列化，此测试已移除
    // 如果需要序列化功能，需要重新设计ErrorInfo结构
    assert!(true);
}

/// 测试验证错误的错误转换
#[test]
fn test_validation_error_conversion() {
    let error = invalid_number("xyz", "age");

    // 转换为标准错误
    let std_error: Box<dyn StdError> = Box::new(error.clone());
    assert!(std_error.to_string().contains("字段 'age' 不是有效的数字"));

    // 验证错误源
    assert!(std_error.source().is_none());
}

/// 测试验证错误的显示格式
#[test]
fn test_validation_error_display() {
    let error = invalid_url("ftp://invalid");

    let display_string = format!("{}", error);
    assert!(display_string.contains("无效的URL地址"));
    assert!(display_string.contains("ftp://invalid"));
}

/// 测试验证错误的调试格式
#[test]
fn test_validation_error_debug() {
    let error = invalid_date("invalid-date");

    let debug_string = format!("{:?}", error);
    assert!(debug_string.contains("ErrorInfo"));
    assert!(debug_string.contains("4006")); // INVALID_DATE code
    assert!(debug_string.contains("Validation"));
}

/// 测试验证错误的比较
#[test]
fn test_validation_error_equality() {
    let error1 = empty_field("username");
    let error2 = empty_field("username");
    let error3 = empty_field("email");

    assert_eq!(error1.code(), error2.code());
    assert_eq!(error1.category(), error2.category());
    assert_eq!(error1.severity(), error2.severity());

    // 消息不同但代码相同
    assert_eq!(error1.code(), error3.code());
    assert_ne!(error1.message(), error3.message());
}

/// 测试验证错误的克隆
#[test]
fn test_validation_error_clone() {
    let original = unsupported_parameter("deprecated_param");

    let cloned = original.clone();

    assert_eq!(cloned.code(), original.code());
    assert_eq!(cloned.message(), original.message());
    assert_eq!(cloned.category(), original.category());
    assert_eq!(cloned.severity(), original.severity());
    assert_eq!(cloned.context(), original.context());
}

/// 测试复杂验证场景（多个验证错误）
#[test]
fn test_complex_validation_scenario() {
    // 模拟用户注册时的多个验证错误
    let errors = vec![
        empty_field("username"),
        field_too_short("password", 8, 5),
        invalid_email("not-an-email"),
        duplicate_value("username", "johndoe"),
    ];

    // 验证所有错误
    for error in &errors {
        assert_eq!(error.category(), ErrorCategory::Validation);
        assert_eq!(error.severity(), ErrorSeverity::Error);
    }

    // 验证具体错误代码
    assert_eq!(errors[0].code(), 4001); // EMPTY_FIELD
    assert_eq!(errors[1].code(), 4002); // FIELD_TOO_SHORT
    assert_eq!(errors[2].code(), 4004); // INVALID_EMAIL
    assert_eq!(errors[3].code(), 4009); // DUPLICATE_VALUE

    // 验证错误消息
    assert!(errors[0].message().contains("字段 'username' 不能为空"));
    assert!(errors[1].message().contains("字段 'password' 太短"));
    assert!(errors[2].message().contains("无效的邮箱地址"));
    assert!(
        errors[3]
            .message()
            .contains("字段 'username' 值 'johndoe' 已存在")
    );
}
