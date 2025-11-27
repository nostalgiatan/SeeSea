"""
Example usage of the TF SDK with full CRUD operations.

Demonstrates:
- Create: Add documents with automatic vectorization
- Read: Retrieve document metadata
- Update: Modify document metadata
- Delete: Remove documents
- Search: Semantic similarity search
- Batch operations with parallel processing
"""

from tf import DocumentStore


def main():
    print("=== TF SDK Example: Full CRUD Operations ===\n")
    
    # Initialize the document store
    print("Step 1: Initialize DocumentStore SDK...")
    store = DocumentStore()
    print(f"✓ SDK initialized (dimension: {store.embedder.get_dimension()})")
    print(f"  Memory efficient: Only vectors + metadata stored!\n")
    
    # CREATE: Add documents
    print("Step 2: CREATE - Adding documents...")
    
    # Single document
    store.add(
        doc_id="doc1",
        content="Python is a high-level programming language known for its simplicity and readability.",
        title="Python Programming",
        url="https://example.com/python",
        summary="Introduction to Python programming language"
    )
    print("  ✓ Added doc1")
    
    # Batch add with parallel processing
    documents = [
        {
            "id": "doc2",
            "title": "Rust Programming",
            "url": "https://example.com/rust",
            "content": "Rust is a systems programming language that runs blazingly fast and prevents segfaults.",
            "summary": "Overview of Rust programming language"
        },
        {
            "id": "doc3",
            "title": "Machine Learning",
            "url": "https://example.com/ml",
            "content": "Machine learning is a subset of artificial intelligence that enables systems to learn from data.",
            "summary": "Introduction to machine learning concepts"
        },
        {
            "id": "doc4",
            "title": "Vector Databases",
            "url": "https://example.com/vectordb",
            "content": "Vector databases are specialized databases for storing and searching high-dimensional vectors.",
            "summary": "Guide to vector database systems"
        }
    ]
    
    store.add_batch(documents, parallel=True)
    print(f"  ✓ Added {len(documents)} documents in parallel")
    print(f"  Total documents: {store.count()}\n")
    
    # READ: Get document metadata
    print("Step 3: READ - Retrieving document metadata...")
    metadata = store.get("doc1")
    if metadata:
        print(f"  Document 'doc1':")
        print(f"    Title: {metadata['title']}")
        print(f"    URL: {metadata['url']}")
        print(f"    Summary: {metadata['summary']}")
        print(f"    [Content NOT stored - memory efficient!]\n")
    
    # SEARCH: Semantic similarity search
    print("Step 4: SEARCH - Semantic similarity search...")
    queries = [
        "What programming language is good for systems development?",
        "How do AI systems learn?",
        "Where can I store high-dimensional embeddings?"
    ]
    
    for query in queries:
        print(f"\n  Query: '{query}'")
        results = store.search(query, k=2)
        print("  Results:")
        for i, result in enumerate(results, 1):
            print(f"    {i}. {result['title']} (score: {result['score']:.4f})")
            print(f"       Summary: {result.get('summary', 'N/A')}")
    
    # UPDATE: Modify document metadata
    print("\n\nStep 5: UPDATE - Modifying document metadata...")
    store.update(
        "doc1",
        title="Python Programming Guide",
        summary="Comprehensive guide to Python programming"
    )
    
    updated = store.get("doc1")
    print(f"  Updated 'doc1':")
    print(f"    New Title: {updated['title']}")
    print(f"    New Summary: {updated['summary']}\n")
    
    # DELETE: Remove documents
    print("Step 6: DELETE - Removing documents...")
    print(f"  Documents before deletion: {store.count()}")
    
    # Single delete
    store.delete("doc4")
    print(f"  ✓ Deleted doc4")
    
    # Batch delete
    store.delete_batch(["doc2", "doc3"])
    print(f"  ✓ Batch deleted doc2, doc3")
    
    print(f"  Documents after deletion: {store.count()}\n")
    
    # Additional features
    print("Step 7: Additional SDK features...")
    print(f"  Is empty: {store.is_empty()}")
    print(f"  Contains 'doc1': {'doc1' in store}")
    print(f"  Contains 'doc99': {'doc99' in store}")
    print(f"  Total count: {len(store)}")
    
    print("\n=== SDK Features Summary ===")
    print("✓ CREATE: add(), add_batch() with parallel processing")
    print("✓ READ: get()")
    print("✓ UPDATE: update()")
    print("✓ DELETE: delete(), delete_batch()")
    print("✓ SEARCH: search(), search_by_vector()")
    print("✓ Memory efficient: content vectorized then discarded")
    print("✓ Thread-safe: concurrent operations supported")
    print("✓ No unsafe blocks: all Rust code is memory-safe")
    
    print("\n=== Example completed successfully! ===")


if __name__ == "__main__":
    main()
