"""
Example demonstrating optimized search with streaming and result objects.

This example shows:
1. Text-to-vector search with automatic sorting by relevance
2. SearchResult objects for structured access
3. Streaming search for memory-efficient iteration
4. Memory optimization techniques
"""

from tf import DocumentStore, SearchResult


def main():
    print("=== Optimized Search Example ===\n")
    
    # Initialize store
    print("Step 1: Initialize DocumentStore...")
    store = DocumentStore()
    print("✓ Store initialized\n")
    
    # Add sample documents
    print("Step 2: Adding documents...")
    documents = [
        {
            "id": "doc1",
            "title": "Introduction to Machine Learning",
            "url": "https://example.com/ml-intro",
            "content": "Machine learning is a subset of artificial intelligence that enables systems to learn and improve from experience without being explicitly programmed. It focuses on the development of computer programs that can access data and use it to learn for themselves.",
            "summary": "ML basics and core concepts"
        },
        {
            "id": "doc2",
            "title": "Deep Learning Fundamentals",
            "url": "https://example.com/dl-fundamentals",
            "content": "Deep learning is a machine learning technique that teaches computers to do what comes naturally to humans: learn by example. Deep learning is a key technology behind driverless cars, voice control, and many other applications.",
            "summary": "Introduction to neural networks and deep learning"
        },
        {
            "id": "doc3",
            "title": "Natural Language Processing",
            "url": "https://example.com/nlp",
            "content": "Natural language processing (NLP) is a branch of artificial intelligence that helps computers understand, interpret and manipulate human language. NLP draws from many disciplines, including computer science and computational linguistics.",
            "summary": "NLP techniques and applications"
        },
        {
            "id": "doc4",
            "title": "Computer Vision Techniques",
            "url": "https://example.com/cv",
            "content": "Computer vision is an interdisciplinary field that deals with how computers can gain high-level understanding from digital images or videos. It seeks to automate tasks that the human visual system can do.",
            "summary": "Image processing and computer vision"
        },
        {
            "id": "doc5",
            "title": "Reinforcement Learning Guide",
            "url": "https://example.com/rl",
            "content": "Reinforcement learning is an area of machine learning concerned with how intelligent agents ought to take actions in an environment in order to maximize the notion of cumulative reward.",
            "summary": "RL algorithms and applications"
        }
    ]
    
    store.add_batch(documents, parallel=True)
    print(f"✓ Added {len(documents)} documents\n")
    
    # Example 1: Standard search with dictionaries
    print("="*60)
    print("Example 1: Standard Search (returns dictionaries)")
    print("="*60)
    query1 = "How do AI systems learn from data?"
    print(f"Query: '{query1}'")
    print(f"\nResults (sorted by relevance, highest first):")
    
    results = store.search(query1, k=3)
    for i, r in enumerate(results, 1):
        print(f"\n{i}. {r['title']}")
        print(f"   Relevance: {r['score']:.4f}")
        print(f"   Summary: {r['summary']}")
        print(f"   URL: {r['url']}")
        # Note: No 'vector' field - memory efficient!
    
    # Example 2: Search with SearchResult objects
    print("\n" + "="*60)
    print("Example 2: Search with Result Objects (structured access)")
    print("="*60)
    query2 = "neural networks and image recognition"
    print(f"Query: '{query2}'")
    print(f"\nResults (sorted by relevance):")
    
    results = store.search(query2, k=3, return_objects=True)
    for i, r in enumerate(results, 1):
        print(f"\n{i}. {r.title}")
        print(f"   Score: {r.score:.4f}")
        print(f"   Summary: {r.summary}")
        print(f"   ID: {r.id}")
        # SearchResult is immutable and memory-efficient with __slots__
    
    # Example 3: Streaming search for large result sets
    print("\n" + "="*60)
    print("Example 3: Streaming Search (memory-efficient iteration)")
    print("="*60)
    query3 = "artificial intelligence applications"
    print(f"Query: '{query3}'")
    print(f"\nResults (streamed one at a time):")
    
    # Process results as they come - no buffering
    for i, result in enumerate(store.search_streaming(query3, k=5), 1):
        print(f"{i}. {result.title[:40]:40s} | Score: {result.score:.4f}")
        # Each result is processed immediately and can be freed
        # Minimal memory footprint even with large k
    
    # Example 4: Memory optimization demonstration
    print("\n" + "="*60)
    print("Example 4: Memory Optimization Features")
    print("="*60)
    
    print("\n✓ Optimizations Applied:")
    print("  1. Text → Vector: Query vectorized, then immediately freed")
    print("  2. Search Results: Sorted by relevance (vecstore native)")
    print("  3. No Vector Return: Only metadata + score returned")
    print("  4. Streaming: Results yielded one-by-one, no buffering")
    print("  5. Result Objects: Immutable with __slots__ for minimal memory")
    print("  6. Content Discarded: Original text never stored")
    
    print("\n✓ Search Pipeline:")
    print("  Query Text")
    print("    ↓ (vectorize)")
    print("  Query Vector [temporary]")
    print("    ↓ (search in vecstore)")
    print("  Sorted Results [by relevance]")
    print("    ↓ (extract metadata only)")
    print("  SearchResult Objects")
    print("    ↓ (stream or return)")
    print("  User receives: {id, score, title, url, summary}")
    print("  NOT included: {vector, content}")
    
    print("\n✓ Memory Savings:")
    print("  - No content storage: 70-95% memory saved")
    print("  - No vector in results: Additional 3KB+ per result saved")
    print("  - Streaming support: Constant memory regardless of k")
    print("  - Immutable results: Safe sharing, no defensive copies")
    
    print("\n=== Example completed successfully! ===")


if __name__ == "__main__":
    main()
