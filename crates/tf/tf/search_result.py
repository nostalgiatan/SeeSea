"""
Search result data structures for memory-efficient result handling.

This module provides optimized result classes that minimize memory usage
while providing structured access to search results.
"""

from typing import Optional
from dataclasses import dataclass


@dataclass(frozen=True, slots=True)
class SearchResult:
    """
    Immutable search result with minimal memory footprint.
    
    Uses frozen dataclass with __slots__ to minimize memory overhead.
    Each result stores only metadata and relevance score, no vectors.
    
    Attributes:
        id: Document identifier
        score: Relevance score (0-1, higher is more relevant)
        title: Document title
        url: Document URL
        summary: Document summary
    """
    id: str
    score: float
    title: str
    url: str
    summary: str
    
    def __lt__(self, other: 'SearchResult') -> bool:
        """Compare by score for sorting (descending order)."""
        return self.score > other.score  # Reverse for descending
    
    def to_dict(self) -> dict:
        """
        Convert to dictionary for backward compatibility.
        
        Returns:
            Dictionary with id, score, title, url, summary
        """
        return {
            'id': self.id,
            'score': self.score,
            'title': self.title,
            'url': self.url,
            'summary': self.summary
        }
    
    def __repr__(self) -> str:
        """String representation showing key fields."""
        return f"SearchResult(id='{self.id}', score={self.score:.4f}, title='{self.title[:30]}...')"


@dataclass(frozen=True, slots=True)
class StreamingSearchResult:
    """
    Lightweight streaming search result for memory-efficient iteration.
    
    Minimal structure for streaming results without buffering.
    
    Attributes:
        id: Document identifier
        score: Relevance score
    """
    id: str
    score: float
    
    def __lt__(self, other: 'StreamingSearchResult') -> bool:
        """Compare by score for sorting (descending order)."""
        return self.score > other.score
