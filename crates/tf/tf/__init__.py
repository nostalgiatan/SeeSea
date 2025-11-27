"""
TF - Text Semantic Retrieval System SDK

A high-performance text semantic retrieval system combining Python and Rust.

Quick Start:
    >>> from tf import DocumentStore
    >>> store = DocumentStore()
    >>> store.add("doc1", "Content here", title="Title", summary="Summary")
    >>> results = store.search("query text", k=5)
    
    # Streaming for memory efficiency
    >>> for result in store.search_streaming("query", k=100):
    ...     print(f"{result.title}: {result.score:.3f}")
"""

from .embeddings import TextEmbedder
from .vector_store import VectorStoreWrapper
from .sdk import DocumentStore, SDK
from .search_result import SearchResult, StreamingSearchResult

__all__ = [
    'TextEmbedder',
    'VectorStoreWrapper', 
    'DocumentStore',
    'SDK',
    'SearchResult',
    'StreamingSearchResult'
]
__version__ = '0.1.0'
