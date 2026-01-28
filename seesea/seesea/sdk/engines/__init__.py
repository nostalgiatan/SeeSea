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
SeeSea 搜索引擎系统

基于事件系统和raming系统的搜索引擎通信接口。
替换原有的回调机制，完全基于事件驱动架构。

提供完整的引擎基础模型，支持自动注册机制。
"""

from .manager import (
    register_search_engine,
    unregister_search_engine,
    list_search_engines,
    has_search_engine,
    send_search_request,
    read_search_response,
    subscribe_search_responses,
    unsubscribe_search_events,
    get_search_engine_status,
)

from .base import (
    BaseSearchEngine,
    SearchEngineMetaclass,
    register_engine_class,
)

from .registry import EngineRegistry

# 导入示例引擎（会自动注册）
from . import examples

__all__ = [
    # 搜索引擎管理函数（函数式API）
    "register_search_engine",
    "unregister_search_engine",
    "list_search_engines",
    "has_search_engine",
    # 搜索请求和响应函数
    "send_search_request",
    "read_search_response",
    "subscribe_search_responses",
    "unsubscribe_search_events",
    # 状态查询
    "get_search_engine_status",
    # 引擎基础模型
    "BaseSearchEngine",
    "SearchEngineMetaclass",
    "register_engine_class",
    # 引擎注册器
    "EngineRegistry",
    # 示例引擎模块
    "examples",
]
