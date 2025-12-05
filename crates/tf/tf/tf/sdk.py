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
from .embeddings import TextEmbedder
from .vector_store import VectorStoreWrapper
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
    - Bloom filter: efficient URL existence checking
    - Content hashing: avoid unnecessary updates
    - Persistent storage: data stored in .tf/data.db by default
    """

    def __init__(
        self,
        embedder: Optional[TextEmbedder] = None,
        model_path: Optional[str] = None,
        device: Optional[str] = None,
        store_path: Optional[str] = None,
    ):
        """
        Initialize the document store.

        Args:
            embedder: TextEmbedder instance (created if None)
            model_path: Path to the GGUF format Qwen3 embedding model (used if embedder is None)
            device: Device to use for embedding model (used if embedder is None)
            store_path: Path to persistent vector store (default: .tf/data.db)
        """
        # Initialize embedder if not provided
        if embedder is None:
            self.embedder = TextEmbedder(model_path=model_path, device=device)
        else:
            self.embedder = embedder

        # Initialize VectorStoreWrapper - this handles all Rust store operations
        self._vector_store = VectorStoreWrapper(embedder=self.embedder, store_path=store_path)

    def add(
        self, doc_id: str, content: str, title: str = "", url: str = "", summary: str = ""
    ) -> bool:
        """
        Add a document to the store (Create operation).

        The content is vectorized via the embedding model, then immediately discarded.
        Only the vector and metadata (title, url, summary) are stored.
        
        This method uses bloom filter and content hash to avoid unnecessary operations:
        1. Checks if URL exists in bloom filter
        2. If exists, checks content hash to determine if update is needed
        3. Only vectorizes and updates if necessary

        Args:
            doc_id: Unique identifier for the document
            content: Document content (will be vectorized then discarded)
            title: Document title (stored)
            url: Document URL (stored)
            summary: Document summary (stored)

        Returns:
            True if the document was added or updated, False if no changes were needed

        Example:
            >>> store.add("doc1", "Long content...", title="My Doc", summary="Brief summary")
        """
        return self._vector_store.add_document(doc_id, content, title, url, summary)

    def add_document_with_vector(
        self, doc_id: str, vector: List[float], title: str = "", url: str = "", summary: str = ""
    ) -> bool:
        """
        Add a document with a pre-computed vector.

        Use this when you already have the vector and want to avoid re-computing it.
        
        This method uses bloom filter and content hash to avoid unnecessary operations.

        Args:
            doc_id: Unique identifier for the document
            vector: Pre-computed embedding vector
            title: Document title (optional)
            url: Document URL (optional)
            summary: Document summary (optional)

        Returns:
            True if the document was added or updated, False if no changes were needed
        """
        return self._vector_store.add_document_with_vector(doc_id, vector, title, url, summary)

    def add_batch(self, documents: List[Dict[str, str]]) -> int:
        """
        Add multiple documents using batch processing (Create operation).
        
        This method is highly optimized:
        1. First filters documents that need to be updated using bloom filter and hash comparison
        2. Batch encodes the documents that need updates
        3. Uses Rust's batch_set method for efficient insertion
        4. Avoids unnecessary vectorization and database operations

        Args:
            documents: List of document dictionaries with keys:
                      - id: Document ID (required)
                      - content: Document content (required)
                      - title: Document title (optional)
                      - url: Document URL (optional)
                      - summary: Document summary (optional)

        Returns:
            Number of documents successfully added or updated

        Example:
            >>> docs = [
            ...     {"id": "1", "content": "...", "title": "...", "summary": "..."},
            ...     {"id": "2", "content": "...", "title": "...", "summary": "..."}
            ... ]
            >>> store.add_batch(docs)
        """
        # Convert documents to the format expected by VectorStoreWrapper.add_documents
        formatted_docs = []
        for doc in documents:
            formatted_docs.append({
                "id": doc["id"],
                "content": doc["content"],
                "title": doc.get("title", ""),
                "url": doc.get("url", ""),
                "summary": doc.get("summary", "")
            })
        return self._vector_store.add_documents(formatted_docs)

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
        return self._vector_store.get_metadata(doc_id)

    def update(
        self,
        doc_id: str,
        title: Optional[str] = None,
        url: Optional[str] = None,
        summary: Optional[str] = None,
    ) -> bool:
        """
        Update document metadata (Update operation).
        
        This method uses content hash to determine if an update is actually needed.
        It will only update the document if the content has changed.

        Args:
            doc_id: Document identifier
            title: New title (optional)
            url: New URL (optional)
            summary: New summary (optional)

        Returns:
            True if the document was updated, False if no changes were needed

        Example:
            >>> store.update("doc1", title="New Title", summary="Updated summary")
        """
        # Use Rust's update method which handles hash comparison
        return self._vector_store.store.update(doc_id, title, url, summary)

    def delete(self, doc_id: str) -> None:
        """
        Delete a document (Delete operation).

        Args:
            doc_id: Document identifier

        Example:
            >>> store.delete("doc1")
        """
        self._vector_store.remove_document(doc_id)

    def delete_batch(self, doc_ids: List[str]) -> None:
        """
        Delete multiple documents.

        Args:
            doc_ids: List of document identifiers

        Example:
            >>> store.delete_batch(["doc1", "doc2", "doc3"])
        """
        for doc_id in doc_ids:
            self._vector_store.remove_document(doc_id)

    def search(
        self, query: str, k: int = 5, return_objects: bool = False
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
        if (
            isinstance(query_embedding, list)
            and len(query_embedding) > 0
            and isinstance(query_embedding[0], list)
        ):
            query_embedding = query_embedding[0]

        # Search in vector database - results already sorted by score (descending)
        raw_results = self._vector_store.search_by_embedding(query_embedding, k)

        # Free embedding memory immediately
        del query_embedding

        if return_objects:
            # Convert to SearchResult objects for structured access
            return [
                SearchResult(
                    id=r["id"],
                    score=r["score"],
                    title=r.get("title", ""),
                    url=r.get("url", ""),
                    summary=r.get("summary", ""),
                )
                for r in raw_results
            ]
        else:
            # Return as dictionaries (backward compatible)
            return raw_results

    def search_streaming(self, query: str, k: int = 5) -> Iterator[SearchResult]:
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
        if (
            isinstance(query_embedding, list)
            and len(query_embedding) > 0
            and isinstance(query_embedding[0], list)
        ):
            query_embedding = query_embedding[0]

        # Search - results already sorted
        raw_results = self._vector_store.search_by_embedding(query_embedding, k)

        # Free embedding memory
        del query_embedding

        # Yield results one at a time for streaming
        for r in raw_results:
            yield SearchResult(
                id=r["id"],
                score=r["score"],
                title=r.get("title", ""),
                url=r.get("url", ""),
                summary=r.get("summary", ""),
            )
            # Each result is yielded and can be processed immediately, no buffering

    def search_by_vector(self, vector: List[float], k: int = 5) -> List[Dict[str, Any]]:
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
        return self._vector_store.search_by_embedding(vector, k)

    def count(self) -> int:
        """
        Get the number of documents in the store.

        Returns:
            Document count

        Example:
            >>> print(f"Total documents: {store.count()}")
        """
        return len(self._vector_store)

    def is_empty(self) -> bool:
        """
        Check if the store is empty.

        Returns:
            True if empty, False otherwise
        """
        return self._vector_store.is_empty()

    def url_exists(self, url: str) -> bool:
        """
        Check if a URL exists in the store using bloom filter.
        This is a fast O(1) operation.

        Args:
            url: URL to check

        Returns:
            True if URL likely exists, False if definitely doesn't exist
        """
        return self._vector_store.store.url_exists(url)

    def url_exists_exact(self, url: str) -> bool:
        """
        Check if a URL exists in the store with exact match.
        This is a slower operation but guarantees accuracy.

        Args:
            url: URL to check

        Returns:
            True if URL exists, False otherwise
        """
        return self._vector_store.store.url_exists_exact(url)

    def get_by_url(self, url: str) -> Optional[Dict[str, Any]]:
        """
        Get document by URL.

        Args:
            url: URL to search for

        Returns:
            Document metadata if found, None otherwise
        """
        return self._vector_store.store.get_by_url(url)

    def queue_document(
        self, doc_id: str, content: str, title: str = "", url: str = "", summary: str = ""
    ) -> bool:
        """
        Queue a document for batch processing.
        This method is thread-safe and doesn't block.

        Args:
            doc_id: Unique identifier for the document
            content: Document content (will be vectorized then discarded)
            title: Document title (stored)
            url: Document URL (stored)
            summary: Document summary (stored)

        Returns:
            True if document was added to queue, False if URL already processed
        """
        return self._vector_store.store.queue_document(doc_id, content, title, url, summary)

    def flush_queue(self) -> int:
        """
        Flush the document queue, processing all pending documents.
        This method blocks until all documents are processed.

        Returns:
            Number of documents successfully processed
        """
        # Create callback function for Rust to call
        def embedding_callback(text: str) -> List[float]:
            """Callback that Rust calls to get the embedding vector."""
            embedding = self.embedder.encode(text)
            # Ensure embedding is a flat list of floats
            if (
                isinstance(embedding, list)
                and len(embedding) > 0
                and isinstance(embedding[0], list)
            ):
                return embedding[0]
            return embedding

        return self._vector_store.store.flush_queue(embedding_callback)

    def __len__(self) -> int:
        """Get document count."""
        return self.count()

    def __contains__(self, doc_id: str) -> bool:
        """Check if document exists."""
        return self.get(doc_id) is not None


# Convenience alias
SDK = DocumentStore