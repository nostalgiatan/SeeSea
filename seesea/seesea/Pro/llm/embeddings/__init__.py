"""
Embeddings module for SeeSea Pro LLM system.
This module provides various embedding models and utilities for text vectorization.
"""

from .llama_cpp_embeddings import LlamaCppEmbedder

__all__ = [
    "LlamaCppEmbedder",
]
