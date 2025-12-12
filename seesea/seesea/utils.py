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
- site:github.com: 站点过滤

性能特性:
- 高效的字符串处理
- 避免不必要的对象创建
- 支持批量处理

与其他模块的关系:
- 与 search 模块紧密协作，用于结果格式化
- 与 api 模块集成，用于查询解析
- 支持 search_types 模块的类型安全结果
"""

from typing import Dict, List, Any, Union
from seesea.search_types import SearchResultItem

# 类型别名
ResultsList = List[Union[SearchResultItem, Dict[str, Any]]]
FormattedResults = List[Dict[str, Any]]
QueryDict = Dict[str, Any]


def format_results(
    results: Union[List[SearchResultItem], ResultsList],
    max_description_length: int = 200,
) -> FormattedResults:
    """
    格式化搜索结果

    Args:
        results: 原始结果列表 (SearchResultItem 对象或字典)
        max_description_length: 描述最大长度

    Returns:
        格式化后的结果列表
    """
    formatted = []
    for item in results:
        if isinstance(item, SearchResultItem):
            # 处理 SearchResultItem 对象
            formatted_item = {
                "title": item.title,
                "url": item.url,
                "description": item.content[:max_description_length],
                "score": item.score,
            }
        else:
            # 处理字典对象 (向后兼容)
            formatted_item = {
                "title": item.get("title", ""),
                "url": item.get("url", ""),
                "description": item.get("content", "")[:max_description_length],
                "score": item.get("score", 0.0),
            }
        formatted.append(formatted_item)
    return formatted


def parse_query(query: str) -> QueryDict:
    """
    解析查询字符串

    Args:
        query: 查询字符串

    Returns:
        解析后的查询参数
    """
    params = {"query": query.strip()}

    # 支持简单的过滤语法
    # 例如: "python lang:en site:github.com"
    parts = query.split()
    filters = {}
    clean_query = []

    for part in parts:
        if ":" in part:
            key, value = part.split(":", 1)
            if key in ["lang", "language"]:
                filters["language"] = value
            elif key == "site":
                filters["site"] = value
        else:
            clean_query.append(part)

    params["query"] = " ".join(clean_query)
    params.update(filters)

    return params
