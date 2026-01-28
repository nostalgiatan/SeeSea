"""
SeeSea 向量存储客户端

提供统一的向量数据库接口，基于PyVectorClient实现完整功能。
"""

from typing import Dict, Any, Optional, List

from ..base import BaseClient
from ..seesea_types.vector_types import VectorData
from ..seesea_types.common_types import Result, Error

try:
    from seesea_core import PyVectorClient

    _CORE_AVAILABLE = True
except ImportError:
    _CORE_AVAILABLE = False


class VectorClient(BaseClient[Dict[str, Any]]):
    """
    SeeSea 向量存储客户端

    提供高层次的向量数据库接口，支持文档存储、检索和相似性搜索。
    继承自BaseClient，提供统一的客户端接口。

    主要功能:
    - 文档存储
    - 向量检索
    - 相似性搜索
    - 批量操作

    示例:
        >>> client = VectorClient()
        >>> with client:
        ...     result = client.add_document("doc1", [0.1, 0.2, 0.3])
        ...     if result.success:
        ...         print("文档已添加")
    """

    def __init__(self, config: Optional[Dict[str, Any]] = None):
        """初始化向量客户端"""
        super().__init__(config)
        self._client: Optional[PyVectorClient] = None

        if not _CORE_AVAILABLE:
            error = Error(
                code="CORE_NOT_AVAILABLE",
                message="SeeSea核心模块不可用",
                details={
                    "error": "请确保已正确安装seesea_core",
                },
            )
            self._error = error
            return

        try:
            self._client = PyVectorClient()
        except Exception as e:
            error = Error(
                code="VECTOR_CLIENT_INIT_FAILED",
                message=f"向量客户端初始化失败: {str(e)}",
                details={"error": str(e)},
            )
            self._error = error

    def add_document(
        self,
        document_id: str,
        vector: List[float],
        metadata: Optional[Dict[str, Any]] = None,
        collection: Optional[str] = None,
    ) -> "Result[str]":
        """
        添加文档到向量数据库

        参数:
            document_id: 文档ID
            vector: 向量数据
            metadata: 元数据
            collection: 集合名称

        返回:
            Result[str]: 文档ID
        """
        if self._client is None:
            return Result.failure_result(self._error)

        try:
            self._client.add_document(
                document_id, vector, metadata or {}, collection or "default"
            )
            return Result.success_result(document_id)
        except Exception as e:
            return Result.failure_result(
                Error(
                    code="ADD_DOCUMENT_FAILED",
                    message=f"添加文档失败: {str(e)}",
                    details={"error": str(e)},
                )
            )

    def search(
        self,
        query_vector: List[float],
        top_k: int = 10,
        collection: Optional[str] = None,
        score_threshold: Optional[float] = None,
    ) -> "Result[List[Dict[str, Any]]]":
        """
        执行向量相似性搜索

        参数:
            query_vector: 查询向量
            top_k: 返回结果数量
            collection: 集合名称
            score_threshold: 相似度阈值

        返回:
            Result[List[Dict[str, Any]]: 搜索结果
        """
        if self._client is None:
            return Result.failure_result(self._error)

        try:
            results = self._client.search(
                query_vector, top_k, collection or "default", score_threshold or 0.0
            )
            return Result.success_result(results)
        except Exception as e:
            return Result.failure_result(
                Error(
                    code="VECTOR_SEARCH_FAILED",
                    message=f"向量搜索失败: {str(e)}",
                    details={"error": str(e)},
                )
            )

    def delete_document(
        self, document_id: str, collection: Optional[str] = None
    ) -> "Result[bool]":
        """
        删除文档

        参数:
            document_id: 文档ID
            collection: 集合名称

        返回:
            Result[bool]: 操作结果
        """
        if self._client is None:
            return Result.failure_result(self._error)

        try:
            self._client.delete_document(document_id, collection or "default")
            return Result.success_result(True)
        except Exception as e:
            return Result.failure_result(
                Error(
                    code="DELETE_DOCUMENT_FAILED",
                    message=f"删除文档失败: {str(e)}",
                    details={"error": str(e)},
                )
            )

    def get_document(
        self, document_id: str, collection: Optional[str] = None
    ) -> "Result[VectorData]":
        """
        获取文档

        参数:
            document_id: 文档ID
            collection: 集合名称

        返回:
            Result[VectorData]: 文档数据
        """
        if self._client is None:
            return Result.failure_result(self._error)

        try:
            doc = self._client.get_document(document_id, collection or "default")
            return Result.success_result(VectorData(**doc))
        except Exception as e:
            return Result.failure_result(
                Error(
                    code="GET_DOCUMENT_FAILED",
                    message=f"获取文档失败: {str(e)}",
                    details={"error": str(e)},
                )
            )

    def add_documents_batch(
        self, documents: List[Dict[str, Any]], collection: Optional[str] = None
    ) -> "Result[int]":
        """
        批量添加文档

        参数:
            documents: 文档列表，每个文档包含id、vector和metadata
            collection: 集合名称

        返回:
            Result[int]: 成功添加的文档数量
        """
        if self._client is None:
            return Result.failure_result(self._error)

        try:
            for doc in documents:
                self._client.add_document(
                    doc["id"],
                    doc["vector"],
                    doc.get("metadata", {}),
                    collection or "default",
                )
            return Result.success_result(len(documents))
        except Exception as e:
            return Result.failure_result(
                Error(
                    code="ADD_DOCUMENTS_BATCH_FAILED",
                    message=f"批量添加文档失败: {str(e)}",
                    details={"error": str(e)},
                )
            )
