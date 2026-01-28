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
SeeSea 搜索引擎基础模型

提供自动注册机制和标准接口的引擎基础模型。
基于raming系统实现自动注册和事件通信。
"""

import json
import uuid
import time
from abc import ABC, abstractmethod, ABCMeta
from typing import Dict, Any, List, Optional, Type

try:
    from seesea_core import (
        raming_write_memory,
        raming_create_memory_region,
        raming_publish_event,
    )

    _RAMING_AVAILABLE = True
except ImportError:
    _RAMING_AVAILABLE = False


class SearchEngineMetaclass(ABCMeta):
    """搜索引擎元类，实现延迟注册功能"""

    def __new__(cls, name, bases, attrs, **kwargs):
        # 创建新的引擎类
        engine_class = super().__new__(cls, name, bases, attrs)

        # 只记录非抽象引擎类，不立即注册
        if not getattr(engine_class, "_is_abstract", False) and hasattr(
            engine_class, "engine_name"
        ):
            # 延迟注册：只添加到待注册列表
            if hasattr(engine_class, "engine_name") and engine_class.engine_name:
                BaseSearchEngine._pending_engines.append(engine_class)

        return engine_class


class BaseSearchEngine(ABC, metaclass=SearchEngineMetaclass):
    """
    搜索引擎基础模型

    继承此类的引擎将被自动注册到系统中。
    所有引擎必须实现search方法。
    """

    _is_abstract = True  # 标记基类为抽象，不会被自动注册
    _registered_engines: Dict[str, Type["BaseSearchEngine"]] = {}
    _pending_engines: List[Type["BaseSearchEngine"]] = []  # 待注册引擎列表

    # 引擎基础信息（子类必须设置）
    engine_name: str = ""
    engine_type: str = "web"
    description: str = ""
    version: str = "1.0.0"
    author: str = ""

    # 引擎能力配置
    supports_pagination: bool = True
    supports_language_filter: bool = False
    supports_region_filter: bool = False
    supports_time_range: bool = False
    max_page_size: int = 50
    default_page_size: int = 10

    # 引擎状态
    _is_available: bool = True
    _last_error: Optional[str] = None
    _request_count: int = 0
    _success_count: int = 0

    def __init__(self):
        """初始化引擎实例"""
        if not self.engine_name:
            raise ValueError("engine_name must be set in subclass")

        self._instance_id = uuid.uuid4().hex
        self._created_at = time.time()
        self._setup_search_listener()

    def _setup_search_listener(self):
        """设置搜索请求监听器"""
        if not _RAMING_AVAILABLE:
            return

        try:
            # 订阅搜索请求事件
            subscriber_id = f"engine_{self.engine_name}_{self._instance_id}"

            def handle_search_request():
                """处理搜索请求的回调函数（无参数）"""
                try:
                    self._process_pending_search_requests()
                except Exception as e:
                    print(f"Error handling search request for {self.engine_name}: {e}")

            from seesea_core import raming_subscribe_event

            success = raming_subscribe_event(
                "search_request", subscriber_id, handle_search_request
            )

            if success:
                self._subscriber_id = subscriber_id
                print(f"✅ {self.engine_name} 已订阅搜索请求事件")
            else:
                print(f"❌ {self.engine_name} 搜索请求事件订阅失败")

        except Exception as e:
            print(
                f"Warning: Failed to setup search listener for {self.engine_name}: {e}"
            )

    def _process_pending_search_requests(self):
        """处理待处理的搜索请求"""
        if not _RAMING_AVAILABLE:
            return

        try:
            from seesea_core import raming_list_memory_regions

            # 扫描所有搜索请求内存区域
            regions = raming_list_memory_regions()
            for region_name in regions:
                if region_name.startswith("search_request_"):
                    self._handle_single_search_request(region_name)

        except Exception as e:
            print(f"Error processing search requests: {e}")

    def _handle_single_search_request(self, request_region: str):
        """处理单个搜索请求"""
        if not _RAMING_AVAILABLE:
            return

        try:
            from seesea_core import (
                raming_read_memory,
                raming_write_memory,
                raming_delete_memory_region,
                raming_publish_event,
            )

            # 读取搜索请求数据
            request_data = raming_read_memory(request_region)
            if not request_data:
                return

            request_json = json.loads(request_data.decode("utf-8"))

            # 检查是否是针对此引擎的请求
            if request_json.get("engine_name") != self.engine_name:
                return

            # 提取搜索参数
            query = request_json.get("query", "")
            page = request_json.get("page", 1)
            page_size = request_json.get("page_size", self.default_page_size)
            request_id = request_json.get("request_id", "")

            # 执行搜索
            start_time = time.time()
            try:
                search_result = self.search_with_validation(
                    query=query,
                    page=page,
                    page_size=page_size,
                    language=request_json.get("language"),
                    region=request_json.get("region"),
                )
            except Exception as e:
                search_result = {
                    "success": False,
                    "error": f"Search execution error: {e}",
                    "results": [],
                    "elapsed_ms": int((time.time() - start_time) * 1000),
                }

            # 构建响应内存区域名称
            response_region = f"search_response_{request_id}"

            # 写入搜索响应
            response_json = json.dumps(search_result, ensure_ascii=False)
            from seesea_core import raming_create_memory_region

            raming_create_memory_region(
                response_region, len(response_json.encode("utf-8")) + 1024
            )
            raming_write_memory(response_region, response_json.encode("utf-8"))

            # 发布搜索响应事件
            raming_publish_event("search_response")

            # 删除处理过的请求
            raming_delete_memory_region(request_region)

        except Exception as e:
            print(f"Error handling search request in {request_region}: {e}")

    @classmethod
    def force_register_all_engines(cls):
        """保险方法：强制注册所有待注册的引擎"""
        if not cls._pending_engines:
            print("No pending engines to register")
            return

        registered_count = 0
        for engine_class in cls._pending_engines[:]:  # 使用副本避免修改问题
            try:
                engine_class._auto_register()
                cls._pending_engines.remove(engine_class)
                registered_count += 1
            except Exception as e:
                print(f"Failed to register {engine_class.__name__}: {e}")

        print(f"✅ 强制注册了 {registered_count} 个引擎")

    @classmethod
    def get_pending_engines(cls):
        """获取待注册引擎列表"""
        return [
            f"{engine.__name__}({engine.engine_name})"
            for engine in cls._pending_engines
        ]

    @classmethod
    def _auto_register(cls):
        """自动注册引擎到系统"""
        if not cls.engine_name:
            print(
                f"Warning: {cls.__name__} does not have engine_name set, skipping registration"
            )
            return

        # 添加到本地注册表
        cls._registered_engines[cls.engine_name] = cls

        # 通知Rust端注册引擎
        cls._notify_rust_registration()

        print(f"✅ 自动注册搜索引擎: {cls.engine_name} ({cls.__name__})")

    @classmethod
    def _notify_rust_registration(cls):
        """通知Rust端引擎注册"""
        if not _RAMING_AVAILABLE:
            return

        try:
            # 尝试写入测试数据来检查 raming 是否已初始化
            # 使用 raming_write_memory 进行测试
            test_data = {"test": "init_check"}
            raming_write_memory("seesea_init_check", test_data, 1)
        except Exception:
            # raming系统未初始化，静默跳过
            return

        try:
            # 构建引擎信息
            engine_info = {
                "name": cls.engine_name,
                "type": cls.engine_type,
                "description": cls.description,
                "version": cls.version,
                "author": cls.author,
                "python_class": f"{cls.__module__}.{cls.__name__}",
                "py_engine": True,  # 标记为Python引擎
                "capabilities": {
                    "supports_pagination": cls.supports_pagination,
                    "supports_language_filter": cls.supports_language_filter,
                    "supports_region_filter": cls.supports_region_filter,
                    "supports_time_range": cls.supports_time_range,
                    "max_page_size": cls.max_page_size,
                    "default_page_size": cls.default_page_size,
                },
                "status": "active",
                "registered_at": time.time(),
            }

            # 使用统一的命名规范：engine_info_{engine_name}
            engine_region = f"engine_info_{cls.engine_name}"
            engine_info_json = json.dumps(engine_info, ensure_ascii=False)

            # 创建内存区域并写入引擎信息
            raming_create_memory_region(
                engine_region, len(engine_info_json.encode("utf-8")) + 1024
            )
            raming_write_memory(engine_region, engine_info_json.encode("utf-8"))

            # 发布引擎注册事件（不传递数据，Rust端会扫描所有engine_info_*区域）
            raming_publish_event("engine_register")

        except Exception:
            # 静默处理失败情况
            pass

    @abstractmethod
    def search(
        self, query: str, page: int = 1, page_size: Optional[int] = None, **kwargs
    ) -> Dict[str, Any]:
        """
        执行搜索（抽象方法，子类必须实现）

        Args:
            query: 搜索查询字符串
            page: 页数（从1开始）
            page_size: 每页结果数（可选）
            **kwargs: 其他搜索参数

        Returns:
            Dict[str, Any]: 搜索结果字典
            {
                "success": True/False,
                "results": [
                    {
                        "title": "标题",
                        "url": "链接",
                        "content": "摘要",
                        "display_url": "显示URL",
                        "site_name": "站点名",
                        "thumbnail": "缩略图URL"（可选）
                    }
                ],
                "total_results": 总结果数（可选）,
                "elapsed_ms": 耗时毫秒数,
                "suggestions": ["建议1", "建议2"]（可选）,
                "error": "错误信息"（失败时）
            }
        """
        pass

    def search_with_validation(
        self, query: str, page: int = 1, page_size: Optional[int] = None, **kwargs
    ) -> Dict[str, Any]:
        """
        带验证的搜索入口

        Args:
            query: 搜索查询字符串
            page: 页数
            page_size: 每页结果数
            **kwargs: 其他参数

        Returns:
            Dict[str, Any]: 搜索结果
        """
        start_time = time.time()
        self._request_count += 1

        try:
            # 参数验证
            validation_error = self._validate_search_params(
                query, page, page_size, **kwargs
            )
            if validation_error:
                return {
                    "success": False,
                    "error": validation_error,
                    "results": [],
                    "elapsed_ms": int((time.time() - start_time) * 1000),
                }

            # 设置默认页面大小
            if page_size is None:
                page_size = self.default_page_size

            # 执行搜索
            result = self.search(query, page, page_size, **kwargs)

            # 确保结果格式正确
            if not isinstance(result, dict):
                result = {
                    "success": False,
                    "error": "Invalid result format",
                    "results": [],
                }

            if "elapsed_ms" not in result:
                result["elapsed_ms"] = int((time.time() - start_time) * 1000)

            if result.get("success", False):
                self._success_count += 1
            else:
                self._last_error = result.get("error", "Unknown error")

            return result

        except Exception as e:
            self._last_error = str(e)
            return {
                "success": False,
                "error": f"Engine error: {e}",
                "results": [],
                "elapsed_ms": int((time.time() - start_time) * 1000),
            }

    def _validate_search_params(
        self, query: str, page: int, page_size: Optional[int], **kwargs
    ) -> Optional[str]:
        """验证搜索参数"""
        if not query or not query.strip():
            return "Query cannot be empty"

        if page < 1:
            return "Page must be >= 1"

        if page_size is not None:
            if page_size < 1:
                return "Page size must be >= 1"
            if page_size > self.max_page_size:
                return f"Page size cannot exceed {self.max_page_size}"

        return None

    def get_engine_info(self) -> Dict[str, Any]:
        """获取引擎信息"""
        return {
            "name": self.engine_name,
            "type": self.engine_type,
            "description": self.description,
            "version": self.version,
            "author": self.author,
            "class_name": f"{self.__class__.__module__}.{self.__class__.__name__}",
            "capabilities": {
                "supports_pagination": self.supports_pagination,
                "supports_language_filter": self.supports_language_filter,
                "supports_region_filter": self.supports_region_filter,
                "supports_time_range": self.supports_time_range,
                "max_page_size": self.max_page_size,
                "default_page_size": self.default_page_size,
            },
            "status": {
                "is_available": self._is_available,
                "last_error": self._last_error,
                "request_count": self._request_count,
                "success_count": self._success_count,
                "success_rate": self._success_count / max(self._request_count, 1),
                "created_at": self._created_at,
            },
            "instance_id": self._instance_id,
        }

    def is_available(self) -> bool:
        """检查引擎是否可用"""
        return self._is_available

    def set_available(self, available: bool, error: Optional[str] = None):
        """设置引擎可用状态"""
        self._is_available = available
        if not available and error:
            self._last_error = error

    @classmethod
    def get_registered_engines(cls) -> Dict[str, Type["BaseSearchEngine"]]:
        """获取所有已注册的引擎类"""
        return cls._registered_engines.copy()

    @classmethod
    def create_engine(cls, engine_name: str) -> Optional["BaseSearchEngine"]:
        """创建引擎实例"""
        if engine_name in cls._registered_engines:
            engine_class = cls._registered_engines[engine_name]
            return engine_class()
        return None


def register_engine_class(
    name: str,
    engine_type: str = "web",
    description: str = "",
    version: str = "1.0.0",
    author: str = "",
    **capabilities,
):
    """
    装饰器：手动注册引擎类

    Args:
        name: 引擎名称
        engine_type: 引擎类型
        description: 描述
        version: 版本
        author: 作者
        **capabilities: 引擎能力配置
    """

    def decorator(engine_class):
        # 设置引擎属性
        engine_class.engine_name = name
        engine_class.engine_type = engine_type
        engine_class.description = description
        engine_class.version = version
        engine_class.author = author

        # 设置能力配置
        for key, value in capabilities.items():
            if hasattr(engine_class, key):
                setattr(engine_class, key, value)

        # 标记为非抽象类，触发自动注册
        engine_class._is_abstract = False
        engine_class._auto_register()

        return engine_class

    return decorator
