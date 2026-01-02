//! API 处理器测试模块
//!
//! 测试各种 API 请求处理器的功能和逻辑

use seesea_api::api::types::*;

// 模拟处理器函数（实际实现需要依赖注入）
async fn mock_search_handler(
    request: ApiSearchRequest,
) -> Result<ApiSearchResponse, ApiErrorResponse> {
    if request.get_query().is_err() {
        return Err(ApiErrorResponse {
            code: "INVALID_QUERY".to_string(),
            message: "查询参数不能为空".to_string(),
            details: Some("请提供有效的查询字符串".to_string()),
        });
    }

    let query = request.get_query().unwrap();

    Ok(ApiSearchResponse {
        query: query.clone(),
        results: vec![ApiSearchResultItem {
            title: format!("搜索结果: {}", query),
            url: "https://example.com".to_string(),
            description: Some(format!("关于 {} 的搜索结果", query)),
            engine: "google".to_string(),
            score: Some(0.95),
            published_date: Some("2024-01-01".to_string()),
        }],
        total_count: 1,
        page: request.page,
        page_size: request.page_size,
        engines_used: vec!["google".to_string()],
        query_time_ms: 150,
        cached: false,
    })
}

async fn mock_health_handler() -> ApiHealthResponse {
    ApiHealthResponse {
        status: "healthy".to_string(),
        version: "2.1.0".to_string(),
        available_engines: 5,
        total_engines: 8,
    }
}

async fn mock_stats_handler() -> ApiStatsResponse {
    use seesea_search::search::SearchStatsResult;

    let stats = SearchStatsResult {
        total_searches: 1000,
        cache_hits: 750,
        cache_misses: 250,
        engine_failures: 5,
        timeouts: 2,
    };

    ApiStatsResponse::from_search_stats(&stats)
}

#[tokio::test]
async fn test_search_handler_success() {
    let request = ApiSearchRequest {
        query: Some("rust programming".to_string()),
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

    let result = mock_search_handler(request).await;
    assert!(result.is_ok());

    let response = result.unwrap();
    assert_eq!(response.query, "rust programming");
    assert_eq!(response.results.len(), 1);
    assert_eq!(response.total_count, 1);
    assert_eq!(response.page, 1);
    assert_eq!(response.page_size, 10);
    assert_eq!(response.engines_used.len(), 1);
    assert_eq!(response.engines_used[0], "google");
}

#[tokio::test]
async fn test_search_handler_invalid_query() {
    let request = ApiSearchRequest {
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

    let result = mock_search_handler(request).await;
    assert!(result.is_err());

    let error = result.unwrap_err();
    assert_eq!(error.code, "INVALID_QUERY");
    assert_eq!(error.message, "查询参数不能为空");
    assert!(error.details.is_some());
}

#[tokio::test]
async fn test_search_handler_empty_query() {
    let request = ApiSearchRequest {
        query: Some("".to_string()),
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

    let result = mock_search_handler(request).await;
    assert!(result.is_ok()); // 空字符串应该被接受

    let response = result.unwrap();
    assert_eq!(response.query, "");
    assert_eq!(response.results.len(), 1);
}

#[tokio::test]
async fn test_health_handler() {
    let response = mock_health_handler().await;
    assert_eq!(response.status, "healthy");
    assert_eq!(response.version, "2.1.0");
    assert_eq!(response.available_engines, 5);
    assert_eq!(response.total_engines, 8);
}

#[tokio::test]
async fn test_stats_handler() {
    let response = mock_stats_handler().await;
    assert_eq!(response.total_searches, 1000);
    assert_eq!(response.cache_hits, 750);
    assert_eq!(response.cache_misses, 250);
    assert_eq!(response.engine_failures, 5);
    assert_eq!(response.timeouts, 2);
    assert_eq!(response.cache_hit_rate, 0.75);
}

#[tokio::test]
async fn test_search_handler_with_pagination() {
    let request = ApiSearchRequest {
        query: Some("machine learning".to_string()),
        _q: None,
        engine_count: None,
        page: 3,
        page_size: 25,
        language: None,
        region: None,
        safe_search: None,
        time_range: None,
        engines: None,
        include_deepweb: false,
    };

    let result = mock_search_handler(request).await;
    assert!(result.is_ok());

    let response = result.unwrap();
    assert_eq!(response.query, "machine learning");
    assert_eq!(response.page, 3);
    assert_eq!(response.page_size, 25);
}

#[tokio::test]
async fn test_search_handler_with_language_filter() {
    let request = ApiSearchRequest {
        query: Some("programming".to_string()),
        _q: None,
        engine_count: None,
        page: 1,
        page_size: 10,
        language: Some("en".to_string()),
        region: None,
        safe_search: None,
        time_range: None,
        engines: None,
        include_deepweb: false,
    };

    let result = mock_search_handler(request).await;
    assert!(result.is_ok());

    let response = result.unwrap();
    assert_eq!(response.query, "programming");
    assert_eq!(response.results.len(), 1);
}

#[tokio::test]
async fn test_search_handler_with_region_filter() {
    let request = ApiSearchRequest {
        query: Some("restaurant".to_string()),
        _q: None,
        engine_count: None,
        page: 1,
        page_size: 10,
        language: None,
        region: Some("us".to_string()),
        safe_search: None,
        time_range: None,
        engines: None,
        include_deepweb: false,
    };

    let result = mock_search_handler(request).await;
    assert!(result.is_ok());

    let response = result.unwrap();
    assert_eq!(response.query, "restaurant");
    assert_eq!(response.results.len(), 1);
}

#[tokio::test]
async fn test_search_handler_with_safe_search() {
    let request = ApiSearchRequest {
        query: Some("safe content".to_string()),
        _q: None,
        engine_count: None,
        page: 1,
        page_size: 10,
        language: None,
        region: None,
        safe_search: Some("strict".to_string()),
        time_range: None,
        engines: None,
        include_deepweb: false,
    };

    let result = mock_search_handler(request).await;
    assert!(result.is_ok());

    let response = result.unwrap();
    assert_eq!(response.query, "safe content");
    assert_eq!(response.results.len(), 1);
}

#[tokio::test]
async fn test_search_handler_with_engines() {
    let request = ApiSearchRequest {
        query: Some("multi engine search".to_string()),
        _q: None,
        engine_count: None,
        page: 1,
        page_size: 10,
        language: None,
        region: None,
        safe_search: None,
        time_range: None,
        engines: Some("google,bing,duckduckgo".to_string()),
        include_deepweb: false,
    };

    let result = mock_search_handler(request).await;
    assert!(result.is_ok());

    let response = result.unwrap();
    assert_eq!(response.query, "multi engine search");
    assert_eq!(response.engines_used.len(), 1); // Mock only returns one engine
}

#[tokio::test]
async fn test_search_handler_with_deepweb() {
    let request = ApiSearchRequest {
        query: Some("deep web content".to_string()),
        _q: None,
        engine_count: None,
        page: 1,
        page_size: 10,
        language: None,
        region: None,
        safe_search: None,
        time_range: None,
        engines: None,
        include_deepweb: true,
    };

    let result = mock_search_handler(request).await;
    assert!(result.is_ok());

    let response = result.unwrap();
    assert_eq!(response.query, "deep web content");
    assert_eq!(response.results.len(), 1);
}
