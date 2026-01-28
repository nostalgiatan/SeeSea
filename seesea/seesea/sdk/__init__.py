"""
SeeSea SDK - 统一的Python接口

SeeSea是一个高性能的搜索和数据处理平台，提供：
- 多引擎搜索聚合
- 智能向量化和嵌入
- 实时股票数据处理
- 分布式缓存系统
- 系统资源管控

此SDK提供了对底层Rust核心的完整Python绑定。
"""

__version__ = "0.1.0"
__author__ = "SeeSea Team"

# 导入核心模块
from .search import SearchClient
from .server import ApiServerManager, ServerStatus
from .configuration import ConfigClient, ConfigHelper
from .feed import RssClient, HotTrendClient
from .stock import StockClient, StockScheduler

# 新增模块
from .cache import CacheClient
from .net import NetClient
from .vector import VectorClient
from .cleaner import CleanerClient
from .browser import BrowserClient
from .date_page import DatePageClient
from .embeddings import EmbeddingCallback

# 类型系统
from . import seesea_types

# 基础客户端
from .base import BaseClient, AsyncBaseClient


# 引擎系统（延迟导入）
def __getattr__(name):
    """延迟导入engines模块"""
    if name == "engines":
        from . import engines

        return engines
    raise AttributeError(f"module '{__name__}' has no attribute '{name}'")


__all__ = [
    # 类型系统
    "seesea_types",
    # 基础客户端
    "BaseClient",
    "AsyncBaseClient",
    # 核心功能
    "SearchClient",
    "ConfigClient",
    "ConfigHelper",
    # 服务器管理
    "ApiServerManager",
    "ServerStatus",
    # 信息源
    "RssClient",
    "HotTrendClient",
    # 股票数据
    "StockClient",
    "StockScheduler",
    # 新增模块
    "CacheClient",
    "NetClient",
    "VectorClient",
    "CleanerClient",
    "BrowserClient",
    "DatePageClient",
    "EmbeddingCallback",
    # 引擎系统（延迟导入）
    "engines",
]
