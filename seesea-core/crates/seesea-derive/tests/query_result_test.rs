//! 查询和结果处理测试

use async_trait::async_trait;
use seesea_derive::*;
use std::error::Error;

// 模拟QueryBuilder实现
struct MockQueryBuilder;

impl QueryBuilder for MockQueryBuilder {
    fn build_base_query(&self, query: &str, engine_type: EngineType) -> SearchQuery {
        SearchQuery {
            query: query.to_string(),
            engine_type,
            language: Some("en".to_string()),
            region: Some("US".to_string()),
            page_size: 20,
            page: 1,
            safe_search: SafeSearchLevel::Moderate,
            time_range: None,
            params: std::collections::HashMap::new(),
        }
    }

    fn with_language(self, _language: impl Into<String>) -> Self {
        // 模拟实现 - 实际应该返回带有新语言的构建器
        self
    }

    fn with_region(self, _region: impl Into<String>) -> Self {
        // 模拟实现 - 实际应该返回带有新地区的构建器
        self
    }

    fn with_page_size(self, _size: usize) -> Self {
        // 模拟实现 - 实际应该返回带有新页面大小的构建器
        self
    }

    fn with_safe_search(self, _level: SafeSearchLevel) -> Self {
        // 模拟实现 - 实际应该返回带有新安全搜索级别的构建器
        self
    }

    fn with_time_range(self, _range: TimeRange) -> Self {
        // 模拟实现 - 实际应该返回带有新时间范围的构建器
        self
    }

    fn with_pagination(self, _page: usize, _page_size: usize) -> Self {
        // 模拟实现 - 实际应该返回带有新页码和页面大小的构建器
        self
    }

    fn build(self) -> SearchQuery {
        // 模拟实现 - 返回最终构建的查询
        SearchQuery {
            query: "mock query".to_string(),
            engine_type: EngineType::General,
            language: Some("en".to_string()),
            region: Some("US".to_string()),
            page_size: 20,
            page: 1,
            safe_search: SafeSearchLevel::Moderate,
            time_range: None,
            params: std::collections::HashMap::new(),
        }
    }
}

#[test]
fn test_query_builder_base_query() {
    // 测试基础查询构建
    let builder = MockQueryBuilder;
    let query = builder.build_base_query("rust programming", EngineType::General);

    assert_eq!(query.query, "rust programming");
    assert_eq!(query.engine_type, EngineType::General);
    assert_eq!(query.language, Some("en".to_string()));
    assert_eq!(query.region, Some("US".to_string()));
    assert_eq!(query.page_size, 20);
}

#[test]
fn test_query_builder_different_engine_types() {
    // 测试不同引擎类型的查询构建
    let builder = MockQueryBuilder;

    let engine_types = vec![
        EngineType::General,
        EngineType::Image,
        EngineType::Video,
        EngineType::News,
        EngineType::Academic,
        EngineType::Code,
    ];

    for engine_type in engine_types {
        let query = builder.build_base_query("test query", engine_type);
        assert_eq!(query.engine_type, engine_type);
        assert_eq!(query.query, "test query");
    }
}

// 模拟ResultParser实现
struct MockResultParser;

#[async_trait]
impl ResultParser for MockResultParser {
    async fn parse(
        &self,
        response: &str,
        query: &SearchQuery,
    ) -> Result<SearchResult, Box<dyn Error + Send + Sync>> {
        // 模拟解析HTML响应
        let mut result = SearchResult {
            engine_name: "MockEngine".to_string(),
            items: vec![],
            total_results: Some(1),
            elapsed_ms: 50,
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

        // 基于响应内容创建结果
        if response.contains("<title>") {
            let title = extract_title(response);
            let content = extract_content(response);

            result.items.push(SearchResultItem {
                title,
                url: "https://example.com/result".to_string(),
                content,
                display_url: Some("example.com".to_string()),
                site_name: Some("Example Site".to_string()),
                score: 0.85,
                result_type: ResultType::Web,
                thumbnail: None,
                published_date: None,
                template: None,
                metadata: std::collections::HashMap::new(),
            });
        }

        Ok(result)
    }
}

fn extract_title(html: &str) -> String {
    // 简单的标题提取逻辑
    if let Some(start) = html.find("<title>") {
        if let Some(end) = html.find("</title>") {
            return html[start + 7..end].to_string();
        }
    }
    "No Title".to_string()
}

fn extract_content(html: &str) -> String {
    // 简单的内容提取逻辑
    if let Some(start) = html.find("<p>") {
        if let Some(end) = html.find("</p>") {
            return html[start + 3..end].to_string();
        }
    }
    "No content available".to_string()
}

#[tokio::test]
async fn test_result_parser_html() {
    // 测试HTML结果解析
    let parser = MockResultParser;
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

    let html_response = r#"
        <html>
            <head><title>Test Result</title></head>
            <body>
                <p>This is a test search result content.</p>
            </body>
        </html>
    "#;

    let result = parser.parse(html_response, &query).await.unwrap();

    assert_eq!(result.engine_name, "MockEngine");
    assert_eq!(result.items.len(), 1);

    let first_result = &result.items[0];
    assert_eq!(first_result.title, "Test Result");
    assert_eq!(
        first_result.content,
        "This is a test search result content."
    );
    assert_eq!(first_result.score, 0.85);
}

#[tokio::test]
async fn test_result_parser_empty_response() {
    // 测试空响应解析
    let parser = MockResultParser;
    let query = SearchQuery {
        query: "empty query".to_string(),
        engine_type: EngineType::General,
        language: None,
        region: None,
        page_size: 10,
        page: 1,
        safe_search: SafeSearchLevel::Moderate,
        time_range: None,
        params: std::collections::HashMap::new(),
    };

    let empty_response = "";
    let result = parser.parse(empty_response, &query).await.unwrap();

    assert_eq!(result.engine_name, "MockEngine");
    assert_eq!(result.items.len(), 0); // 空响应应该没有结果
}

// 模拟JsonResultParser实现
struct MockJsonResultParser;

impl JsonResultParser for MockJsonResultParser {
    fn parse_item(
        &self,
        raw: &serde_json::Value,
    ) -> Result<SearchResultItem, Box<dyn Error + Send + Sync>> {
        let title = self.extract_title(raw)?;
        let url = self.extract_url(raw)?;
        let content = self.extract_content(raw)?;

        Ok(SearchResultItem {
            title,
            url,
            content,
            display_url: self.extract_display_url(raw).ok(),
            site_name: self.extract_site_name(raw).ok(),
            score: self.extract_score(raw).unwrap_or(0.0),
            result_type: self.extract_result_type(raw).unwrap_or(ResultType::Web),
            thumbnail: self.extract_thumbnail(raw).ok(),
            published_date: self.extract_published_date(raw).ok(),
            template: None,
            metadata: std::collections::HashMap::new(),
        })
    }

    fn extract_title(
        &self,
        raw: &serde_json::Value,
    ) -> Result<String, Box<dyn Error + Send + Sync>> {
        raw["title"]
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| "Missing title".into())
    }

    fn extract_url(&self, raw: &serde_json::Value) -> Result<String, Box<dyn Error + Send + Sync>> {
        raw["url"]
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| "Missing URL".into())
    }

    fn extract_content(
        &self,
        raw: &serde_json::Value,
    ) -> Result<String, Box<dyn Error + Send + Sync>> {
        Ok(raw["content"]
            .as_str()
            .unwrap_or("No content available")
            .to_string())
    }

    fn extract_display_url(
        &self,
        raw: &serde_json::Value,
    ) -> Result<String, Box<dyn Error + Send + Sync>> {
        raw["displayUrl"]
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| "Missing display URL".into())
    }

    fn extract_site_name(
        &self,
        raw: &serde_json::Value,
    ) -> Result<String, Box<dyn Error + Send + Sync>> {
        raw["siteName"]
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| "Missing site name".into())
    }

    fn extract_score(&self, raw: &serde_json::Value) -> Result<f64, Box<dyn Error + Send + Sync>> {
        raw["score"].as_f64().ok_or_else(|| "Missing score".into())
    }

    fn extract_result_type(
        &self,
        raw: &serde_json::Value,
    ) -> Result<ResultType, Box<dyn Error + Send + Sync>> {
        let result_type_str = raw["type"].as_str().ok_or_else(|| "Missing result type")?;

        match result_type_str {
            "web" => Ok(ResultType::Web),
            "image" => Ok(ResultType::Image),
            "video" => Ok(ResultType::Video),
            "news" => Ok(ResultType::News),
            _ => Ok(ResultType::Web),
        }
    }

    fn extract_thumbnail(
        &self,
        raw: &serde_json::Value,
    ) -> Result<String, Box<dyn Error + Send + Sync>> {
        raw["thumbnail"]
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| "Missing thumbnail".into())
    }

    fn extract_published_date(
        &self,
        raw: &serde_json::Value,
    ) -> Result<chrono::DateTime<chrono::Utc>, Box<dyn Error + Send + Sync>> {
        let date_str = raw["publishedDate"]
            .as_str()
            .ok_or_else(|| "Missing published date")?;

        chrono::DateTime::parse_from_rfc3339(date_str)
            .map(|dt| dt.with_timezone(&chrono::Utc))
            .map_err(|e| e.into())
    }

    fn extract_metadata(
        &self,
        raw: &serde_json::Value,
    ) -> Result<std::collections::HashMap<String, String>, Box<dyn Error + Send + Sync>> {
        // 从JSON中提取元数据，如果没有则返回空HashMap
        if let Some(metadata_obj) = raw.get("metadata") {
            if let Some(obj) = metadata_obj.as_object() {
                let mut metadata = std::collections::HashMap::new();
                for (key, value) in obj {
                    if let Some(str_value) = value.as_str() {
                        metadata.insert(key.clone(), str_value.to_string());
                    }
                }
                return Ok(metadata);
            }
        }
        Ok(std::collections::HashMap::new())
    }
}

#[test]
fn test_json_result_parser_valid_data() {
    // 测试JSON结果解析器处理有效数据
    let parser = MockJsonResultParser;

    let json_data = serde_json::json!({
        "title": "Test Result",
        "url": "https://example.com/test",
        "content": "This is test content",
        "displayUrl": "example.com",
        "siteName": "Example Site",
        "score": 0.95,
        "type": "web",
        "thumbnail": "https://example.com/thumb.jpg",
        "publishedDate": "2025-01-01T00:00:00Z"
    });

    let item = parser.parse_item(&json_data).unwrap();

    assert_eq!(item.title, "Test Result");
    assert_eq!(item.url, "https://example.com/test");
    assert_eq!(item.content, "This is test content");
    assert_eq!(item.display_url, Some("example.com".to_string()));
    assert_eq!(item.site_name, Some("Example Site".to_string()));
    assert_eq!(item.score, 0.95);
    assert_eq!(item.result_type, ResultType::Web);
    assert_eq!(
        item.thumbnail,
        Some("https://example.com/thumb.jpg".to_string())
    );
    assert!(item.published_date.is_some());
}

#[test]
fn test_json_result_parser_missing_optional_fields() {
    // 测试JSON结果解析器处理缺少可选字段的数据
    let parser = MockJsonResultParser;

    let json_data = serde_json::json!({
        "title": "Minimal Result",
        "url": "https://example.com/minimal",
        "content": "Minimal content",
        "score": 0.5,
        "type": "web"
        // 缺少可选字段
    });

    let item = parser.parse_item(&json_data).unwrap();

    assert_eq!(item.title, "Minimal Result");
    assert_eq!(item.url, "https://example.com/minimal");
    assert_eq!(item.content, "Minimal content");
    assert_eq!(item.score, 0.5);
    assert_eq!(item.result_type, ResultType::Web);

    // 可选字段应该为None或使用默认值
    assert_eq!(item.display_url, None);
    assert_eq!(item.site_name, None);
    assert_eq!(item.thumbnail, None);
    assert_eq!(item.published_date, None);
}

#[test]
fn test_json_result_parser_invalid_data() {
    // 测试JSON结果解析器处理无效数据
    let parser = MockJsonResultParser;

    // 缺少必需字段
    let invalid_json = serde_json::json!({
        "content": "This has no title or URL"
    });

    let result = parser.parse_item(&invalid_json);
    assert!(result.is_err());

    let error = result.unwrap_err();
    assert!(error.to_string().contains("Missing title"));
}

#[test]
fn test_result_type_parsing() {
    // 测试结果类型解析
    let parser = MockJsonResultParser;

    let test_cases = vec![
        ("web", ResultType::Web),
        ("image", ResultType::Image),
        ("video", ResultType::Video),
        ("news", ResultType::News),
        ("unknown", ResultType::Web), // 未知类型应该默认为Web
    ];

    for (type_str, expected_type) in test_cases {
        let json_data = serde_json::json!({
            "title": "Test",
            "url": "https://example.com",
            "content": "Test content",
            "displayUrl": "example.com",
            "siteName": "Example",
            "score": 0.8,
            "type": type_str,
            "thumbnail": "https://example.com/thumb.jpg",
            "publishedDate": "2025-01-01T00:00:00Z"
        });

        let item = parser.parse_item(&json_data).unwrap();
        assert_eq!(item.result_type, expected_type);
    }
}
