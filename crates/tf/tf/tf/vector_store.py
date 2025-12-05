"""
Vector store wrapper that integrates Python embeddings with Rust vector storage.

MEMORY EFFICIENT DESIGN:
- Content is NOT stored in the vector database
- Only vectors and metadata (title, url) are stored
- Content is discarded immediately after vectorization
- This minimizes memory usage and maximizes performance
"""

from typing import List, Dict, Optional, Any
from .embeddings import TextEmbedder


class VectorStoreWrapper:
    """
    High-level wrapper that combines TextEmbedder with the Rust VectorStore.

    This class provides a memory-efficient interface for adding documents,
    searching by text, and managing the vector database.

    KEY FEATURE: Content is NOT stored - only vectors and metadata!
    """

    def __init__(
        self,
        embedder: Optional[TextEmbedder] = None,
        rust_store=None,
        store_path=None,
        n_threads: Optional[int] = None,
    ):
        """
        Initialize the vector store wrapper.

        Args:
            embedder: TextEmbedder instance (created if None)
            rust_store: Rust VectorStore instance (created if None)
            store_path: Path to persistent vector store (default: .tf/data.db)
            n_threads: Number of threads to use for embedding generation. If None, auto-detects based on CPU cores.
        """
        # Initialize embedder
        if embedder is None:
            self.embedder = TextEmbedder(n_threads=n_threads)
        else:
            self.embedder = embedder

        # Initialize Rust store
        if rust_store is None:
            try:
                from tf_rust import VectorStore  # type: ignore[import-not-found]

                # Create Rust VectorStore without n_threads parameter
                self.store = VectorStore(self.embedder.get_dimension(), store_path)  # type: ignore[assignment]
            except ImportError as e:
                raise ImportError(
                    "Failed to import tf_rust module. "
                    "Please build the Rust extension first using 'maturin develop' or 'maturin build'."
                ) from e
        else:
            self.store = rust_store  # type: ignore[assignment]

        # Import hashlib for content hashing
        import hashlib

        self._hashlib = hashlib

        # Batch queue for automatic batch processing
        from typing import Tuple

        self._batch_queue: list[Tuple[str, str, str, str, str]] = []
        self._batch_threshold = (
            10  # Automatically upgrade to batch processing when queue reaches this size
        )
        self._last_access_time = self._get_current_time()
        self._idle_timeout = 300  # 5 minutes idle timeout in seconds
        self._is_connected = True

    def add_document(
        self, doc_id: str, content: str, title: str = "", url: str = "", summary: str = ""
    ) -> bool:
        """
        Add a document to the vector store.

        MEMORY EFFICIENT: The content is vectorized and then immediately discarded.
        Only the vector and metadata (title, url, summary) are stored.

        OPTIMIZATION: Uses bloom filter and content hash to avoid unnecessary operations:
        1. Checks if URL exists in bloom filter
        2. If exists, checks content hash to determine if update is needed
        3. Only vectorizes and updates if necessary

        Args:
            doc_id: Unique identifier for the document
            content: Document content (will be vectorized then discarded)
            title: Document title (optional, will be stored)
            url: Document URL (optional, will be stored)
            summary: Document summary (optional, will be stored)

        Returns:
            True if the document was added or updated, False if no changes were needed
        """
        # Check if URL already exists using bloom filter (fast check)
        url_exists = self.store.url_exists(url)

        # Generate content hash
        content_hash = self._hashlib.sha256(f"{title}{url}{summary}".encode()).hexdigest()

        # If URL exists, check if we need to update
        if url_exists:
            # Get current document metadata
            current_doc = self.get_metadata(doc_id)
            if current_doc:
                # Check if hash exists and matches
                if "hash" in current_doc and current_doc["hash"] == content_hash:
                    # No changes needed, skip update
                    return False

        # Need to add or update the document
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
                return embedding[0]  # type: ignore[return-value]
            return embedding  # type: ignore[return-value]

        # Add to batch queue
        self._batch_queue.append((doc_id, content, title, url, summary))

        # Check if we should process the batch
        if len(self._batch_queue) >= self._batch_threshold:
            # Process the batch and return True if at least one document was processed
            processed = self._flush_batch_queue()
            return processed > 0

        # Document added to queue, will be processed later
        return True

    def flush(self) -> int:
        """
        Flush the batch queue and process all documents immediately.

        Returns:
            Number of documents successfully processed
        """
        return self._flush_batch_queue()

    def _get_current_time(self) -> float:
        """
        Get current time in seconds since epoch.
        """
        import time

        return time.time()

    def _check_idle_timeout(self) -> None:
        """
        Check if the connection has been idle for too long and should be closed.
        """
        current_time = self._get_current_time()
        if current_time - self._last_access_time > self._idle_timeout:
            # Connection has been idle too long, close it
            # Note: In a real implementation, this would close the database connection
            # For now, we just mark it as disconnected
            self._is_connected = False

    def _ensure_connected(self) -> None:
        """
        Ensure the database connection is active, reconnect if needed.
        """
        if not self._is_connected:
            # In a real implementation, this would reconnect to the database
            # For now, we just mark it as connected
            self._is_connected = True
        # Update last access time
        self._last_access_time = self._get_current_time()

    def _flush_batch_queue(self) -> int:
        """
        Flush the batch queue and process all documents using batch_set.

        Returns:
            Number of documents successfully processed
        """
        if not self._batch_queue:
            return 0

        # Ensure connection is active
        self._ensure_connected()

        # Filter documents that need to be added or updated (redundant check, but safe)
        docs_to_process = []
        for doc_id, content, title, url, summary in self._batch_queue:
            # Check if URL already exists using bloom filter (fast check)
            url_exists = self.store.url_exists(url)

            # Generate content hash
            content_hash = self._hashlib.sha256(f"{title}{url}{summary}".encode()).hexdigest()

            # If URL exists, check if we need to update
            needs_update = True
            if url_exists:
                # Get current document metadata
                current_doc = self.get_metadata(doc_id)
                if current_doc:
                    # Check if hash exists and matches
                    if "hash" in current_doc and current_doc["hash"] == content_hash:
                        # No changes needed, skip update
                        needs_update = False

            if needs_update:
                docs_to_process.append((doc_id, content, title, url, summary))

        if not docs_to_process:
            # Clear the queue
            self._batch_queue.clear()
            return 0

        # Batch encode the documents that need updates
        # First collect all content to encode
        contents = [doc[1] for doc in docs_to_process]

        # Encode all contents at once (more efficient than one by one)
        embeddings = self.embedder.encode(contents)

        # Ensure embeddings is a list of lists
        if isinstance(embeddings[0], float):
            embeddings = [embeddings]  # type: ignore[assignment]

        # Prepare batch data for Rust
        batch_data = []
        for i, (doc_id, content, title, url, summary) in enumerate(docs_to_process):
            batch_data.append((doc_id, embeddings[i], title, url, summary))

        # Use Rust's batch_set method for efficient insertion
        result = self.store.batch_set(batch_data)

        # Clear the queue
        self._batch_queue.clear()

        return result

    def add_document_with_vector(
        self, doc_id: str, vector: List[float], title: str = "", url: str = "", summary: str = ""
    ) -> bool:
        """
        Add a document with a pre-computed vector.

        Use this when you already have the vector and want to avoid re-computing it.

        OPTIMIZATION: Uses bloom filter and content hash to avoid unnecessary operations:
        1. Checks if URL exists in bloom filter
        2. If exists, checks content hash to determine if update is needed
        3. Only updates if necessary

        Args:
            doc_id: Unique identifier for the document
            vector: Pre-computed embedding vector
            title: Document title (optional)
            url: Document URL (optional)
            summary: Document summary (optional)

        Returns:
            True if the document was added or updated, False if no changes were needed
        """
        # Check if URL already exists using bloom filter (fast check)
        url_exists = self.store.url_exists(url)

        # Generate content hash
        content_hash = self._hashlib.sha256(f"{title}{url}{summary}".encode()).hexdigest()

        # If URL exists, check if we need to update
        if url_exists:
            # Get current document metadata
            current_doc = self.get_metadata(doc_id)
            if current_doc:
                # Check if hash exists and matches
                if "hash" in current_doc and current_doc["hash"] == content_hash:
                    # No changes needed, skip update
                    return False

        # Need to add or update the document
        result = self.store.set_vector(doc_id, vector, title, url, summary if summary else None)
        return result

    def add_documents(self, documents: List[Dict[str, str]]) -> int:
        """
        Add multiple documents at once with optimized batch processing.

        OPTIMIZATIONS:
        1. Uses bloom filter to avoid processing unchanged documents
        2. Uses content hash to detect actual changes
        3. Batch encodes documents for efficiency
        4. Uses Rust's batch_set for fast insertion
        5. Discards content after vectorization

        Args:
            documents: List of document dictionaries with keys:
                      - id: Document ID (required)
                      - content: Document content (required, will be discarded)
                      - title: Document title (optional, will be stored)
                      - url: Document URL (optional, will be stored)
                      - summary: Document summary (optional, will be stored)

        Returns:
            Number of documents successfully added or updated
        """
        # Filter documents that need to be added or updated
        docs_to_process = []

        for doc in documents:
            doc_id = doc.get("id")
            content = doc.get("content")

            if not doc_id or not content:
                raise ValueError("Each document must have 'id' and 'content' fields")

            title = doc.get("title", "")
            url = doc.get("url", "")
            summary = doc.get("summary", "")

            # Check if URL already exists using bloom filter (fast check)
            url_exists = self.store.url_exists(url)

            # Generate content hash
            content_hash = self._hashlib.sha256(f"{title}{url}{summary}".encode()).hexdigest()

            # If URL exists, check if we need to update
            needs_update = True
            if url_exists:
                # Get current document metadata
                current_doc = self.get_metadata(doc_id)
                if current_doc:
                    # Check if hash exists and matches
                    if "hash" in current_doc and current_doc["hash"] == content_hash:
                        # No changes needed, skip update
                        needs_update = False

            if needs_update:
                docs_to_process.append((doc_id, content, title, url, summary))

        if not docs_to_process:
            return 0

        # Batch encode the documents that need updates
        # First collect all content to encode
        contents = [doc[1] for doc in docs_to_process]

        # Encode all contents at once (more efficient than one by one)
        embeddings = self.embedder.encode(contents)

        # Ensure embeddings is a list of lists
        if isinstance(embeddings[0], float):
            embeddings = [embeddings]  # type: ignore[assignment]

        # Prepare batch data for Rust
        batch_data = []
        for i, (doc_id, content, title, url, summary) in enumerate(docs_to_process):
            batch_data.append((doc_id, embeddings[i], title, url, summary))

        # Use Rust's batch_set method for efficient insertion
        result = self.store.batch_set(batch_data)
        return result

    def search(self, query: str, k: int = 5) -> List[Dict[str, Any]]:
        """
        Search for documents similar to the query text.

        Args:
            query: Query text
            k: Number of results to return

        Returns:
            List of result dictionaries with keys: id, score, title, url, summary
            Note: 'content' is NOT included since we don't store it!
        """
        # Generate embedding for query
        query_embedding = self.embedder.encode(query)

        # Ensure embedding is a flat list of floats
        if (
            isinstance(query_embedding, list)
            and len(query_embedding) > 0
            and isinstance(query_embedding[0], list)
        ):
            query_embedding = query_embedding[0]

        # Search in Rust store
        results = self.store.search(query_embedding, k)

        return results  # type: ignore[no-any-return]

    def search_by_embedding(self, embedding: List[float], k: int = 5) -> List[Dict[str, Any]]:
        """
        Search for documents using a pre-computed embedding.

        Args:
            embedding: Query embedding (list of floats)
            k: Number of results to return

        Returns:
            List of result dictionaries with keys: id, score, title, url
        """
        return self.store.search(embedding, k)  # type: ignore[no-any-return]

    def remove_document(self, doc_id: str) -> None:
        """
        Remove a document from the store.

        Args:
            doc_id: Document ID to remove
        """
        self.store.rm(doc_id)

    def get_metadata(self, doc_id: str) -> Optional[Dict[str, str]]:
        """
        Get metadata for a specific document.

        Args:
            doc_id: Document ID

        Returns:
            Dictionary with title, url, and summary (no content!)
        """
        return self.store.get_metadata(doc_id)  # type: ignore[no-any-return]

    def __len__(self) -> int:
        """Get the number of documents in the store."""
        return self.store.len()  # type: ignore[no-any-return]

    def is_empty(self) -> bool:
        """Check if the store is empty."""
        return self.store.is_empty()  # type: ignore[no-any-return]
