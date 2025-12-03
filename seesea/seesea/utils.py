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
"""

from typing import Dict, List, Any, Union
from seesea.search_types import SearchResultItem


def format_results(
    results: List[Union[SearchResultItem, Dict[str, Any]]], max_description_length: int = 200
) -> List[Dict[str, Any]]:
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


def parse_query(query: str) -> Dict[str, Any]:
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
