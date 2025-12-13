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
SeeSea Utilities - 工具函数

提供 SeeSea 搜索引擎的通用工具函数，包括结果格式化和查询解析功能。

主要功能:
- 搜索结果格式化
- 查询字符串解析
- 支持简单的过滤语法
- 结果描述长度控制

设计原则:
- 向后兼容: 支持旧版本的字典结果和新版本的 SearchResultItem 对象
- 易用性: 简洁的函数接口，默认参数合理
- 高性能: 高效的字符串处理，避免不必要的拷贝
- 类型安全: 支持类型化输入和输出

使用示例:
    >>> from seesea import format_results, parse_query
    >>>
    >>> # 格式化结果
    >>> results = [SearchResultItem(title="Test", url="https://example.com", content="This is a test", score=0.8)]
    >>> formatted = format_results(results, max_description_length=100)
    >>> print(formatted[0]['title'])  # 输出: Test
    >>> print(formatted[0]['description'])  # 输出: This is a test
    >>>
    >>> # 解析查询
    >>> query = "python lang:en site:github.com"
    >>> parsed = parse_query(query)
    >>> print(parsed['query'])  # 输出: python
    >>> print(parsed['language'])  # 输出: en
    >>> print(parsed['site'])  # 输出: github.com

支持的查询过滤语法:
- lang:en 或 language:en: 语言过滤
- site:example.com: 站点过滤
- type:news 或 filetype:pdf: 内容类型过滤
- time:week 或 time:month: 时间过滤
- region:us: 地区过滤

模块向前兼容保证:
- 支持旧版本 SearchResult 字典格式输入
- 支持新版本 SearchResultItem 对象格式输入
"""

import re
from typing import Dict, List, Union, Any, TYPE_CHECKING

# 类型定义
if TYPE_CHECKING:
    from ..search_types import SearchResultItem  # type: ignore

    ResultsList = List["SearchResultItem"]  # type: ignore
else:
    SearchResultItem = Any
    ResultsList = Any

FormattedResult = Dict[str, Any]
FormattedResults = List[FormattedResult]
QueryDict = Dict[str, Any]


def format_results(
    results: Union[List[SearchResultItem], ResultsList],
    max_description_length: int = 200,
) -> FormattedResults:
    """
    格式化搜索结果

    Args:
        results: 搜索结果列表，可以是 SearchResultItem 对象列表或字典列表
        max_description_length: 描述最大长度，超过将被截断

    Returns:
        格式化后的结果字典列表

    Examples:
        >>> results = [SearchResultItem(title="Test", url="https://example.com", content="Test content")]
        >>> formatted = format_results(results, max_description_length=50)
        >>> print(formatted[0]['title'])
        Test
    """
    formatted = []

    # 统一处理不同类型的输入
    if hasattr(results, "results"):
        # ResultsList 对象
        items = results.results
    else:
        # List[SearchResultItem] 或 List[Dict]
        items = results

    for item in items:
        if isinstance(item, dict):
            # 字典格式 - 向后兼容
            formatted_item = _format_dict_result(item, max_description_length)
        else:
            # SearchResultItem 对象格式
            formatted_item = _format_object_result(item, max_description_length)

        formatted.append(formatted_item)

    return formatted


def parse_query(query: str) -> QueryDict:
    """
    解析查询字符串，提取过滤条件

    Args:
        query: 原始查询字符串

    Returns:
        解析后的查询字典，包含 query 和各种过滤条件

    Examples:
        >>> parsed = parse_query("python lang:en site:github.com")
        >>> print(parsed['query'])
        python
        >>> print(parsed['language'])
        en
        >>> print(parsed['site'])
        github.com
    """
    # 默认结果
    parsed: QueryDict = {
        "query": query,
        "language": None,
        "site": None,
        "filetype": None,
        "time": None,
        "region": None,
    }

    # 提取过滤条件的正则表达式
    filters = [
        (r"lang(?:uage)?:(\w+)", "language"),
        (r"site:([^\s]+)", "site"),
        (r"(?:type|filetype):(\w+)", "filetype"),
        (r"time:(\w+)", "time"),
        (r"region:(\w+)", "region"),
    ]

    clean_query = query
    for pattern, key in filters:
        match = re.search(pattern, query, re.IGNORECASE)
        if match:
            parsed[key] = match.group(1)
            # 从查询中移除过滤条件
            clean_query = re.sub(pattern, "", clean_query, flags=re.IGNORECASE)

    # 清理查询字符串
    parsed["query"] = " ".join(clean_query.split())

    return parsed


def _format_dict_result(item: Dict[str, Any], max_length: int) -> FormattedResult:
    """格式化字典格式的结果项"""
    return {
        "title": item.get("title", "无标题"),
        "url": item.get("url", ""),
        "description": _truncate_text(
            item.get("content", item.get("description", "")), max_length
        ),
        "score": item.get("score", 0.0),
        "source": item.get("source", "未知"),
        "timestamp": item.get("timestamp"),
    }


def _format_object_result(item: SearchResultItem, max_length: int) -> FormattedResult:
    """格式化 SearchResultItem 对象格式的结果项"""
    return {
        "title": getattr(item, "title", "无标题"),
        "url": getattr(item, "url", ""),
        "description": _truncate_text(
            getattr(item, "content", getattr(item, "description", "")), max_length
        ),
        "score": getattr(item, "score", 0.0),
        "source": getattr(item, "source", "未知"),
        "timestamp": getattr(item, "timestamp", None),
    }


def _truncate_text(text: str, max_length: int) -> str:
    """
    截断文本到指定长度

    Args:
        text: 原始文本
        max_length: 最大长度

    Returns:
        截断后的文本，如果被截断会添加省略号
    """
    if not text or max_length <= 0:
        return ""

    if len(text) <= max_length:
        return text

    # 尝试在单词边界截断
    truncated = text[: max_length - 3]  # 为省略号留出空间

    # 寻找最后一个空格，避免截断单词
    last_space = truncated.rfind(" ")
    if last_space > max_length * 0.8:  # 如果空格位置合理
        truncated = truncated[:last_space]

    return truncated + "..."
