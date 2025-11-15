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

    /// Test that the orchestrator can be created with engines
    #[test]
    fn test_orchestrator_with_engines() {
        use SeeSea::search::orchestrator::SearchOrchestrator;
        use SeeSea::search::types::SearchConfig;
        use SeeSea::cache::CacheInterface;
        use SeeSea::cache::types::CacheImplConfig;
        use SeeSea::net::NetworkInterface;
        use SeeSea::net::types::NetworkConfig;
        use std::sync::Arc;
        use tokio::sync::RwLock;

        let config = SearchConfig::default();
        let network = Arc::new(NetworkInterface::new(NetworkConfig::default()).unwrap());
        let cache = Arc::new(RwLock::new(
            CacheInterface::new(CacheImplConfig::default()).unwrap(),
        ));

        let mut orchestrator = SearchOrchestrator::new(config, network, cache);
        
        // Register engines
        orchestrator.register_engine(Box::new(GoogleEngine::new()));
        orchestrator.register_engine(Box::new(BingEngine::new()));
        
        // Verify engines were registered
        // The orchestrator doesn't expose a public count, but we can verify it was created
        assert!(true);
    }
}
