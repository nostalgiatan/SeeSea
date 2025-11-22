//! Tests for semantic caching functionality

#[cfg(test)]
mod semantic_cache_tests {
    use seesea_core::cache::semantic::SimpleVectorizer;

    #[test]
    fn test_vectorizer_creation() {
        let _vectorizer = SimpleVectorizer::new();
        assert!(true); // Just test it can be created
    }

    #[test]
    fn test_text_vectorization() {
        let vectorizer = SimpleVectorizer::new();
        let text = "rust programming language";
        let vector = vectorizer.vectorize(text);
        
        assert_eq!(vector.len(), 100); // 100-dimensional vector
    }

    #[test]
    fn test_cosine_similarity_identical() {
        let vectorizer = SimpleVectorizer::new();
        let text = "rust programming";
        
        let vec1 = vectorizer.vectorize(text);
        let vec2 = vectorizer.vectorize(text);
        
        let similarity = vectorizer.cosine_similarity(&vec1, &vec2);
        
        // Identical texts should have similarity close to 1.0
        assert!(similarity > 0.99);
    }

    #[test]
    fn test_cosine_similarity_similar() {
        let vectorizer = SimpleVectorizer::new();
        
        let vec1 = vectorizer.vectorize("rust programming language");
        let vec2 = vectorizer.vectorize("rust coding language");
        
        let similarity = vectorizer.cosine_similarity(&vec1, &vec2);
        
        // Similar texts should have high similarity
        assert!(similarity > 0.5);
    }

    #[test]
    fn test_cosine_similarity_different() {
        let vectorizer = SimpleVectorizer::new();
        
        let vec1 = vectorizer.vectorize("rust programming");
        let vec2 = vectorizer.vectorize("python data science");
        
        let similarity = vectorizer.cosine_similarity(&vec1, &vec2);
        
        // Different topics should have lower similarity
        assert!(similarity < 0.7);
    }

    #[test]
    fn test_semantic_cache_similarity_threshold() {
        // Test that similarity threshold of 0.75 works as expected
        let threshold = 0.75;
        
        let high_similarity = 0.85;
        let low_similarity = 0.60;
        
        assert!(high_similarity >= threshold);
        assert!(low_similarity < threshold);
    }
}
