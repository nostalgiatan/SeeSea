# -*- coding: utf-8 -*-
"""
模块名称：vector_utils
职责范围：提供统一的数据向量化接口和向量数据库使用接口
期望实现计划：
1. 实现统一的数据向量化接口
2. 实现向量数据库的使用接口
3. 提供向量操作的工具函数
已实现功能：
1. 数据向量化接口
2. 向量数据库操作接口
3. 向量相似度计算
使用依赖：
- tf
- numpy
主要接口：
- Vectorizer：数据向量化类
- VectorDatabase：向量数据库操作类
- compute_similarity：向量相似度计算函数
注意事项：
- 需要确保tf模块已正确安装
- 向量数据库操作需要rust扩展支持
"""

from typing import List, Dict, Optional, Any, Union
import numpy as np  # type: ignore[import-not-found]


class Vectorizer:
    """
    数据向量化类

    使用tf模块的TextEmbedder实现数据向量化，
    提供统一的向量化接口。
    """

    def __init__(self, model_path: Optional[str] = None, device: Optional[str] = None):
        """
        初始化向量化器

        Args:
            model_path: 模型文件路径，默认为None，自动下载
            device: 运行设备，可选值：'cuda'、'cpu'或None（自动检测）
        """
        try:
            from tf.embeddings import TextEmbedder  # type: ignore[import-not-found]

            # 初始化TextEmbedder
            self.embedder = TextEmbedder(model_path=model_path, device=device)  # type: ignore[assignment]

            # 获取嵌入维度
            self.dimension = self.embedder.get_dimension()

        except ImportError as e:
            raise ImportError("未安装Pro特性，不开放Pro功能") from e

    def embed_text(
        self, text: Union[str, List[str]]
    ) -> Union[List[float], List[List[float]]]:
        """
        将文本转换为向量表示

        Args:
            text: 单个文本字符串或文本列表

        Returns:
            Union[List[float], List[List[float]]]: 单个向量或向量列表
        """
        try:
            return self.embedder.encode(text)  # type: ignore[no-any-return]
        except Exception as e:
            raise RuntimeError(f"文本向量化失败: {str(e)}") from e

    def embed_documents(self, documents: List[Dict[str, Any]]) -> List[Dict[str, Any]]:
        """
        批量向量化文档

        Args:
            documents: 文档列表，每个文档包含'content'字段

        Returns:
            List[Dict[str, Any]]: 包含向量的文档列表，新增'vector'字段
        """
        try:
            # 提取文档内容
            contents = [doc["content"] for doc in documents]

            # 批量向量化
            vectors = self.embedder.encode(contents)  # type: ignore[assignment]

            # 将向量添加到文档中
            for doc, vector in zip(documents, vectors):
                doc["vector"] = vector  # type: ignore[assignment]

            return documents
        except Exception as e:
            raise RuntimeError(f"文档向量化失败: {str(e)}") from e

    def get_dimension(self) -> int:
        """
        获取向量维度

        Returns:
            int: 向量维度
        """
        return self.dimension  # type: ignore[no-any-return]


class VectorDatabase:
    """
    向量数据库操作类

    使用tf模块的DocumentStore实现向量数据库操作，
    提供统一的数据库接口。
    """

    def __init__(
        self,
        model_path: Optional[str] = None,
        device: Optional[str] = None,
        store_path: Optional[str] = None,
    ):
        """
        初始化向量数据库

        Args:
            model_path: 模型文件路径，默认为None，自动下载
            device: 运行设备，可选值：'cuda'、'cpu'或None（自动检测）
            store_path: 存储路径，默认为None，使用默认持久化存储路径
        """
        try:
            from tf import DocumentStore  # type: ignore[import-not-found]

            # 初始化DocumentStore，使用持久化存储路径
            self.store = DocumentStore(model_path=model_path, device=device, store_path=store_path)  # type: ignore[assignment]

        except ImportError as e:
            raise ImportError("未安装Pro特性，不开放Pro功能") from e

    def add_document(self, doc_id: str, content: str, **kwargs) -> bool:
        """
        添加文档到向量数据库

        Args:
            doc_id: 文档唯一标识符
            content: 文档内容
            **kwargs: 文档元数据，如title、url、summary等

        Returns:
            bool: 添加成功返回True，文档已存在且内容未变化返回False
        """
        try:
            from typing import cast

            return cast(bool, self.store.add(doc_id=doc_id, content=content, **kwargs))
        except Exception as e:
            raise RuntimeError(f"添加文档失败: {str(e)}") from e

    def add_documents(self, documents: List[Dict[str, Any]]) -> int:
        """
        批量添加文档到向量数据库

        Args:
            documents: 文档列表，每个文档包含'id'和'content'字段，可选元数据字段

        Returns:
            int: 成功更新或添加的文档数量
        """
        try:
            # 转换为DocumentStore所需的格式
            docs = []
            for doc in documents:
                # 确保文档有id和content字段，并且不为空
                if "id" not in doc or not doc["id"]:
                    raise ValueError(
                        f"Document must have a non-empty 'id' field: {doc}"
                    )
                if "content" not in doc or not doc["content"]:
                    raise ValueError(
                        f"Document must have a non-empty 'content' field: {doc}"
                    )

                doc_dict = {"id": doc["id"], "content": doc["content"]}
                # 添加可选元数据
                for key, value in doc.items():
                    if key not in ["id", "content"]:
                        doc_dict[key] = value
                docs.append(doc_dict)

            # 批量添加，返回更新的文档数量
            from typing import cast

            return cast(int, self.store.add_batch(docs))
        except Exception as e:
            raise RuntimeError(f"批量添加文档失败: {str(e)}") from e

    def search(
        self, query: str, k: int = 5, return_objects: bool = False
    ) -> List[Dict[str, Any]]:
        """
        在向量数据库中搜索相似文档

        Args:
            query: 搜索查询文本
            k: 返回结果数量
            return_objects: 是否返回SearchResult对象，默认为False返回字典

        Returns:
            List[Dict[str, Any]]: 搜索结果列表
        """
        try:
            results = self.store.search(query=query, k=k, return_objects=return_objects)  # type: ignore[no-any-return]

            # 确保返回的是List[Dict[str, Any]]类型
            if return_objects:
                # 如果返回的是SearchResult对象列表，转换为字典列表
                dict_results = []
                for result in results:  # type: ignore[union-attr]
                    # 假设SearchResult对象有to_dict方法
                    if hasattr(result, "to_dict"):
                        dict_results.append(result.to_dict())  # type: ignore[attr-defined]
                    else:
                        # 否则，尝试使用字典推导式转换
                        dict_results.append({k: v for k, v in result.__dict__.items() if not k.startswith("_")})  # type: ignore[union-attr]
                return dict_results
            else:
                # 如果已经是字典列表，直接返回
                from typing import cast

                return cast(List[Dict[str, Any]], results)
        except Exception as e:
            raise RuntimeError(f"搜索失败: {str(e)}") from e

    def search_by_vector(self, vector: List[float], k: int = 5) -> List[Dict[str, Any]]:
        """
        使用向量在数据库中搜索相似文档

        Args:
            vector: 查询向量
            k: 返回结果数量

        Returns:
            List[Dict[str, Any]]: 搜索结果列表
        """
        try:
            return self.store.search_by_vector(vector=vector, k=k)  # type: ignore[no-any-return]
        except Exception as e:
            raise RuntimeError(f"向量搜索失败: {str(e)}") from e

    def get_document(self, doc_id: str) -> Optional[Dict[str, Any]]:
        """
        获取文档元数据

        Args:
            doc_id: 文档唯一标识符

        Returns:
            Optional[Dict[str, Any]]: 文档元数据，不存在返回None
        """
        try:
            return self.store.get(doc_id=doc_id)  # type: ignore[no-any-return]
        except Exception as e:
            raise RuntimeError(f"获取文档失败: {str(e)}") from e

    def update_document(self, doc_id: str, **kwargs) -> None:
        """
        更新文档元数据

        Args:
            doc_id: 文档唯一标识符
            **kwargs: 要更新的元数据字段
        """
        try:
            self.store.update(doc_id=doc_id, **kwargs)
        except Exception as e:
            raise RuntimeError(f"更新文档失败: {str(e)}") from e

    def delete_document(self, doc_id: str) -> None:
        """
        删除文档

        Args:
            doc_id: 文档唯一标识符
        """
        try:
            self.store.delete(doc_id=doc_id)
        except Exception as e:
            raise RuntimeError(f"删除文档失败: {str(e)}") from e

    def delete_documents(self, doc_ids: List[str]) -> None:
        """
        批量删除文档

        Args:
            doc_ids: 文档唯一标识符列表
        """
        try:
            self.store.delete_batch(doc_ids=doc_ids)
        except Exception as e:
            raise RuntimeError(f"批量删除文档失败: {str(e)}") from e

    def count_documents(self) -> int:
        """
        获取数据库中文档数量

        Returns:
            int: 文档数量
        """
        try:
            return self.store.count()  # type: ignore[no-any-return]
        except Exception as e:
            raise RuntimeError(f"获取文档数量失败: {str(e)}") from e

    def is_empty(self) -> bool:
        """
        检查数据库是否为空

        Returns:
            bool: 空返回True，否则返回False
        """
        try:
            return self.store.is_empty()  # type: ignore[no-any-return]
        except Exception as e:
            raise RuntimeError(f"检查数据库状态失败: {str(e)}") from e


def compute_similarity(
    vector1: Union[List[float], np.ndarray], vector2: Union[List[float], np.ndarray]
) -> float:
    """
    计算两个向量的余弦相似度

    Args:
        vector1: 第一个向量
        vector2: 第二个向量

    Returns:
        float: 余弦相似度，范围[-1, 1]
    """
    try:
        # 转换为numpy数组
        vec1 = np.array(vector1)
        vec2 = np.array(vector2)

        # 计算向量范数
        norm1 = np.linalg.norm(vec1)
        norm2 = np.linalg.norm(vec2)

        # 处理范数为0的情况
        if norm1 == 0 or norm2 == 0:
            return 0.0

        # 计算余弦相似度
        similarity = np.dot(vec1, vec2) / (norm1 * norm2)

        return float(similarity)

    except Exception as e:
        # 捕获所有异常，返回0.0作为默认值
        print(f"计算相似度失败，返回默认值0.0: {str(e)}")
        return 0.0


def normalize_vector(vector: List[float]) -> List[float]:
    """
    归一化向量

    Args:
        vector: 输入向量

    Returns:
        List[float]: 归一化后的向量
    """
    try:
        # 转换为numpy数组
        vec = np.array(vector)

        # 计算向量范数
        norm = np.linalg.norm(vec)

        # 归一化
        normalized_vec = vec / norm if norm != 0 else vec

        return normalized_vec.tolist()  # type: ignore[no-any-return]

    except Exception as e:
        raise RuntimeError(f"向量归一化失败: {str(e)}") from e


class BatchProcessor:
    """
    批处理管理器类，用于积累文档并批量写入向量数据库

    支持根据配置的批量大小或内存使用情况自动触发批量处理，
    提高处理大量文档时的效率，减少网络和IO开销。

    注意：该实现不使用Python线程锁，而是依赖Rust层的线程安全实现，
    这样可以避开Python的GIL（全局解释器锁），提高性能。
    """

    def __init__(
        self,
        batch_size: int = 50,  # 减小默认批处理大小，避免过大的解码上下文
        max_memory_mb: int = 512,  # 减小默认最大内存使用量
        store_path: Optional[str] = None,
        model_path: Optional[str] = None,
        device: Optional[str] = None,
    ):
        """
        初始化批处理管理器

        Args:
            batch_size: 批处理大小，达到此大小自动触发批量处理
            max_memory_mb: 最大内存使用量（MB），超过此值自动触发批量处理
            store_path: 向量数据库存储路径
            model_path: 嵌入模型路径
            device: 运行设备，可选值：'cuda'、'cpu'或None（自动检测）
        """
        self.batch_size = batch_size
        self.max_memory_mb = max_memory_mb
        self.store_path = store_path
        self.model_path = model_path
        self.device = device

        # 用于积累文档的列表
        self.batch: List[Dict[str, Any]] = []
        # 用于估计内存使用量
        self.estimated_memory_mb = 0.0
        # 向量数据库实例（Rust层已实现线程安全）
        self.database = VectorDatabase(
            model_path=model_path, device=device, store_path=store_path
        )
        # 用于去重的集合，存储已处理的URL和内容哈希
        self.processed_urls: set[str] = set()
        self.processed_content_hashes: set[str] = set()
        # 用于快速计算内容哈希
        import hashlib

        self._hashlib = hashlib

    def add_document(self, doc_id: str, content: str, **kwargs) -> None:
        """
        添加单个文档到批处理队列

        Args:
            doc_id: 文档唯一标识符
            content: 文档内容
            **kwargs: 文档元数据，如title、url、summary等
        """
        # 1. 检查URL是否已处理，避免重复处理相同网页
        url = kwargs.get("url", "")
        if url and url in self.processed_urls:
            # URL已处理，跳过
            return

        # 2. 计算内容哈希，避免重复处理相同内容
        content_hash = self._hashlib.sha256(content.encode()).hexdigest()
        if content_hash in self.processed_content_hashes:
            # 内容已处理，跳过
            return

        # 3. 创建文档字典
        doc = {"id": doc_id, "content": content, **kwargs}

        # 4. 估计文档占用的内存大小（字节）
        estimated_size = len(str(doc).encode("utf-8")) / (1024 * 1024)  # 转换为MB

        # 5. 标记URL和内容为已处理
        if url:
            self.processed_urls.add(url)
        self.processed_content_hashes.add(content_hash)

        # 6. 添加到批处理队列
        self.batch.append(doc)
        self.estimated_memory_mb += estimated_size

        # 7. 检查是否需要触发批处理
        if (
            len(self.batch) >= self.batch_size
            or self.estimated_memory_mb >= self.max_memory_mb
        ):
            self.process_batch()

    def add_documents(self, documents: List[Dict[str, Any]]) -> None:
        """
        添加多个文档到批处理队列

        Args:
            documents: 文档列表，每个文档包含'id'和'content'字段，可选元数据字段
        """
        for doc in documents:
            # 确保文档有id和content字段，并且不为空
            if "id" not in doc or not doc["id"]:
                raise ValueError(f"Document must have a non-empty 'id' field: {doc}")
            if "content" not in doc or not doc["content"]:
                raise ValueError(
                    f"Document must have a non-empty 'content' field: {doc}"
                )
            # 使用add_document方法添加，这样可以共享去重逻辑
            self.add_document(
                doc_id=doc["id"],
                content=doc["content"],
                **{k: v for k, v in doc.items() if k not in ["id", "content"]},
            )

    def process_batch(self) -> int:
        """
        处理当前批处理队列中的文档

        Returns:
            int: 成功处理的文档数量
        """
        if not self.batch:
            return 0

        # 复制当前批处理队列并清空
        batch_to_process = self.batch.copy()
        self.batch.clear()
        self.estimated_memory_mb = 0.0

        # 处理批处理
        try:
            return self.database.add_documents(batch_to_process)
        except Exception as e:
            print(f"批处理失败: {e}")
            # 如果处理失败，将文档重新添加到队列
            self.batch.extend(batch_to_process)
            self.estimated_memory_mb += sum(
                len(str(doc).encode("utf-8")) / (1024 * 1024)
                for doc in batch_to_process
            )
            return 0

    def flush(self) -> int:
        """
        强制处理当前所有文档

        Returns:
            int: 成功处理的文档数量
        """
        return self.process_batch()

    def __del__(self):
        """
        析构函数，确保在对象销毁时处理剩余文档

        注意：如果已经显式调用了flush方法，此方法不会再次处理文档
        """
        # 只有当批处理队列不为空时才调用flush方法
        if self.batch:
            self.flush()


class VectorUtils:
    """
    向量工具类，提供简化的向量操作接口

    组合了Vectorizer和VectorDatabase的功能，提供更简单的接口
    """

    def __init__(
        self,
        model_path: Optional[str] = None,
        device: Optional[str] = None,
        store_path: Optional[str] = None,
        batch_size: int = 100,
        max_memory_mb: int = 1024,
    ):
        """
        初始化向量工具类

        Args:
            model_path: 模型文件路径，默认为None，自动下载
            device: 运行设备，可选值：'cuda'、'cpu'或None（自动检测）
            store_path: 存储路径，默认为None，使用默认持久化存储路径
            batch_size: 批处理大小，达到此大小自动触发批量处理
            max_memory_mb: 最大内存使用量（MB），超过此值自动触发批量处理
        """
        self.model_path = model_path
        self.device = device
        self.store_path = store_path
        self.batch_size = batch_size
        self.max_memory_mb = max_memory_mb
        self._vectorizer = None
        self._database = None
        self._batch_processor = None

    @property
    def vectorizer(self):
        """延迟初始化向量器"""
        if self._vectorizer is None:
            self._vectorizer = Vectorizer(self.model_path, self.device)
        return self._vectorizer

    @property
    def database(self):
        """延迟初始化向量数据库"""
        if self._database is None:
            self._database = VectorDatabase(
                self.model_path, self.device, self.store_path
            )
        return self._database

    @property
    def batch_processor(self):
        """延迟初始化批处理处理器"""
        if self._batch_processor is None:
            self._batch_processor = BatchProcessor(
                batch_size=self.batch_size,
                max_memory_mb=self.max_memory_mb,
                store_path=self.store_path,
                model_path=self.model_path,
                device=self.device,
            )
        return self._batch_processor

    def add_document(
        self,
        content: str,
        metadata: Optional[Dict[str, Any]] = None,
        doc_id: Optional[str] = None,
    ) -> None:
        """
        添加文档到向量数据库（支持批处理）

        Args:
            content: 文档内容
            metadata: 文档元数据，如title、url、summary等
            doc_id: 文档唯一标识符，不提供则自动生成
        """
        from uuid import uuid4

        # 生成唯一ID（如果未提供）
        if doc_id is None:
            doc_id = str(uuid4())

        # 转换元数据为字典
        kwargs = {}
        if metadata:
            kwargs.update(metadata)

        # 使用批处理处理器添加文档
        self.batch_processor.add_document(doc_id, content, **kwargs)

    def add_documents(self, documents: List[Dict[str, Any]]) -> None:
        """
        批量添加文档到向量数据库

        Args:
            documents: 文档列表，每个文档包含'content'字段，可选'id'和其他元数据字段
        """
        processed_docs = []
        for doc in documents:
            # 确保每个文档都有id和content字段，并且content不为空
            if "id" not in doc:
                from uuid import uuid4

                doc["id"] = str(uuid4())
            if "content" not in doc or not doc["content"]:
                raise ValueError(
                    f"Document must have a non-empty 'content' field: {doc}"
                )
            processed_docs.append(doc)

        # 使用批处理处理器添加文档
        self.batch_processor.add_documents(processed_docs)

    def flush(self) -> int:
        """
        强制处理当前所有文档

        Returns:
            int: 成功处理的文档数量
        """
        from typing import cast

        return cast(int, self.batch_processor.flush())

    def search(self, query: str, limit: int = 10) -> List[Dict[str, Any]]:
        """
        在向量数据库中搜索相似文档

        Args:
            query: 搜索查询文本
            limit: 返回结果数量

        Returns:
            List[Dict[str, Any]]: 搜索结果列表
        """
        try:
            results = self.database.search(query, k=limit, return_objects=False)
            # 确保结果是List[Dict[str, Any]]类型
            if isinstance(results, list):
                return results
            return []
        except Exception as e:
            # 如果搜索失败，返回空列表
            print(f"向量搜索失败，返回空列表: {str(e)}")
            return []
