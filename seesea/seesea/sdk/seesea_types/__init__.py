"""
SeeSea 类型系统

定义了所有SeeSea模块使用的数据类型和接口定义。
避免与Python标准库同名的类型冲突。
"""

# 导入所有类型模块
from .search_types import (
    SearchQuery,
    SearchResult,
    SearchResultItem,
    SearchEngine,
    SearchConfig,
)
from .api_types import (
    ApiRequest,
    ApiResponse,
    ApiConfig,
    HttpStatus,
)
from .config_types import (
    ConfigSection,
    ConfigValue,
    SystemConfig,
)
from .cache_types import (
    CacheKey,
    CacheValue,
    CacheStats,
    TTLConfig,
)
from .stock_types import (
    StockInfo,
    StockPrice,
    MarketData,
    Exchange,
)
from .vector_types import (
    VectorData,
    EmbeddingResult,
    SimilarityScore,
)
from .common_types import (
    Result,
    Error,
    Status,
    Timestamp,
)

__all__ = [
    # 搜索相关类型
    "SearchQuery",
    "SearchResult",
    "SearchResultItem",
    "SearchEngine",
    "SearchConfig",
    # API相关类型
    "ApiRequest",
    "ApiResponse",
    "ApiConfig",
    "HttpStatus",
    # 配置类型
    "ConfigSection",
    "ConfigValue",
    "SystemConfig",
    # 缓存类型
    "CacheKey",
    "CacheValue",
    "CacheStats",
    "TTLConfig",
    # 股票类型
    "StockInfo",
    "StockPrice",
    "MarketData",
    "Exchange",
    # 向量类型
    "VectorData",
    "EmbeddingResult",
    "SimilarityScore",
    # 通用类型
    "Result",
    "Error",
    "Status",
    "Timestamp",
]
