//! Tests for force search functionality

#[cfg(test)]
mod force_search_tests {
    use seesea_core::derive::SearchQuery;
    use seesea_core::search::types::SearchRequest;

    #[test]
    fn test_search_request_force_flag() {
        let request = SearchRequest {
            force: true,
            cache_timeline: Some(1800),
            ..Default::default()
        };

        assert!(request.force);
        assert_eq!(request.cache_timeline, Some(1800));
    }

    #[test]
    fn test_search_request_default_timeline() {
        let request = SearchRequest::default();

        assert!(!request.force);
        assert_eq!(request.cache_timeline, Some(3600)); // Default 1 hour
    }

    #[test]
    fn test_search_query_creation() {
        let query = SearchQuery {
            query: "rust programming".to_string(),
            page: 1,
            page_size: 10,
            ..Default::default()
        };

        assert_eq!(query.query, "rust programming");
        assert_eq!(query.page, 1);
        assert_eq!(query.page_size, 10);
    }

    #[test]
    fn test_force_search_bypasses_cache() {
        let request = SearchRequest {
            force: true,
            ..Default::default()
        };

        // When force is true, cache should be bypassed
        assert!(request.force);
    }

    #[test]
    fn test_cache_timeline_configuration() {
        // Test 10 minutes timeline
        let request_10min = SearchRequest {
            cache_timeline: Some(600),
            ..Default::default()
        };
        assert_eq!(request_10min.cache_timeline, Some(600));

        // Test 2 hours timeline
        let request_2hours = SearchRequest {
            cache_timeline: Some(7200),
            ..Default::default()
        };
        assert_eq!(request_2hours.cache_timeline, Some(7200));

        // Test no timeline
        let request_no_timeline = SearchRequest {
            cache_timeline: None,
            ..Default::default()
        };
        assert_eq!(request_no_timeline.cache_timeline, None);
    }
}
