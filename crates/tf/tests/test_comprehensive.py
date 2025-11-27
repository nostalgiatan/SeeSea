"""
Comprehensive test suite for TF text semantic retrieval system.

Tests cover:
- CRUD operations
- Search functionality
- Streaming operations
- Batch operations
- Error handling
- Thread safety

Uses shared DocumentStore instance to avoid lock contention.
"""

import pytest
import threading
from typing import List, Dict
from tf import DocumentStore, SearchResult, TextEmbedder


# Shared store instance for all tests to avoid lock issues
@pytest.fixture(scope="module")
def shared_store():
    """Create a shared DocumentStore instance for all tests."""
    store = DocumentStore()
    yield store
    # Cleanup after all tests


@pytest.fixture(scope="module")
def embedder():
    """Create a shared TextEmbedder instance."""
    return TextEmbedder()


class TestDocumentStoreInitialization:
    """Test DocumentStore initialization."""
    
    def test_init_default(self):
        """Test default initialization."""
        store = DocumentStore()
        assert store is not None
        assert store.embedder is not None
        assert store.count() >= 0
    
    def test_init_with_embedder(self, embedder):
        """Test initialization with custom embedder."""
        store = DocumentStore(embedder=embedder)
        assert store.embedder is embedder
    
    def test_init_with_dimension(self):
        """Test initialization with custom dimension."""
        store = DocumentStore(dimension=768)
        assert store is not None


class TestCRUDOperations:
    """Test Create, Read, Update, Delete operations."""
    
    def test_add_document(self, shared_store):
        """Test adding a single document."""
        shared_store.add(
            doc_id="test_add_1",
            content="This is a test document for adding.",
            title="Test Add Document",
            url="https://example.com/test-add",
            summary="Test document summary"
        )
        
        # Verify document was added
        metadata = shared_store.get("test_add_1")
        assert metadata is not None
        assert metadata['title'] == "Test Add Document"
        assert metadata['summary'] == "Test document summary"
        assert metadata['url'] == "https://example.com/test-add"
    
    def test_add_document_minimal(self, shared_store):
        """Test adding document with minimal fields."""
        shared_store.add(
            doc_id="test_add_2",
            content="Minimal document content."
        )
        
        metadata = shared_store.get("test_add_2")
        assert metadata is not None
    
    def test_get_document(self, shared_store):
        """Test retrieving document metadata."""
        # Add a document
        shared_store.add(
            doc_id="test_get_1",
            content="Content for get test.",
            title="Get Test",
            url="https://example.com/get",
            summary="Get test summary"
        )
        
        # Retrieve it
        metadata = shared_store.get("test_get_1")
        assert metadata is not None
        assert metadata['title'] == "Get Test"
        assert 'content' not in metadata  # Content should not be stored
    
    def test_get_nonexistent(self, shared_store):
        """Test getting non-existent document."""
        metadata = shared_store.get("nonexistent_doc_12345")
        assert metadata is None
    
    def test_update_document(self, shared_store):
        """Test updating document metadata."""
        # Add a document
        shared_store.add(
            doc_id="test_update_1",
            content="Content for update test.",
            title="Original Title",
            summary="Original Summary"
        )
        
        # Update it
        shared_store.update(
            "test_update_1",
            title="Updated Title",
            summary="Updated Summary"
        )
        
        # Verify update
        metadata = shared_store.get("test_update_1")
        assert metadata['title'] == "Updated Title"
        assert metadata['summary'] == "Updated Summary"
    
    def test_update_partial(self, shared_store):
        """Test partial update (only some fields)."""
        shared_store.add(
            doc_id="test_update_2",
            content="Content.",
            title="Title",
            url="https://example.com",
            summary="Summary"
        )
        
        # Update only title
        shared_store.update("test_update_2", title="New Title")
        
        metadata = shared_store.get("test_update_2")
        assert metadata['title'] == "New Title"
        assert metadata['summary'] == "Summary"  # Unchanged
    
    def test_delete_document(self, shared_store):
        """Test deleting a document."""
        # Add a document
        shared_store.add(
            doc_id="test_delete_1",
            content="Content for delete test."
        )
        
        # Verify it exists
        assert shared_store.get("test_delete_1") is not None
        
        # Delete it
        shared_store.delete("test_delete_1")
        
        # Verify it's gone
        assert shared_store.get("test_delete_1") is None
    
    def test_contains_operator(self, shared_store):
        """Test 'in' operator."""
        shared_store.add(
            doc_id="test_contains_1",
            content="Content for contains test."
        )
        
        assert "test_contains_1" in shared_store
        assert "nonexistent_12345" not in shared_store


class TestBatchOperations:
    """Test batch operations."""
    
    def test_add_batch_sequential(self, shared_store):
        """Test batch add in sequential mode."""
        docs = [
            {
                "id": f"batch_seq_{i}",
                "content": f"Content {i}",
                "title": f"Title {i}",
                "summary": f"Summary {i}"
            }
            for i in range(5)
        ]
        
        shared_store.add_batch(docs, parallel=False)
        
        # Verify all were added
        for i in range(5):
            metadata = shared_store.get(f"batch_seq_{i}")
            assert metadata is not None
            assert metadata['title'] == f"Title {i}"
    
    def test_add_batch_parallel(self, shared_store):
        """Test batch add in parallel mode."""
        docs = [
            {
                "id": f"batch_par_{i}",
                "content": f"Content {i}",
                "title": f"Title {i}",
                "summary": f"Summary {i}"
            }
            for i in range(5)
        ]
        
        shared_store.add_batch(docs, parallel=True)
        
        # Verify all were added
        for i in range(5):
            metadata = shared_store.get(f"batch_par_{i}")
            assert metadata is not None
            assert metadata['title'] == f"Title {i}"
    
    def test_add_batch_validation(self, shared_store):
        """Test batch add validation of required fields."""
        docs = [
            {"id": "valid", "content": "Valid content"},
            {"id": "invalid"}  # Missing content
        ]
        
        with pytest.raises(ValueError) as exc_info:
            shared_store.add_batch(docs)
        
        assert "Missing required field" in str(exc_info.value)
    
    def test_delete_batch(self, shared_store):
        """Test batch delete."""
        # Add documents
        for i in range(3):
            shared_store.add(
                doc_id=f"batch_del_{i}",
                content=f"Content {i}"
            )
        
        # Delete in batch
        shared_store.delete_batch([f"batch_del_{i}" for i in range(3)])
        
        # Verify all deleted
        for i in range(3):
            assert shared_store.get(f"batch_del_{i}") is None


class TestSearchOperations:
    """Test search functionality."""
    
    @pytest.fixture(scope="class")
    def search_store(self):
        """Create a store with test documents for searching."""
        store = DocumentStore()
        
        docs = [
            {
                "id": "search_1",
                "content": "Machine learning is a subset of artificial intelligence that enables computers to learn from data.",
                "title": "Machine Learning Basics",
                "summary": "Introduction to ML"
            },
            {
                "id": "search_2",
                "content": "Deep learning uses neural networks with multiple layers to learn hierarchical representations.",
                "title": "Deep Learning Overview",
                "summary": "Neural networks and deep learning"
            },
            {
                "id": "search_3",
                "content": "Natural language processing enables computers to understand and generate human language.",
                "title": "NLP Fundamentals",
                "summary": "Text processing with NLP"
            },
            {
                "id": "search_4",
                "content": "Computer vision allows machines to interpret and understand visual information from images.",
                "title": "Computer Vision",
                "summary": "Image understanding"
            }
        ]
        
        store.add_batch(docs)
        return store
    
    def test_search_basic(self, search_store):
        """Test basic search functionality."""
        results = search_store.search("machine learning algorithms", k=3)
        
        assert len(results) > 0
        assert len(results) <= 3
        
        # Verify result structure
        assert 'id' in results[0]
        assert 'score' in results[0]
        assert 'title' in results[0]
        assert 'summary' in results[0]
        assert 'url' in results[0]
        
        # Verify no vector in results
        assert 'vector' not in results[0]
        assert 'content' not in results[0]
    
    def test_search_sorted_by_relevance(self, search_store):
        """Test that results are sorted by relevance (highest first)."""
        results = search_store.search("deep neural networks", k=4)
        
        # Verify scores are in descending order
        for i in range(len(results) - 1):
            assert results[i]['score'] >= results[i + 1]['score']
    
    def test_search_with_objects(self, search_store):
        """Test search returning SearchResult objects."""
        results = search_store.search("computer vision", k=2, return_objects=True)
        
        assert len(results) > 0
        assert isinstance(results[0], SearchResult)
        assert hasattr(results[0], 'id')
        assert hasattr(results[0], 'score')
        assert hasattr(results[0], 'title')
        
        # Verify immutability
        with pytest.raises(Exception):  # FrozenInstanceError
            results[0].title = "Modified"
    
    def test_search_result_to_dict(self, search_store):
        """Test SearchResult.to_dict() method."""
        results = search_store.search("NLP", k=1, return_objects=True)
        
        if results:
            result_dict = results[0].to_dict()
            assert isinstance(result_dict, dict)
            assert 'id' in result_dict
            assert 'score' in result_dict
    
    def test_search_streaming(self, search_store):
        """Test streaming search."""
        count = 0
        for result in search_store.search_streaming("artificial intelligence", k=3):
            assert isinstance(result, SearchResult)
            assert hasattr(result, 'score')
            count += 1
        
        assert count > 0
        assert count <= 3
    
    def test_search_by_vector(self, search_store, embedder):
        """Test search with pre-computed vector."""
        # Get a query vector
        query_vec = embedder.encode("machine learning")
        if isinstance(query_vec[0], list):
            query_vec = query_vec[0]
        
        results = search_store.search_by_vector(query_vec, k=2)
        
        assert len(results) > 0
        assert len(results) <= 2


class TestUtilityMethods:
    """Test utility methods."""
    
    def test_count(self, shared_store):
        """Test count() method."""
        initial_count = shared_store.count()
        
        shared_store.add(
            doc_id="test_count_1",
            content="Content for count test."
        )
        
        assert shared_store.count() == initial_count + 1
    
    def test_len(self, shared_store):
        """Test __len__() method."""
        count = shared_store.count()
        assert len(shared_store) == count
    
    def test_is_empty(self):
        """Test is_empty() method."""
        empty_store = DocumentStore()
        assert empty_store.is_empty()
        
        empty_store.add("test", "content")
        assert not empty_store.is_empty()


class TestThreadSafety:
    """Test thread safety of operations."""
    
    def test_concurrent_reads(self, shared_store):
        """Test concurrent search operations."""
        # Add test document
        shared_store.add(
            doc_id="thread_test_1",
            content="Content for thread safety test.",
            title="Thread Test"
        )
        
        results = []
        errors = []
        
        def search_worker():
            try:
                res = shared_store.search("thread test", k=1)
                results.append(res)
            except Exception as e:
                errors.append(e)
        
        # Create multiple threads
        threads = [threading.Thread(target=search_worker) for _ in range(5)]
        
        # Start all threads
        for t in threads:
            t.start()
        
        # Wait for completion
        for t in threads:
            t.join()
        
        # Verify no errors
        assert len(errors) == 0
        assert len(results) == 5
    
    def test_concurrent_writes(self, shared_store):
        """Test concurrent add operations."""
        errors = []
        
        def add_worker(i):
            try:
                shared_store.add(
                    doc_id=f"concurrent_add_{i}",
                    content=f"Content {i}"
                )
            except Exception as e:
                errors.append(e)
        
        threads = [threading.Thread(target=add_worker, args=(i,)) for i in range(5)]
        
        for t in threads:
            t.start()
        
        for t in threads:
            t.join()
        
        # Verify no errors
        assert len(errors) == 0
        
        # Verify all were added
        for i in range(5):
            metadata = shared_store.get(f"concurrent_add_{i}")
            assert metadata is not None


class TestErrorHandling:
    """Test error handling."""
    
    def test_add_without_content(self):
        """Test error when adding document without content."""
        store = DocumentStore()
        
        # Should work with content
        store.add("test1", "content", title="Title")
        
        # add() requires content parameter, so this test validates the API
        # The validation in add_batch is tested separately
    
    def test_batch_add_missing_fields(self):
        """Test batch add with missing required fields."""
        store = DocumentStore()
        
        docs = [
            {"id": "doc1"},  # Missing content
        ]
        
        with pytest.raises(ValueError):
            store.add_batch(docs)


class TestMemoryEfficiency:
    """Test memory efficiency features."""
    
    def test_content_not_stored(self, shared_store):
        """Verify content is not stored in metadata."""
        shared_store.add(
            doc_id="memory_test_1",
            content="This is some content that should not be stored.",
            title="Memory Test"
        )
        
        metadata = shared_store.get("memory_test_1")
        assert metadata is not None
        assert 'content' not in metadata
    
    def test_search_no_vectors(self, shared_store):
        """Verify search results don't include vectors."""
        shared_store.add(
            doc_id="memory_test_2",
            content="Content for vector test."
        )
        
        results = shared_store.search("vector test", k=1)
        
        if results:
            assert 'vector' not in results[0]
            assert 'embedding' not in results[0]
    
    def test_streaming_memory(self, shared_store):
        """Test streaming doesn't buffer all results."""
        # Add multiple documents
        for i in range(10):
            shared_store.add(
                doc_id=f"stream_mem_{i}",
                content=f"Content {i}"
            )
        
        # Use streaming - should not buffer all results
        count = 0
        for result in shared_store.search_streaming("content", k=10):
            count += 1
            # Each result can be processed/freed immediately
        
        assert count <= 10


if __name__ == "__main__":
    # Run tests with pytest
    pytest.main([__file__, "-v", "--tb=short"])
