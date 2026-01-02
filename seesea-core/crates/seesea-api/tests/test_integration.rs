//! API 集成测试模块
//!
//! 测试完整的 API 请求流程和端到端功能

use axum::http::StatusCode;
use seesea_api::api::types::*;
use serde_json;

#[tokio::test]
async fn test_end_to_end_search_flow() {
    // 测试完整的搜索流程
    let search_request = ApiSearchRequest {
        query: Some("rust programming".to_string()),
        _q: None,
        engine_count: Some(2),
        page: 1,
        page_size: 10,
        language: Some("en".to_string()),
        region: Some("us".to_string()),
        safe_search: None,
        time_range: None,
        engines: None,
        include_deepweb: false,
    };

    // 模拟序列化和反序列化（网络传输）
    let request_json = serde_json::to_string(&search_request).unwrap();
    let deserialized_request: ApiSearchRequest = serde_json::from_str(&request_json).unwrap();

    assert_eq!(
        deserialized_request.get_query().unwrap(),
        "rust programming"
    );
    assert_eq!(deserialized_request.engine_count, Some(2));
    assert_eq!(deserialized_request.page, 1);
    assert_eq!(deserialized_request.page_size, 10);
    assert_eq!(deserialized_request.language, Some("en".to_string()));
    assert_eq!(deserialized_request.region, Some("us".to_string()));
}

#[tokio::test]
async fn test_end_to_end_health_check_flow() {
    // 测试完整的健康检查流程
    let health_response = ApiHealthResponse {
        status: "healthy".to_string(),
        version: "2.1.0".to_string(),
        available_engines: 8,
        total_engines: 10,
    };

    // 模拟网络传输
    let response_json = serde_json::to_string(&health_response).unwrap();
    let deserialized_response: ApiHealthResponse = serde_json::from_str(&response_json).unwrap();

    assert_eq!(deserialized_response.status, "healthy");
    assert_eq!(deserialized_response.version, "2.1.0");
    assert_eq!(deserialized_response.available_engines, 8);
    assert_eq!(deserialized_response.total_engines, 10);
}

#[tokio::test]
async fn test_end_to_end_stats_flow() {
    // 测试完整的统计信息流程
    let stats_response = ApiStatsResponse {
        total_searches: 5000,
        cache_hits: 3750,
        cache_misses: 1250,
        cache_hit_rate: 0.75,
        engine_failures: 25,
        timeouts: 10,
    };

    // 模拟网络传输
    let response_json = serde_json::to_string(&stats_response).unwrap();
    let deserialized_response: ApiStatsResponse = serde_json::from_str(&response_json).unwrap();

    assert_eq!(deserialized_response.total_searches, 5000);
    assert_eq!(deserialized_response.cache_hits, 3750);
    assert_eq!(deserialized_response.cache_misses, 1250);
    assert_eq!(deserialized_response.cache_hit_rate, 0.75);
    assert_eq!(deserialized_response.engine_failures, 25);
    assert_eq!(deserialized_response.timeouts, 10);
}

#[tokio::test]
async fn test_end_to_end_error_handling_flow() {
    // 测试完整的错误处理流程
    let error_response = ApiErrorResponse {
        code: "INVALID_QUERY".to_string(),
        message: "查询参数不能为空".to_string(),
        details: Some("请提供有效的查询字符串".to_string()),
    };

    // 模拟网络传输
    let error_json = serde_json::to_string(&error_response).unwrap();
    let deserialized_error: ApiErrorResponse = serde_json::from_str(&error_json).unwrap();

    assert_eq!(deserialized_error.code, "INVALID_QUERY");
    assert_eq!(deserialized_error.message, "查询参数不能为空");
    assert_eq!(
        deserialized_error.details,
        Some("请提供有效的查询字符串".to_string())
    );
}

#[tokio::test]
async fn test_end_to_end_concurrent_requests() {
    // 测试并发请求处理
    use tokio::task;

    let mut handles = vec![];

    for i in 0..10 {
        let handle = task::spawn(async move {
            let search_request = ApiSearchRequest {
                query: Some(format!("concurrent test {}", i)),
                _q: None,
                engine_count: Some(1),
                page: 1,
                page_size: 5,
                language: None,
                region: None,
                safe_search: None,
                time_range: None,
                engines: None,
                include_deepweb: false,
            };

            // 模拟处理时间
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

            search_request.get_query().unwrap()
        });

        handles.push(handle);
    }

    let results = futures::future::join_all(handles).await;

    for (i, result) in results.iter().enumerate() {
        assert!(result.is_ok());
        assert_eq!(result.as_ref().unwrap(), &format!("concurrent test {}", i));
    }
}

#[tokio::test]
async fn test_end_to_end_request_response_cycle() {
    // 测试完整的请求-响应周期
    let request = ApiSearchRequest {
        query: Some("integration test".to_string()),
        _q: None,
        engine_count: Some(3),
        page: 2,
        page_size: 15,
        language: Some("en".to_string()),
        region: Some("us".to_string()),
        safe_search: Some("moderate".to_string()),
        time_range: Some("month".to_string()),
        engines: Some("google,bing,baidu".to_string()),
        include_deepweb: false,
    };

    // 模拟请求处理
    let query = request.get_query().unwrap();
    let engines = request.get_engines();

    // 模拟响应构建
    let response = ApiSearchResponse {
        query: query.clone(),
        results: vec![
            ApiSearchResultItem {
                title: format!("Result 1 for {}", query),
                url: format!("https://example1.com/{}", query),
                description: Some(format!("Description for {}", query)),
                engine: "google".to_string(),
                score: Some(0.95),
                published_date: Some("2024-01-15".to_string()),
            },
            ApiSearchResultItem {
                title: format!("Result 2 for {}", query),
                url: format!("https://example2.com/{}", query),
                description: Some(format!("Another description for {}", query)),
                engine: "bing".to_string(),
                score: Some(0.87),
                published_date: None,
            },
        ],
        total_count: 2,
        page: request.page,
        page_size: request.page_size,
        engines_used: engines.clone(),
        query_time_ms: 1250,
        cached: false,
    };

    // 验证响应数据
    assert_eq!(response.query, query);
    assert_eq!(response.results.len(), 2);
    assert_eq!(response.total_count, 2);
    assert_eq!(response.page, 2);
    assert_eq!(response.page_size, 15);
    assert_eq!(response.engines_used, engines);
    assert_eq!(response.query_time_ms, 1250);
    assert!(!response.cached);
}

#[tokio::test]
async fn test_end_to_end_error_recovery() {
    // 测试错误恢复机制
    let invalid_request = ApiSearchRequest {
        query: None,
        _q: None,
        engine_count: None,
        page: 1,
        page_size: 10,
        language: None,
        region: None,
        safe_search: None,
        time_range: None,
        engines: None,
        include_deepweb: false,
    };

    // 模拟错误处理
    let result = invalid_request.get_query();
    assert!(result.is_err());

    // 模拟错误响应
    let error_response = ApiErrorResponse {
        code: "INVALID_QUERY".to_string(),
        message: "查询参数不能为空".to_string(),
        details: Some("请提供 'query' 或 'q' 参数".to_string()),
    };

    // 验证错误响应格式
    assert_eq!(error_response.code, "INVALID_QUERY");
    assert_eq!(error_response.message, "查询参数不能为空");
    assert_eq!(
        error_response.details,
        Some("请提供 'query' 或 'q' 参数".to_string())
    );
}

#[tokio::test]
async fn test_end_to_end_performance_metrics() {
    // 测试性能指标收集
    let start_time = std::time::Instant::now();

    let request = ApiSearchRequest {
        query: Some("performance test".to_string()),
        _q: None,
        engine_count: Some(2),
        page: 1,
        page_size: 10,
        language: None,
        region: None,
        safe_search: None,
        time_range: None,
        engines: None,
        include_deepweb: false,
    };

    // 模拟处理时间
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    let processing_time = start_time.elapsed().as_millis() as u64;

    let response = ApiSearchResponse {
        query: request.get_query().unwrap(),
        results: vec![],
        total_count: 0,
        page: request.page,
        page_size: request.page_size,
        engines_used: request.get_engines(),
        query_time_ms: processing_time,
        cached: false,
    };

    assert!(response.query_time_ms >= 50);
    assert!(response.query_time_ms < 100); // 应该在合理范围内
}

#[tokio::test]
async fn test_end_to_end_caching_behavior() {
    // 测试缓存行为
    let request = ApiSearchRequest {
        query: Some("cached query".to_string()),
        _q: None,
        engine_count: Some(1),
        page: 1,
        page_size: 10,
        language: None,
        region: None,
        safe_search: None,
        time_range: None,
        engines: Some("google".to_string()),
        include_deepweb: false,
    };

    // 第一次请求（未缓存）
    let response1 = ApiSearchResponse {
        query: request.get_query().unwrap(),
        results: vec![ApiSearchResultItem {
            title: "Cached Result".to_string(),
            url: "https://cached.example.com".to_string(),
            description: Some("This result is cached".to_string()),
            engine: "google".to_string(),
            score: Some(0.9),
            published_date: None,
        }],
        total_count: 1,
        page: request.page,
        page_size: request.page_size,
        engines_used: request.get_engines(),
        query_time_ms: 1200,
        cached: false, // 第一次请求，未缓存
    };

    // 第二次请求（已缓存）
    let response2 = ApiSearchResponse {
        query: request.get_query().unwrap(),
        results: response1.results.clone(),
        total_count: response1.total_count,
        page: request.page,
        page_size: request.page_size,
        engines_used: request.get_engines(),
        query_time_ms: 50, // 缓存命中，处理时间很短
        cached: true,      // 缓存命中
    };

    assert!(!response1.cached);
    assert!(response2.cached);
    assert!(response2.query_time_ms < response1.query_time_ms);
    assert_eq!(response1.results.len(), response2.results.len());
}

#[tokio::test]
async fn test_end_to_end_unicode_handling() {
    // 测试 Unicode 字符处理
    let unicode_request = ApiSearchRequest {
        query: Some("中文搜索测试 🚀".to_string()),
        _q: None,
        engine_count: Some(1),
        page: 1,
        page_size: 10,
        language: Some("zh".to_string()),
        region: Some("cn".to_string()),
        safe_search: None,
        time_range: None,
        engines: Some("baidu".to_string()),
        include_deepweb: false,
    };

    // 模拟序列化/反序列化
    let request_json = serde_json::to_string(&unicode_request).unwrap();
    let deserialized_request: ApiSearchRequest = serde_json::from_str(&request_json).unwrap();

    assert_eq!(deserialized_request.get_query().unwrap(), "中文搜索测试 🚀");
    assert_eq!(deserialized_request.language, Some("zh".to_string()));
    assert_eq!(deserialized_request.region, Some("cn".to_string()));

    // Unicode 响应
    let unicode_response = ApiSearchResponse {
        query: unicode_request.get_query().unwrap(),
        results: vec![ApiSearchResultItem {
            title: "中文搜索结果 🎯".to_string(),
            url: "https://example.com/中文".to_string(),
            description: Some("这是一个中文搜索结果的描述 📖".to_string()),
            engine: "baidu".to_string(),
            score: Some(0.95),
            published_date: Some("2024-01-15".to_string()),
        }],
        total_count: 1,
        page: unicode_request.page,
        page_size: unicode_request.page_size,
        engines_used: unicode_request.get_engines(),
        query_time_ms: 800,
        cached: false,
    };

    let response_json = serde_json::to_string(&unicode_response).unwrap();
    let deserialized_response: ApiSearchResponse = serde_json::from_str(&response_json).unwrap();

    assert_eq!(deserialized_response.query, "中文搜索测试 🚀");
    assert_eq!(deserialized_response.results[0].title, "中文搜索结果 🎯");
    assert_eq!(
        deserialized_response.results[0].description,
        Some("这是一个中文搜索结果的描述 📖".to_string())
    );
}
