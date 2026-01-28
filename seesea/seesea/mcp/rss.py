"""
SeeSea MCP RSS 工具

直接定义 RSS 相关的 MCP 工具。
"""

from typing import Optional, List, Dict, Any, Tuple
from fastmcp import FastMCP
from ..sdk.feed.rss_client import RssClient

# 初始化 RSS 客户端
_rss_client = RssClient()


async def fetch_feed(
    url: str,
    max_items: Optional[int] = None,
    filter_keywords: Optional[List[str]] = None,
) -> Dict[str, Any]:
    """
    获取 RSS feed

    Args:
        url: RSS feed URL
        max_items: 最大项目数（可选）
        filter_keywords: 过滤关键词列表（可选）

    Returns:
        RSS feed 数据
    """
    try:
        result = _rss_client.connect()
        if not result.success:
            return {
                "success": False,
                "error": result.error.message if result.error else "连接失败",
            }

        feed_result = _rss_client.fetch_feed(url, max_items, filter_keywords)
        _rss_client.disconnect()

        if not feed_result.success:
            return {
                "success": False,
                "error": (
                    feed_result.error.message if feed_result.error else "获取 feed 失败"
                ),
            }

        return {"success": True, "feed": feed_result.data}

    except Exception as e:
        return {"success": False, "error": f"获取 RSS feed 失败: {str(e)}"}


async def parse_feed(content: str) -> Dict[str, Any]:
    """
    解析 RSS feed 内容

    Args:
        content: RSS feed XML 内容

    Returns:
        解析后的 RSS feed 数据
    """
    try:
        result = _rss_client.connect()
        if not result.success:
            return {
                "success": False,
                "error": result.error.message if result.error else "连接失败",
            }

        parse_result = _rss_client.parse_feed(content)
        _rss_client.disconnect()

        if not parse_result.success:
            return {
                "success": False,
                "error": (
                    parse_result.error.message
                    if parse_result.error
                    else "解析 feed 失败"
                ),
            }

        return {"success": True, "feed": parse_result.data}

    except Exception as e:
        return {"success": False, "error": f"解析 RSS feed 失败: {str(e)}"}


async def list_templates() -> Dict[str, Any]:
    """
    列出所有可用的 RSS 模板

    Returns:
        模板列表
    """
    try:
        result = _rss_client.connect()
        if not result.success:
            return {
                "success": False,
                "error": result.error.message if result.error else "连接失败",
            }

        templates_result = _rss_client.list_templates()
        _rss_client.disconnect()

        if not templates_result.success:
            return {
                "success": False,
                "error": (
                    templates_result.error.message
                    if templates_result.error
                    else "获取模板列表失败"
                ),
            }

        return {"success": True, "templates": templates_result.data}

    except Exception as e:
        return {"success": False, "error": f"获取模板列表失败: {str(e)}"}


async def add_from_template(
    template_name: str,
    categories: Optional[List[str]] = None,
) -> Dict[str, Any]:
    """
    从模板添加 RSS feeds

    Args:
        template_name: 模板名称（如 "xinhua"）
        categories: 要添加的分类列表（可选，默认添加所有）

    Returns:
        添加的 feed 数量
    """
    try:
        result = _rss_client.connect()
        if not result.success:
            return {
                "success": False,
                "error": result.error.message if result.error else "连接失败",
            }

        add_result = _rss_client.add_from_template(template_name, categories)
        _rss_client.disconnect()

        if not add_result.success:
            return {
                "success": False,
                "error": (
                    add_result.error.message if add_result.error else "从模板添加失败"
                ),
            }

        return {
            "success": True,
            "count": add_result.data,
            "message": f"已添加 {add_result.data} 个 RSS feeds",
        }

    except Exception as e:
        return {"success": False, "error": f"从模板添加失败: {str(e)}"}


async def create_ranking(
    feed_urls: List[str],
    keywords: List[Tuple[str, float]],
    min_score: float = 0.0,
    max_results: int = 100,
) -> Dict[str, Any]:
    """
    创建 RSS 榜单 - 基于关键词对 RSS 项目进行评分和排名

    Args:
        feed_urls: RSS Feed URL 列表
        keywords: 关键词及权重列表，格式为 [("关键词1", 权重1), ...]
        min_score: 最小评分阈值（默认 0.0）
        max_results: 最大结果数（默认 100）

    Returns:
        RSS 榜单数据
    """
    try:
        result = _rss_client.connect()
        if not result.success:
            return {
                "success": False,
                "error": result.error.message if result.error else "连接失败",
            }

        ranking_result = _rss_client.create_ranking(
            feed_urls, keywords, min_score, max_results
        )
        _rss_client.disconnect()

        if not ranking_result.success:
            return {
                "success": False,
                "error": (
                    ranking_result.error.message
                    if ranking_result.error
                    else "创建榜单失败"
                ),
            }

        return {"success": True, "ranking": ranking_result.data}

    except Exception as e:
        return {"success": False, "error": f"创建榜单失败: {str(e)}"}


async def get_template_info(template_name: str) -> Dict[str, Any]:
    """
    获取模板详细信息

    Args:
        template_name: 模板名称

    Returns:
        模板信息
    """
    try:
        result = _rss_client.connect()
        if not result.success:
            return {
                "success": False,
                "error": result.error.message if result.error else "连接失败",
            }

        info_result = _rss_client.get_template_info(template_name)
        _rss_client.disconnect()

        if not info_result.success:
            return {
                "success": False,
                "error": (
                    info_result.error.message
                    if info_result.error
                    else "获取模板信息失败"
                ),
            }

        return {"success": True, "info": info_result.data}

    except Exception as e:
        return {"success": False, "error": f"获取模板信息失败: {str(e)}"}


async def get_rss_info() -> Dict[str, Any]:
    """
    获取 RSS 客户端信息

    Returns:
        客户端信息
    """
    try:
        result = _rss_client.connect()
        if not result.success:
            return {
                "success": False,
                "error": result.error.message if result.error else "连接失败",
            }

        info_result = _rss_client.get_info()
        _rss_client.disconnect()

        if not info_result.success:
            return {
                "success": False,
                "error": (
                    info_result.error.message if info_result.error else "获取信息失败"
                ),
            }

        return {"success": True, "info": info_result.data}

    except Exception as e:
        return {"success": False, "error": f"获取客户端信息失败: {str(e)}"}


def register_tools(mcp: FastMCP) -> None:
    """注册 RSS 工具到 MCP 服务器"""
    mcp.tool()(fetch_feed)
    mcp.tool()(parse_feed)
    mcp.tool()(list_templates)
    mcp.tool()(add_from_template)
    mcp.tool()(create_ranking)
    mcp.tool()(get_template_info)
    mcp.tool()(get_rss_info)
