//! Integration tests for search engines

#[cfg(test)]
mod search_engine_tests {
    use SeeSea::search::engines::google::GoogleEngine;
    use SeeSea::search::engines::bing::BingEngine;
    use SeeSea::derive::{SearchEngine, SearchQuery};

    /// Test that Google engine can be created
    #[test]
    fn test_google_engine_creation() {
        let engine = GoogleEngine::new();
        assert_eq!(engine.info().name, "Google");
    }

    /// Test that Bing engine can be created
    #[test]
    fn test_bing_engine_creation() {
        let engine = BingEngine::new();
        assert_eq!(engine.info().name, "Bing");
    }

    /// Test that default SearchQuery can be created
    #[test]
    fn test_search_query_creation() {
        let query = SearchQuery::default();
        assert_eq!(query.query, "");
        assert_eq!(query.page, 1);
        assert_eq!(query.page_size, 10);
    }
}
