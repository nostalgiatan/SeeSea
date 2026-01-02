//! 搜索引擎核心trait测试

use async_trait::async_trait;
use seesea_derive::*;
use std::error::Error;

// 模拟搜索引擎实现
struct MockEngine {
    info: EngineInfo,
}

impl MockEngine {
    fn new() -> Self {
        Self {
            info: EngineInfo {
                name: "MockEngine".to_string(),
                engine_type: EngineType::General,
                description: "A mock search engine for testing".to_string(),
                status: EngineStatus::Active,
                categories: vec!["general".to_string()],
                capabilities: EngineCapabilities {
                    result_types: vec![ResultType::Web],
                    supported_params: vec!["q".to_string()],
                    max_page_size: 10,
                    supports_pagination: true,
                    supports_time_range: false,
                    supports_language_filter: false,
                    supports_region_filter: false,
                    supports_safe_search: true,
                    rate_limit: Some(60),
                },
                about: AboutInfo {
                    website: Some("https://mock.example.com".to_string()),
                    wikidata_id: Some("QMock123".to_string()),
                    official_api_documentation: None,
                    use_official_api: false,
                    require_api_key: false,
                    results: "HTML".to_string(),
                },
                shortcut: None,
                timeout: Some(30),
                version: None,
                last_checked: None,
                disabled: false,
                inactive: false,
                using_tor_proxy: false,
                display_error_messages: true,
                tokens: Vec::new(),
                max_page: 0,
            },
        }
    }
}

#[async_trait]
impl SearchEngine for MockEngine {
    fn info(&self) -> &EngineInfo {
        &self.info
    }

    async fn search(
        &self,
        query: &SearchQuery,
    ) -> Result<SearchResult, Box<dyn Error + Send + Sync>> {
        // 模拟搜索结果
        let mut result = SearchResult {
            engine_name: self.info.name.clone(),
            total_results: Some(1),
            elapsed_ms: 100,
            items: vec![],
            pagination: Some(PaginationInfo {
                current_page: query.page,
                page_size: query.page_size,
                total_pages: Some(1),
                next_page: None,
                prev_page: None,
            }),
            suggestions: vec![],
            metadata: std::collections::HashMap::new(),
        };

        // 添加一个模拟结果
        result.items.push(SearchResultItem {
            title: format!("Mock result for: {}", query.query),
            url: "https://mock.example.com/result".to_string(),
            content: "This is a mock search result".to_string(),
            display_url: Some("mock.example.com".to_string()),
            site_name: Some("Mock Site".to_string()),
            score: 0.8,
            result_type: ResultType::Web,
            thumbnail: None,
            published_date: None,
            template: None,
            metadata: std::collections::HashMap::new(),
        });

        Ok(result)
    }

    async fn is_available(&self) -> bool {
        true
    }

    async fn health_check(&self) -> Result<EngineHealth, Box<dyn Error + Send + Sync>> {
        Ok(EngineHealth {
            status: EngineStatus::Active,
            response_time_ms: 50,
            error_message: None,
        })
    }
}

#[tokio::test]
async fn test_search_engine_info() {
    // 测试搜索引擎信息
    let engine = MockEngine::new();
    let info = engine.info();

    assert_eq!(info.name, "MockEngine");
    assert_eq!(info.description, "A mock search engine for testing");
    assert!(info.capabilities.supports_pagination);
    assert!(info.capabilities.supports_safe_search);
    assert!(!info.capabilities.supports_time_range);
}

#[tokio::test]
async fn test_search_engine_search() {
    // 测试搜索引擎搜索功能
    let engine = MockEngine::new();

    let query = SearchQuery {
        query: "rust programming".to_string(),
        engine_type: EngineType::General,
        language: Some("en".to_string()),
        region: Some("US".to_string()),
        page_size: 10,
        page: 1,
        safe_search: SafeSearchLevel::Moderate,
        time_range: None,
        params: std::collections::HashMap::new(),
    };

    let result = engine.search(&query).await.unwrap();

    assert_eq!(result.engine_name, "MockEngine");
    assert_eq!(result.items.len(), 1);
    assert_eq!(result.pagination.as_ref().unwrap().current_page, 1);
    assert_eq!(result.total_results, Some(1));
}

#[tokio::test]
async fn test_search_engine_availability() {
    // 测试搜索引擎可用性检查
    let engine = MockEngine::new();

    let is_available = engine.is_available().await;
    assert!(is_available);
}

#[tokio::test]
async fn test_search_engine_health_check() {
    // 测试搜索引擎健康检查
    let engine = MockEngine::new();

    let health = engine.health_check().await.unwrap();
    assert_eq!(health.status, EngineStatus::Active);
    assert_eq!(health.response_time_ms, 50);
    assert!(health.error_message.is_none());
}

#[tokio::test]
async fn test_search_with_empty_query() {
    // 测试空查询的处理
    let engine = MockEngine::new();

    let query = SearchQuery {
        query: "".to_string(),
        engine_type: EngineType::General,
        language: None,
        region: None,
        page_size: 10,
        page: 1,
        safe_search: SafeSearchLevel::Moderate,
        time_range: None,
        params: std::collections::HashMap::new(),
    };

    let result = engine.search(&query).await.unwrap();

    // 即使查询为空，也应该返回结果（模拟行为）
    assert_eq!(result.items.len(), 1);
}

#[tokio::test]
async fn test_search_with_different_languages() {
    // 测试不同语言的搜索
    let engine = MockEngine::new();

    let languages = vec!["en", "zh", "ja", "de", "fr"];

    for lang in languages {
        let query = SearchQuery {
            query: "programming".to_string(),
            engine_type: EngineType::General,
            language: Some(lang.to_string()),
            region: None,
            page_size: 10,
            page: 1,
            safe_search: SafeSearchLevel::Moderate,
            time_range: None,
            params: std::collections::HashMap::new(),
        };

        let result = engine.search(&query).await.unwrap();
        assert_eq!(result.items.len(), 1);
    }
}

#[tokio::test]
async fn test_search_with_safe_search_levels() {
    // 测试不同安全搜索级别
    let engine = MockEngine::new();

    let safe_search_levels = vec![
        SafeSearchLevel::None,
        SafeSearchLevel::Moderate,
        SafeSearchLevel::Strict,
    ];

    for level in safe_search_levels {
        let query = SearchQuery {
            query: "test query".to_string(),
            engine_type: EngineType::General,
            language: None,
            region: None,
            page_size: 10,
            page: 1,
            safe_search: level,
            time_range: None,
            params: std::collections::HashMap::new(),
        };

        let result = engine.search(&query).await.unwrap();
        assert_eq!(result.items.len(), 1);
    }
}

// 测试查询验证
struct TestEngine;

#[async_trait]
impl SearchEngine for TestEngine {
    fn info(&self) -> &EngineInfo {
        static INFO: std::sync::OnceLock<EngineInfo> = std::sync::OnceLock::new();
        INFO.get_or_init(|| EngineInfo {
            name: "TestEngine".to_string(),
            engine_type: EngineType::General,
            description: "Test engine".to_string(),
            status: EngineStatus::Active,
            categories: vec![],
            capabilities: EngineCapabilities {
                result_types: vec![ResultType::Web],
                supported_params: vec!["q".to_string()],
                max_page_size: 10,
                supports_pagination: false,
                supports_time_range: false,
                supports_language_filter: false,
                supports_region_filter: false,
                supports_safe_search: false,
                rate_limit: Some(60),
            },
            about: AboutInfo {
                website: Some("https://test.engine".to_string()),
                wikidata_id: None,
                official_api_documentation: None,
                use_official_api: false,
                require_api_key: false,
                results: "JSON".to_string(),
            },
            shortcut: None,
            timeout: Some(30),
            disabled: false,
            inactive: false,
            version: Some("1.0.0".to_string()),
            last_checked: Some(chrono::Utc::now()),
            using_tor_proxy: false,
            display_error_messages: true,
            tokens: Vec::new(),
            max_page: 0,
        })
    }

    async fn search(
        &self,
        _query: &SearchQuery,
    ) -> Result<SearchResult, Box<dyn Error + Send + Sync>> {
        Ok(SearchResult {
            engine_name: "TestEngine".to_string(),
            total_results: Some(0),
            elapsed_ms: 0,
            items: vec![],
            pagination: Some(PaginationInfo {
                current_page: 1,
                page_size: 10,
                total_pages: Some(0),
                next_page: None,
                prev_page: None,
            }),
            suggestions: vec![],
            metadata: std::collections::HashMap::new(),
        })
    }

    fn validate_query(&self, query: &SearchQuery) -> Result<(), ValidationError> {
        if query.query.trim().is_empty() {
            return Err(ValidationError {
                code: "EMPTY_QUERY".to_string(),
                field: Some("query".to_string()),
                message: "Query cannot be empty".to_string(),
            });
        }

        if query.query.len() > 1000 {
            return Err(ValidationError {
                code: "QUERY_TOO_LONG".to_string(),
                field: Some("query".to_string()),
                message: "Query too long".to_string(),
            });
        }

        Ok(())
    }
}

#[test]
fn test_query_validation_empty() {
    // 测试空查询验证
    let engine = TestEngine;

    let query = SearchQuery {
        query: "".to_string(),
        engine_type: EngineType::General,
        language: None,
        region: None,
        page_size: 10,
        page: 1,
        safe_search: SafeSearchLevel::Moderate,
        time_range: None,
        params: std::collections::HashMap::new(),
    };

    let result = engine.validate_query(&query);
    assert!(result.is_err());

    let error = result.unwrap_err();
    assert_eq!(error.field, Some("query".to_string()));
    assert_eq!(error.message, "Query cannot be empty");
}

#[test]
fn test_query_validation_too_long() {
    // 测试查询过长验证
    let engine = TestEngine;

    let long_query = "a".repeat(1001);
    let query = SearchQuery {
        query: long_query,
        engine_type: EngineType::General,
        language: None,
        region: None,
        page_size: 10,
        page: 1,
        safe_search: SafeSearchLevel::Moderate,
        time_range: None,
        params: std::collections::HashMap::new(),
    };

    let result = engine.validate_query(&query);
    assert!(result.is_err());

    let error = result.unwrap_err();
    assert_eq!(error.field, Some("query".to_string()));
    assert_eq!(error.message, "Query too long");
}

#[test]
fn test_query_validation_valid() {
    // 测试有效查询验证
    let engine = TestEngine;

    let query = SearchQuery {
        query: "valid query".to_string(),
        engine_type: EngineType::General,
        language: None,
        region: None,
        page_size: 10,
        page: 1,
        safe_search: SafeSearchLevel::Moderate,
        time_range: None,
        params: std::collections::HashMap::new(),
    };

    let result = engine.validate_query(&query);
    assert!(result.is_ok());
}
