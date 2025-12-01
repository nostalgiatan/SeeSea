# -*- coding: utf-8 -*-
"""
模块名称：vectorizer_singleton
职责范围：实现单例模式的向量化器
期望实现计划：
1. 实现单例模式的向量化器
2. 确保线程安全
3. 提供全局访问点
已实现功能：
1. 线程安全的单例向量化器
2. 全局访问点
3. 延迟初始化
使用依赖：
- tf
主要接口：
- VectorizerSingleton：单例模式的向量化器类
注意事项：
- 线程安全设计
- 延迟初始化，节省资源
- 全局唯一实例
"""

from typing import Optional, Union, List, Dict
import threading


class VectorizerSingleton:
    """
    单例模式的向量化器

    确保整个应用中只有一个向量化器实例，
    提高资源利用率，避免重复初始化模型。

    线程安全设计，支持多线程环境。
    包含向量缓存机制，避免重复计算。
    """

    # 类变量，存储唯一实例
    _instance: Optional["VectorizerSingleton"] = None
    # 线程锁，确保线程安全
    _lock: threading.Lock = threading.Lock()

    def __new__(cls, model_path: Optional[str] = None, device: Optional[str] = None):
        """
        创建或获取单例实例

        Args:
            model_path: 模型文件路径
            device: 运行设备

        Returns:
            VectorizerSingleton: 单例实例
        """
        with cls._lock:
            if cls._instance is None:
                cls._instance = super(VectorizerSingleton, cls).__new__(cls)
                # 延迟初始化，只在第一次创建实例时初始化
                cls._instance._initialize(model_path, device)
            return cls._instance

    def _initialize(self, model_path: Optional[str] = None, device: Optional[str] = None):
        """
        初始化向量化器

        Args:
            model_path: 模型文件路径
            device: 运行设备
        """
        try:
            from tf.embeddings import TextEmbedder  # type: ignore[import-not-found]

            # 初始化TextEmbedder
            self.embedder = TextEmbedder(model_path=model_path, device=device)  # type: ignore[assignment]

            # 获取嵌入维度
            self.dimension = self.embedder.get_dimension()

            # 初始化向量缓存
            self._vector_cache: Dict[str, List[float]] = {}
            # 缓存锁，确保线程安全
            self._cache_lock = threading.Lock()

        except ImportError as e:
            raise ImportError("未安装Pro特性，不开放Pro功能") from e

    def embed_text(self, text: Union[str, List[str]]) -> Union[List[float], List[List[float]]]:
        """
        将文本转换为向量表示

        Args:
            text: 单个文本字符串或文本列表

        Returns:
            Union[List[float], List[List[float]]]: 单个向量或向量列表
        """
        # 处理单个文本
        if isinstance(text, str):
            with self._cache_lock:
                # 检查缓存中是否已经存在
                if text in self._vector_cache:
                    return self._vector_cache[text]

                # 计算向量
                vector = self.embedder.encode(text)

                # 存入缓存
                self._vector_cache[text] = vector  # type: ignore[assignment]

                return vector  # type: ignore[no-any-return]

        # 处理文本列表
        else:
            vectors = []
            for t in text:
                with self._cache_lock:
                    # 检查缓存中是否已经存在
                    if t in self._vector_cache:
                        vectors.append(self._vector_cache[t])
                    else:
                        # 计算向量
                        vector = self.embedder.encode(t)

                        # 存入缓存
                        self._vector_cache[t] = vector  # type: ignore[assignment]

                        vectors.append(vector)  # type: ignore[arg-type]

            return vectors

    def clear_cache(self) -> None:
        """
        清除向量缓存
        """
        with self._cache_lock:
            self._vector_cache.clear()

    def cache_size(self) -> int:
        """
        获取缓存大小

        Returns:
            int: 缓存中的向量数量
        """
        with self._cache_lock:
            return len(self._vector_cache)

    def get_dimension(self) -> int:
        """
        获取向量维度

        Returns:
            int: 向量维度
        """
        return self.dimension  # type: ignore[no-any-return]

    @classmethod
    def get_instance(cls) -> "VectorizerSingleton":
        """
        获取单例实例，如果实例不存在则创建

        Returns:
            VectorizerSingleton: 单例实例
        """
        if cls._instance is None:
            return cls()
        return cls._instance

    @classmethod
    def reset_instance(cls) -> None:
        """
        重置单例实例，用于测试或资源释放
        """
        with cls._lock:
            cls._instance = None
