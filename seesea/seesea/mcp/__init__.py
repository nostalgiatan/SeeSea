"""
SeeSea MCP - Model Context Protocol 实现

提供基于 MCP 协议的各种功能模块。
"""

__version__ = "2.2.2"
__author__ = "SeeSea Team"

from fastmcp import FastMCP

from . import search
from . import rss
from . import stock
from . import cleaner
from . import hot


def create_mcp_server(
    name: str = "seesea",
    version: str = "2.2.2",
    description: str = "SeeSea - 提供隐私保护的搜索和数据获取功能",
) -> FastMCP:
    """
    创建 SeeSea MCP 服务器

    Args:
        name: 服务器名称
        version: 服务器版本
        description: 服务器描述

    Returns:
        FastMCP 服务器实例
    """
    # 创建 MCP 服务器
    mcp = FastMCP(name=name)

    # 注册搜索工具
    search.register_tools(mcp)

    # 注册 RSS 工具
    rss.register_tools(mcp)

    # 注册股票工具
    stock.register_tools(mcp)

    # 注册清洗工具
    cleaner.register_tools(mcp)

    # 注册热点趋势工具
    hot.register_hot_tools(mcp)

    return mcp


__all__ = ["create_mcp_server"]
