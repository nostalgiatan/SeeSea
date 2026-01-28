# Copyright (C) 2025 nostalgiatan
#
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU Affero General Public License as published
# by the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
#
# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU Affero General Public License for more details.
#
# You should have received a copy of the GNU Affero General Public License
# along with this program.  If not, see <https://www.gnu.org/licenses/>.

"""
SeeSea - Privacy-focused Metasearch Engine with Unified SDK
==========================================================

SeeSea 是一个基于 Rust 的高性能隐私保护型元搜索引擎，通过统一的 Python SDK 提供简单易用的接口。

主要功能：
- 多引擎并发搜索（12个搜索引擎及其变体）
- 智能结果聚合
- RSS feed 订阅和解析
- 股票数据查询
- 高性能（共享连接池，87.5% 内存优化）
- 完整的 REST API 服务器管理
- 隐私保护（无追踪、支持代理）
- 类型安全（统一的类型系统）
- Pro 功能（可选）: LLM 支持、向量数据库、语义搜索

快速开始：
    >>> from seesea import SearchClient, RssClient, HotTrendClient
    >>>
    >>> # 搜索 - 使用统一客户端接口
    >>> with SearchClient() as client:
    ...     result = client.search("python programming")
    ...     if result.success:
    ...         print(f"找到 {len(result.data.results)} 个结果")
    ...         for item in result.data.results:
    ...             print(f"{item.title}: {item.url}")
    >>>
    >>> # RSS feed 订阅
    >>> with RssClient() as rss:
    ...     result = rss.fetch_feed("https://example.com/rss", max_items=10)
    ...     if result.success:
    ...         for item in result.data['items']:
    ...             print(item['title'])
    >>>
    >>> # 热点趋势数据
    >>> with HotTrendClient() as hot:
    ...     result = hot.fetch_platform("weibo")
    ...     if result.success:
    ...         print(f"获取到 {len(result.data['items'])} 条热点")
    >>>
    >>> # 服务器管理
    >>> from seesea import ApiServerManager
    >>> server = ApiServerManager()
    >>> server.start({'host': '127.0.0.1', 'port': 8080})

Pro 功能 (可选):
    Pro 功能包括高级的 LLM 和向量搜索能力，需要额外的依赖：

    安装方式:
    1. 预编译包 (推荐，快速):
       pip install llama-cpp-python --index-url https://abetlen.github.io/llama-cpp-python/whl/cpu

    2. 本地编译 (针对系统优化):
       pip install llama-cpp-python

    注意: Pro 功能默认不启用，避免自动下载模型。使用 enable_pro=True 显式启用。
"""

__version__ = "2.2.2"
__author__ = "SeeSea Team"

# 导入 Rust 核心模块
try:
    from seesea_core import (
        PySearchClient,
        PyConfig,
        PyCacheStats,
        PyCacheInterface,
        PyRssClient,
        PyHotTrendClient,
        # 缓存和清理功能
        PyCleaner,
        # 引擎注册函数（不再是类）
        register_engine,
        unregister_engine,
        list_engines,
        has_engine,
        # 网络客户端函数
        get,
        post,
        get_file,
        post_file,
    )
except ImportError as e:
    import warnings

    warnings.warn(
        f"Failed to import Rust core module: {e}. Please install seesea_core with 'pip install seesea_core'"
    )
    PySearchClient = None
    PyConfig = None
    PyCacheStats = None
    PyCacheInterface = None
    PyRssClient = None
    PyHotTrendClient = None
    PyCleaner = None
    register_engine = None
    unregister_engine = None
    list_engines = None
    has_engine = None
    get = None
    post = None

# CLI 入口
from .cli import cli as cli_main

# 工具函数
from .utils import format_results

# SDK 接口
from .sdk import (
    seesea_types,
    BaseClient,
    AsyncBaseClient,
    SearchClient,
    ConfigClient,
    ConfigHelper,
    ApiServerManager,
    ServerStatus,
    RssClient,
    HotTrendClient,
    StockClient,
    StockScheduler,
    CacheClient,
    NetClient,
    VectorClient,
    CleanerClient,
    BrowserClient,
    DatePageClient,
    EmbeddingCallback,
)

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
    # Rust 核心类（高级用户）
    "PySearchClient",
    "PyRssClient",
    "PyHotTrendClient",
    "PyConfig",
    "PyCacheStats",
    "PyCacheInterface",
    "PyCleaner",
    # 网络客户端函数
    "get",
    "post",
    "get_file",
    "post_file",
    # 工具函数
    "format_results",
    # CLI
    "cli_main",
    # 版本信息
    "__version__",
]
