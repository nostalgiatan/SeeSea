//! 示例搜索引擎实现
//!
//! 本示例展示了如何使用 derive 模块创建一个完整的搜索引擎实现。

use SeeSea::derive::*;
use SeeSea::config::common::SafeSearchLevel;
use async_trait::async_trait;
use std::collections::HashMap;
use chrono::Utc;

/// 示例搜索引擎
///
/// 这是一个简单的示例引擎，展示了如何实现 SearchEngine trait
pub struct ExampleEngine {
    /// HTTP 客户端
    client: HttpClient,
    /// 引擎信息
    info: EngineInfo,
    /// 速率限制器
    rate_limiter: RateLimiter,
    /// 缓存
    cache: MemoryCache,
}

impl ExampleEngine {
    /// 创建新的示例搜索引擎
    pub fn new() -> Result<Self> {
        // 创建 HTTP 客户端
        let client = ClientBuilder::new()
            .timeout(30)
            .user_agent("ExampleEngine/1.0")
            .build()?;
        let http_client = HttpClient::with_client(client);

        // 定义引擎信息
        let info = EngineInfo {
            name: "ExampleEngine".to_string(),
            engine_type: EngineType::General,
            description: "一个示例搜索引擎".to_string(),
            website: Some("https://example.com".to_string()),
            status: EngineStatus::Active,
            categories: vec!["general".to_string(), "example".to_string()],
            capabilities: EngineCapabilities {
                result_types: vec![ResultType::Web],
                supported_params: vec![
                    "q".to_string(),
                    "lang".to_string(),
                    "page".to_string(),
                    "safe".to_string(),
                ],
                max_page_size: 50,
                supports_pagination: true,
                supports_time_range: false,
                supports_language_filter: true,
                supports_region_filter: false,
                supports_safe_search: true,
                rate_limit: Some(60),
            },
            timeout: Some(30),
            version: Some("1.0.0".to_string()),
            last_checked: Some(Utc::now()),
        };

        // 创建速率限制器
        let rate_limiter = RateLimiter::new(RateLimiterConfig {
            requests_per_minute: 60,
            burst_capacity: 10,
        });

        // 创建缓存
        let cache = MemoryCache::new(CacheConfig {
            default_ttl_secs: 300,
            max_entries: 100,
            enabled: true,
        });

        Ok(Self {
            client: http_client,
            info,
            rate_limiter,
            cache,
        })
    }

    /// 构建搜索URL
    fn build_search_url(&self, query: &SearchQuery) -> Result<String> {
        let mut url = String::from("https://api.example.com/search?");
        
        // 添加查询参数
        url.push_str(&format!("q={}", urlencoding::encode(&query.query)));
        url.push_str(&format!("&page={}", query.page));
        url.push_str(&format!("&size={}", query.page_size));
        
        // 添加语言参数
        if let Some(lang) = &query.language {
            url.push_str(&format!("&lang={}", urlencoding::encode(lang)));
        }
        
        // 添加安全搜索参数
        match query.safe_search {
            SafeSearchLevel::None => {}
            SafeSearchLevel::Moderate => url.push_str("&safe=1"),
            SafeSearchLevel::Strict => url.push_str("&safe=2"),
        }
        
        Ok(url)
    }
}

#[async_trait]
impl SearchEngine for ExampleEngine {
    fn info(&self) -> &EngineInfo {
        &self.info
    }

    async fn search(&self, query: &SearchQuery) -> Result<SearchResult> {
        // 验证查询
        self.validate_query(query)?;

        // 生成缓存键
        let cache_key = MemoryCache::generate_key(
            &self.info.name,
            &query.query,
            query.page,
            Some(&query.params),
        );

        // 检查缓存
        if let Some(cached_result) = self.cache.get(&cache_key)? {
            tracing::info!("返回缓存的搜索结果: {}", query.query);
            return Ok(cached_result);
        }

        // 速率限制检查
        self.rate_limiter.acquire(&self.info.name).await?;

        // 构建URL
        let _url = self.build_search_url(query)?;
        
        tracing::info!("发送搜索请求: {}", query.query);

        // 为了示例，创建一个模拟结果
        let result = SearchResult {
            engine_name: self.info.name.clone(),
            total_results: Some(1000),
            elapsed_ms: 150,
            items: vec![
                SearchResultItem {
                    title: format!("示例结果 - {}", query.query),
                    url: "https://example.com/result1".to_string(),
                    content: "这是一个示例搜索结果的内容摘要".to_string(),
                    display_url: Some("example.com".to_string()),
                    site_name: Some("Example Site".to_string()),
                    score: 0.95,
                    result_type: ResultType::Web,
                    thumbnail: None,
                    published_date: None,
                    metadata: HashMap::new(),
                },
            ],
            pagination: Some(PaginationInfo {
                current_page: query.page,
                page_size: query.page_size,
                total_pages: Some(100),
                next_page: None,
                prev_page: None,
            }),
            suggestions: vec!["相关搜索1".to_string(), "相关搜索2".to_string()],
            metadata: HashMap::new(),
        };

        // 存储到缓存
        self.cache.set(cache_key, result.clone(), None)?;

        Ok(result)
    }

    async fn is_available(&self) -> bool {
        // 这里可以实现健康检查
        true
    }

    async fn health_check(&self) -> Result<EngineHealth> {
        Ok(EngineHealth {
            status: EngineStatus::Active,
            response_time_ms: 50,
            error_message: None,
        })
    }
}

/// 运行示例
#[tokio::main]
async fn main() -> Result<()> {
    // 初始化日志
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    // 创建搜索引擎
    let engine = ExampleEngine::new()?;
    
    println!("引擎信息:");
    println!("  名称: {}", engine.info().name);
    println!("  类型: {:?}", engine.info().engine_type);
    println!("  状态: {:?}", engine.info().status);
    println!("  最大页面大小: {}", engine.info().capabilities.max_page_size);
    
    // 创建搜索查询
    let query = SearchQuery {
        query: "rust programming".to_string(),
        engine_type: EngineType::General,
        language: Some("zh".to_string()),
        region: None,
        page_size: 10,
        page: 1,
        safe_search: SafeSearchLevel::Moderate,
        time_range: None,
        params: HashMap::new(),
    };

    println!("\n执行搜索: {}", query.query);
    
    // 执行搜索
    match engine.search(&query).await {
        Ok(result) => {
            println!("\n搜索结果:");
            println!("  总结果数: {:?}", result.total_results);
            println!("  耗时: {}ms", result.elapsed_ms);
            println!("  结果数量: {}", result.items.len());
            
            for (i, item) in result.items.iter().enumerate() {
                println!("\n  {}. {}", i + 1, item.title);
                println!("     URL: {}", item.url);
                println!("     评分: {:.2}", item.score);
                println!("     摘要: {}", item.content);
            }

            if !result.suggestions.is_empty() {
                println!("\n  相关搜索:");
                for suggestion in &result.suggestions {
                    println!("    - {}", suggestion);
                }
            }
        }
        Err(e) => {
            eprintln!("搜索失败: {}", e);
        }
    }

    // 第二次搜索应该命中缓存
    println!("\n\n执行第二次相同搜索（应该命中缓存）:");
    match engine.search(&query).await {
        Ok(_) => println!("搜索成功（来自缓存）"),
        Err(e) => eprintln!("搜索失败: {}", e),
    }

    Ok(())
}
