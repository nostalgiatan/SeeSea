"""
SeeSea MCP Cleaner 工具

直接定义数据清洗相关的 MCP 工具。
"""

from typing import List, Dict, Any
from fastmcp import FastMCP
from ..sdk.cleaner.client import CleanerClient


async def clean_text(text: str) -> Dict[str, Any]:
    """
    清洗文本

    Args:
        text: 原始文本

    Returns:
        清洗后的文本
    """
    try:
        client = CleanerClient()
        result = client.clean_text(text)
        if not result.success:
            return {
                "success": False,
                "error": result.error.message if result.error else "清洗失败",
            }

        return {"success": True, "cleaned_text": result.data}

    except Exception as e:
        return {"success": False, "error": f"清洗文本失败: {str(e)}"}


async def remove_html(html: str) -> Dict[str, Any]:
    """
    移除HTML标签

    Args:
        html: HTML文本

    Returns:
        纯文本
    """
    try:
        client = CleanerClient()
        result = client.remove_html(html)
        if not result.success:
            return {
                "success": False,
                "error": result.error.message if result.error else "移除HTML失败",
            }

        return {"success": True, "text": result.data}

    except Exception as e:
        return {"success": False, "error": f"移除HTML失败: {str(e)}"}


async def normalize_text(text: str) -> Dict[str, Any]:
    """
    标准化文本

    Args:
        text: 原始文本

    Returns:
        标准化后的文本
    """
    try:
        client = CleanerClient()
        result = client.normalize_text(text)
        if not result.success:
            return {
                "success": False,
                "error": result.error.message if result.error else "标准化失败",
            }

        return {"success": True, "normalized_text": result.data}

    except Exception as e:
        return {"success": False, "error": f"标准化文本失败: {str(e)}"}


async def extract_urls(text: str) -> Dict[str, Any]:
    """
    提取URL

    Args:
        text: 原始文本

    Returns:
        URL列表
    """
    try:
        client = CleanerClient()
        result = client.extract_urls(text)
        if not result.success:
            return {
                "success": False,
                "error": result.error.message if result.error else "提取URL失败",
            }

        return {"success": True, "urls": result.data}

    except Exception as e:
        return {"success": False, "error": f"提取URL失败: {str(e)}"}


async def clean_batch(texts: List[str]) -> Dict[str, Any]:
    """
    批量清洗文本

    Args:
        texts: 文本列表

    Returns:
        清洗后的文本列表
    """
    try:
        client = CleanerClient()
        result = client.clean_batch(texts)
        if not result.success:
            return {
                "success": False,
                "error": result.error.message if result.error else "批量清洗失败",
            }

        return {"success": True, "cleaned_texts": result.data}

    except Exception as e:
        return {"success": False, "error": f"批量清洗失败: {str(e)}"}


def register_tools(mcp: FastMCP) -> None:
    """注册清洗工具到 MCP 服务器"""
    mcp.tool()(clean_text)
    mcp.tool()(remove_html)
    mcp.tool()(normalize_text)
    mcp.tool()(extract_urls)
    mcp.tool()(clean_batch)
