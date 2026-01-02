//! 类型系统测试

use seesea_derive::*;
use serde_json;

#[test]
fn test_engine_type_serialization() {
    // 测试引擎类型的序列化和反序列化
    let engine_types = vec![
        EngineType::General,
        EngineType::Image,
        EngineType::Video,
        EngineType::News,
        EngineType::Academic,
        EngineType::Code,
        EngineType::Shopping,
        EngineType::Music,
        EngineType::Custom,
    ];

    for engine_type in engine_types {
        let json = serde_json::to_string(&engine_type).unwrap();
        let deserialized: EngineType = serde_json::from_str(&json).unwrap();
        assert_eq!(engine_type, deserialized);
    }
}

#[test]
fn test_search_query_creation() {
    // 测试搜索查询的创建
    let query = SearchQuery {
        query: "test query".to_string(),
        engine_type: EngineType::General,
        language: Some("en".to_string()),
        region: Some("US".to_string()),
        page_size: 10,
        page: 1,
        safe_search: SafeSearchLevel::Moderate,
        time_range: None,
        params: std::collections::HashMap::new(),
    };

    assert_eq!(query.query, "test query");
    assert_eq!(query.engine_type, EngineType::General);
    assert_eq!(query.page_size, 10);
    assert_eq!(query.page, 1);
}

#[test]
fn test_search_query_serialization() {
    // 测试搜索查询的序列化
    let mut params = std::collections::HashMap::new();
    params.insert("custom".to_string(), "value".to_string());

    let query = SearchQuery {
        query: "rust programming".to_string(),
        engine_type: EngineType::Code,
        language: Some("en".to_string()),
        region: Some("US".to_string()),
        page_size: 20,
        page: 2,
        safe_search: SafeSearchLevel::Strict,
        time_range: Some(TimeRange::Week),
        params,
    };

    let json = serde_json::to_string(&query).unwrap();
    let deserialized: SearchQuery = serde_json::from_str(&json).unwrap();

    assert_eq!(query.query, deserialized.query);
    assert_eq!(query.engine_type, deserialized.engine_type);
    assert_eq!(query.page_size, deserialized.page_size);
}

#[test]
fn test_search_result_item_creation() {
    // 测试搜索结果项的创建
    let item = SearchResultItem {
        title: "Test Title".to_string(),
        url: "https://example.com".to_string(),
        content: "Test content".to_string(),
        display_url: Some("example.com".to_string()),
        site_name: Some("Example".to_string()),
        score: 0.95,
        result_type: ResultType::Web,
        thumbnail: Some("https://example.com/thumb.jpg".to_string()),
        published_date: None,
        template: None,
        metadata: std::collections::HashMap::new(),
    };

    assert_eq!(item.title, "Test Title");
    assert_eq!(item.url, "https://example.com");
    assert_eq!(item.score, 0.95);
    assert_eq!(item.result_type, ResultType::Web);
}

#[test]
fn test_engine_info_creation() {
    // 测试引擎信息的创建
    let info = EngineInfo {
        name: "TestEngine".to_string(),
        engine_type: EngineType::General,
        description: "A test search engine".to_string(),
        status: EngineStatus::Active,
        categories: vec!["general".to_string(), "test".to_string()],
        capabilities: EngineCapabilities {
            result_types: vec![ResultType::Web],
            supported_params: vec!["q".to_string()],
            max_page_size: 20,
            supports_pagination: true,
            supports_time_range: false,
            supports_language_filter: true,
            supports_region_filter: false,
            supports_safe_search: true,
            rate_limit: None,
        },
        about: AboutInfo {
            website: Some("https://test.com".to_string()),
            wikidata_id: Some("Q12345".to_string()),
            official_api_documentation: None,
            use_official_api: false,
            require_api_key: false,
            results: "HTML".to_string(),
        },
        shortcut: None,
        timeout: None,
        disabled: false,
        inactive: false,
        version: None,
        last_checked: None,
        using_tor_proxy: false,
        display_error_messages: true,
        tokens: vec![],
        max_page: 0,
    };

    assert_eq!(info.name, "TestEngine");
    assert_eq!(info.categories.len(), 2);
    assert!(info.capabilities.supports_pagination);
    assert!(!info.capabilities.supports_time_range);
    assert!(info.capabilities.supports_safe_search);
}

#[test]
fn test_safe_search_level_ordering() {
    // 测试安全搜索级别的顺序
    use std::cmp::Ordering;

    assert_eq!(
        SafeSearchLevel::None.cmp(&SafeSearchLevel::Moderate),
        Ordering::Less
    );
    assert_eq!(
        SafeSearchLevel::Moderate.cmp(&SafeSearchLevel::Strict),
        Ordering::Less
    );
    assert_eq!(
        SafeSearchLevel::None.cmp(&SafeSearchLevel::Strict),
        Ordering::Less
    );
}

#[test]
fn test_time_range_serialization() {
    // 测试时间范围的序列化
    let time_ranges = vec![
        TimeRange::Day,
        TimeRange::Week,
        TimeRange::Month,
        TimeRange::Year,
    ];

    for time_range in time_ranges {
        let json = serde_json::to_string(&time_range).unwrap();
        let deserialized: TimeRange = serde_json::from_str(&json).unwrap();
        assert_eq!(time_range, deserialized);
    }
}

#[test]
fn test_result_type_variants() {
    // 测试结果类型的变体
    let result_types = vec![
        ResultType::Web,
        ResultType::Image,
        ResultType::Video,
        ResultType::News,
        ResultType::Map,
        ResultType::Shopping,
        ResultType::Academic,
        ResultType::Code,
        ResultType::Music,
        ResultType::Other,
    ];

    for result_type in result_types {
        let json = serde_json::to_string(&result_type).unwrap();
        let deserialized: ResultType = serde_json::from_str(&json).unwrap();
        assert_eq!(result_type, deserialized);
    }
}

#[test]
fn test_search_result_creation() {
    // 测试搜索结果的创建
    let mut result = SearchResult {
        engine_name: "TestEngine".to_string(),
        total_results: Some(100),
        elapsed_ms: 150,
        items: vec![],
        pagination: Some(PaginationInfo {
            current_page: 1,
            page_size: 10,
            total_pages: Some(10),
            next_page: None,
            prev_page: None,
        }),
        suggestions: vec!["suggestion1".to_string(), "suggestion2".to_string()],
        metadata: std::collections::HashMap::new(),
    };

    // 添加结果项
    result.items.push(SearchResultItem {
        title: "Result 1".to_string(),
        url: "https://example1.com".to_string(),
        content: "Content 1".to_string(),
        display_url: None,
        site_name: None,
        score: 0.9,
        result_type: ResultType::Web,
        thumbnail: None,
        published_date: None,
        template: None,
        metadata: std::collections::HashMap::new(),
    });

    assert_eq!(result.engine_name, "TestEngine");
    assert_eq!(result.items.len(), 1);
    assert_eq!(result.total_results, Some(100));
    assert_eq!(result.suggestions.len(), 2);
}

#[test]
fn test_engine_health_status() {
    // 测试引擎健康状态
    let health = EngineHealth {
        status: EngineStatus::Active,
        response_time_ms: 150,
        error_message: None,
    };

    assert_eq!(health.status, EngineStatus::Active);
    assert_eq!(health.response_time_ms, 150);
    assert!(health.error_message.is_none());

    let health_with_error = EngineHealth {
        status: EngineStatus::Error,
        response_time_ms: 0,
        error_message: Some("Connection timeout".to_string()),
    };

    assert_eq!(health_with_error.status, EngineStatus::Error);
    assert_eq!(
        health_with_error.error_message,
        Some("Connection timeout".to_string())
    );
}

#[test]
fn test_validation_error_creation() {
    // 测试验证错误的创建
    let error = ValidationError {
        field: Some("query".to_string()),
        message: "Query cannot be empty".to_string(),
        code: "EMPTY_QUERY".to_string(),
    };

    assert_eq!(error.field, Some("query".to_string()));
    assert_eq!(error.message, "Query cannot be empty");
    assert_eq!(error.code, "EMPTY_QUERY");
}

#[test]
fn test_request_params_creation() {
    // 测试请求参数的创建
    let mut params = RequestParams::new();
    params.insert("q".to_string(), "rust".to_string());
    params.insert("lang".to_string(), "en".to_string());

    assert_eq!(params.get("q"), Some(&"rust".to_string()));
    assert_eq!(params.get("lang"), Some(&"en".to_string()));
    assert_eq!(params.len(), 2);
}

#[test]
fn test_rss_feed_item_creation() {
    // 测试RSS Feed项目的创建
    let item = RssFeedItem {
        title: "Test RSS Item".to_string(),
        link: "https://example.com/item".to_string(),
        description: Some("Test description".to_string()),
        author: Some("Test Author".to_string()),
        pub_date: Some("2025-01-01".to_string()),
        content: Some("Full content".to_string()),
        categories: vec!["tech".to_string(), "rust".to_string()],
        guid: Some("unique-id-123".to_string()),
        enclosures: vec![],
        custom_fields: std::collections::HashMap::new(),
    };

    assert_eq!(item.title, "Test RSS Item");
    assert_eq!(item.link, "https://example.com/item");
    assert_eq!(item.categories.len(), 2);
    assert_eq!(item.author, Some("Test Author".to_string()));
}

#[test]
fn test_rss_enclosure_creation() {
    // 测试RSS附件的创建
    let enclosure = RssEnclosure {
        url: "https://example.com/image.jpg".to_string(),
        length: Some(1024),
        mime_type: Some("image/jpeg".to_string()),
    };

    assert_eq!(enclosure.url, "https://example.com/image.jpg");
    assert_eq!(enclosure.length, Some(1024));
    assert_eq!(enclosure.mime_type, Some("image/jpeg".to_string()));
}
