"""
缓存相关类型定义
"""

from typing import Dict, List, Optional, Any, Union, Literal
from dataclasses import dataclass
from datetime import datetime

CacheKeyType = Union[str, bytes]
CacheValueType = Union[str, bytes, int, float, Dict[str, Any], List[Any]]

__all__ = [
    "CacheKeyType",
    "CacheValueType",
    "CacheKey",
    "CacheValue",
    "CacheStats",
    "CacheStrategyType",
    "TTLConfig",
    "CacheConfig",
    "CacheOperation",
]


@dataclass
class CacheKey:
    """缓存键"""

    key: CacheKeyType
    namespace: str
    ttl_seconds: Optional[int] = None
    tags: Optional[List[str]] = None


@dataclass
class CacheValue:
    """缓存值"""

    value: CacheValueType
    size_bytes: int
    created_at: datetime
    expires_at: Optional[datetime] = None
    access_count: int = 0
    last_accessed: Optional[datetime] = None


@dataclass
class CacheStats:
    """缓存统计"""

    hit_count: int
    miss_count: int
    hit_rate: float
    total_keys: int
    total_size_bytes: int
    eviction_count: int
    expired_count: int


CacheStrategyType = Literal["LRU", "LFU", "FIFO", "TTL"]


@dataclass
class TTLConfig:
    """TTL配置"""

    default_ttl_seconds: int = 3600
    max_ttl_seconds: int = 86400  # 24小时
    min_ttl_seconds: int = 60
    cleanup_interval_seconds: int = 300  # 5分钟


@dataclass
class CacheConfig:
    """缓存配置"""

    strategy: CacheStrategyType
    max_size_bytes: int
    max_keys: int
    ttl_config: TTLConfig
    enable_compression: bool = False
    enable_encryption: bool = False
    backup_interval_seconds: Optional[int] = None


@dataclass
class CacheOperation:
    """缓存操作"""

    operation: Literal["GET", "SET", "DELETE", "CLEAR", "STATS"]
    key: Optional[CacheKeyType] = None
    value: Optional[CacheValueType] = None
    ttl_seconds: Optional[int] = None
    namespace: Optional[str] = None
