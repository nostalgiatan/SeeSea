"""
TF SDK - High-level API for text semantic retrieval

This module provides a clean SDK interface for document management with full CRUD operations:
- Create: add documents with automatic vectorization
- Read: retrieve document metadata
- Update: modify document metadata
- Delete: remove documents
- Search: semantic similarity search with streaming support

All operations are thread-safe and memory-efficient.
"""

from typing import List, Dict, Optional, Any, Iterator, Union
from concurrent.futures import ThreadPoolExecutor, as_completed
import threading
from .embeddings import TextEmbedder
from .search_result import SearchResult


class DocumentStore:
    """
    High-level SDK for document storage and retrieval.
    
    This class provides a clean API with full CRUD operations:
    - add() / add_batch(): Create documents
    - get(): Read document metadata
    - update(): Update document metadata
    - delete() / delete_batch(): Delete documents
    - search(): Semantic search
    
    Features:
    - Memory efficient: content is vectorized then discarded
    - Thread-safe: concurrent operations supported
    - Batch operations: parallel processing for bulk operations
    """
    
    def __init__(self, embedder: TextEmbedder = None, dimension: Optional[int] = None):
        """
        Initialize the document store.
        
        Args:
            embedder: TextEmbedder instance (created if None)
            dimension: Vector dimension (auto-detected from embedder if None)
        """
        # Initialize embedder
        if embedder is None:
            self.embedder = TextEmbedder()
        else:
            self.embedder = embedder
        
        # Get dimension
        if dimension is None:
            dimension = self.embedder.get_dimension()
        
        # Initialize Rust store
        try:
            from tf_rust import VectorStore
            self._store = VectorStore(dimension)
        except ImportError as e:
            raise ImportError(
                "Failed to import tf_rust module. "
                "Please build the Rust extension first using 'maturin develop'."
            ) from e
        
        # Thread pool for parallel operations
        self._executor = ThreadPoolExecutor(max_workers=4)
        self._lock = threading.Lock()
    
    def add(
        self,
        doc_id: str,
        content: str,
        title: str = "",
        url: str = "",
        summary: str = ""
    ) -> None:
        """
        Add a document to the store (Create operation).
        
        The content is vectorized via the embedding model, then immediately discarded.
        Only the vector and metadata (title, url, summary) are stored.
        
        Args:
            doc_id: Unique identifier for the document
            content: Document content (will be vectorized then discarded)
            title: Document title (stored)
            url: Document URL (stored)
            summary: Document summary (stored)
        
        Example:
            >>> store.add("doc1", "Long content...", title="My Doc", summary="Brief summary")
        """
        def embedding_callback(text: str) -> List[float]:
            """Callback for Rust to get embedding."""
            embedding = self.embedder.encode(text)
            # Ensure flat list
            if isinstance(embedding, list) and len(embedding) > 0 and isinstance(embedding[0], list):
                embedding = embedding[0]
            return embedding
        
        # Call Rust with callback
        with self._lock:
            self._store.set(doc_id, content, title, url, summary, embedding_callback)
    
    def add_batch(
        self,
        documents: List[Dict[str, str]],
        parallel: bool = True
    ) -> None:
        """
        Add multiple documents in parallel (Create operation).
        
        Args:
            documents: List of document dictionaries with keys:
                      - id: Document ID (required)
                      - content: Document content (required)
                      - title: Document title (optional)
                      - url: Document URL (optional)
                      - summary: Document summary (optional)
            parallel: Use parallel processing (default: True)
        
        Example:
            >>> docs = [
            ...     {"id": "1", "content": "...", "title": "...", "summary": "..."},
            ...     {"id": "2", "content": "...", "title": "...", "summary": "..."}
            ... ]
            >>> store.add_batch(docs)
        """
        if parallel:
            # Parallel processing with thread pool
            futures = []
            for doc in documents:
                doc_id = doc.get('id')
                content = doc.get('content')
                
                if not doc_id or not content:
                    raise ValueError(f"Document missing required fields 'id' and/or 'content': {doc}")
                
                future = self._executor.submit(
                    self.add,
                    doc_id,
                    content,
                    doc.get('title', ''),
                    doc.get('url', ''),
                    doc.get('summary', '')
                )
                futures.append(future)
            
            # Wait for all to complete
            for future in as_completed(futures):
                future.result()  # Raise any exceptions
        else:
            # Sequential processing
            for doc in documents:
                doc_id = doc.get('id')
                content = doc.get('content')
                
                if not doc_id or not content:
                    raise ValueError(f"Document missing required fields 'id' and/or 'content': {doc}")
                
                self.add(
                    doc_id,
                    content,
                    doc.get('title', ''),
                    doc.get('url', ''),
                    doc.get('summary', '')
                )
    
    def get(self, doc_id: str) -> Optional[Dict[str, str]]:
        """
        Get document metadata (Read operation).
        
        Args:
            doc_id: Document identifier
        
        Returns:
            Dictionary with title, url, and summary, or None if not found.
            Note: content is NOT included as it's not stored.
        
        Example:
            >>> metadata = store.get("doc1")
            >>> print(metadata['title'], metadata['summary'])
        """
        return self._store.get(doc_id)
    
    def update(
        self,
        doc_id: str,
        title: Optional[str] = None,
        url: Optional[str] = None,
        summary: Optional[str] = None
    ) -> None:
        """
        Update document metadata (Update operation).
        
        Args:
            doc_id: Document identifier
            title: New title (optional)
            url: New URL (optional)
            summary: New summary (optional)
        
        Example:
            >>> store.update("doc1", title="New Title", summary="Updated summary")
        """
        with self._lock:
            self._store.update(doc_id, title, url, summary)
    
    def delete(self, doc_id: str) -> None:
        """
        Delete a document (Delete operation).
        
        Args:
            doc_id: Document identifier
        
        Example:
            >>> store.delete("doc1")
        """
        with self._lock:
            self._store.rm(doc_id)
    
    def delete_batch(self, doc_ids: List[str]) -> None:
        """
        Delete multiple documents.
        
        Args:
            doc_ids: List of document identifiers
        
        Example:
            >>> store.delete_batch(["doc1", "doc2", "doc3"])
        """
        with self._lock:
            for doc_id in doc_ids:
                self._store.rm(doc_id)
    
    def search(
        self,
        query: str,
        k: int = 5,
        return_objects: bool = False
    ) -> Union[List[Dict[str, Any]], List[SearchResult]]:
        """
        Search for similar documents (Search operation).
        
        Text is automatically converted to vector, then searched in vector database.
        Results are sorted by relevance score (highest first).
        Only metadata and relevance scores are returned - vectors are NOT included.
        
        Args:
            query: Query text (will be vectorized automatically)
            k: Number of results to return
            return_objects: If True, return SearchResult objects; if False, return dicts
        
        Returns:
            List of results sorted by relevance (highest score first):
            - As SearchResult objects (if return_objects=True)
            - As dictionaries (if return_objects=False) with keys:
              * id: Document identifier
              * score: Relevance score (0-1, higher is more relevant)
              * title: Document title
              * url: Document URL
              * summary: Document summary
        
        Example:
            >>> # Default: returns dictionaries
            >>> results = store.search("machine learning", k=10)
            >>> for r in results:
            ...     print(f"{r['title']}: score={r['score']:.3f}")
            
            >>> # With objects: memory-efficient structured results
            >>> results = store.search("AI systems", k=5, return_objects=True)
            >>> for r in results:
            ...     print(f"{r.title}: {r.score:.3f}")
        """
        # Generate query embedding - memory efficient, vector discarded after search
        query_embedding = self.embedder.encode(query)
        
        # Ensure flat list
        if isinstance(query_embedding, list) and len(query_embedding) > 0 and isinstance(query_embedding[0], list):
            query_embedding = query_embedding[0]
        
        # Search in vector database - results already sorted by score (descending)
        raw_results = self._store.search(query_embedding, k)
        
        # Free embedding memory immediately
        del query_embedding
        
        if return_objects:
            # Convert to SearchResult objects for structured access
            return [
                SearchResult(
                    id=r['id'],
                    score=r['score'],
                    title=r.get('title', ''),
                    url=r.get('url', ''),
                    summary=r.get('summary', '')
                )
                for r in raw_results
            ]
        else:
            # Return as dictionaries (backward compatible)
            return raw_results
    
    def search_streaming(
        self,
        query: str,
        k: int = 5
    ) -> Iterator[SearchResult]:
        """
        Streaming search for memory-efficient result iteration.
        
        Text is vectorized, searched, and results are yielded one at a time.
        This minimizes memory usage by not buffering all results.
        
        Args:
            query: Query text
            k: Number of results to return
        
        Yields:
            SearchResult objects one at a time, sorted by relevance
        
        Example:
            >>> for result in store.search_streaming("deep learning", k=100):
            ...     print(f"{result.title}: {result.score:.3f}")
            ...     # Process result immediately, no buffering
        """
        # Generate query embedding
        query_embedding = self.embedder.encode(query)
        
        # Ensure flat list
        if isinstance(query_embedding, list) and len(query_embedding) > 0 and isinstance(query_embedding[0], list):
            query_embedding = query_embedding[0]
        
        # Search - results already sorted
        raw_results = self._store.search(query_embedding, k)
        
        # Free embedding memory
        del query_embedding
        
        # Yield results one at a time for streaming
        for r in raw_results:
            yield SearchResult(
                id=r['id'],
                score=r['score'],
                title=r.get('title', ''),
                url=r.get('url', ''),
                summary=r.get('summary', '')
            )
            # Each result is yielded and can be processed/freed immediately
    
    def search_by_vector(
        self,
        vector: List[float],
        k: int = 5
    ) -> List[Dict[str, Any]]:
        """
        Search using a pre-computed vector.
        
        Args:
            vector: Query vector
            k: Number of results to return
        
        Returns:
            List of result dictionaries
        
        Example:
            >>> vec = embedder.encode("some text")
            >>> results = store.search_by_vector(vec, k=5)
        """
        return self._store.search(vector, k)
    
    def count(self) -> int:
        """
        Get the number of documents in the store.
        
        Returns:
            Document count
        
        Example:
            >>> print(f"Total documents: {store.count()}")
        """
        return self._store.len()
    
    def is_empty(self) -> bool:
        """
        Check if the store is empty.
        
        Returns:
            True if empty, False otherwise
        """
        return self._store.is_empty()
    
    def __len__(self) -> int:
        """Get document count."""
        return self.count()
    
    def __contains__(self, doc_id: str) -> bool:
        """Check if document exists."""
        return self.get(doc_id) is not None
    
    def __del__(self):
        """Cleanup thread pool."""
        if hasattr(self, '_executor'):
            self._executor.shutdown(wait=True)


# Convenience alias
SDK = DocumentStore
