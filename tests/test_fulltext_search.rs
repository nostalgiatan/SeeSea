//! 全文搜索功能测试
//!
//! 测试数据库缓存搜索、RSS搜索和完整的全文搜索集成

use std::collections::HashMap;
use std::time::Duration;
use serial_test::serial;
use seesea_core::cache::on::CacheInterface;
use seesea_core::cache::types::{CacheImplConfig, CacheMode};
use seesea_core::derive::types::{SearchQuery, SearchResultItem, EngineType, ResultType};
use seesea_core::derive::SearchResult;
use seesea_core::config::common::SafeSearchLevel;
use seesea_core::derive::rss::{RssFeed, RssFeedItem, RssFeedMeta};

/// 创建临时缓存配置用于测试
fn temp_cache_config() -> CacheImplConfig {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    
    let temp_dir = std::env::temp_dir();
    let unique_id = COUNTER.fetch_add(1, Ordering::SeqCst);
    let db_path = temp_dir.join(format!(
        "test_fulltext_{}_{}", 
        std::process::id(),
        unique_id
    ));
    
    CacheImplConfig {
        db_path: db_path.to_string_lossy().to_string(),
        default_ttl_secs: 3600,
        max_size_bytes: 1024 * 1024 * 10, // 10MB
        enabled: true,
        compression: false,
        mode: CacheMode::HighThroughput,
    }
}

/// 创建示例搜索查询
fn sample_query(query_text: &str) -> SearchQuery {
    SearchQuery {
        query: query_text.to_string(),
        engine_type: EngineType::General,
        language: Some("en".to_string()),
        region: None,
        page_size: 10,
        page: 1,
        safe_search: SafeSearchLevel::Moderate,
        time_range: None,
        params: HashMap::new(),
    }
}

/// 创建示例搜索结果项
fn sample_result_item(title: &str, url: &str, content: &str) -> SearchResultItem {
    SearchResultItem {
        title: title.to_string(),
        url: url.to_string(),
        content: content.to_string(),
        display_url: Some(url.to_string()),
        site_name: None,
        score: 0.85,
        result_type: ResultType::Web,
        thumbnail: None,
        published_date: None,
        template: None,
        metadata: HashMap::new(),
    }
}

/// 创建示例搜索结果
fn sample_search_result(engine: &str, items: Vec<SearchResultItem>) -> SearchResult {
    SearchResult {
        engine_name: engine.to_string(),
        total_results: Some(items.len()),
        elapsed_ms: 150,
        items,
        pagination: None,
        suggestions: Vec::new(),
        metadata: HashMap::new(),
    }
}

#[test]
#[serial]
fn test_result_cache_fulltext_search() {
    // 创建缓存
    let config = temp_cache_config();
    let cache_interface = CacheInterface::new(config).expect("创建缓存接口失败");
    let result_cache = cache_interface.results();
    
    // 创建并缓存一些搜索结果
    let query1 = sample_query("rust programming");
    let items1 = vec![
        sample_result_item("Rust Programming Language", "https://www.rust-lang.org", "Rust is a systems programming language"),
        sample_result_item("Learn Rust", "https://doc.rust-lang.org", "Official Rust documentation and tutorials"),
    ];
    let result1 = sample_search_result("TestEngine", items1);
    
    let query2 = sample_query("python development");
    let items2 = vec![
        sample_result_item("Python.org", "https://www.python.org", "Python is a programming language"),
        sample_result_item("Python Tutorial", "https://docs.python.org", "Learn Python programming"),
    ];
    let result2 = sample_search_result("TestEngine", items2);
    
    // 缓存结果
    result_cache.set(&query1, "TestEngine", &result1, None).expect("缓存失败");
    result_cache.set(&query2, "TestEngine", &result2, None).expect("缓存失败");
    
    // 测试全文搜索 - 搜索包含 "programming" 的结果
    let keywords = vec!["programming".to_string()];
    let search_results = result_cache.search_fulltext(&keywords, false, None)
        .expect("全文搜索失败");
    
    // 应该找到包含 "programming" 的结果
    assert!(!search_results.is_empty(), "应该找到至少一个结果");
    
    // 验证结果包含关键词
    for item in &search_results {
        let contains_keyword = item.title.to_lowercase().contains("programming")
            || item.content.to_lowercase().contains("programming")
            || item.url.to_lowercase().contains("programming");
        assert!(contains_keyword, "结果应该包含关键词 'programming'");
    }
    
    // 测试全文搜索 - 搜索包含 "rust" 的结果
    let keywords = vec!["rust".to_string()];
    let rust_results = result_cache.search_fulltext(&keywords, false, None)
        .expect("全文搜索失败");
    
    assert!(!rust_results.is_empty(), "应该找到 Rust 相关结果");
    
    // 测试限制结果数量
    let keywords = vec!["programming".to_string()];
    let limited_results = result_cache.search_fulltext(&keywords, false, Some(1))
        .expect("全文搜索失败");
    
    assert_eq!(limited_results.len(), 1, "应该只返回一个结果");
}

#[test]
#[serial]
fn test_result_cache_search_with_stale() {
    // 创建缓存
    let config = temp_cache_config();
    let cache_interface = CacheInterface::new(config).expect("创建缓存接口失败");
    let result_cache = cache_interface.results();
    
    // 创建并缓存一个会很快过期的结果
    let query = sample_query("test query");
    let items = vec![
        sample_result_item("Test Result", "https://example.com", "This is a test result"),
    ];
    let result = sample_search_result("TestEngine", items);
    
    // 设置1秒过期
    result_cache.set(&query, "TestEngine", &result, Some(Duration::from_secs(1)))
        .expect("缓存失败");
    
    // 立即搜索 - 应该找到结果（不包含过期的）
    let keywords = vec!["test".to_string()];
    let fresh_results = result_cache.search_fulltext(&keywords, false, None)
        .expect("搜索失败");
    assert_eq!(fresh_results.len(), 1, "应该找到新鲜的结果");
    
    // 等待过期
    std::thread::sleep(Duration::from_millis(1100));
    
    // 搜索 - 不包含过期结果
    let no_stale_results = result_cache.search_fulltext(&keywords, false, None)
        .expect("搜索失败");
    assert_eq!(no_stale_results.len(), 0, "不应该找到过期的结果");
    
    // 搜索 - 包含过期结果
    let with_stale_results = result_cache.search_fulltext(&keywords, true, None)
        .expect("搜索失败");
    assert_eq!(with_stale_results.len(), 1, "应该找到过期的结果");
}

#[test]
#[serial]
fn test_rss_cache_fulltext_search() {
    // 创建缓存
    let config = temp_cache_config();
    let cache_interface = CacheInterface::new(config).expect("创建缓存接口失败");
    let rss_cache = cache_interface.rss();
    
    // 创建示例 RSS feed
    let feed_meta = RssFeedMeta {
        title: "Tech News".to_string(),
        link: "https://example.com/rss".to_string(),
        description: Some("Technology news and updates".to_string()),
        language: Some("en".to_string()),
        copyright: None,
        last_build_date: None,
        pub_date: None,
        image: None,
    };
    
    let feed_items = vec![
        RssFeedItem {
            title: "Rust 1.70 Released".to_string(),
            link: "https://example.com/rust-1.70".to_string(),
            description: Some("New version of Rust programming language released".to_string()),
            author: Some("Rust Team".to_string()),
            pub_date: Some("2023-06-01".to_string()),
            content: Some("Rust 1.70 brings many improvements to the language".to_string()),
            categories: vec!["programming".to_string(), "rust".to_string()],
            guid: Some("rust-1.70".to_string()),
            enclosures: vec![],
            custom_fields: HashMap::new(),
        },
        RssFeedItem {
            title: "Python 3.12 Beta".to_string(),
            link: "https://example.com/python-3.12".to_string(),
            description: Some("Python 3.12 beta is now available".to_string()),
            author: Some("Python Team".to_string()),
            pub_date: Some("2023-05-15".to_string()),
            content: Some("Try the new Python 3.12 beta release".to_string()),
            categories: vec!["programming".to_string(), "python".to_string()],
            guid: Some("python-3.12".to_string()),
            enclosures: vec![],
            custom_fields: HashMap::new(),
        },
    ];
    
    let feed = RssFeed {
        meta: feed_meta,
        items: feed_items,
    };
    
    // 缓存 RSS feed
    rss_cache.set("https://example.com/rss", &feed, false, None, None)
        .expect("缓存 RSS feed 失败");
    
    // 测试全文搜索 - 搜索包含 "rust" 的 RSS 项
    let keywords = vec!["rust".to_string()];
    let search_results = rss_cache.search_fulltext(&keywords, false, None)
        .expect("RSS 全文搜索失败");
    
    assert!(!search_results.is_empty(), "应该找到包含 'rust' 的 RSS 项");
    
    // 验证结果
    for (feed_url, item) in &search_results {
        assert_eq!(feed_url, "https://example.com/rss");
        let contains_keyword = item.title.to_lowercase().contains("rust")
            || item.description.as_ref().map(|d| d.to_lowercase().contains("rust")).unwrap_or(false)
            || item.content.as_ref().map(|c| c.to_lowercase().contains("rust")).unwrap_or(false);
        assert!(contains_keyword, "RSS 项应该包含关键词 'rust'");
    }
    
    // 测试搜索 "programming" - 应该找到至少一个结果（因为在categories中）
    let keywords = vec!["programming".to_string()];
    let prog_results = rss_cache.search_fulltext(&keywords, false, None)
        .expect("RSS 全文搜索失败");
    
    // 由于我们的搜索只检查 title, description, content 和 link，
    // 而 "programming" 只在 categories 中，不在这些字段中，所以可能找不到
    // 但是 description 中有 "programming language" 和 "programming"，所以应该能找到
    assert!(!prog_results.is_empty(), "应该找到至少一个包含 'programming' 的 RSS 项");
}

#[test]
#[serial]
fn test_rss_cache_list_all_feeds() {
    // 创建缓存
    let config = temp_cache_config();
    let cache_interface = CacheInterface::new(config).expect("创建缓存接口失败");
    let rss_cache = cache_interface.rss();
    
    // 创建并缓存两个 RSS feeds
    let feed1 = RssFeed {
        meta: RssFeedMeta {
            title: "Feed 1".to_string(),
            link: "https://example1.com".to_string(),
            description: None,
            language: None,
            copyright: None,
            last_build_date: None,
            pub_date: None,
            image: None,
        },
        items: vec![],
    };
    
    let feed2 = RssFeed {
        meta: RssFeedMeta {
            title: "Feed 2".to_string(),
            link: "https://example2.com".to_string(),
            description: None,
            language: None,
            copyright: None,
            last_build_date: None,
            pub_date: None,
            image: None,
        },
        items: vec![],
    };
    
    rss_cache.set("https://example1.com/rss", &feed1, false, None, None)
        .expect("缓存 feed 1 失败");
    rss_cache.set("https://example2.com/rss", &feed2, false, None, None)
        .expect("缓存 feed 2 失败");
    
    // 列出所有 feeds
    let all_feeds = rss_cache.list_all_feeds().expect("列出 feeds 失败");
    
    // 应该至少有我们添加的两个 feeds
    assert!(all_feeds.len() >= 2, "应该至少有两个缓存的 feeds");
    
    // 验证返回的 URLs 包含我们添加的
    let urls: Vec<String> = all_feeds.iter().map(|(url, _)| url.clone()).collect();
    assert!(urls.contains(&"https://example1.com/rss".to_string()));
    assert!(urls.contains(&"https://example2.com/rss".to_string()));
}

#[test]
#[serial]
fn test_multiple_keyword_search() {
    // 创建缓存
    let config = temp_cache_config();
    let cache_interface = CacheInterface::new(config).expect("创建缓存接口失败");
    let result_cache = cache_interface.results();
    
    // 创建包含多个关键词的结果
    let query = sample_query("rust async programming");
    let items = vec![
        sample_result_item(
            "Async Rust Programming",
            "https://example.com/async-rust",
            "Learn asynchronous programming in Rust"
        ),
        sample_result_item(
            "Rust Tutorials",
            "https://example.com/rust-tutorials",
            "Comprehensive Rust programming tutorials"
        ),
        sample_result_item(
            "JavaScript Async",
            "https://example.com/js-async",
            "Asynchronous JavaScript programming"
        ),
    ];
    let result = sample_search_result("TestEngine", items);
    
    result_cache.set(&query, "TestEngine", &result, None).expect("缓存失败");
    
    // 搜索包含 "rust" 和 "async" 的结果
    let keywords = vec!["rust".to_string(), "async".to_string()];
    let search_results = result_cache.search_fulltext(&keywords, false, None)
        .expect("搜索失败");
    
    // 应该找到至少包含一个关键词的所有结果
    assert!(search_results.len() >= 2, "应该找到至少2个结果");
    
    // 只搜索 "rust"
    let rust_only = vec!["rust".to_string()];
    let rust_results = result_cache.search_fulltext(&rust_only, false, None)
        .expect("搜索失败");
    
    assert!(rust_results.len() >= 2, "应该找到包含 'rust' 的结果");
}
