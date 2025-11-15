"""
SeeSea - Privacy-focused Metasearch Engine
==========================================

SeeSea 是一个基于 Rust 的高性能隐私保护型元搜索引擎，通过 Python SDK 提供简单易用的接口。

主要功能：
- 多引擎并发搜索（16个搜索引擎）
- 智能结果聚合
- 高性能（共享连接池，87.5% 内存优化）
- 完整的 REST API 服务器
- 隐私保护（无追踪、支持代理）

快速开始：
    >>> from seesea import SearchClient
    >>> client = SearchClient()
    >>> results = client.search("python programming")
    >>> print(results['total_count'])
"""

__version__ = "0.1.0"
__author__ = "SeeSea Team"

# 导入 Rust 核心模块
try:
    from seesea_core import (
        PySearchClient,
        PyApiServer,
        PyConfig,
        PyCacheStats,
    )
except ImportError as e:
    import warnings
    warnings.warn(f"Failed to import Rust core module: {e}. Please build with 'maturin develop'")
    PySearchClient = None
    PyApiServer = None
    PyConfig = None
    PyCacheStats = None

# Python 高层接口
from .search import SearchClient
from .api import ApiServer
from .config import Config
from .utils import format_results, parse_query
from .cli import main as cli_main

__all__ = [
    # 主要类
    'SearchClient',
    'ApiServer',
    'Config',
    
    # Rust 核心类（高级用户）
    'PySearchClient',
    'PyApiServer',
    'PyConfig',
    'PyCacheStats',
    
    # 工具函数
    'format_results',
    'parse_query',
    
    # CLI
    'cli_main',
    
    # 版本信息
    '__version__',
]
