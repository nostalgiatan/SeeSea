//! 集成测试 - 测试整个搜索流程

use async_trait::async_trait;
use seesea_derive::query::QueryBuilder;
use seesea_derive::*;
use std::error::Error;
use std::sync::Arc;

// 模拟完整的搜索引擎实现
struct IntegrationTestEngine {
    name: String,
    response_delay: std::time::Duration,
    should_fail: bool,
}

impl IntegrationTestEngine {
    fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            response_delay: std::time::Duration::from_millis(10),
            should_fail: false,
        }
    }

    #[allow(dead_code)]
    fn with_delay(mut self, delay_ms: u64) -> Self {
        self.response_delay = std::time::Duration::from_millis(delay_ms);
        self
    }

    fn with_failure(mut self) -> Self {
        self.should_fail = true;
        self
    }
}

#[async_trait]
impl SearchEngine for IntegrationTestEngine {
    fn info(&self) -> &EngineInfo {
        static INFO: std::sync::OnceLock<EngineInfo> = std::sync::OnceLock::new();
        INFO.get_or_init(|| EngineInfo {
            name: "IntegrationTestEngine".to_string(),
            engine_type: EngineType::General,
            description: "Integration test search engine".to_string(),
            status: EngineStatus::Active,
            categories: vec!["integration".to_string(), "test".to_string()],
            capabilities: EngineCapabilities {
                result_types: vec![ResultType::Web],
                supported_params: vec!["q".to_string()],
                max_page_size: 10,
                supports_pagination: true,
                supports_time_range: true,
                supports_language_filter: false,
                supports_region_filter: false,
                supports_safe_search: true,
                rate_limit: Some(60),
            },
            about: AboutInfo {
                website: Some("https://integration.test".to_string()),
                wikidata_id: Some("QIntegration123".to_string()),
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
        query: &SearchQuery,
    ) -> Result<SearchResult, Box<dyn Error + Send + Sync>> {
        // 模拟网络延迟
        tokio::time::sleep(self.response_delay).await;

        if self.should_fail {
            return Err("Simulated engine failure".into());
        }

        // 模拟不同的响应场景
        let mut result = SearchResult {
            engine_name: self.name.clone(),
            total_results: Some(10),
            elapsed_ms: self.response_delay.as_millis() as u64,
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

        // 根据查询生成不同的结果
        if query.query.contains("empty") {
            result.total_results = Some(0);
            if let Some(ref mut pagination) = result.pagination {
                pagination.total_pages = Some(0);
            }
        } else if query.query.contains("error") {
            return Err("Query contains error keyword".into());
        } else {
            // 生成模拟结果，考虑分页
            let start_index = (query.page - 1) * query.page_size;
            let total_results = 10;
            result.total_results = Some(total_results);

            if let Some(ref mut pagination) = result.pagination {
                pagination.total_pages =
                    Some((total_results + query.page_size - 1) / query.page_size);
                if query.page < pagination.total_pages.unwrap() {
                    pagination.next_page = Some(format!("?page={}", query.page + 1));
                }
                if query.page > 1 {
                    pagination.prev_page = Some(format!("?page={}", query.page - 1));
                }
            }

            for i in 0..std::cmp::min(
                query.page_size as usize,
                total_results.saturating_sub(start_index),
            ) {
                let result_index = start_index + i;
                result.items.push(SearchResultItem {
                    title: format!("Result {} for: {}", result_index + 1, query.query),
                    url: format!("https://example.com/result/{}", result_index),
                    content: format!(
                        "This is result number {} for the query '{}'",
                        result_index + 1,
                        query.query
                    ),
                    display_url: Some(format!("example.com/result/{}", result_index)),
                    site_name: Some("Example Site".to_string()),
                    score: 0.9 - (result_index as f64 * 0.05),
                    result_type: match query.engine_type {
                        EngineType::Image => ResultType::Image,
                        EngineType::Video => ResultType::Video,
                        EngineType::News => ResultType::News,
                        EngineType::Academic => ResultType::Academic,
                        EngineType::Code => ResultType::Code,
                        _ => ResultType::Web,
                    },
                    thumbnail: if query.engine_type == EngineType::Image {
                        Some(format!("https://example.com/thumb/{}", result_index))
                    } else {
                        None
                    },
                    published_date: Some(chrono::Utc::now()),
                    template: None,
                    metadata: std::collections::HashMap::new(),
                });
            }
        }

        Ok(result)
    }

    async fn is_available(&self) -> bool {
        !self.should_fail
    }

    async fn health_check(&self) -> Result<EngineHealth, Box<dyn Error + Send + Sync>> {
        if self.should_fail {
            Ok(EngineHealth {
                status: EngineStatus::Error,
                response_time_ms: 0,
                error_message: Some("Simulated health check failure".to_string()),
            })
        } else {
            Ok(EngineHealth {
                status: EngineStatus::Active,
                response_time_ms: self.response_delay.as_millis() as u64,
                error_message: None,
            })
        }
    }
}

// 查询构建器集成测试
struct IntegrationQueryBuilder {
    base_query: SearchQuery,
}

impl IntegrationQueryBuilder {
    fn new(query: &str) -> Self {
        Self {
            base_query: SearchQuery {
                query: query.to_string(),
                engine_type: EngineType::General,
                language: None,
                region: None,
                page_size: 10,
                page: 1,
                safe_search: SafeSearchLevel::Moderate,
                time_range: None,
                params: std::collections::HashMap::new(),
            },
        }
    }
}

impl QueryBuilder for IntegrationQueryBuilder {
    fn build_base_query(&self, query: &str, engine_type: EngineType) -> SearchQuery {
        SearchQuery {
            query: query.to_string(),
            engine_type,
            ..self.base_query.clone()
        }
    }

    fn with_language(mut self, language: impl Into<String>) -> Self {
        self.base_query.language = Some(language.into());
        self
    }

    fn with_region(mut self, region: impl Into<String>) -> Self {
        self.base_query.region = Some(region.into());
        self
    }

    fn with_page_size(mut self, size: usize) -> Self {
        self.base_query.page_size = size;
        self
    }

    fn with_safe_search(mut self, level: SafeSearchLevel) -> Self {
        self.base_query.safe_search = level;
        self
    }

    fn with_time_range(mut self, range: TimeRange) -> Self {
        self.base_query.time_range = Some(range);
        self
    }

    fn with_pagination(mut self, page: usize, page_size: usize) -> Self {
        self.base_query.page = page;
        self.base_query.page_size = page_size;
        self
    }

    fn build(self) -> SearchQuery {
        self.base_query
    }
}

#[tokio::test]
async fn test_full_search_workflow() {
    // 测试完整的搜索工作流程
    let engine = IntegrationTestEngine::new("FullWorkflowEngine");

    // 使用查询构建器创建查询
    let builder = IntegrationQueryBuilder::new("rust programming");
    let query = builder
        .with_language("en")
        .with_region("US")
        .with_page_size(5)
        .with_safe_search(SafeSearchLevel::Moderate)
        .build();

    // 执行搜索
    let result = engine.search(&query).await.unwrap();

    // 验证结果
    assert_eq!(result.engine_name, "FullWorkflowEngine");
    assert_eq!(result.items.len(), 5); // page_size = 5
    assert!(result.total_results.is_some());
    assert!(result.elapsed_ms > 0);
}

#[tokio::test]
async fn test_search_with_different_engine_types() {
    // 测试不同类型的搜索引擎
    let engine = IntegrationTestEngine::new("MultiTypeEngine");

    let engine_types = vec![
        EngineType::General,
        EngineType::Image,
        EngineType::Video,
        EngineType::News,
        EngineType::Academic,
        EngineType::Code,
    ];

    for engine_type in engine_types {
        let query = SearchQuery {
            query: "test query".to_string(),
            engine_type,
            language: None,
            region: None,
            page_size: 2,
            page: 1,
            safe_search: SafeSearchLevel::Moderate,
            time_range: None,
            params: std::collections::HashMap::new(),
        };

        let result = engine.search(&query).await.unwrap();

        assert_eq!(result.items.len(), 2);

        // 验证结果类型是否与引擎类型匹配
        for item in &result.items {
            match engine_type {
                EngineType::Image => assert_eq!(item.result_type, ResultType::Image),
                EngineType::Video => assert_eq!(item.result_type, ResultType::Video),
                EngineType::News => assert_eq!(item.result_type, ResultType::News),
                EngineType::Academic => assert_eq!(item.result_type, ResultType::Academic),
                EngineType::Code => assert_eq!(item.result_type, ResultType::Code),
                _ => assert_eq!(item.result_type, ResultType::Web),
            }
        }
    }
}

#[tokio::test]
async fn test_search_with_pagination() {
    // 测试分页功能
    let engine = IntegrationTestEngine::new("PaginationEngine");

    // 第一页
    let query_page1 = SearchQuery {
        query: "pagination test".to_string(),
        engine_type: EngineType::General,
        language: None,
        region: None,
        page_size: 3,
        page: 1,
        safe_search: SafeSearchLevel::Moderate,
        time_range: None,
        params: std::collections::HashMap::new(),
    };

    let result_page1 = engine.search(&query_page1).await.unwrap();
    assert_eq!(result_page1.items.len(), 3);
    assert_eq!(result_page1.pagination.as_ref().unwrap().current_page, 1);

    // 第二页
    let query_page2 = SearchQuery {
        query: "pagination test".to_string(),
        engine_type: EngineType::General,
        language: None,
        region: None,
        page_size: 3,
        page: 2,
        safe_search: SafeSearchLevel::Moderate,
        time_range: None,
        params: std::collections::HashMap::new(),
    };

    let result_page2 = engine.search(&query_page2).await.unwrap();
    assert_eq!(result_page2.items.len(), 3);
    assert_eq!(result_page2.pagination.as_ref().unwrap().current_page, 2);

    // 验证两页的结果不同
    let titles_page1: Vec<String> = result_page1.items.iter().map(|r| r.title.clone()).collect();
    let titles_page2: Vec<String> = result_page2.items.iter().map(|r| r.title.clone()).collect();

    assert_ne!(titles_page1, titles_page2);
}

#[tokio::test]
async fn test_search_with_empty_results() {
    // 测试空结果的情况
    let engine = IntegrationTestEngine::new("EmptyResultsEngine");

    let query = SearchQuery {
        query: "empty results query".to_string(),
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

    assert_eq!(result.engine_name, "EmptyResultsEngine");
    assert_eq!(result.items.len(), 0);
    assert_eq!(result.total_results, Some(0));
    assert_eq!(result.pagination.as_ref().unwrap().total_pages, Some(0));
}

#[tokio::test]
async fn test_search_with_error_handling() {
    // 测试错误处理
    let engine = IntegrationTestEngine::new("ErrorEngine").with_failure();

    let query = SearchQuery {
        query: "test query".to_string(),
        engine_type: EngineType::General,
        language: None,
        region: None,
        page_size: 10,
        page: 1,
        safe_search: SafeSearchLevel::Moderate,
        time_range: None,
        params: std::collections::HashMap::new(),
    };

    let result = engine.search(&query).await;
    assert!(result.is_err());

    let error = result.unwrap_err();
    assert!(error.to_string().contains("Simulated engine failure"));
}

#[tokio::test]
async fn test_health_check_integration() {
    // 测试健康检查集成
    let healthy_engine = IntegrationTestEngine::new("HealthyEngine");
    let unhealthy_engine = IntegrationTestEngine::new("UnhealthyEngine").with_failure();

    // 健康引擎
    let health = healthy_engine.health_check().await.unwrap();
    assert_eq!(health.status, EngineStatus::Active);
    assert!(health.error_message.is_none());
    assert!(healthy_engine.is_available().await);

    // 不健康引擎
    let health = unhealthy_engine.health_check().await.unwrap();
    assert_eq!(health.status, EngineStatus::Error);
    assert!(health.error_message.is_some());
    assert!(!unhealthy_engine.is_available().await);
}

#[tokio::test]
async fn test_concurrent_searches() {
    // 测试并发搜索
    let engine = Arc::new(IntegrationTestEngine::new("ConcurrentEngine"));

    let mut tasks = vec![];

    for i in 0..5 {
        let engine_clone = Arc::clone(&engine);
        let task = tokio::spawn(async move {
            let query = SearchQuery {
                query: format!("concurrent query {}", i),
                engine_type: EngineType::General,
                language: None,
                region: None,
                page_size: 2,
                page: 1,
                safe_search: SafeSearchLevel::Moderate,
                time_range: None,
                params: std::collections::HashMap::new(),
            };

            engine_clone.search(&query).await
        });

        tasks.push(task);
    }

    // 等待所有任务完成
    let results = futures::future::join_all(tasks).await;

    // 验证所有搜索都成功
    for (_i, result) in results.iter().enumerate() {
        let search_result = result.as_ref().unwrap().as_ref().unwrap();
        assert_eq!(search_result.engine_name, "ConcurrentEngine");
        assert_eq!(search_result.items.len(), 2);
    }
}

#[tokio::test]
async fn test_query_builder_integration() {
    // 测试查询构建器集成
    let engine = IntegrationTestEngine::new("QueryBuilderEngine");

    let builder = IntegrationQueryBuilder::new("integration test");
    let query = builder
        .with_language("en")
        .with_region("US")
        .with_pagination(1, 3)
        .with_safe_search(SafeSearchLevel::Strict)
        .with_time_range(TimeRange::Week)
        .build();

    let result = engine.search(&query).await.unwrap();

    assert_eq!(result.engine_name, "QueryBuilderEngine");
    assert_eq!(result.items.len(), 3);

    // 验证查询参数被正确使用
    assert_eq!(query.language, Some("en".to_string()));
    assert_eq!(query.region, Some("US".to_string()));
    assert_eq!(query.page_size, 3);
    assert_eq!(query.safe_search, SafeSearchLevel::Strict);
    assert_eq!(query.time_range, Some(TimeRange::Week));
}

// 需要添加这个依赖到Cargo.toml
// [dev-dependencies]
// futures = "0.3"
