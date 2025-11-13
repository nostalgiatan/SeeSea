//! 搜索引擎派生模块的集成测试
//!
//! 本测试文件包含对 derive 模块所有功能的综合测试。

#[cfg(test)]
mod tests {
    use SeeSea::derive::*;
    use SeeSea::config::common::SafeSearchLevel;
    use std::collections::HashMap;

    /// 测试搜索查询的创建和验证
    #[test]
    fn test_search_query_creation() {
        let query = SearchQuery {
            query: "rust programming".to_string(),
            engine_type: EngineType::General,
            language: Some("zh".to_string()),
            region: Some("CN".to_string()),
            page_size: 10,
            page: 1,
            safe_search: SafeSearchLevel::Moderate,
            time_range: Some(TimeRange::Week),
            params: HashMap::new(),
        };

        assert_eq!(query.query, "rust programming");
        assert_eq!(query.engine_type, EngineType::General);
        assert_eq!(query.language, Some("zh".to_string()));
        assert_eq!(query.page_size, 10);
    }

    /// 测试搜索结果项的创建
    #[test]
    fn test_search_result_item_creation() {
        let mut metadata = HashMap::new();
        metadata.insert("source".to_string(), "test".to_string());

        let item = SearchResultItem {
            title: "Test Result".to_string(),
            url: "https://example.com".to_string(),
            content: "This is test content".to_string(),
            display_url: Some("example.com".to_string()),
            site_name: Some("Example Site".to_string()),
            score: 0.95,
            result_type: ResultType::Web,
            thumbnail: None,
            published_date: None,
            metadata,
        };

        assert_eq!(item.title, "Test Result");
        assert_eq!(item.score, 0.95);
        assert_eq!(item.result_type, ResultType::Web);
    }

    /// 测试引擎信息的完整性
    #[test]
    fn test_engine_info_creation() {
        let info = EngineInfo {
            name: "TestEngine".to_string(),
            engine_type: EngineType::General,
            description: "A test search engine".to_string(),
            website: Some("https://test.com".to_string()),
            status: EngineStatus::Active,
            categories: vec!["general".to_string(), "test".to_string()],
            capabilities: EngineCapabilities {
                result_types: vec![ResultType::Web, ResultType::News],
                supported_params: vec![
                    "q".to_string(),
                    "lang".to_string(),
                    "page".to_string(),
                ],
                max_page_size: 100,
                supports_pagination: true,
                supports_time_range: true,
                supports_language_filter: true,
                supports_region_filter: true,
                supports_safe_search: true,
                rate_limit: Some(60),
            },
            timeout: Some(30),
            version: Some("1.0.0".to_string()),
            last_checked: None,
        };

        assert_eq!(info.name, "TestEngine");
        assert_eq!(info.status, EngineStatus::Active);
        assert_eq!(info.capabilities.max_page_size, 100);
        assert!(info.capabilities.supports_pagination);
    }

    /// 测试分页信息
    #[test]
    fn test_pagination_info() {
        let pagination = PaginationInfo {
            current_page: 2,
            page_size: 10,
            total_pages: Some(100),
            next_page: Some("https://example.com?page=3".to_string()),
            prev_page: Some("https://example.com?page=1".to_string()),
        };

        assert_eq!(pagination.current_page, 2);
        assert_eq!(pagination.page_size, 10);
        assert_eq!(pagination.total_pages, Some(100));
    }

    /// 测试引擎类型的序列化
    #[test]
    fn test_engine_type_serialization() {
        let engine_type = EngineType::General;
        let serialized = serde_json::to_string(&engine_type).expect("序列化失败");
        assert_eq!(serialized, "\"general\"");

        let deserialized: EngineType =
            serde_json::from_str(&serialized).expect("反序列化失败");
        assert_eq!(deserialized, EngineType::General);
    }

    /// 测试时间范围的序列化
    #[test]
    fn test_time_range_serialization() {
        let time_range = TimeRange::Week;
        let serialized = serde_json::to_string(&time_range).expect("序列化失败");
        assert_eq!(serialized, "\"week\"");

        let deserialized: TimeRange =
            serde_json::from_str(&serialized).expect("反序列化失败");
        assert_eq!(deserialized, TimeRange::Week);
    }

    /// 测试结果类型的默认值
    #[test]
    fn test_result_type_default() {
        let default_type = ResultType::default();
        assert_eq!(default_type, ResultType::Web);
    }

    /// 测试引擎状态的默认值
    #[test]
    fn test_engine_status_default() {
        let default_status = EngineStatus::default();
        assert_eq!(default_status, EngineStatus::Active);
    }

    /// 测试时间范围的默认值
    #[test]
    fn test_time_range_default() {
        let default_range = TimeRange::default();
        assert_eq!(default_range, TimeRange::Any);
    }

    /// 测试引擎类型的默认值
    #[test]
    fn test_engine_type_default() {
        let default_type = EngineType::default();
        assert_eq!(default_type, EngineType::General);
    }

    /// 测试所有引擎类型枚举值
    #[test]
    fn test_all_engine_types() {
        let types = vec![
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

        assert_eq!(types.len(), 9);
        assert!(types.contains(&EngineType::General));
        assert!(types.contains(&EngineType::Image));
    }

    /// 测试所有结果类型枚举值
    #[test]
    fn test_all_result_types() {
        let types = vec![
            ResultType::Web,
            ResultType::Image,
            ResultType::Video,
            ResultType::News,
            ResultType::Academic,
            ResultType::Code,
            ResultType::Shopping,
            ResultType::Music,
        ];

        assert_eq!(types.len(), 8);
        assert!(types.contains(&ResultType::Web));
        assert!(types.contains(&ResultType::News));
    }

    /// 测试所有时间范围枚举值
    #[test]
    fn test_all_time_ranges() {
        let ranges = vec![
            TimeRange::Any,
            TimeRange::Hour,
            TimeRange::Day,
            TimeRange::Week,
            TimeRange::Month,
            TimeRange::Year,
        ];

        assert_eq!(ranges.len(), 6);
        assert!(ranges.contains(&TimeRange::Any));
        assert!(ranges.contains(&TimeRange::Week));
    }

    /// 测试所有引擎状态枚举值
    #[test]
    fn test_all_engine_statuses() {
        let statuses = vec![
            EngineStatus::Active,
            EngineStatus::Maintenance,
            EngineStatus::Disabled,
            EngineStatus::Error,
        ];

        assert_eq!(statuses.len(), 4);
        assert!(statuses.contains(&EngineStatus::Active));
        assert!(statuses.contains(&EngineStatus::Error));
    }

    /// 测试搜索结果的JSON序列化
    #[test]
    fn test_search_result_serialization() {
        let result = SearchResult {
            engine_name: "TestEngine".to_string(),
            total_results: Some(1000),
            elapsed_ms: 150,
            items: vec![],
            pagination: None,
            suggestions: vec!["rust".to_string(), "programming".to_string()],
            metadata: HashMap::new(),
        };

        let json = serde_json::to_string(&result).expect("序列化失败");
        assert!(json.contains("TestEngine"));
        assert!(json.contains("1000"));

        let deserialized: SearchResult =
            serde_json::from_str(&json).expect("反序列化失败");
        assert_eq!(deserialized.engine_name, "TestEngine");
        assert_eq!(deserialized.total_results, Some(1000));
    }
}
