"""
Vector store wrapper that integrates Python embeddings with Rust vector storage.

MEMORY EFFICIENT DESIGN:
- Content is NOT stored in the vector database
- Only vectors and metadata (title, url) are stored
- Content is discarded immediately after vectorization
- This minimizes memory usage and maximizes performance
"""

from typing import List, Dict, Optional, Any, Callable
from .embeddings import TextEmbedder


class VectorStoreWrapper:
    """
    High-level wrapper that combines TextEmbedder with the Rust VectorStore.
    
    This class provides a memory-efficient interface for adding documents,
    searching by text, and managing the vector database.
    
    KEY FEATURE: Content is NOT stored - only vectors and metadata!
    """
    
    def __init__(self, embedder: TextEmbedder = None, rust_store=None):
        """
        Initialize the vector store wrapper.
        
        Args:
            embedder: TextEmbedder instance (created if None)
            rust_store: Rust VectorStore instance (created if None)
        """
        # Initialize embedder
        if embedder is None:
            self.embedder = TextEmbedder()
        else:
            self.embedder = embedder
        
        # Initialize Rust store
        if rust_store is None:
            try:
                from tf_rust import VectorStore
                self.store = VectorStore(self.embedder.get_dimension())
            except ImportError as e:
                raise ImportError(
                    "Failed to import tf_rust module. "
                    "Please build the Rust extension first using 'maturin develop' or 'maturin build'."
                ) from e
        else:
            self.store = rust_store
    
    def add_document(
        self,
        doc_id: str,
        content: str,
        title: str = "",
        url: str = "",
        summary: str = ""
    ) -> None:
        """
        Add a document to the vector store.
        
        MEMORY EFFICIENT: The content is vectorized and then immediately discarded.
        Only the vector and metadata (title, url, summary) are stored.
        
        Args:
            doc_id: Unique identifier for the document
            content: Document content (will be vectorized then discarded)
            title: Document title (optional, will be stored)
            url: Document URL (optional, will be stored)
            summary: Document summary (optional, will be stored)
        """
        # Create callback function for Rust to call
        def embedding_callback(text: str) -> List[float]:
            """Callback that Rust calls to get the embedding vector."""
            embedding = self.embedder.encode(text)
            # Ensure embedding is a flat list of floats
            if isinstance(embedding, list) and len(embedding) > 0 and isinstance(embedding[0], list):
                embedding = embedding[0]
            return embedding
        
        # Call Rust's set method with the callback
        # Rust will:
        # 1. Call embedding_callback(content) to get the vector
        # 2. Store the vector with metadata (title, url, summary)
        # 3. Discard the content - it's never stored!
        self.store.set(doc_id, content, title, url, summary, embedding_callback)
    
    def add_document_with_vector(
        self,
        doc_id: str,
        vector: List[float],
        title: str = "",
        url: str = "",
        summary: str = ""
    ) -> None:
        """
        Add a document with a pre-computed vector.
        
        Use this when you already have the vector and want to avoid re-computing it.
        
        Args:
            doc_id: Unique identifier for the document
            vector: Pre-computed embedding vector
            title: Document title (optional)
            url: Document URL (optional)
            summary: Document summary (optional)
        """
        self.store.set_vector(doc_id, vector, title, url, summary if summary else None)
    
    def add_documents(self, documents: List[Dict[str, str]]) -> None:
        """
        Add multiple documents at once.
        
        MEMORY EFFICIENT: Each document's content is vectorized and discarded
        before processing the next one.
        
        Args:
            documents: List of document dictionaries with keys:
                      - id: Document ID (required)
                      - content: Document content (required, will be discarded)
                      - title: Document title (optional, will be stored)
                      - url: Document URL (optional, will be stored)
        """
        for doc in documents:
            doc_id = doc.get('id')
            content = doc.get('content')
            
            if not doc_id or not content:
                raise ValueError("Each document must have 'id' and 'content' fields")
            
            title = doc.get('title', '')
            url = doc.get('url', '')
            summary = doc.get('summary', '')
            
            self.add_document(doc_id, content, title, url, summary)
            # At this point, content has been vectorized and discarded!
    
    def search(
        self,
        query: str,
        k: int = 5
    ) -> List[Dict[str, Any]]:
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
        if isinstance(query_embedding, list) and len(query_embedding) > 0 and isinstance(query_embedding[0], list):
            query_embedding = query_embedding[0]
        
        # Search in Rust store
        results = self.store.search(query_embedding, k)
        
        return results
    
    def search_by_embedding(
        self,
        embedding: List[float],
        k: int = 5
    ) -> List[Dict[str, Any]]:
        """
        Search for documents using a pre-computed embedding.
        
        Args:
            embedding: Query embedding (list of floats)
            k: Number of results to return
            
        Returns:
            List of result dictionaries with keys: id, score, title, url
        """
        return self.store.search(embedding, k)
    
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
        return self.store.get_metadata(doc_id)
    
    def __len__(self) -> int:
        """Get the number of documents in the store."""
        return self.store.len()
    
    def is_empty(self) -> bool:
        """Check if the store is empty."""
        return self.store.is_empty()

