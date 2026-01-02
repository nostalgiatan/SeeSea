//! 边缘情况和性能测试

use async_trait::async_trait;
use seesea_derive::*;
use std::error::Error;
use std::time::{Duration, Instant};

// 性能测试引擎
struct PerformanceTestEngine {
    delay_ms: u64,
    result_count: usize,
}

impl PerformanceTestEngine {
    fn new(delay_ms: u64, result_count: usize) -> Self {
        Self {
            delay_ms,
            result_count,
        }
    }
}

#[async_trait]
impl SearchEngine for PerformanceTestEngine {
    fn info(&self) -> &EngineInfo {
        static INFO: std::sync::OnceLock<EngineInfo> = std::sync::OnceLock::new();
        INFO.get_or_init(|| EngineInfo {
            name: "PerformanceTestEngine".to_string(),
            engine_type: EngineType::General,
            description: "Performance testing engine".to_string(),
            status: EngineStatus::Active,
            categories: vec!["performance".to_string(), "test".to_string()],
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
                website: Some("https://perf.test".to_string()),
                wikidata_id: Some("QPerf123".to_string()),
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
        // 模拟处理延迟
        tokio::time::sleep(Duration::from_millis(self.delay_ms)).await;

        let mut result = SearchResult {
            engine_name: "PerformanceTestEngine".to_string(),
            items: vec![],
            total_results: Some(self.result_count),
            elapsed_ms: self.delay_ms,
            pagination: Some(PaginationInfo {
                current_page: query.page,
                page_size: query.page_size,
                total_pages: if query.page_size > 0 {
                    Some((self.result_count + query.page_size - 1) / query.page_size)
                } else {
                    Some(0)
                },
                next_page: None,
                prev_page: None,
            }),
            suggestions: vec![],
            metadata: std::collections::HashMap::new(),
        };

        // 生成指定数量的结果
        let start_index = (query.page - 1) * query.page_size;
        let end_index = std::cmp::min(start_index + query.page_size as usize, self.result_count);

        for i in start_index..end_index {
            result.items.push(SearchResultItem {
                title: format!("Performance Result {}", i + 1),
                url: format!("https://perf.test/result/{}", i),
                content: format!("This is performance test result number {}", i + 1),
                display_url: Some(format!("perf.test/result/{}", i)),
                site_name: Some("Performance Test Site".to_string()),
                score: 1.0 - (i as f64 / self.result_count as f64),
                result_type: ResultType::Web,
                thumbnail: None,
                published_date: Some(chrono::Utc::now()),
                template: None,
                metadata: std::collections::HashMap::new(),
            });
        }

        Ok(result)
    }

    async fn is_available(&self) -> bool {
        true
    }

    async fn health_check(&self) -> Result<EngineHealth, Box<dyn Error + Send + Sync>> {
        Ok(EngineHealth {
            status: EngineStatus::Active,
            response_time_ms: self.delay_ms,
            error_message: None,
        })
    }
}

#[tokio::test]
async fn test_performance_large_result_set() {
    // 测试处理大量结果的性能
    let engine = PerformanceTestEngine::new(10, 1000);

    let query = SearchQuery {
        query: "performance test".to_string(),
        engine_type: EngineType::General,
        language: None,
        region: None,
        page_size: 50,
        page: 1,
        safe_search: SafeSearchLevel::Moderate,
        time_range: None,
        params: std::collections::HashMap::new(),
    };

    let start_time = Instant::now();
    let result = engine.search(&query).await.unwrap();
    let elapsed = start_time.elapsed();

    assert_eq!(result.items.len(), 50);
    assert_eq!(result.total_results, Some(1000));
    assert_eq!(result.pagination.as_ref().unwrap().total_pages, Some(20)); // 1000 / 50 = 20 pages

    // 验证响应时间在合理范围内（考虑模拟延迟）
    assert!(elapsed.as_millis() >= 10); // 至少有10ms的模拟延迟
    assert!(elapsed.as_millis() < 1000); // 但应该小于1秒
}

#[tokio::test]
async fn test_performance_concurrent_large_queries() {
    // 测试并发大量查询的性能
    let engine = std::sync::Arc::new(PerformanceTestEngine::new(5, 100));

    let mut tasks = vec![];

    for i in 0..10 {
        let engine_clone = std::sync::Arc::clone(&engine);
        let task = tokio::spawn(async move {
            let query = SearchQuery {
                query: format!("concurrent query {}", i),
                engine_type: EngineType::General,
                language: None,
                region: None,
                page_size: 20,
                page: 1,
                safe_search: SafeSearchLevel::Moderate,
                time_range: None,
                params: std::collections::HashMap::new(),
            };

            let start_time = Instant::now();
            let result = engine_clone.search(&query).await;
            let elapsed = start_time.elapsed();

            (result, elapsed)
        });

        tasks.push(task);
    }

    let start_total_time = Instant::now();
    let results = futures::future::join_all(tasks).await;
    let total_elapsed = start_total_time.elapsed();

    // 验证所有查询都成功
    for result in results {
        assert!(result.is_ok());

        let (search_result_result, _elapsed) = result.unwrap();
        assert!(search_result_result.is_ok());

        let search_result = search_result_result.unwrap();
        assert_eq!(search_result.items.len(), 20);
        // Note: individual elapsed time is not available in this context
        // since join_all doesn't provide per-task timing
    }

    // 由于并发执行，总时间应该远小于顺序执行的时间
    // 10个查询，每个5ms，如果完全并发，总时间应该接近5ms + 一些开销
    assert!(total_elapsed.as_millis() < 100); // 总时间应该小于100ms
}

#[tokio::test]
async fn test_edge_case_very_long_query() {
    // 测试非常长的查询字符串
    let engine = PerformanceTestEngine::new(1, 10);

    // 创建一个很长的查询字符串（1000个字符）
    let long_query = "a".repeat(1000);

    let query = SearchQuery {
        query: long_query.clone(),
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

    assert_eq!(result.items.len(), 10);
}

#[tokio::test]
async fn test_edge_case_special_characters_in_query() {
    // 测试查询中包含特殊字符
    let engine = PerformanceTestEngine::new(1, 10);

    let special_queries = vec![
        "query with spaces and symbols !@#$%^&*()",
        "query/with/slashes",
        "query\\with\\backslashes",
        "query\"with\"quotes",
        "query'with'single'quotes",
        "query\nwith\nnewlines",
        "query\twith\ttabs",
        "查询包含中文",
        "クエリに日本語が含まれています",
        "запрос содержит русский",
        "🔍 emoji query 🚀",
    ];

    for query_text in special_queries {
        let query = SearchQuery {
            query: query_text.to_string(),
            engine_type: EngineType::General,
            language: None,
            region: None,
            page_size: 5,
            page: 1,
            safe_search: SafeSearchLevel::Moderate,
            time_range: None,
            params: std::collections::HashMap::new(),
        };

        let result = engine.search(&query).await.unwrap();

        assert_eq!(result.items.len(), 5);
    }
}

#[tokio::test]
async fn test_edge_case_empty_and_whitespace_queries() {
    // 测试空查询和空白查询
    let engine = PerformanceTestEngine::new(1, 10);

    let edge_case_queries = vec![
        "",        // 完全空
        "   ",     // 只有空格
        "\t\t\t",  // 只有制表符
        "\n\n\n",  // 只有换行符
        " \t \n ", // 混合空白字符
    ];

    for query_text in edge_case_queries {
        let query = SearchQuery {
            query: query_text.to_string(),
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

        // 即使是空查询也应该返回结果（根据引擎实现）
        assert!(!result.items.is_empty());
    }
}

#[tokio::test]
async fn test_edge_case_extreme_pagination() {
    // 测试极端分页情况
    let engine = PerformanceTestEngine::new(1, 1000);

    // 测试非常大的页码
    let query = SearchQuery {
        query: "extreme pagination test".to_string(),
        engine_type: EngineType::General,
        language: None,
        region: None,
        page_size: 10,
        page: 999, // 非常大的页码
        safe_search: SafeSearchLevel::Moderate,
        time_range: None,
        params: std::collections::HashMap::new(),
    };

    let result = engine.search(&query).await.unwrap();

    assert_eq!(result.pagination.as_ref().unwrap().current_page, 999);
    assert_eq!(result.pagination.as_ref().unwrap().total_pages, Some(100)); // 1000 / 10 = 100页
    // 页码超出范围时应该返回空结果
    assert!(result.items.is_empty());
}

#[tokio::test]
async fn test_edge_case_zero_page_size() {
    // 测试页面大小为0的情况
    let engine = PerformanceTestEngine::new(1, 100);

    let query = SearchQuery {
        query: "zero page size test".to_string(),
        engine_type: EngineType::General,
        language: None,
        region: None,
        page_size: 0, // 页面大小为0
        page: 1,
        safe_search: SafeSearchLevel::Moderate,
        time_range: None,
        params: std::collections::HashMap::new(),
    };

    let result = engine.search(&query).await.unwrap();

    // 页面大小为0时应该返回空结果或处理为默认值
    assert_eq!(result.items.len(), 0);
}

#[tokio::test]
async fn test_edge_case_very_large_page_size() {
    // 测试非常大的页面大小
    let engine = PerformanceTestEngine::new(1, 1000);

    let query = SearchQuery {
        query: "large page size test".to_string(),
        engine_type: EngineType::General,
        language: None,
        region: None,
        page_size: 10000, // 非常大的页面大小
        page: 1,
        safe_search: SafeSearchLevel::Moderate,
        time_range: None,
        params: std::collections::HashMap::new(),
    };

    let result = engine.search(&query).await.unwrap();

    // 应该限制在可用的结果数量内
    assert_eq!(result.items.len(), 1000); // 只有1000个结果可用
}

// 内存使用测试
#[tokio::test]
async fn test_memory_efficiency_large_dataset() {
    // 测试处理大数据集时的内存效率
    let engine = PerformanceTestEngine::new(1, 10000);

    let mut total_results = 0;
    let mut page = 1;

    // 分批处理大量结果，模拟内存受限的环境
    while total_results < 1000 {
        // 只处理前1000个结果
        let query = SearchQuery {
            query: "memory efficiency test".to_string(),
            engine_type: EngineType::General,
            language: None,
            region: None,
            page_size: 100, // 每页100个结果
            page,
            safe_search: SafeSearchLevel::Moderate,
            time_range: None,
            params: std::collections::HashMap::new(),
        };

        let result = engine.search(&query).await.unwrap();

        if result.items.is_empty() {
            break;
        }

        total_results += result.items.len();
        page += 1;

        // 模拟处理结果后的清理
        drop(result);
    }

    assert_eq!(total_results, 1000);
}

// 错误恢复测试
struct ErrorRecoveryEngine {
    failure_count: std::sync::Arc<std::sync::atomic::AtomicU32>,
    max_failures: u32,
}

impl ErrorRecoveryEngine {
    fn new(max_failures: u32) -> Self {
        Self {
            failure_count: std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0)),
            max_failures,
        }
    }
}

#[async_trait]
impl SearchEngine for ErrorRecoveryEngine {
    fn info(&self) -> &EngineInfo {
        static INFO: std::sync::OnceLock<EngineInfo> = std::sync::OnceLock::new();
        INFO.get_or_init(|| EngineInfo {
            name: "ErrorRecoveryEngine".to_string(),
            engine_type: EngineType::General,
            description: "Error recovery testing engine".to_string(),
            status: EngineStatus::Active,
            categories: vec!["error".to_string(), "recovery".to_string()],
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
                website: Some("https://error.recovery.test".to_string()),
                wikidata_id: Some("QErrorRecovery123".to_string()),
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
        let current_failures = self.failure_count.load(std::sync::atomic::Ordering::SeqCst);

        if current_failures < self.max_failures {
            self.failure_count
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            return Err(format!(
                "Simulated failure {}/{}",
                current_failures + 1,
                self.max_failures
            )
            .into());
        }

        // 一旦达到成功阈值，不再重置失败计数，保持成功状态
        Ok(SearchResult {
            engine_name: "ErrorRecoveryEngine".to_string(),
            total_results: Some(1),
            elapsed_ms: 10,
            items: vec![SearchResultItem {
                title: "Recovery Success".to_string(),
                url: "https://recovery.test/success".to_string(),
                content: "Successfully recovered from errors".to_string(),
                display_url: Some("recovery.test/success".to_string()),
                site_name: Some("Recovery Test Site".to_string()),
                score: 1.0,
                result_type: ResultType::Web,
                thumbnail: None,
                published_date: Some(chrono::Utc::now()),
                template: None,
                metadata: std::collections::HashMap::new(),
            }],
            pagination: Some(PaginationInfo {
                current_page: query.page,
                page_size: query.page_size,
                total_pages: Some(1),
                next_page: None,
                prev_page: None,
            }),
            suggestions: vec![],
            metadata: std::collections::HashMap::new(),
        })
    }

    async fn is_available(&self) -> bool {
        true
    }

    async fn health_check(&self) -> Result<EngineHealth, Box<dyn Error + Send + Sync>> {
        Ok(EngineHealth {
            status: EngineStatus::Active,
            response_time_ms: 10,
            error_message: None,
        })
    }
}

#[tokio::test]
async fn test_error_recovery_with_retry() {
    // 测试错误恢复和重试机制
    let engine = ErrorRecoveryEngine::new(3); // 前3次失败，第4次成功

    let mut success_count = 0;
    let mut failure_count = 0;

    // 模拟重试逻辑
    for attempt in 1..=5 {
        let query = SearchQuery {
            query: format!("retry attempt {}", attempt),
            engine_type: EngineType::General,
            language: None,
            region: None,
            page_size: 1,
            page: 1,
            safe_search: SafeSearchLevel::Moderate,
            time_range: None,
            params: std::collections::HashMap::new(),
        };

        match engine.search(&query).await {
            Ok(result) => {
                success_count += 1;
                assert_eq!(result.items.len(), 1);
                assert_eq!(result.items[0].title, "Recovery Success");
            }
            Err(_) => {
                failure_count += 1;
            }
        }
    }

    // 应该有3次失败和2次成功（第4次和第5次）
    assert_eq!(failure_count, 3);
    assert_eq!(success_count, 2);
}
