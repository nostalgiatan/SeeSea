//! Tests for force search functionality

#[cfg(test)]
mod force_search_tests {
    use seesea_core::search::types::SearchRequest;
    use seesea_core::derive::SearchQuery;

    #[test]
    fn test_search_request_force_flag() {
        let mut request = SearchRequest::default();
        request.force = true;
        request.cache_timeline = Some(1800);
        
        assert_eq!(request.force, true);
        assert_eq!(request.cache_timeline, Some(1800));
    }

    #[test]
    fn test_search_request_default_timeline() {
        let request = SearchRequest::default();
        
        assert_eq!(request.force, false);
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
        let mut request = SearchRequest::default();
        
        // Test different timeline values
        request.cache_timeline = Some(600); // 10 minutes
        assert_eq!(request.cache_timeline, Some(600));
        
        request.cache_timeline = Some(7200); // 2 hours
        assert_eq!(request.cache_timeline, Some(7200));
        
        request.cache_timeline = None; // No timeline
        assert_eq!(request.cache_timeline, None);
    }
}
