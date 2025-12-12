# -*- coding: utf-8 -*-
"""
嵌入模型基类和管理器
"""

from abc import ABC, abstractmethod
from enum import Enum
from typing import List, Optional, Union, Callable, cast
import os
import threading


class EmbeddingMode(Enum):
    """嵌入模式"""

    STANDARD = "standard"  # 标准模式：轻量级模型
    PRO = "pro"  # Pro模式：高质量模型


class BaseEmbedder(ABC):
    """嵌入器基类"""

    @abstractmethod
    def encode(
        self, texts: Union[str, List[str]], batch_size: int = 8
    ) -> Union[List[float], List[List[float]]]:
        """
        将文本编码为向量

        Args:
            texts: 单个文本或文本列表
            batch_size: 批处理大小

        Returns:
            单个向量或向量列表
        """
        pass

    @abstractmethod
    def get_dimension(self) -> int:
        """获取向量维度"""
        pass

    def encode_callback(self, text: str) -> List[float]:
        """
        Rust回调接口

        Args:
            text: 要编码的文本

        Returns:
            向量列表
        """
        result = cast(List[float], self.encode(text))
        return result


class EmbeddingManager:
    """
    嵌入模型管理器

    支持标准模式和Pro模式的切换，提供统一的嵌入接口。
    线程安全，支持并发访问。
    """

    _instance: Optional["EmbeddingManager"] = None
    _lock = threading.Lock()

    def __new__(cls, *args, **kwargs):
        """单例模式"""
        if cls._instance is None:
            with cls._lock:
                if cls._instance is None:
                    cls._instance = super().__new__(cls)
        return cls._instance

    def __init__(
        self,
        mode: EmbeddingMode = EmbeddingMode.STANDARD,
        device: Optional[str] = None,
        n_threads: Optional[int] = None,
    ):
        """
        初始化嵌入管理器

        Args:
            mode: 嵌入模式（标准或Pro）
            device: 运行设备（'cuda', 'cpu', None自动检测）
            n_threads: 线程数（None自动检测）
        """
        # 避免重复初始化
        self._initialized: bool = False
        if hasattr(self, "_initialized") and self._initialized:
            return

        self._mode = mode
        self._device = device
        self._n_threads = n_threads or os.cpu_count() or 4
        self._embedder: Optional[BaseEmbedder] = None
        self._callback_registered = False
        self._init_lock = threading.Lock()
        self._initialized = True

    @property
    def mode(self) -> EmbeddingMode:
        """当前嵌入模式"""
        return self._mode

    @property
    def embedder(self) -> BaseEmbedder:
        """获取嵌入器（懒加载）"""
        if self._embedder is None:
            with self._init_lock:
                if self._embedder is None:
                    self._embedder = self._create_embedder()
        return self._embedder

    def _create_embedder(self) -> BaseEmbedder:
        """创建嵌入器"""
        if self._mode == EmbeddingMode.PRO:
            from .pro import ProEmbedder

            return ProEmbedder(device=self._device, n_threads=self._n_threads)
        else:
            from .standard import StandardEmbedder

            return StandardEmbedder(device=self._device, n_threads=self._n_threads)

    def switch_mode(self, mode: EmbeddingMode) -> None:
        """
        切换嵌入模式

        Args:
            mode: 新的嵌入模式
        """
        if mode != self._mode:
            with self._init_lock:
                self._mode = mode
                self._embedder = None  # 重置嵌入器

    def encode(
        self, texts: Union[str, List[str]], batch_size: int = 8
    ) -> Union[List[float], List[List[float]]]:
        """
        编码文本

        Args:
            texts: 单个文本或文本列表
            batch_size: 批处理大小

        Returns:
            向量或向量列表
        """
        return self.embedder.encode(texts, batch_size)

    def get_dimension(self) -> int:
        """获取向量维度"""
        return self.embedder.get_dimension()

    def encode_callback(self, text: str) -> List[float]:
        """
        Rust回调接口

        Args:
            text: 要编码的文本

        Returns:
            向量列表
        """
        return self.embedder.encode_callback(text)

    def register_callback(self) -> Callable[[str], List[float]]:
        """
        获取用于注册到Rust的回调函数

        Returns:
            回调函数
        """
        self._callback_registered = True
        return self.encode_callback

    @classmethod
    def get_instance(
        cls,
        mode: Optional[EmbeddingMode] = None,
        device: Optional[str] = None,
        n_threads: Optional[int] = None,
    ) -> "EmbeddingManager":
        """
        获取单例实例

        Args:
            mode: 嵌入模式（仅首次创建时有效）
            device: 运行设备（仅首次创建时有效）
            n_threads: 线程数（仅首次创建时有效）

        Returns:
            EmbeddingManager实例
        """
        if cls._instance is None:
            return cls(
                mode=mode or EmbeddingMode.STANDARD,
                device=device,
                n_threads=n_threads,
            )
        return cls._instance

    @classmethod
    def reset_instance(cls) -> None:
        """重置单例实例"""
        with cls._lock:
            cls._instance = None
