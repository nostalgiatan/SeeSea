"""
Example usage of the TF text semantic retrieval system.

This script demonstrates the MEMORY-EFFICIENT design:
1. Initialize the vector store with embeddings
2. Add documents - content is vectorized then DISCARDED (not stored!)
3. Search for similar documents - returns metadata (title, url) but NOT content
4. Remove documents

MEMORY EFFICIENCY:
- Content is NEVER stored in the vector database
- Only vectors and metadata (title, url) are kept
- This allows storing millions of documents with minimal memory footprint
"""

from tf import TextEmbedder, VectorStoreWrapper


def main():
    print("=== TF Memory-Efficient Text Semantic Retrieval System ===\n")
    
    # Initialize the embedder and vector store
    print("Step 1: Initializing embedder and vector store...")
    embedder = TextEmbedder()
    store = VectorStoreWrapper(embedder)
    print(f"✓ Vector store initialized (dimension: {embedder.get_dimension()})")
    print(f"  MEMORY EFFICIENT: Only vectors and metadata stored!\n")
    
    # Add some example documents
    print("Step 2: Adding documents...")
    print("  Note: Content will be vectorized then DISCARDED\n")
    
    documents = [
        {
            "id": "doc1",
            "title": "Python Programming",
            "url": "https://example.com/python",
            "content": "Python is a high-level, interpreted programming language known for its simplicity and readability."
        },
        {
            "id": "doc2",
            "title": "Rust Programming",
            "url": "https://example.com/rust",
            "content": "Rust is a systems programming language that runs blazingly fast and prevents segfaults."
        },
        {
            "id": "doc3",
            "title": "JavaScript Basics",
            "url": "https://example.com/javascript",
            "content": "JavaScript is a versatile programming language primarily used for web development."
        },
        {
            "id": "doc4",
            "title": "Machine Learning",
            "url": "https://example.com/ml",
            "content": "Machine learning is a subset of artificial intelligence that enables systems to learn from data."
        },
        {
            "id": "doc5",
            "title": "Vector Databases",
            "url": "https://example.com/vectordb",
            "content": "Vector databases are specialized databases for storing and searching high-dimensional vectors."
        }
    ]
    
    for doc in documents:
        print(f"  Adding: {doc['title']}")
        store.add_document(
            doc_id=doc['id'],
            content=doc['content'],  # Will be vectorized then discarded!
            title=doc['title'],
            url=doc['url']
        )
        print(f"    ✓ Vectorized and stored (content discarded)")
    
    print(f"\n✓ Added {len(store)} documents\n")
    
    # Search for similar documents
    print("Step 3: Searching for documents...")
    queries = [
        "What is a programming language for systems development?",
        "Tell me about databases for embeddings",
        "How does AI learn from information?"
    ]
    
    for query in queries:
        print(f"\nQuery: '{query}'")
        results = store.search(query, k=3)
        
        print("Results:")
        for i, result in enumerate(results, 1):
            print(f"  {i}. {result['title']} (score: {result['score']:.4f})")
            print(f"     URL: {result['url']}")
            # Note: No 'content' field - we don't store it!
            print(f"     [Content not stored - memory efficient!]")
    
    # Get metadata for a specific document
    print("\n\nStep 4: Getting metadata for a document...")
    metadata = store.get_metadata("doc2")
    if metadata:
        print(f"Document 'doc2' metadata:")
        print(f"  Title: {metadata['title']}")
        print(f"  URL: {metadata['url']}")
        print(f"  Content: [Not stored - memory efficient!]")
    
    # Remove a document
    print("\n\nStep 5: Removing a document...")
    print(f"Documents before removal: {len(store)}")
    store.remove_document("doc3")
    print(f"Documents after removal: {len(store)}")
    print("✓ Document 'doc3' removed")
    
    print("\n\n=== Memory Efficiency Summary ===")
    print(f"Total documents: {len(store)}")
    print(f"Stored per document:")
    print(f"  ✓ Vector ({embedder.get_dimension()} dimensions)")
    print(f"  ✓ Title")
    print(f"  ✓ URL")
    print(f"  ✗ Content (NOT stored - maximum memory efficiency!)")
    
    print("\n=== Example completed successfully! ===")


if __name__ == "__main__":
    main()
