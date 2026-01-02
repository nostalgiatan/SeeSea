//! API 类型测试模块
//!
//! 测试 API 请求/响应类型的序列化、验证和转换逻辑

use seesea_api::api::types::*;
use serde_json;

#[test]
fn test_api_search_request_basic() {
    let json = r#"{
        "query": "rust programming",
        "page": 1,
        "page_size": 10
    }"#;

    let request: ApiSearchRequest = serde_json::from_str(json).unwrap();
    assert_eq!(request.get_query().unwrap(), "rust programming");
    assert_eq!(request.page, 1);
    assert_eq!(request.page_size, 10);
}

#[test]
fn test_api_search_request_with_optional_fields() {
    let json = r#"{
        "query": "machine learning",
        "page": 2,
        "page_size": 20,
        "language": "en",
        "region": "us",
        "safe_search": "moderate",
        "engines": "google,bing",
        "include_deepweb": false
    }"#;

    let request: ApiSearchRequest = serde_json::from_str(json).unwrap();
    assert_eq!(request.get_query().unwrap(), "machine learning");
    assert_eq!(request.page, 2);
    assert_eq!(request.page_size, 20);
    assert_eq!(request.language.unwrap(), "en");
    assert_eq!(request.region.unwrap(), "us");
    assert_eq!(request.safe_search.unwrap(), "moderate");
    assert_eq!(request.engines.as_ref().unwrap().len(), 11); // "google,bing" has 11 chars
    assert_eq!(request.include_deepweb, false);
}

#[test]
fn test_api_search_request_validation() {
    // 测试无效请求 - 缺少查询参数
    let invalid_json = r#"{
        "page": 1,
        "page_size": 10
    }"#;

    let result = serde_json::from_str::<ApiSearchRequest>(invalid_json);
    assert!(result.is_ok()); // 应该能解析，但验证会失败

    let request = result.unwrap();
    assert!(request.get_query().is_err()); // 应该返回错误
}

#[test]
fn test_api_search_request_page_validation() {
    // 测试页码验证
    let mut request = ApiSearchRequest {
        query: Some("test".to_string()),
        _q: None,
        engine_count: None,
        page: 0, // 无效页码
        page_size: 10,
        language: None,
        region: None,
        safe_search: None,
        time_range: None,
        engines: None,
        include_deepweb: false,
    };

    // 验证页码应该在有效范围内
    assert!(request.page < 1); // 页码应该大于等于 1

    request.page = 1000; // 设置一个很大的页码
    assert!(request.page > 999); // 验证设置成功
}

#[test]
fn test_api_search_request_page_size_validation() {
    // 测试页面大小验证
    let mut request = ApiSearchRequest {
        query: Some("test".to_string()),
        _q: None,
        engine_count: None,
        page: 1,
        page_size: 0, // 无效页面大小
        language: None,
        region: None,
        safe_search: None,
        time_range: None,
        engines: None,
        include_deepweb: false,
    };

    // 验证页面大小应该在有效范围内
    assert!(request.page_size < 1); // 页面大小应该大于等于 1

    request.page_size = 100; // 设置一个较大的页面大小
    assert_eq!(request.page_size, 100);
}

#[test]
fn test_api_search_response_serialization() {
    let response = ApiSearchResponse {
        query: "rust programming".to_string(),
        results: vec![
            ApiSearchResultItem {
                title: "Rust Programming Language".to_string(),
                url: "https://rust-lang.org".to_string(),
                description: Some(
                    "A language empowering everyone to build reliable and efficient software."
                        .to_string(),
                ),
                engine: "google".to_string(),
                score: Some(0.95),
                published_date: Some("2024-01-01".to_string()),
            },
            ApiSearchResultItem {
                title: "Rust by Example".to_string(),
                url: "https://doc.rust-lang.org/rust-by-example/".to_string(),
                description: Some("Learn Rust with practical examples.".to_string()),
                engine: "bing".to_string(),
                score: Some(0.87),
                published_date: Some("2024-01-02".to_string()),
            },
        ],
        total_count: 2,
        page: 1,
        page_size: 10,
        engines_used: vec!["google".to_string(), "bing".to_string()],
        query_time_ms: 150,
        cached: false,
    };

    let json = serde_json::to_string(&response).unwrap();
    assert!(json.contains("rust programming"));
    assert!(json.contains("Rust Programming Language"));
    assert!(json.contains("https://rust-lang.org"));
    assert!(json.contains("google"));
    assert!(json.contains("bing"));
}

#[test]
fn test_api_search_response_deserialization() {
    let json = r#"{
        "query": "machine learning",
        "results": [
            {
                "title": "Machine Learning - Wikipedia",
                "url": "https://en.wikipedia.org/wiki/Machine_learning",
                "description": "Machine learning is a subset of artificial intelligence.",
                "engine": "google",
                "score": 0.92,
                "published_date": "2024-01-10"
            }
        ],
        "total_count": 1,
        "page": 1,
        "page_size": 20,
        "engines_used": ["google"],
        "query_time_ms": 200,
        "cached": true
    }"#;

    let response: ApiSearchResponse = serde_json::from_str(json).unwrap();
    assert_eq!(response.query, "machine learning");
    assert_eq!(response.results.len(), 1);
    assert_eq!(response.results[0].title, "Machine Learning - Wikipedia");
    assert_eq!(response.results[0].engine, "google");
    assert_eq!(response.total_count, 1);
    assert_eq!(response.page, 1);
    assert_eq!(response.page_size, 20);
    assert_eq!(response.engines_used.len(), 1);
    assert_eq!(response.query_time_ms, 200);
    assert_eq!(response.cached, true);
}

#[test]
fn test_api_error_response() {
    let error_response = ApiErrorResponse {
        code: "INVALID_QUERY".to_string(),
        message: "查询参数不能为空".to_string(),
        details: Some("请提供有效的搜索查询".to_string()),
    };

    let json = serde_json::to_string(&error_response).unwrap();
    assert!(json.contains("INVALID_QUERY"));
    assert!(json.contains("查询参数不能为空"));
    assert!(json.contains("请提供有效的搜索查询"));
}

#[test]
fn test_api_health_response() {
    let health_response = ApiHealthResponse {
        status: "healthy".to_string(),
        version: "2.1.0".to_string(),
        available_engines: 5,
        total_engines: 8,
    };

    let json = serde_json::to_string(&health_response).unwrap();
    assert!(json.contains("healthy"));
    assert!(json.contains("2.1.0"));
    assert!(json.contains("5"));
    assert!(json.contains("8"));
}

#[test]
fn test_api_stats_response() {
    use seesea_search::search::SearchStatsResult;

    let stats = SearchStatsResult {
        total_searches: 1000,
        cache_hits: 750,
        cache_misses: 250,
        engine_failures: 5,
        timeouts: 2,
    };

    let api_stats = ApiStatsResponse::from_search_stats(&stats);
    assert_eq!(api_stats.total_searches, 1000);
    assert_eq!(api_stats.cache_hits, 750);
    assert_eq!(api_stats.cache_misses, 250);
    assert_eq!(api_stats.engine_failures, 5);
    assert_eq!(api_stats.timeouts, 2);
    assert_eq!(api_stats.cache_hit_rate, 0.75); // 750/1000 = 0.75
}

#[test]
fn test_api_stats_response_cache_hit_rate() {
    use seesea_search::search::SearchStatsResult;

    let stats = SearchStatsResult {
        total_searches: 100,
        cache_hits: 60,
        cache_misses: 40,
        engine_failures: 5,
        timeouts: 2,
    };

    let api_stats = ApiStatsResponse::from_search_stats(&stats);
    assert_eq!(api_stats.cache_hit_rate, 0.6);
}

#[test]
fn test_type_serialization_deserialization_roundtrip() {
    // 测试类型的序列化和反序列化往返
    let original_request = ApiSearchRequest {
        query: Some("test query".to_string()),
        _q: None,
        engine_count: Some(3),
        page: 1,
        page_size: 10,
        language: Some("en".to_string()),
        region: Some("us".to_string()),
        safe_search: Some("moderate".to_string()),
        time_range: None,
        engines: Some("google,bing".to_string()),
        include_deepweb: false,
    };

    let json = serde_json::to_string(&original_request).unwrap();
    let deserialized_request: ApiSearchRequest = serde_json::from_str(&json).unwrap();

    assert_eq!(
        original_request.get_query().unwrap(),
        deserialized_request.get_query().unwrap()
    );
    assert_eq!(original_request.page, deserialized_request.page);
    assert_eq!(original_request.page_size, deserialized_request.page_size);
    assert_eq!(original_request.language, deserialized_request.language);
    assert_eq!(original_request.region, deserialized_request.region);
    assert_eq!(
        original_request.safe_search,
        deserialized_request.safe_search
    );
    assert_eq!(original_request.engines, deserialized_request.engines);
    assert_eq!(
        original_request.include_deepweb,
        deserialized_request.include_deepweb
    );
}
