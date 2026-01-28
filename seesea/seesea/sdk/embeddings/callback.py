"""
SeeSea 嵌入回调模块

提供统一的嵌入模型回调注册和管理接口。
"""

from typing import List, Callable
from ..seesea_types.common_types import Result, Error

try:
    from seesea_core import (
        register_embedding_callback as _register_embedding_callback,
        unregister_embedding_callback as _unregister_embedding_callback,
        get_embedding_mode as _get_embedding_mode,
        get_embedding_dimension as _get_embedding_dimension,
    )

    _CORE_AVAILABLE = True
except ImportError:
    _CORE_AVAILABLE = False


class EmbeddingCallback:
    """
    SeeSea 嵌入回调管理器

    提供嵌入模型回调的注册、注销和查询功能。

    主要功能:
    - 回调注册
    - 回调注销
    - 模式查询
    - 维度查询

    示例:
        >>> def my_embedding(text: str) -> List[float]:
        ...     return [0.1, 0.2, 0.3]
        >>> EmbeddingCallback.register(my_embedding, 768, "standard")
    """

    @staticmethod
    def register(
        callback: Callable[[str], List[float]],
        dimension: int,
        mode: str = "standard",
        max_concurrency: int = 4,
    ) -> "Result[bool]":
        """
        注册嵌入回调

        参数:
            callback: 嵌入函数，接收文本返回向量
            dimension: 向量维度
            mode: 嵌入模式（"standard", "pro"）
            max_concurrency: 最大并发数

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
            _register_embedding_callback(callback, dimension, mode, max_concurrency)
            return Result.success_result(True)
        except Exception as e:
            return Result.failure_result(
                Error(
                    code="REGISTER_CALLBACK_FAILED",
                    message=f"注册回调失败: {str(e)}",
                    details={"error": str(e)},
                )
            )

    @staticmethod
    def unregister() -> "Result[bool]":
        """
        注销嵌入回调

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
            _unregister_embedding_callback()
            return Result.success_result(True)
        except Exception as e:
            return Result.failure_result(
                Error(
                    code="UNREGISTER_CALLBACK_FAILED",
                    message=f"注销回调失败: {str(e)}",
                    details={"error": str(e)},
                )
            )

    @staticmethod
    def get_mode() -> "Result[str]":
        """
        获取当前嵌入模式

        返回:
            Result[str]: 嵌入模式（"standard", "pro", "none"）
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
            mode = _get_embedding_mode()
            return Result.success_result(mode)
        except Exception as e:
            return Result.failure_result(
                Error(
                    code="GET_MODE_FAILED",
                    message=f"获取模式失败: {str(e)}",
                    details={"error": str(e)},
                )
            )

    @staticmethod
    def get_dimension() -> "Result[int]":
        """
        获取当前嵌入维度

        返回:
            Result[int]: 向量维度
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
            dimension = _get_embedding_dimension()
            return Result.success_result(dimension)
        except Exception as e:
            return Result.failure_result(
                Error(
                    code="GET_DIMENSION_FAILED",
                    message=f"获取维度失败: {str(e)}",
                    details={"error": str(e)},
                )
            )

    @staticmethod
    def embed_text(text: str) -> "Result[List[float]]":
        """
        嵌入文本（使用注册的回调）

        参数:
            text: 要嵌入的文本

        返回:
            Result[List[float]]: 嵌入向量
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
            from seesea_core import embed_text as _embed_text

            vector = _embed_text(text)
            return Result.success_result(vector)
        except Exception as e:
            return Result.failure_result(
                Error(
                    code="EMBED_TEXT_FAILED",
                    message=f"文本嵌入失败: {str(e)}",
                    details={"error": str(e)},
                )
            )

    @staticmethod
    def embed_batch(texts: List[str]) -> "Result[List[List[float]]]":
        """
        批量嵌入文本

        参数:
            texts: 要嵌入的文本列表

        返回:
            Result[List[List[float]]]: 嵌入向量列表
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
            from seesea_core import embed_texts as _embed_texts

            vectors = _embed_texts(texts)
            return Result.success_result(vectors)
        except Exception as e:
            return Result.failure_result(
                Error(
                    code="EMBED_BATCH_FAILED",
                    message=f"批量嵌入失败: {str(e)}",
                    details={"error": str(e)},
                )
            )
