"""
搜索相关类型定义
"""

from typing import Dict, List, Optional, Any, Literal
from dataclasses import dataclass
from datetime import datetime

__all__ = [
    "SearchQuery",
    "SearchResultItem",
    "SearchResult",
    "SearchEngineType",
    "SearchEngine",
    "SearchConfig",
]


@dataclass
class SearchQuery:
    """搜索查询"""

    query: str
    page: int = 1
    per_page: int = 10
    sort_by: Optional[str] = None
    filters: Optional[Dict[str, Any]] = None
    include_content: bool = True
    max_results: Optional[int] = None


@dataclass
class SearchResultItem:
    """搜索结果项"""

    title: str
    content: str
    url: str
    score: float
    engine: str
    timestamp: datetime
    metadata: Optional[Dict[str, Any]] = None
    snippet: Optional[str] = None
    thumbnail: Optional[str] = None


@dataclass
class SearchResult:
    """搜索结果"""

    items: List[SearchResultItem]
    total: int
    page: int
    per_page: int
    query: str
    took_ms: int
    engines_used: List[str]
    has_more: bool = False


SearchEngineType = Literal["network", "cache", "rss", "local"]


@dataclass
class SearchEngine:
    """搜索引擎配置"""

    name: str
    engine_type: SearchEngineType
    enabled: bool = True
    priority: int = 1
    timeout_ms: int = 5000
    max_results: int = 100
    config: Optional[Dict[str, Any]] = None


@dataclass
class SearchConfig:
    """搜索配置"""

    engines: List[SearchEngine]
    default_timeout_ms: int = 10000
    max_concurrent_searches: int = 5
    enable_result_caching: bool = True
    cache_ttl_seconds: int = 3600
    enable_content_deduplication: bool = True
    similarity_threshold: float = 0.85
