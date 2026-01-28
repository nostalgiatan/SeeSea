"""
SeeSea MCP Search 工具

直接定义搜索相关的 MCP 工具。
"""

from typing import Optional, List, Dict, Any
from fastmcp import FastMCP
from ..sdk.search import SearchClient

# 初始化搜索客户端
_search_client = SearchClient()


async def search(
    query: str,
    page: int = 1,
    page_size: int = 10,
    language: Optional[str] = None,
    region: Optional[str] = None,
    engines: Optional[List[str]] = None,
    engine_type: Optional[str] = None,
    force: bool = False,
    include_deepweb: bool = False,
    cache_timeline: Optional[int] = None,
) -> Dict[str, Any]:
    """
    执行搜索查询

    Args:
        query: 搜索关键词
        page: 页码，从1开始
        page_size: 每页结果数量
        language: 语言代码 (如: zh, en)
        region: 地区代码 (如: cn, us)
        engines: 指定使用的搜索引擎列表
        engine_type: 引擎类型 (text/image/video)
        force: 是否强制刷新缓存
        include_deepweb: 是否包含深网内容
        cache_timeline: 缓存刷新时间线（秒）

    Returns:
        搜索结果，包含 items, total, page 等信息
    """
    try:
        result = _search_client.connect()
        if not result.success:
            return {
                "success": False,
                "error": result.error.message if result.error else "连接失败",
            }

        search_result = _search_client.search(
            query=query,
            page=page,
            page_size=page_size,
            language=language,
            region=region,
            engines=engines,
            force=force,
            include_deepweb=include_deepweb,
            engine_type=engine_type,
            cache_timeline=cache_timeline,
        )

        _search_client.disconnect()

        if not search_result.success:
            return {
                "success": False,
                "error": (
                    search_result.error.message if search_result.error else "搜索失败"
                ),
            }

        data = search_result.data
        if data is None:
            return {"success": False, "error": "搜索返回空数据"}

        return {
            "success": True,
            "query": data.query,
            "total": data.total,
            "page": data.page,
            "per_page": data.per_page,
            "has_more": data.has_more,
            "engines_used": data.engines_used,
            "took_ms": data.took_ms,
            "items": [
                {
                    "title": item.title,
                    "url": item.url,
                    "content": item.content,
                    "snippet": item.snippet,
                    "thumbnail": item.thumbnail,
                    "source": item.engine,
                    "score": item.score,
                    "timestamp": item.timestamp.isoformat() if item.timestamp else None,
                    "metadata": item.metadata,
                }
                for item in data.items
            ],
        }

    except Exception as e:
        return {"success": False, "error": f"搜索过程中发生错误: {str(e)}"}


async def search_images(
    query: str,
    page: int = 1,
    page_size: int = 10,
    language: Optional[str] = None,
) -> Dict[str, Any]:
    """
    搜索图片

    Args:
        query: 搜索关键词
        page: 页码，从1开始
        page_size: 每页结果数量
        language: 语言代码 (如: zh, en)

    Returns:
        图片搜索结果
    """
    return await search(
        query=query,
        page=page,
        page_size=page_size,
        language=language,
        engine_type="image",
    )


async def search_videos(
    query: str,
    page: int = 1,
    page_size: int = 10,
    language: Optional[str] = None,
) -> Dict[str, Any]:
    """
    搜索视频

    Args:
        query: 搜索关键词
        page: 页码，从1开始
        page_size: 每页结果数量
        language: 语言代码 (如: zh, en)

    Returns:
        视频搜索结果
    """
    return await search(
        query=query,
        page=page,
        page_size=page_size,
        language=language,
        engine_type="video",
    )


async def list_engines() -> Dict[str, Any]:
    """
    获取可用的搜索引擎列表

    Returns:
        搜索引擎列表和健康状态
    """
    try:
        result = _search_client.connect()
        if not result.success:
            return {
                "success": False,
                "error": result.error.message if result.error else "连接失败",
            }

        health_result = _search_client.health_check()
        _search_client.disconnect()

        if not health_result.success:
            return {
                "success": False,
                "error": (
                    health_result.error.message
                    if health_result.error
                    else "获取健康状态失败"
                ),
            }

        health_data = health_result.data
        if health_data is None:
            return {"success": False, "error": "健康状态返回空数据"}

        return {
            "success": True,
            "engines": health_data.get("engines_health", {}),
            "total": health_data.get("total_engines", 0),
            "healthy": health_data.get("healthy_engines", 0),
        }

    except Exception as e:
        return {"success": False, "error": f"获取引擎列表失败: {str(e)}"}


async def get_search_info() -> Dict[str, Any]:
    """
    获取搜索客户端信息

    Returns:
        客户端信息
    """
    try:
        result = _search_client.connect()
        if not result.success:
            return {
                "success": False,
                "error": result.error.message if result.error else "连接失败",
            }

        info_result = _search_client.get_info()
        _search_client.disconnect()

        if not info_result.success:
            return {
                "success": False,
                "error": (
                    info_result.error.message if info_result.error else "获取信息失败"
                ),
            }

        return {
            "success": True,
            "info": info_result.data,
        }

    except Exception as e:
        return {
            "success": False,
            "error": f"获取客户端信息失败: {str(e)}",
        }


async def clear_cache() -> Dict[str, Any]:
    """
    清除所有搜索缓存

    Returns:
        操作结果
    """
    try:
        result = _search_client.connect()
        if not result.success:
            return {
                "success": False,
                "error": result.error.message if result.error else "连接失败",
            }

        clear_result = _search_client.clear_cache()
        _search_client.disconnect()

        if not clear_result.success:
            return {
                "success": False,
                "error": (
                    clear_result.error.message if clear_result.error else "清除缓存失败"
                ),
            }

        return {"success": True, "message": "缓存已清除"}

    except Exception as e:
        return {"success": False, "error": f"清除缓存失败: {str(e)}"}


async def get_stats() -> Dict[str, Any]:
    """
    获取搜索统计信息

    Returns:
        统计信息，包括缓存命中率等
    """
    try:
        result = _search_client.connect()
        if not result.success:
            return {
                "success": False,
                "error": result.error.message if result.error else "连接失败",
            }

        stats_result = _search_client.get_stats()
        _search_client.disconnect()

        if not stats_result.success:
            return {
                "success": False,
                "error": (
                    stats_result.error.message
                    if stats_result.error
                    else "获取统计信息失败"
                ),
            }

        return {"success": True, "stats": stats_result.data}

    except Exception as e:
        return {"success": False, "error": f"获取统计信息失败: {str(e)}"}


def register_tools(mcp: FastMCP) -> None:
    """注册搜索工具到 MCP 服务器"""
    mcp.tool()(search)
    mcp.tool()(search_images)
    mcp.tool()(search_videos)
    mcp.tool()(list_engines)
    mcp.tool()(get_search_info)
    mcp.tool()(clear_cache)
    mcp.tool()(get_stats)
