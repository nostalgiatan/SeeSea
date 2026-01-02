use error::ErrorCategory;
use seesea_errors::search::search_error;
use seesea_errors::*;
use std::error::Error as StdError;

/// 测试搜索错误创建
#[test]
fn test_search_error_creation() {
    let error = search_error("搜索参数无效");

    assert_eq!(error.category(), ErrorCategory::Search);
    assert_eq!(error.severity(), ErrorSeverity::Error);
    assert_eq!(error.code(), 2000); // SEARCH_ERROR_BASE
    assert!(error.message().contains("搜索参数无效"));
    assert!(error.context().is_empty());
}

/// 测试引擎不可用错误
#[test]
fn test_engine_unavailable() {
    let error = engine_unavailable("Elasticsearch");

    assert_eq!(error.category(), ErrorCategory::Search);
    assert_eq!(error.severity(), ErrorSeverity::Error);
    assert_eq!(error.code(), 2001); // ENGINE_UNAVAILABLE
    assert!(error.message().contains("搜索引擎"));
    assert!(error.message().contains("Elasticsearch"));
    assert!(error.message().contains("不可用"));
}

/// 测试搜索超时错误
#[test]
fn test_search_timeout() {
    let error = search_timeout("Elasticsearch");

    assert_eq!(error.category(), ErrorCategory::Search);
    assert_eq!(error.severity(), ErrorSeverity::Error);
    assert_eq!(error.code(), 2002); // SEARCH_TIMEOUT
    assert!(error.message().contains("搜索超时"));
    assert!(error.message().contains("Elasticsearch"));
}

/// 测试零结果错误
#[test]
fn test_zero_results() {
    let error = zero_results("Elasticsearch");

    assert_eq!(error.category(), ErrorCategory::Search);
    assert_eq!(error.severity(), ErrorSeverity::Warning);
    assert_eq!(error.code(), 2003); // ZERO_RESULTS
    assert!(error.message().contains("返回零结果"));
    assert!(error.message().contains("Elasticsearch"));
}

/// 测试无效查询错误
#[test]
fn test_invalid_query() {
    let error = invalid_query("rust programming", "缺少引号");

    assert_eq!(error.category(), ErrorCategory::Search);
    assert_eq!(error.severity(), ErrorSeverity::Error);
    assert_eq!(error.code(), 2004); // INVALID_QUERY
    assert!(error.message().contains("无效查询"));
    assert!(error.message().contains("rust programming"));
    assert!(error.message().contains("缺少引号"));
}

/// 测试不支持的搜索类型错误
#[test]
fn test_unsupported_search_type() {
    let error = unsupported_search_type("语义搜索", "Elasticsearch");

    assert_eq!(error.category(), ErrorCategory::Search);
    assert_eq!(error.severity(), ErrorSeverity::Error);
    assert_eq!(error.code(), 2005); // UNSUPPORTED_SEARCH_TYPE
    assert!(error.message().contains("不支持"));
    assert!(error.message().contains("语义搜索"));
    assert!(error.message().contains("Elasticsearch"));
}

/// 测试引擎错误
#[test]
fn test_engine_error() {
    let error = engine_error("Elasticsearch", "连接超时");

    assert_eq!(error.category(), ErrorCategory::Search);
    assert_eq!(error.severity(), ErrorSeverity::Error);
    assert_eq!(error.code(), 2006); // ENGINE_ERROR
    assert!(error.message().contains("搜索引擎"));
    assert!(error.message().contains("Elasticsearch"));
    assert!(error.message().contains("连接超时"));
}

/// 测试搜索结果错误创建
#[test]
fn test_search_result_error_creation() {
    let error = result_parse_failed("Elasticsearch", "返回结果数量不匹配");

    assert_eq!(error.category(), ErrorCategory::Search);
    assert_eq!(error.severity(), ErrorSeverity::Error);
    assert_eq!(error.code(), 2007); // RESULT_PARSE_FAILED
    assert!(error.message().contains("结果解析失败"));
}

/// 测试搜索配置错误创建
#[test]
fn test_search_config_error_creation() {
    let error = invalid_search_scope("最大返回结果数设置过小");

    assert_eq!(error.category(), ErrorCategory::Search);
    assert_eq!(error.severity(), ErrorSeverity::Error);
    assert_eq!(error.code(), 2010); // INVALID_SEARCH_SCOPE
    assert!(error.message().contains("最大返回结果数设置过小"));
}

/// 测试搜索错误链（多个错误组合）
#[test]
fn test_search_error_chain() {
    let parse_error = invalid_query("rust programming", "语法错误");
    let timeout_error = search_timeout("Elasticsearch");

    // 验证两个错误都是搜索类别
    assert_eq!(parse_error.category(), ErrorCategory::Search);
    assert_eq!(timeout_error.category(), ErrorCategory::Search);

    // 验证错误代码不同
    assert_ne!(parse_error.code(), timeout_error.code());
    assert_eq!(parse_error.code(), 2004); // INVALID_QUERY
    assert_eq!(timeout_error.code(), 2002); // SEARCH_TIMEOUT
}

/// 测试搜索错误的错误转换
#[test]
fn test_search_error_conversion() {
    let error = search_error("搜索失败");

    // 转换为标准错误
    let std_error: Box<dyn StdError> = Box::new(error.clone());
    assert!(std_error.to_string().contains("搜索失败"));

    // 验证错误源
    assert!(std_error.source().is_none());
}

/// 测试搜索错误的显示格式
#[test]
fn test_search_error_display() {
    let error = search_error("搜索失败");

    let display_string = format!("{}", error);
    assert!(display_string.contains("搜索失败"));
    assert!(display_string.contains("2000")); // SEARCH_ERROR_BASE code
}

/// 测试搜索错误的调试格式
#[test]
fn test_search_error_debug() {
    let error = search_error("搜索失败");

    let debug_string = format!("{:?}", error);
    assert!(debug_string.contains("ErrorInfo"));
}

/// 测试搜索错误的比较
#[test]
fn test_search_error_equality() {
    let error1 = search_error("搜索失败");
    let error2 = search_error("搜索失败");
    let error3 = search_error("索引损坏");

    assert_eq!(error1.code(), error2.code());
    assert_eq!(error1.category(), error2.category());
    assert_eq!(error1.severity(), error2.severity());

    // 消息不同但代码相同
    assert_eq!(error1.code(), error3.code());
    assert_ne!(error1.message(), error3.message());
}

/// 测试搜索错误的克隆
#[test]
fn test_search_error_clone() {
    let original = search_error("搜索失败");

    let cloned = original.clone();

    assert_eq!(cloned.code(), original.code());
    assert_eq!(cloned.message(), original.message());
    assert_eq!(cloned.category(), original.category());
    assert_eq!(cloned.severity(), original.severity());
}

/// 测试结果解析失败错误
#[test]
fn test_result_parse_failed() {
    let error = result_parse_failed("Elasticsearch", "JSON格式错误");

    assert_eq!(error.category(), ErrorCategory::Search);
    assert_eq!(error.severity(), ErrorSeverity::Error);
    assert_eq!(error.code(), 2007); // RESULT_PARSE_FAILED
    assert!(error.message().contains("结果解析失败"));
    assert!(error.message().contains("Elasticsearch"));
    assert!(error.message().contains("JSON格式错误"));
}

/// 测试搜索速率限制错误
#[test]
fn test_search_rate_limited() {
    let error = search_rate_limited("Elasticsearch");

    assert_eq!(error.category(), ErrorCategory::Search);
    assert_eq!(error.severity(), ErrorSeverity::Warning);
    assert_eq!(error.code(), 2008); // SEARCH_RATE_LIMITED
    assert!(error.message().contains("搜索频率受限"));
    assert!(error.message().contains("Elasticsearch"));
}

/// 测试搜索深度过大错误
#[test]
fn test_search_depth_too_large() {
    let error = search_depth_too_large(100, 50);

    assert_eq!(error.category(), ErrorCategory::Search);
    assert_eq!(error.severity(), ErrorSeverity::Error);
    assert_eq!(error.code(), 2009); // SEARCH_DEPTH_TOO_LARGE
    assert!(error.message().contains("搜索深度"));
    assert!(error.message().contains("100"));
    assert!(error.message().contains("50"));
}

/// 测试无效搜索范围错误
#[test]
fn test_invalid_search_scope() {
    let error = invalid_search_scope("invalid_scope");

    assert_eq!(error.category(), ErrorCategory::Search);
    assert_eq!(error.severity(), ErrorSeverity::Error);
    assert_eq!(error.code(), 2010); // INVALID_SEARCH_SCOPE
    assert!(error.message().contains("无效搜索范围"));
    assert!(error.message().contains("invalid_scope"));
}

/// 测试搜索错误的默认值
#[test]
fn test_search_error_default() {
    let error = search_error("默认搜索错误");

    assert_eq!(error.category(), ErrorCategory::Search);
    assert_eq!(error.severity(), ErrorSeverity::Error);
    assert_eq!(error.code(), 2000); // SEARCH_ERROR_BASE
    assert_eq!(error.message(), "默认搜索错误");
}
