//! 错误处理测试

use error::{ErrorCategory, ErrorSeverity};
use seesea_derive::errors::*;

#[test]
fn test_macro_expansion_error() {
    let error = macro_expansion_error("engine_metadata!", "无法识别的属性");
    assert_eq!(error.code(), MACRO_EXPANSION_ERROR);
    assert!(error.message().contains("engine_metadata!"));
    assert!(error.message().contains("展开失败"));
    assert_eq!(error.category(), ErrorCategory::Parse);
    assert_eq!(error.severity(), ErrorSeverity::Error);
}

#[test]
fn test_attribute_parse_error() {
    let error = attribute_parse_error("categories", "[invalid", "数组格式");
    assert_eq!(error.code(), ATTRIBUTE_PARSE_ERROR);
    assert!(error.message().contains("categories"));
    assert!(error.message().contains("解析失败"));
    assert!(error.message().contains("数组格式"));
}

#[test]
fn test_type_validation_error() {
    let error = type_validation_error("page_size", "u32", "String");
    assert_eq!(error.code(), TYPE_VALIDATION_ERROR);
    assert!(error.message().contains("page_size"));
    assert!(error.message().contains("类型验证失败"));
    assert!(error.message().contains("u32"));
    assert!(error.message().contains("String"));
}

#[test]
fn test_code_generation_error() {
    let error = code_generation_error("impl SearchEngine", "缺少必要方法");
    assert_eq!(error.code(), CODE_GENERATION_ERROR);
    assert!(error.message().contains("代码生成失败"));
    assert!(error.message().contains("impl SearchEngine"));
}

#[test]
fn test_trait_implementation_error() {
    let missing_methods = vec!["search", "info"];
    let error = trait_implementation_error("SearchEngine", "MyEngine", &missing_methods);
    assert_eq!(error.code(), TRAIT_IMPLEMENTATION_ERROR);
    assert!(error.message().contains("MyEngine"));
    assert!(error.message().contains("SearchEngine"));
    assert!(error.message().contains("缺少方法"));
    assert!(error.message().contains("search"));
    assert!(error.message().contains("info"));
}

#[test]
fn test_missing_required_field() {
    let error = missing_required_field("name", "EngineInfo");
    assert_eq!(error.code(), MISSING_REQUIRED_FIELD);
    assert!(error.message().contains("EngineInfo"));
    assert!(error.message().contains("缺少必填字段"));
    assert!(error.message().contains("name"));
}

#[test]
fn test_invalid_field_type() {
    let expected_types = vec!["String", "&str"];
    let error = invalid_field_type("title", &expected_types, "i32");
    assert_eq!(error.code(), INVALID_FIELD_TYPE);
    assert!(error.message().contains("title"));
    assert!(error.message().contains("类型无效"));
    assert!(error.message().contains("String"));
    assert!(error.message().contains("&str"));
    assert!(error.message().contains("i32"));
}

#[test]
fn test_unsupported_attribute() {
    let supported = vec!["name", "categories", "paging"];
    let error = unsupported_attribute("invalid_attr", &supported);
    assert_eq!(error.code(), UNSUPPORTED_ATTRIBUTE);
    assert!(error.message().contains("invalid_attr"));
    assert!(error.message().contains("不受支持"));
    assert!(error.message().contains("name"));
    assert!(error.message().contains("categories"));
    assert!(error.message().contains("paging"));
}

#[test]
fn test_duplicate_attribute() {
    let error = duplicate_attribute("categories");
    assert_eq!(error.code(), DUPLICATE_ATTRIBUTE);
    assert!(error.message().contains("categories"));
    assert!(error.message().contains("重复定义"));
}

#[test]
fn test_invalid_enum_variant() {
    let allowed = vec!["Web", "Image", "Video", "News"];
    let error = invalid_enum_variant("InvalidType", "EngineType", &allowed);
    assert_eq!(error.code(), INVALID_ENUM_VARIANT);
    assert!(error.message().contains("EngineType"));
    assert!(error.message().contains("InvalidType"));
    assert!(error.message().contains("变体"));
    assert!(error.message().contains("Web"));
    assert!(error.message().contains("Image"));
}

#[test]
fn test_engine_config_error() {
    let error = engine_config_error("GoogleEngine", "api_key", "不能为空");
    assert_eq!(error.code(), 11011);
    assert!(error.message().contains("GoogleEngine"));
    assert!(error.message().contains("配置字段"));
    assert!(error.message().contains("api_key"));
}

#[test]
fn test_query_build_error() {
    let error = query_build_error("query", "包含非法字符");
    assert_eq!(error.code(), 11012);
    assert!(error.message().contains("查询字段"));
    assert!(error.message().contains("query"));
    assert!(error.message().contains("构建失败"));
}

#[test]
fn test_result_parse_error() {
    let error = result_parse_error("JsonParser", "JSON格式错误");
    assert_eq!(error.code(), 11013);
    assert!(error.message().contains("结果解析器"));
    assert!(error.message().contains("JsonParser"));
    assert!(error.message().contains("解析失败"));
}

#[test]
fn test_macro_param_validation_error() {
    let error = macro_param_validation_error("timeout", "0", "必须为正数");
    assert_eq!(error.code(), 11014);
    assert!(error.message().contains("宏参数"));
    assert!(error.message().contains("timeout"));
    assert!(error.message().contains("验证失败"));
}

#[test]
fn test_derive_error() {
    let error = derive_error("宏展开", "无法识别的语法");
    assert_eq!(error.code(), DERIVE_ERROR_BASE);
    assert!(error.message().contains("派生宏错误"));
    assert!(error.message().contains("宏展开"));
    assert!(error.message().contains("无法识别的语法"));
}

#[test]
fn test_error_categories_and_severities() {
    let errors = vec![
        macro_expansion_error("test", "test"),
        attribute_parse_error("test", "test", "test"),
        code_generation_error("test", "test"),
        result_parse_error("test", "test"),
    ];

    for error in errors {
        assert!(
            error.category() == ErrorCategory::Parse
                || error.category() == ErrorCategory::Validation
        );
        assert_eq!(error.severity(), ErrorSeverity::Error);
    }
}

#[test]
fn test_error_message_formatting() {
    // 测试错误消息的格式化和内容
    let error = type_validation_error("engine_type", "EngineType", "String");
    let message = error.message();

    assert!(message.contains("字段"));
    assert!(message.contains("engine_type"));
    assert!(message.contains("类型验证失败"));
    assert!(message.contains("期望"));
    assert!(message.contains("EngineType"));
    assert!(message.contains("实际"));
    assert!(message.contains("String"));
}

#[test]
fn test_error_chaining_and_context() {
    // 测试错误链和上下文信息
    let base_error = derive_error("初始化", "基础配置错误");
    assert_eq!(base_error.code(), DERIVE_ERROR_BASE);

    let detail_error = engine_config_error("TestEngine", "timeout", "值超出范围");
    assert_eq!(detail_error.code(), 11011);

    // 验证错误信息完整性
    assert!(!base_error.message().is_empty());
    assert!(!detail_error.message().is_empty());
}
