//! API 错误处理测试模块
//!
//! 测试各种 API 错误场景和处理逻辑

use seesea_api::api::types::*;
use serde_json;

#[test]
fn test_api_error_response_serialization() {
    // 测试错误响应序列化
    let error_response = ApiErrorResponse {
        code: "INVALID_QUERY".to_string(),
        message: "查询参数不能为空".to_string(),
        details: Some("请提供 'query' 或 'q' 参数".to_string()),
    };

    let json = serde_json::to_string(&error_response).unwrap();
    assert!(json.contains("INVALID_QUERY"));
    assert!(json.contains("查询参数不能为空"));
    assert!(json.contains("请提供 'query' 或 'q' 参数"));
}

#[test]
fn test_api_error_response_deserialization() {
    // 测试错误响应反序列化
    let json = r#"{
        "code": "RATE_LIMIT_EXCEEDED",
        "message": "请求频率超出限制",
        "details": "请等待 60 秒后再试"
    }"#;

    let error_response: ApiErrorResponse = serde_json::from_str(json).unwrap();
    assert_eq!(error_response.code, "RATE_LIMIT_EXCEEDED");
    assert_eq!(error_response.message, "请求频率超出限制");
    assert_eq!(error_response.details.unwrap(), "请等待 60 秒后再试");
}

#[test]
fn test_api_error_response_without_optional_fields() {
    // 测试没有可选字段的错误响应
    let error_response = ApiErrorResponse {
        code: "INTERNAL_ERROR".to_string(),
        message: "内部服务器错误".to_string(),
        details: None,
    };

    let json = serde_json::to_string(&error_response).unwrap();
    assert!(json.contains("INTERNAL_ERROR"));
    assert!(json.contains("内部服务器错误"));
    assert!(!json.contains("details"));
}

#[test]
fn test_validation_error_scenarios() {
    // 测试各种验证错误场景
    let validation_errors = vec![
        ApiErrorResponse {
            code: "INVALID_QUERY".to_string(),
            message: "查询参数不能为空".to_string(),
            details: Some("请提供有效的搜索查询".to_string()),
        },
        ApiErrorResponse {
            code: "INVALID_PAGE".to_string(),
            message: "页码必须在 1-1000 之间".to_string(),
            details: Some("当前页码: 0".to_string()),
        },
        ApiErrorResponse {
            code: "INVALID_PAGE_SIZE".to_string(),
            message: "页面大小必须在 1-100 之间".to_string(),
            details: Some("当前页面大小: 0".to_string()),
        },
    ];

    for error in validation_errors {
        let json = serde_json::to_string(&error).unwrap();
        assert!(json.contains(&error.code));
        assert!(json.contains(&error.message));
        if let Some(details) = &error.details {
            assert!(json.contains(details));
        }
    }
}

#[test]
fn test_rate_limit_error_scenarios() {
    // 测试速率限制错误场景
    let rate_limit_error = ApiErrorResponse {
        code: "RATE_LIMIT_EXCEEDED".to_string(),
        message: "请求频率超出限制".to_string(),
        details: Some("每分钟最多允许 60 个请求，请等待 45 秒后再试".to_string()),
    };

    let json = serde_json::to_string(&rate_limit_error).unwrap();
    assert!(json.contains("RATE_LIMIT_EXCEEDED"));
    assert!(json.contains("请求频率超出限制"));
    assert!(json.contains("每分钟最多允许 60 个请求"));
}

#[test]
fn test_authentication_error_scenarios() {
    // 测试认证错误场景
    let auth_errors = vec![
        ApiErrorResponse {
            code: "INVALID_API_KEY".to_string(),
            message: "无效的 API 密钥".to_string(),
            details: Some("请检查您的 API 密钥是否正确".to_string()),
        },
        ApiErrorResponse {
            code: "API_KEY_EXPIRED".to_string(),
            message: "API 密钥已过期".to_string(),
            details: Some("请更新您的 API 密钥".to_string()),
        },
        ApiErrorResponse {
            code: "INSUFFICIENT_PERMISSIONS".to_string(),
            message: "权限不足".to_string(),
            details: Some("您的账户没有访问此资源的权限".to_string()),
        },
    ];

    for error in auth_errors {
        let json = serde_json::to_string(&error).unwrap();
        assert!(json.contains(&error.code));
        assert!(json.contains(&error.message));
        if let Some(details) = &error.details {
            assert!(json.contains(details));
        }
    }
}

#[test]
fn test_engine_specific_error_scenarios() {
    // 测试搜索引擎特定的错误场景
    let engine_errors = vec![
        ApiErrorResponse {
            code: "ENGINE_UNAVAILABLE".to_string(),
            message: "搜索引擎不可用".to_string(),
            details: Some("Google 搜索引擎暂时不可用，请稍后再试".to_string()),
        },
        ApiErrorResponse {
            code: "ENGINE_TIMEOUT".to_string(),
            message: "搜索引擎请求超时".to_string(),
            details: Some("Bing 搜索引擎响应超时（30秒）".to_string()),
        },
        ApiErrorResponse {
            code: "ENGINE_RATE_LIMIT".to_string(),
            message: "搜索引擎速率限制".to_string(),
            details: Some("DuckDuckGo 搜索引擎速率限制，请等待 60 秒".to_string()),
        },
    ];

    for error in engine_errors {
        let json = serde_json::to_string(&error).unwrap();
        assert!(json.contains(&error.code));
        assert!(json.contains(&error.message));
        if let Some(details) = &error.details {
            assert!(json.contains(details));
        }
    }
}

#[test]
fn test_network_timeout_error_scenarios() {
    // 测试网络超时错误场景
    let timeout_error = ApiErrorResponse {
        code: "NETWORK_TIMEOUT".to_string(),
        message: "网络请求超时".to_string(),
        details: Some("连接到搜索引擎的网络请求超时（配置：30秒）".to_string()),
    };

    let json = serde_json::to_string(&timeout_error).unwrap();
    assert!(json.contains("NETWORK_TIMEOUT"));
    assert!(json.contains("网络请求超时"));
    assert!(json.contains("30秒"));
}

#[test]
fn test_internal_server_error_scenarios() {
    // 测试内部服务器错误场景
    let internal_errors = vec![
        ApiErrorResponse {
            code: "INTERNAL_ERROR".to_string(),
            message: "内部服务器错误".to_string(),
            details: Some("处理搜索请求时发生未知错误".to_string()),
        },
        ApiErrorResponse {
            code: "DATABASE_ERROR".to_string(),
            message: "数据库错误".to_string(),
            details: Some("无法连接到缓存数据库".to_string()),
        },
        ApiErrorResponse {
            code: "CACHE_ERROR".to_string(),
            message: "缓存错误".to_string(),
            details: Some("缓存系统不可用".to_string()),
        },
    ];

    for error in internal_errors {
        let json = serde_json::to_string(&error).unwrap();
        assert!(json.contains(&error.code));
        assert!(json.contains(&error.message));
        if let Some(details) = &error.details {
            assert!(json.contains(details));
        }
    }
}

#[test]
fn test_error_response_consistency() {
    // 测试错误响应的一致性
    let error = ApiErrorResponse {
        code: "TEST_ERROR".to_string(),
        message: "测试错误".to_string(),
        details: Some("这是一个测试错误".to_string()),
    };

    // 序列化和反序列化往返测试
    let json = serde_json::to_string(&error).unwrap();
    let deserialized: ApiErrorResponse = serde_json::from_str(&json).unwrap();

    assert_eq!(error.code, deserialized.code);
    assert_eq!(error.message, deserialized.message);
    assert_eq!(error.details, deserialized.details);
}
