"""
SeeSea 引擎注册模块

提供统一的搜索引擎注册和管理接口。
"""

from typing import Dict, Any, List, Callable
from ..seesea_types.common_types import Result, Error

try:
    from seesea_core import (
        register_engine as _register_engine,
        unregister_engine as _unregister_engine,
        list_engines as _list_engines,
    )

    _CORE_AVAILABLE = True
except ImportError:
    _CORE_AVAILABLE = False


class EngineRegistry:
    """
    SeeSea 引擎注册器

    提供搜索引擎的注册、注销和查询功能。

    主要功能:
    - 引擎注册
    - 引擎注销
    - 引擎列表查询

    示例:
        >>> def my_engine(query, page, page_size):
        ...     return []
        >>> EngineRegistry.register("my_engine", "general", my_engine)
    """

    @staticmethod
    def register(
        name: str,
        engine_type: str,
        handler: Callable,
        priority: int = 5,
        enabled: bool = True,
    ) -> "Result[bool]":
        """
        注册搜索引擎

        参数:
            name: 引擎名称
            engine_type: 引擎类型（"general", "images", "videos", "news"等）
            handler: 搜索处理函数
            priority: 引擎优先级
            enabled: 是否启用

        返回:
            Result[bool]: 操作结果
        """
        if not _CORE_AVAILABLE:
            return Result.failure_result(
                Error(
                    code="CORE_NOT_AVAILABLE",
                    message="SeeSea核心模块不可用",
                    details={
                        "error": "请确保已正确安装seesea_core",
                    },
                )
            )

        try:
            _register_engine(name, engine_type, handler, priority, enabled)
            return Result.success_result(True)
        except Exception as e:
            return Result.failure_result(
                Error(
                    code="REGISTER_ENGINE_FAILED",
                    message=f"注册引擎失败: {str(e)}",
                    details={"error": str(e)},
                )
            )

    @staticmethod
    def unregister(name: str) -> "Result[bool]":
        """
        注销搜索引擎

        参数:
            name: 引擎名称

        返回:
            Result[bool]: 操作结果
        """
        if not _CORE_AVAILABLE:
            return Result.failure_result(
                Error(
                    code="CORE_NOT_AVAILABLE",
                    message="SeeSea核心模块不可用",
                    details={
                        "error": "请确保已正确安装seesea_core",
                    },
                )
            )

        try:
            _unregister_engine(name)
            return Result.success_result(True)
        except Exception as e:
            return Result.failure_result(
                Error(
                    code="UNREGISTER_ENGINE_FAILED",
                    message=f"注销引擎失败: {str(e)}",
                    details={"error": str(e)},
                )
            )

    @staticmethod
    def list_engines() -> "Result[List[Dict[str, Any]]]":
        """
        列出所有已注册的引擎

        返回:
            Result[List[Dict[str, Any]]]: 引擎列表
        """
        if not _CORE_AVAILABLE:
            return Result.failure_result(
                Error(
                    code="CORE_NOT_AVAILABLE",
                    message="SeeSea核心模块不可用",
                    details={
                        "error": "请确保已正确安装seesea_core",
                    },
                )
            )

        try:
            engines = _list_engines()
            return Result.success_result(engines)
        except Exception as e:
            return Result.failure_result(
                Error(
                    code="LIST_ENGINES_FAILED",
                    message=f"列出引擎失败: {str(e)}",
                    details={"error": str(e)},
                )
            )
