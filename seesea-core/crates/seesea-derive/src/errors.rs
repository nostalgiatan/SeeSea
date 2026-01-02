//! 搜索引擎派生宏错误处理模块
//!
//! 提供专门针对搜索引擎派生宏的错误处理功能，包括：
//! - 宏展开错误
//! - 属性解析错误  
//! - 类型验证错误
//! - 代码生成错误

use error::ErrorCategory;
use seesea_errors::{ErrorInfo, ErrorSeverity};

/// 派生宏错误码常量
///
/// 派生宏错误码范围：11000-11999 (避免与seesea-errors冲突)
pub const DERIVE_ERROR_BASE: u32 = 11000;
pub const MACRO_EXPANSION_ERROR: u32 = DERIVE_ERROR_BASE + 1;
pub const ATTRIBUTE_PARSE_ERROR: u32 = DERIVE_ERROR_BASE + 2;
pub const TYPE_VALIDATION_ERROR: u32 = DERIVE_ERROR_BASE + 3;
pub const CODE_GENERATION_ERROR: u32 = DERIVE_ERROR_BASE + 4;
pub const TRAIT_IMPLEMENTATION_ERROR: u32 = DERIVE_ERROR_BASE + 5;
pub const MISSING_REQUIRED_FIELD: u32 = DERIVE_ERROR_BASE + 6;
pub const INVALID_FIELD_TYPE: u32 = DERIVE_ERROR_BASE + 7;
pub const UNSUPPORTED_ATTRIBUTE: u32 = DERIVE_ERROR_BASE + 8;
pub const DUPLICATE_ATTRIBUTE: u32 = DERIVE_ERROR_BASE + 9;
pub const INVALID_ENUM_VARIANT: u32 = DERIVE_ERROR_BASE + 10;

/// 创建宏展开错误
///
/// # 参数
/// - `macro_name`: 宏名称
/// - `details`: 错误详情
///
/// # 返回
/// 包含宏展开错误信息的错误对象
pub fn macro_expansion_error(macro_name: &str, details: &str) -> ErrorInfo {
    ErrorInfo::new(
        MACRO_EXPANSION_ERROR,
        format!("宏 '{}' 展开失败: {}", macro_name, details),
    )
    .with_category(ErrorCategory::Parse)
    .with_severity(ErrorSeverity::Error)
}

/// 创建属性解析错误
///
/// # 参数
/// - `attribute`: 属性名称
/// - `value`: 属性值
/// - `expected_format`: 期望的格式
///
/// # 返回
/// 包含属性解析错误信息的错误对象
pub fn attribute_parse_error(attribute: &str, value: &str, expected_format: &str) -> ErrorInfo {
    ErrorInfo::new(
        ATTRIBUTE_PARSE_ERROR,
        format!(
            "属性 '{}' 值 '{}' 解析失败: 期望格式: {}",
            attribute, value, expected_format
        ),
    )
    .with_category(ErrorCategory::Parse)
    .with_severity(ErrorSeverity::Error)
}

/// 创建类型验证错误
///
/// # 参数
/// - `field_name`: 字段名称
/// - `expected_type`: 期望的类型
/// - `actual_type`: 实际类型
///
/// # 返回
/// 包含类型验证错误信息的错误对象
pub fn type_validation_error(
    field_name: &str,
    expected_type: &str,
    actual_type: &str,
) -> ErrorInfo {
    ErrorInfo::new(
        TYPE_VALIDATION_ERROR,
        format!(
            "字段 '{}' 类型验证失败: 期望 '{}', 实际 '{}'",
            field_name, expected_type, actual_type
        ),
    )
    .with_category(ErrorCategory::Validation)
    .with_severity(ErrorSeverity::Error)
}

/// 创建代码生成错误
///
/// # 参数
/// - `context`: 错误上下文
/// - `details`: 错误详情
///
/// # 返回
/// 包含代码生成错误信息的错误对象
pub fn code_generation_error(context: &str, details: &str) -> ErrorInfo {
    ErrorInfo::new(
        CODE_GENERATION_ERROR,
        format!("代码生成失败 [{}]: {}", context, details),
    )
    .with_category(ErrorCategory::Parse)
    .with_severity(ErrorSeverity::Error)
}

/// 创建trait实现错误
///
/// # 参数
/// - `trait_name`: trait名称
/// - `struct_name`: 结构体名称
/// - `missing_methods`: 缺少的方法列表
///
/// # 返回
/// 包含trait实现错误信息的错误对象
pub fn trait_implementation_error(
    trait_name: &str,
    struct_name: &str,
    missing_methods: &[&str],
) -> ErrorInfo {
    let methods = missing_methods.join(", ");
    ErrorInfo::new(
        TRAIT_IMPLEMENTATION_ERROR,
        format!(
            "结构体 '{}' 实现 trait '{}' 失败: 缺少方法: [{}]",
            struct_name, trait_name, methods
        ),
    )
    .with_category(ErrorCategory::Validation)
    .with_severity(ErrorSeverity::Error)
}

/// 创建缺少必填字段错误
///
/// # 参数
/// - `field_name`: 字段名称
/// - `struct_name`: 结构体名称
///
/// # 返回
/// 包含缺少必填字段错误信息的错误对象
pub fn missing_required_field(field_name: &str, struct_name: &str) -> ErrorInfo {
    ErrorInfo::new(
        MISSING_REQUIRED_FIELD,
        format!("结构体 '{}' 缺少必填字段: '{}'", struct_name, field_name),
    )
    .with_category(ErrorCategory::Validation)
    .with_severity(ErrorSeverity::Error)
}

/// 创建无效字段类型错误
///
/// # 参数
/// - `field_name`: 字段名称
/// - `expected_types`: 期望的类型列表
/// - `actual_type`: 实际类型
///
/// # 返回
/// 包含无效字段类型错误信息的错误对象
pub fn invalid_field_type(
    field_name: &str,
    expected_types: &[&str],
    actual_type: &str,
) -> ErrorInfo {
    let expected = expected_types.join(" | ");
    ErrorInfo::new(
        INVALID_FIELD_TYPE,
        format!(
            "字段 '{}' 类型无效: 期望 [{}], 实际 '{}'",
            field_name, expected, actual_type
        ),
    )
    .with_category(ErrorCategory::Validation)
    .with_severity(ErrorSeverity::Error)
}

/// 创建不支持的属性错误
///
/// # 参数
/// - `attribute`: 属性名称
/// - `supported_attributes`: 支持的属性列表
///
/// # 返回
/// 包含不支持的属性错误信息的错误对象
pub fn unsupported_attribute(attribute: &str, supported_attributes: &[&str]) -> ErrorInfo {
    let supported = supported_attributes.join(", ");
    ErrorInfo::new(
        UNSUPPORTED_ATTRIBUTE,
        format!("属性 '{}' 不受支持: 支持的属性: [{}]", attribute, supported),
    )
    .with_category(ErrorCategory::Validation)
    .with_severity(ErrorSeverity::Error)
}

/// 创建重复属性错误
///
/// # 参数
/// - `attribute`: 属性名称
///
/// # 返回
/// 包含重复属性错误信息的错误对象
pub fn duplicate_attribute(attribute: &str) -> ErrorInfo {
    ErrorInfo::new(
        DUPLICATE_ATTRIBUTE,
        format!("属性 '{}' 重复定义", attribute),
    )
    .with_category(ErrorCategory::Validation)
    .with_severity(ErrorSeverity::Error)
}

/// 创建无效枚举变体错误
///
/// # 参数
/// - `variant`: 变体名称
/// - `enum_name`: 枚举名称
/// - `allowed_variants`: 允许的变体列表
///
/// # 返回
/// 包含无效枚举变体错误信息的错误对象
pub fn invalid_enum_variant(
    variant: &str,
    enum_name: &str,
    allowed_variants: &[&str],
) -> ErrorInfo {
    let allowed = allowed_variants.join(", ");
    ErrorInfo::new(
        INVALID_ENUM_VARIANT,
        format!(
            "枚举 '{}' 变体 '{}' 无效: 允许的变体: [{}]",
            enum_name, variant, allowed
        ),
    )
    .with_category(ErrorCategory::Validation)
    .with_severity(ErrorSeverity::Error)
}

/// 创建搜索引擎配置错误
///
/// # 参数
/// - `engine_name`: 引擎名称
/// - `config_field`: 配置字段
/// - `details`: 错误详情
///
/// # 返回
/// 包含搜索引擎配置错误信息的错误对象
pub fn engine_config_error(engine_name: &str, config_field: &str, details: &str) -> ErrorInfo {
    ErrorInfo::new(
        DERIVE_ERROR_BASE + 11,
        format!(
            "搜索引擎 '{}' 配置字段 '{}' 错误: {}",
            engine_name, config_field, details
        ),
    )
    .with_category(ErrorCategory::Validation)
    .with_severity(ErrorSeverity::Error)
}

/// 创建查询构建错误
///
/// # 参数
/// - `query_field`: 查询字段
/// - `details`: 错误详情
///
/// # 返回
/// 包含查询构建错误信息的错误对象
pub fn query_build_error(query_field: &str, details: &str) -> ErrorInfo {
    ErrorInfo::new(
        DERIVE_ERROR_BASE + 12,
        format!("查询字段 '{}' 构建失败: {}", query_field, details),
    )
    .with_category(ErrorCategory::Validation)
    .with_severity(ErrorSeverity::Error)
}

/// 创建结果解析错误
///
/// # 参数
/// - `parser_type`: 解析器类型
/// - `details`: 错误详情
///
/// # 返回
/// 包含结果解析错误信息的错误对象
pub fn result_parse_error(parser_type: &str, details: &str) -> ErrorInfo {
    ErrorInfo::new(
        DERIVE_ERROR_BASE + 13,
        format!("结果解析器 '{}' 解析失败: {}", parser_type, details),
    )
    .with_category(ErrorCategory::Parse)
    .with_severity(ErrorSeverity::Error)
}

/// 创建宏参数验证错误
///
/// # 参数
/// - `param_name`: 参数名称
/// - `param_value`: 参数值
/// - `validation_rule`: 验证规则
///
/// # 返回
/// 包含宏参数验证错误信息的错误对象
pub fn macro_param_validation_error(
    param_name: &str,
    param_value: &str,
    validation_rule: &str,
) -> ErrorInfo {
    ErrorInfo::new(
        DERIVE_ERROR_BASE + 14,
        format!(
            "宏参数 '{}' 值 '{}' 验证失败: {}",
            param_name, param_value, validation_rule
        ),
    )
    .with_category(ErrorCategory::Validation)
    .with_severity(ErrorSeverity::Error)
}

/// 创建派生宏通用错误
///
/// # 参数
/// - `error_type`: 错误类型
/// - `message`: 错误消息
///
/// # 返回
/// 包含派生宏错误信息的错误对象
pub fn derive_error(error_type: &str, message: impl Into<String>) -> ErrorInfo {
    ErrorInfo::new(
        DERIVE_ERROR_BASE,
        format!("派生宏错误 [{}]: {}", error_type, message.into()),
    )
    .with_category(ErrorCategory::Validation)
    .with_severity(ErrorSeverity::Error)
}
