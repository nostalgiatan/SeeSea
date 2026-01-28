"""
MCP 热点趋势工具

提供热点数据获取相关的 MCP 工具。
"""

from typing import Optional, Dict, Any
from fastmcp import FastMCP

from ..sdk.feed import HotTrendClient

# 全局客户端实例
_hot_client: Optional[HotTrendClient] = None


def get_hot_client() -> HotTrendClient:
    """获取或创建热点趋势客户端"""
    global _hot_client
    if _hot_client is None:
        _hot_client = HotTrendClient()
        _hot_client.connect()
    return _hot_client


async def fetch_hot_platform(platform_id: str) -> Dict[str, Any]:
    """
    获取单个平台的热点数据

    Args:
        platform_id: 平台ID，如 zhihu、weibo、bilibili 等

    Returns:
        dict: 包含平台信息和热点数据的字典
    """
    try:
        client = get_hot_client()
        result = client.fetch_platform(platform_id)

        if result.success:
            return {"success": True, "data": result.data, "took_ms": result.took_ms}
        else:
            return {
                "success": False,
                "error": {
                    "code": result.error.code if result.error else "UNKNOWN",
                    "message": (
                        result.error.message if result.error else "Unknown error"
                    ),
                },
            }
    except Exception as e:
        return {
            "success": False,
            "error": {
                "code": "FETCH_HOT_PLATFORM_ERROR",
                "message": f"获取平台热点失败: {str(e)}",
            },
        }


async def fetch_all_hot_platforms() -> Dict[str, Any]:
    """
    获取所有支持平台的热点数据

    Returns:
        dict: 包含所有平台热点数据的字典
    """
    try:
        client = get_hot_client()
        result = client.fetch_all_platforms()

        if result.success:
            return {
                "success": True,
                "data": result.data,
                "platforms_count": len(result.data) if result.data else 0,
                "took_ms": result.took_ms,
            }
        else:
            return {
                "success": False,
                "error": {
                    "code": result.error.code if result.error else "UNKNOWN",
                    "message": (
                        result.error.message if result.error else "Unknown error"
                    ),
                },
            }
    except Exception as e:
        return {
            "success": False,
            "error": {
                "code": "FETCH_ALL_HOT_PLATFORMS_ERROR",
                "message": f"获取所有平台热点失败: {str(e)}",
            },
        }


async def fetch_multiple_hot_platforms(platform_ids: str) -> Dict[str, Any]:
    """
    批量获取多个平台的热点数据

    Args:
        platform_ids: 平台ID列表，逗号分隔，如 "zhihu,weibo,bilibili"

    Returns:
        dict: 包含多个平台热点数据的字典
    """
    try:
        if not platform_ids:
            return {
                "success": False,
                "error": {
                    "code": "INVALID_PARAMETERS",
                    "message": "platform_ids 不能为空",
                },
            }

        # 解析平台ID列表
        pid_list = [pid.strip() for pid in platform_ids.split(",") if pid.strip()]

        if not pid_list:
            return {
                "success": False,
                "error": {
                    "code": "INVALID_PARAMETERS",
                    "message": "platform_ids 解析后为空",
                },
            }

        client = get_hot_client()
        result = client.fetch_multiple_platforms(pid_list)

        if result.success:
            return {
                "success": True,
                "data": result.data,
                "platforms_count": len(result.data) if result.data else 0,
                "took_ms": result.took_ms,
            }
        else:
            return {
                "success": False,
                "error": {
                    "code": result.error.code if result.error else "UNKNOWN",
                    "message": (
                        result.error.message if result.error else "Unknown error"
                    ),
                },
            }
    except Exception as e:
        return {
            "success": False,
            "error": {
                "code": "FETCH_MULTIPLE_HOT_PLATFORMS_ERROR",
                "message": f"批量获取平台热点失败: {str(e)}",
            },
        }


async def list_hot_platforms() -> Dict[str, Any]:
    """
    获取所有支持的平台列表

    Returns:
        dict: 包含平台ID到平台名称映射的字典
    """
    try:
        client = get_hot_client()
        result = client.list_platforms()

        if result.success:
            return {
                "success": True,
                "data": result.data,
                "platforms_count": len(result.data) if result.data else 0,
            }
        else:
            return {
                "success": False,
                "error": {
                    "code": result.error.code if result.error else "UNKNOWN",
                    "message": (
                        result.error.message if result.error else "Unknown error"
                    ),
                },
            }
    except Exception as e:
        return {
            "success": False,
            "error": {
                "code": "LIST_HOT_PLATFORMS_ERROR",
                "message": f"获取平台列表失败: {str(e)}",
            },
        }


async def search_hot_platforms(query: str) -> Dict[str, Any]:
    """
    根据查询字符串搜索平台

    Args:
        query: 查询字符串，可以匹配平台ID或平台名称

    Returns:
        dict: 匹配的平台列表
    """
    try:
        if not query:
            return {
                "success": False,
                "error": {"code": "INVALID_PARAMETERS", "message": "query 不能为空"},
            }

        client = get_hot_client()
        result = client.search_platforms(query)

        if result.success:
            return {
                "success": True,
                "data": result.data,
                "match_count": len(result.data) if result.data else 0,
            }
        else:
            return {
                "success": False,
                "error": {
                    "code": result.error.code if result.error else "UNKNOWN",
                    "message": (
                        result.error.message if result.error else "Unknown error"
                    ),
                },
            }
    except Exception as e:
        return {
            "success": False,
            "error": {
                "code": "SEARCH_HOT_PLATFORMS_ERROR",
                "message": f"搜索平台失败: {str(e)}",
            },
        }


async def get_hot_client_info() -> Dict[str, Any]:
    """
    获取热点趋势客户端信息

    Returns:
        dict: 客户端信息
    """
    try:
        client = get_hot_client()
        result = client.get_info()

        if result.success:
            return {"success": True, "info": result.data}
        else:
            return {
                "success": False,
                "error": {
                    "code": result.error.code if result.error else "UNKNOWN",
                    "message": (
                        result.error.message if result.error else "Unknown error"
                    ),
                },
            }
    except Exception as e:
        return {
            "success": False,
            "error": {
                "code": "GET_HOT_CLIENT_INFO_ERROR",
                "message": f"获取客户端信息失败: {str(e)}",
            },
        }


def register_hot_tools(mcp: FastMCP) -> None:
    """注册热点趋势工具到 MCP 服务器"""
    mcp.tool()(fetch_hot_platform)
    mcp.tool()(fetch_all_hot_platforms)
    mcp.tool()(fetch_multiple_hot_platforms)
    mcp.tool()(list_hot_platforms)
    mcp.tool()(search_hot_platforms)
    mcp.tool()(get_hot_client_info)
