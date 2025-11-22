//! Tests for RSS functionality

#[cfg(test)]
mod rss_tests {
    use seesea_core::derive::rss::{RssFeedItem, RssFeedMeta, RssFeed, RssFeedQuery};

    #[test]
    fn test_rss_feed_item_creation() {
        let item = RssFeedItem {
            title: "Test Article".to_string(),
            link: "https://example.com/article".to_string(),
            description: Some("Test description".to_string()),
            content: None,
            author: Some("Test Author".to_string()),
            pub_date: None,
            guid: Some("unique-id-123".to_string()),
            categories: vec!["tech".to_string(), "rust".to_string()],
            enclosures: vec![],
            custom_fields: std::collections::HashMap::new(),
        };
        
        assert_eq!(item.title, "Test Article");
        assert_eq!(item.link, "https://example.com/article");
        assert_eq!(item.categories.len(), 2);
    }

    #[test]
    fn test_rss_feed_meta_creation() {
        let meta = RssFeedMeta {
            title: "Test Feed".to_string(),
            link: "https://example.com".to_string(),
            description: Some("Test RSS Feed".to_string()),
            language: Some("en".to_string()),
            copyright: None,
            image: None,
            pub_date: None,
            last_build_date: None,
        };
        
        assert_eq!(meta.title, "Test Feed");
        assert_eq!(meta.link, "https://example.com");
    }

    #[test]
    fn test_rss_feed_creation() {
        let meta = RssFeedMeta {
            title: "Test Feed".to_string(),
            link: "https://example.com".to_string(),
            description: Some("Test RSS Feed".to_string()),
            language: Some("en".to_string()),
            copyright: None,
            image: None,
            pub_date: None,
            last_build_date: None,
        };
        
        let feed = RssFeed {
            meta,
            items: vec![],
        };
        
        assert_eq!(feed.meta.title, "Test Feed");
        assert_eq!(feed.items.len(), 0);
    }

    #[test]
    fn test_rss_feed_query_defaults() {
        let query = RssFeedQuery {
            url: "https://example.com/rss".to_string(),
            max_items: None,
            filter_keywords: vec![],
            after_date: None,
        };
        
        assert_eq!(query.url, "https://example.com/rss");
        assert!(query.max_items.is_none());
        assert!(query.filter_keywords.is_empty());
    }

    #[test]
    fn test_rss_feed_query_with_filters() {
        let query = RssFeedQuery {
            url: "https://example.com/rss".to_string(),
            max_items: Some(10),
            filter_keywords: vec!["rust".to_string(), "programming".to_string()],
            after_date: None,
        };
        
        assert_eq!(query.max_items, Some(10));
        assert_eq!(query.filter_keywords.len(), 2);
    }
}
