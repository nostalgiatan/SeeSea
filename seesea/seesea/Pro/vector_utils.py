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

    def embed_text(self, text: Union[str, List[str]]) -> Union[List[float], List[List[float]]]:
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

    def __init__(self, model_path: Optional[str] = None, device: Optional[str] = None):
        """
        初始化向量数据库

        Args:
            model_path: 模型文件路径，默认为None，自动下载
            device: 运行设备，可选值：'cuda'、'cpu'或None（自动检测）
        """
        try:
            from tf import DocumentStore  # type: ignore[import-not-found]

            # 初始化DocumentStore
            self.store = DocumentStore(model_path=model_path, device=device)  # type: ignore[assignment]

        except ImportError as e:
            raise ImportError("未安装Pro特性，不开放Pro功能") from e

    def add_document(self, doc_id: str, content: str, **kwargs) -> None:
        """
        添加文档到向量数据库

        Args:
            doc_id: 文档唯一标识符
            content: 文档内容
            **kwargs: 文档元数据，如title、url、summary等
        """
        try:
            self.store.add(doc_id=doc_id, content=content, **kwargs)
        except Exception as e:
            raise RuntimeError(f"添加文档失败: {str(e)}") from e

    def add_documents(self, documents: List[Dict[str, Any]]) -> None:
        """
        批量添加文档到向量数据库

        Args:
            documents: 文档列表，每个文档包含'id'和'content'字段，可选元数据字段
        """
        try:
            # 转换为DocumentStore所需的格式
            docs = []
            for doc in documents:
                doc_dict = {"id": doc["id"], "content": doc["content"]}
                # 添加可选元数据
                for key, value in doc.items():
                    if key not in ["id", "content"]:
                        doc_dict[key] = value
                docs.append(doc_dict)

            # 批量添加
            self.store.add_batch(docs)
        except Exception as e:
            raise RuntimeError(f"批量添加文档失败: {str(e)}") from e

    def search(self, query: str, k: int = 5, return_objects: bool = False) -> List[Dict[str, Any]]:
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
            return self.store.search(query=query, k=k, return_objects=return_objects)
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
            return self.store.search_by_vector(vector=vector, k=k)
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


def compute_similarity(vector1: Union[List[float], np.ndarray], vector2: Union[List[float], np.ndarray]) -> float:
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

        # 计算余弦相似度
        similarity = np.dot(vec1, vec2) / (np.linalg.norm(vec1) * np.linalg.norm(vec2))

        return float(similarity)

    except Exception as e:
        raise RuntimeError(f"计算相似度失败: {str(e)}") from e


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
