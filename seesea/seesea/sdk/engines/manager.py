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
SeeSea 搜索引擎管理器

基于raming系统的搜索引擎事件驱动管理。
使用共享内存交换数据，事件系统进行通知。
"""

import json
import uuid
from typing import Dict, Any, Optional, List, Callable

try:
    from seesea_core import (
        raming_write_memory,
        raming_read_memory,
        raming_create_memory_region,
        raming_delete_memory_region,
        raming_publish_event,
        raming_subscribe_event,
        raming_unsubscribe_event,
        raming_get_system_status,
    )

    _RAMING_AVAILABLE = True
except ImportError:
    _RAMING_AVAILABLE = False


# 全局搜索引擎注册表
_SEARCH_ENGINES: Dict[str, Dict[str, Any]] = {}
_EVENT_SUBSCRIBERS: Dict[str, str] = {}  # subscriber_id -> event_type


def register_search_engine(
    engine_name: str,
    engine_type: str,
    description: str,
    search_function: Callable[[Dict[str, Any]], Dict[str, Any]],
) -> bool:
    """
    注册搜索引擎

    Args:
        engine_name: 引擎名称
        engine_type: 引擎类型（web, news, images, videos等）
        description: 引擎描述
        search_function: 搜索函数，接受查询参数字典，返回结果字典

    Returns:
        bool: 注册是否成功
    """
    if not _RAMING_AVAILABLE:
        print("Warning: Raming system not available, using fallback mode")
        _SEARCH_ENGINES[engine_name] = {
            "type": engine_type,
            "description": description,
            "search_function": search_function,
            "status": "registered",
        }
        return True

    try:
        # 创建引擎专用的共享内存区域
        request_region = f"search_engine_{engine_name}_request"
        response_region = f"search_engine_{engine_name}_response"

        # 创建内存区域（1MB each）
        raming_create_memory_region(request_region, 1024 * 1024)
        raming_create_memory_region(response_region, 1024 * 1024)

        # 注册引擎信息
        _SEARCH_ENGINES[engine_name] = {
            "type": engine_type,
            "description": description,
            "search_function": search_function,
            "request_region": request_region,
            "response_region": response_region,
            "status": "registered",
        }

        # 订阅搜索请求事件
        subscriber_id = f"engine_{engine_name}_{uuid.uuid4().hex[:8]}"

        def handle_search_request():
            """处理搜索请求的回调函数（无参数）"""
            try:
                # 从共享内存读取请求数据
                request_data = raming_read_memory(request_region)
                if request_data:
                    # 解析请求
                    request_json = json.loads(request_data.decode("utf-8"))

                    # 调用搜索函数
                    result = search_function(request_json)

                    # 写入响应到共享内存
                    response_json = json.dumps(result, ensure_ascii=False)
                    raming_write_memory(response_region, response_json.encode("utf-8"))

                    # 发布响应事件
                    raming_publish_event("search_response", response_region)

            except Exception as e:
                print(f"Error handling search request for {engine_name}: {e}")

        # 订阅搜索请求事件
        success = raming_subscribe_event(
            "search_request", subscriber_id, handle_search_request
        )

        if success:
            _EVENT_SUBSCRIBERS[subscriber_id] = "search_request"
            _SEARCH_ENGINES[engine_name]["subscriber_id"] = subscriber_id
            print(f"✅ 搜索引擎 '{engine_name}' 注册成功")
            return True
        else:
            print(f"❌ 搜索引擎 '{engine_name}' 事件订阅失败")
            return False

    except Exception as e:
        print(f"❌ 搜索引擎 '{engine_name}' 注册失败: {e}")
        return False


def unregister_search_engine(engine_name: str) -> bool:
    """
    注销搜索引擎

    Args:
        engine_name: 引擎名称

    Returns:
        bool: 注销是否成功
    """
    if engine_name not in _SEARCH_ENGINES:
        return False

    engine_info = _SEARCH_ENGINES[engine_name]

    if _RAMING_AVAILABLE:
        try:
            # 取消事件订阅
            if "subscriber_id" in engine_info:
                subscriber_id = engine_info["subscriber_id"]
                raming_unsubscribe_event("search_request", subscriber_id)
                if subscriber_id in _EVENT_SUBSCRIBERS:
                    del _EVENT_SUBSCRIBERS[subscriber_id]

            # 删除共享内存区域
            if "request_region" in engine_info:
                raming_delete_memory_region(engine_info["request_region"])
            if "response_region" in engine_info:
                raming_delete_memory_region(engine_info["response_region"])

        except Exception as e:
            print(f"Warning: Error during cleanup for {engine_name}: {e}")

    # 从注册表中移除
    del _SEARCH_ENGINES[engine_name]
    print(f"✅ 搜索引擎 '{engine_name}' 注销成功")
    return True


def list_search_engines() -> List[str]:
    """
    列出已注册的搜索引擎

    Returns:
        List[str]: 引擎名称列表
    """
    return list(_SEARCH_ENGINES.keys())


def has_search_engine(engine_name: str) -> bool:
    """
    检查搜索引擎是否已注册

    Args:
        engine_name: 引擎名称

    Returns:
        bool: 是否已注册
    """
    return engine_name in _SEARCH_ENGINES


def send_search_request(
    engine_name: str, query: str, page: int = 1, page_size: int = 10, **kwargs
) -> str:
    """
    发送搜索请求

    Args:
        engine_name: 目标引擎名称
        query: 搜索查询
        page: 页数
        page_size: 每页结果数
        **kwargs: 其他参数

    Returns:
        str: 请求ID
    """
    if not _RAMING_AVAILABLE:
        raise RuntimeError("Raming system not available")

    if engine_name not in _SEARCH_ENGINES:
        raise ValueError(f"Search engine '{engine_name}' not registered")

    engine_info = _SEARCH_ENGINES[engine_name]
    request_region = engine_info["request_region"]

    # 构建请求数据
    request_id = uuid.uuid4().hex
    request_data = {
        "request_id": request_id,
        "engine_name": engine_name,
        "query": query,
        "page": page,
        "page_size": page_size,
        **kwargs,
    }

    # 写入共享内存
    request_json = json.dumps(request_data, ensure_ascii=False)
    raming_write_memory(request_region, request_json.encode("utf-8"))

    # 发布搜索请求事件
    raming_publish_event("search_request", request_region)

    return request_id


def read_search_response(engine_name: str) -> Optional[Dict[str, Any]]:
    """
    读取搜索响应

    Args:
        engine_name: 引擎名称

    Returns:
        Optional[Dict[str, Any]]: 响应数据，如果没有则返回None
    """
    if not _RAMING_AVAILABLE:
        return None

    if engine_name not in _SEARCH_ENGINES:
        return None

    engine_info = _SEARCH_ENGINES[engine_name]
    response_region = engine_info["response_region"]

    try:
        # 从共享内存读取响应
        response_data = raming_read_memory(response_region)
        if response_data:
            result: Dict[str, Any] = json.loads(response_data.decode("utf-8"))
            return result
        return None

    except Exception as e:
        print(f"Error reading search response from {engine_name}: {e}")
        return None


def subscribe_search_responses(callback: Callable[[], None]) -> str:
    """
    订阅搜索响应事件

    Args:
        callback: 回调函数（无参数），当有搜索响应时被调用

    Returns:
        str: 订阅者ID
    """
    if not _RAMING_AVAILABLE:
        raise RuntimeError("Raming system not available")

    subscriber_id = f"search_response_subscriber_{uuid.uuid4().hex[:8]}"

    success = raming_subscribe_event("search_response", subscriber_id, callback)

    if success:
        _EVENT_SUBSCRIBERS[subscriber_id] = "search_response"
        return subscriber_id
    else:
        raise RuntimeError("Failed to subscribe to search response events")


def unsubscribe_search_events(subscriber_id: str) -> bool:
    """
    取消订阅搜索事件

    Args:
        subscriber_id: 订阅者ID

    Returns:
        bool: 是否成功
    """
    if not _RAMING_AVAILABLE:
        return False

    if subscriber_id not in _EVENT_SUBSCRIBERS:
        return False

    event_type = _EVENT_SUBSCRIBERS[subscriber_id]
    success: bool = bool(raming_unsubscribe_event(event_type, subscriber_id))

    if success:
        del _EVENT_SUBSCRIBERS[subscriber_id]

    return success


def get_search_engine_status() -> Dict[str, Any]:
    """
    获取搜索引擎系统状态

    Returns:
        Dict[str, Any]: 状态信息
    """
    status = {
        "engines_count": len(_SEARCH_ENGINES),
        "registered_engines": list(_SEARCH_ENGINES.keys()),
        "active_subscribers": len(_EVENT_SUBSCRIBERS),
        "raming_available": _RAMING_AVAILABLE,
    }

    if _RAMING_AVAILABLE:
        try:
            raming_status = raming_get_system_status()
            status["raming_status"] = raming_status
        except Exception as e:
            status["raming_error"] = str(e)

    return status
