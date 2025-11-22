//! Integration tests for search engines

#[cfg(test)]
mod search_engine_tests {
    use seesea_core::search::engines::bing::BingEngine;
    use seesea_core::derive::{SearchEngine, SearchQuery};

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
    /// Note: This test is disabled because SearchOrchestrator module is not yet implemented
    #[test]
    #[ignore]
    fn test_orchestrator_with_engines() {
        // This test requires environment isolation and orchestrator module implementation
        // TODO: Re-enable when orchestrator module is implemented
        /*
        use seesea_core::search::orchestrator::SearchOrchestrator;
        use seesea_core::search::types::SearchConfig;
        use seesea_core::cache::CacheInterface;
        use seesea_core::cache::types::CacheImplConfig;
        use seesea_core::net::NetworkInterface;
        use seesea_core::net::types::NetworkConfig;
        use std::sync::Arc;
        use tokio::sync::RwLock;

        let config = SearchConfig::default();
        let network = Arc::new(NetworkInterface::new(NetworkConfig::default()).unwrap());
        let cache = Arc::new(RwLock::new(
            CacheInterface::new(CacheImplConfig::default()).unwrap(),
        ));

        let mut orchestrator = SearchOrchestrator::new(config, network, cache);
        
        // Register engines
        orchestrator.register_engine(Box::new(BingEngine::new()));
        
        // Verify engines were registered
        assert!(true);
        */
    }
}
