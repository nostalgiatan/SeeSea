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
SeeSea - Privacy-focused Metasearch Engine with RSS and Browser Support
========================================================================

SeeSea 是一个基于 Rust 的高性能隐私保护型元搜索引擎，通过 Python SDK 提供简单易用的接口。

主要功能：
- 多引擎并发搜索（12个搜索引擎及其变体）
- 智能结果聚合
- RSS feed 订阅和解析
- 浏览器引擎支持（Playwright集成）
- 高性能（共享连接池，87.5% 内存优化）
- 完整的 REST API 服务器
- 隐私保护（无追踪、支持代理）
- 类型安全（Python dataclass 对象）
- Pro 功能（可选）: LLM 支持、向量数据库、语义搜索

快速开始：
    >>> from seesea import SearchClient, RssClient
    >>>
    >>> # 搜索 - 返回类型安全的对象
    >>> client = SearchClient()
    >>> response = client.search("python programming")
    >>> print(f"找到 {response.total_count} 个结果")
    >>> for item in response.results:
    ...     print(f"{item.title}: {item.url} (score: {item.score})")
    >>>
    >>> # 全文搜索 - 整合网络、缓存和RSS
    >>> response = client.search_fulltext("rust async")
    >>> print(f"来源: {response.engines_used}")
    >>> for item in response:  # 可以直接迭代
    ...     print(f"{item.title} - {item.score:.2f}")
    >>>
    >>> # RSS feed
    >>> rss_client = RssClient()
    >>> feed = rss_client.fetch_feed("https://example.com/rss")
    >>> for item in feed['items']:
    ...     print(item['title'])
    >>>
    >>> # Browser engine (需要安装 playwright)
    >>> from seesea import BrowserEngineClient, BrowserConfig
    >>> config = BrowserConfig(headless=True, stealth=True)
    >>> browser_client = BrowserEngineClient(config)
    >>>
    >>> # API Server with Pro features (可选)
    >>> from seesea import ApiServer
    >>> # 默认不启用 Pro 功能
    >>> server = ApiServer(host="127.0.0.1", port=8080)
    >>> # 启用 Pro 功能 (需要安装 llama-cpp-python)
    >>> server = ApiServer(host="127.0.0.1", port=8080, enable_pro=True)
    >>> server.start()

Pro 功能 (可选):
    Pro 功能包括高级的 LLM 和向量搜索能力，需要额外的依赖：

    安装方式:
    1. 预编译包 (推荐，快速):
       pip install llama-cpp-python --index-url https://abetlen.github.io/llama-cpp-python/whl/cpu

    2. 本地编译 (针对系统优化):
       pip install llama-cpp-python

    注意: Pro 功能默认不启用，避免自动下载模型。使用 enable_pro=True 显式启用。
"""

__version__ = "2.0.0"
__author__ = "SeeSea Team"

# 导入 Rust 核心模块
try:
    from seesea_core import (
        PySearchClient,
        PyApiServer,
        PyConfig,
        PyCacheStats,
        PyCacheInterface,
        PyRssClient,
        PyBrowserConfig,
        PyBrowserEngineClient,
        PyNetClient,
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
        # HTML解析器函数
        determine_page_type,
        get_html_meta_info,
    )
except ImportError as e:
    import warnings

    warnings.warn(
        f"Failed to import Rust core module: {e}. Please install seesea_core with 'pip install seesea_core'"
    )
    PySearchClient = None
    PyApiServer = None
    PyConfig = None
    PyCacheStats = None
    PyCacheInterface = None
    PyRssClient = None
    PyBrowserConfig = None
    PyBrowserEngineClient = None
    PyNetClient = None
    register_engine = None
    unregister_engine = None
    list_engines = None
    has_engine = None
    get = None
    post = None
    determine_page_type = None
    get_html_meta_info = None

# Python 高层接口
from .search import SearchClient
from .api import ApiServer
from .config import Config
from .rss import RssClient
from .browser import (
    BrowserEngineClient,
    BrowserConfig,
    BrowserEngine,
    BaseBrowserEngine,
    XinhuaEngine,
    create_xinhua_callback,
)
from .utils import format_results, parse_query
from .cli import cli as cli_main

# 类型定义（提供类型安全）
from .search_types import (
    SearchResultItem,
    SearchResponse,
    EngineState,
    CacheInfo,
    SearchStats,
    PrivacyStats,
)


def _auto_register_engines() -> None:
    """
    自动注册所有可用的Python引擎

    自动发现browser目录下的所有模块（除了base.py），并注册它们。
    引擎名称就是模块名称。
    """
    if register_engine is None:
        return

    import os
    import importlib
    import inspect

    registered_count = 0
    failed_count = 0

    # 获取browser目录路径
    browser_dir = os.path.join(os.path.dirname(__file__), "browser")

    # 遍历browser目录下的所有.py文件
    for filename in os.listdir(browser_dir):
        if not filename.endswith(".py"):
            continue
        if filename.startswith("_"):  # 跳过__init__.py等
            continue
        if filename == "base.py":  # 跳过base.py
            continue

        module_name = filename[:-3]  # 移除.py后缀

        try:
            # 动态导入模块
            module = importlib.import_module(
                f".browser.{module_name}", package="seesea"
            )

            # 查找回调函数（通常命名为create_<module>_callback_sync）
            callback_name = f"create_{module_name}_callback_sync"
            if hasattr(module, callback_name):
                callback = getattr(module, callback_name)

                # 尝试从模块获取engine_type，如果没有则默认为"general"
                engine_type = getattr(module, "ENGINE_TYPE", "general")
                description = getattr(
                    module, "ENGINE_DESCRIPTION", f"{module_name.title()} Search Engine"
                )
                categories = getattr(module, "ENGINE_CATEGORIES", ["general"])

                # 注册引擎
                try:
                    register_engine(
                        name=module_name,
                        engine_type=engine_type,
                        description=description,
                        categories=categories,
                        callback=callback,
                    )
                    registered_count += 1
                except Exception:
                    import traceback

                    traceback.print_exc()
            else:
                # 如果没有找到标准回调函数，尝试其他可能的名称
                # 例如：create_callback_sync, search_callback等
                for attr_name in dir(module):
                    if "callback" in attr_name.lower() and callable(
                        getattr(module, attr_name)
                    ):
                        # 找到了一个可能的回调函数
                        callback = getattr(module, attr_name)

                        # 检查函数签名是否合适（应该接受一个参数）
                        sig = inspect.signature(callback)
                        if len(sig.parameters) == 1:
                            engine_type = getattr(module, "ENGINE_TYPE", "general")
                            description = getattr(
                                module,
                                "ENGINE_DESCRIPTION",
                                f"{module_name.title()} Search Engine",
                            )
                            categories = getattr(
                                module, "ENGINE_CATEGORIES", ["general"]
                            )

                            register_engine(
                                name=module_name,
                                engine_type=engine_type,
                                description=description,
                                categories=categories,
                                callback=callback,
                            )
                            registered_count += 1
                            break

        except Exception:
            # 显示所有错误，不静默
            failed_count += 1

    # 只在成功注册时显示消息
    if registered_count > 0:
        import sys

        if hasattr(sys, "ps1"):  # 仅在交互式模式下打印
            pass

    if failed_count > 0:
        import sys

        if hasattr(sys, "ps1"):
            print(f"⚠️ SeeSea: {failed_count} 引擎注册失败")


# 在导入时自动注册引擎
try:
    _auto_register_engines()
except Exception as e:
    # 显示错误，帮助调试
    import warnings

    warnings.warn(f"Failed to auto-register engines: {e}")

__all__ = [
    # 主要类
    "SearchClient",
    "RssClient",
    "BrowserEngineClient",
    "BrowserConfig",
    "BrowserEngine",
    "BaseBrowserEngine",
    "XinhuaEngine",
    "ApiServer",
    "Config",
    # 类型定义（类型安全）
    "SearchResponse",
    "SearchResultItem",
    "EngineState",
    "CacheInfo",
    "SearchStats",
    "PrivacyStats",
    # Rust 核心类（高级用户）
    "PySearchClient",
    "PyRssClient",
    "PyBrowserConfig",
    "PyBrowserEngineClient",
    "PyApiServer",
    "PyConfig",
    "PyCacheStats",
    "PyCacheInterface",
    "PyNetClient",
    # 引擎注册函数
    "register_engine",
    "unregister_engine",
    "list_engines",
    "has_engine",
    # 网络客户端函数
    "get",
    "post",
    "get_file",
    "post_file",
    # HTML解析器函数
    "determine_page_type",
    "get_html_meta_info",
    # 工具函数
    "format_results",
    "parse_query",
    # CLI
    "cli_main",
    # 浏览器回调
    "create_xinhua_callback",
    # 版本信息
    "__version__",
]
