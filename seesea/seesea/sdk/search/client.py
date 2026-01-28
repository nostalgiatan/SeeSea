"""
SeeSea 搜索客户端

提供统一的搜索接口，继承自BaseClient。
基于现有PySearchClient实现完整功能。
"""

from typing import Dict, List, Optional, Any, Callable
from datetime import datetime

from ..base import BaseClient
from ..seesea_types.search_types import SearchResult, SearchResultItem
from ..seesea_types.common_types import Result, Error

try:
    from seesea_core import PySearchClient

    _CORE_AVAILABLE = True
except ImportError:
    _CORE_AVAILABLE = False


class SearchClient(BaseClient[SearchResult]):
    """
    SeeSea 搜索客户端

    提供高层次的搜索接口，自动处理并发、缓存和结果聚合。
    继承自BaseClient，提供统一的客户端接口。

    主要功能:
    - 多引擎并发搜索
    - 智能结果聚合与排序
    - 自动缓存管理
    - 类型安全的结果返回
    - 支持流式搜索
    - 引擎健康检查

    示例:
        >>> client = SearchClient()
        >>> with client:
        ...     result = client.search("Python编程")
        ...     if result.success:
        ...         print(f"找到 {len(result.data.items)} 个结果")
    """

    def __init__(self, config: Optional[Dict[str, Any]] = None):
        """初始化搜索客户端"""
        super().__init__(config)
        self._core_client: Optional[PySearchClient] = None

        if not _CORE_AVAILABLE:
            error = Error(
                code="SearchClient.init",
                message="seesea_core not available. Please install SeeSea core modules.",
                source="SearchClient",
            )
            self._set_error(error)

    def connect(self) -> Result[bool]:
        """连接到搜索服务"""
        with self._handle_operation("connect"):
            if not _CORE_AVAILABLE:
                return Result(
                    success=False,
                    error=Error(
                        code="SearchClient.connect",
                        message="Core modules not available",
                    ),
                )

            try:
                self._core_client = PySearchClient()
                self._is_connected = True
                return Result(success=True, data=True)
            except Exception as e:
                return Result(
                    success=False,
                    error=Error(
                        code="SearchClient.connect",
                        message=f"Failed to initialize core client: {e}",
                    ),
                )

    def disconnect(self) -> Result[bool]:
        """断开搜索服务连接"""
        with self._handle_operation("disconnect"):
            self._core_client = None
            self._is_connected = False
            return Result(success=True, data=True)

    def health_check(self) -> Result[Dict[str, Any]]:
        """搜索服务健康检查"""
        if not self._is_connected or not self._core_client:
            return Result(
                success=False,
                error=Error(
                    code="SearchClient.health_check", message="Client not connected"
                ),
            )

        try:
            engines_health = self._core_client.health_check()
            health_data = {
                "status": "healthy",
                "core_available": _CORE_AVAILABLE,
                "engines_health": engines_health,
                "total_engines": len(engines_health),
                "healthy_engines": sum(1 for h in engines_health.values() if h),
            }
            return Result(success=True, data=health_data)
        except Exception as e:
            return Result(
                success=False,
                error=Error(
                    code="SearchClient.health_check",
                    message=f"Health check failed: {e}",
                ),
            )

    def get_info(self) -> Result[Dict[str, Any]]:
        """获取搜索客户端信息"""
        info = {
            "client_type": "SearchClient",
            "version": "0.1.0",
            "core_available": _CORE_AVAILABLE,
            "connected": self._is_connected,
        }

        if self._is_connected and self._core_client:
            try:
                engines = self._core_client.list_engines()
                info["available_engines"] = engines
                info["engines_count"] = len(engines)
            except Exception:
                info["engines_count"] = 0

        return Result(success=True, data=info)

    def search(
        self,
        query: str,
        page: Optional[int] = 1,
        page_size: Optional[int] = 10,
        language: Optional[str] = None,
        region: Optional[str] = None,
        engines: Optional[List[str]] = None,
        force: Optional[bool] = False,
        cache_timeline: Optional[int] = None,
        include_deepweb: Optional[bool] = False,
        engine_type: Optional[str] = None,
    ) -> Result[SearchResult]:
        """
        执行搜索

        Args:
            query: 搜索关键词
            page: 页码（从1开始）
            page_size: 每页结果数
            language: 语言过滤（如 "zh", "en"）
            region: 地区过滤（如 "cn", "us"）
            engines: 指定使用的搜索引擎列表
            force: 强制搜索，绕过缓存（默认 False）
            cache_timeline: 缓存刷新时间线（秒）
            include_deepweb: 是否包含深网搜索
            engine_type: 搜索引擎类型（"general", "image", "video", "news"等）

        Returns:
            Result[SearchResult]: 搜索结果
        """
        if not self._is_connected or not self._core_client:
            return Result(
                success=False,
                error=Error(
                    code="SearchClient.search",
                    message="Client not connected. Call connect() first.",
                ),
            )

        with self._handle_operation("search"):
            try:
                start_time = datetime.now()

                result_dict = self._core_client.search(
                    query,
                    page,
                    page_size,
                    language,
                    region,
                    engines,
                    force,
                    cache_timeline,
                    include_deepweb,
                    engine_type,
                )

                # 转换为标准类型
                items = []
                for item_data in result_dict.get("results", []):
                    item = SearchResultItem(
                        title=item_data.get("title", ""),
                        content=item_data.get("content", ""),
                        url=item_data.get("url", ""),
                        score=float(item_data.get("score", 0.0)),
                        engine=item_data.get("engine", ""),
                        timestamp=datetime.now(),  # 可能需要从结果中解析
                        metadata=item_data.get("metadata"),
                        snippet=item_data.get("snippet"),
                        thumbnail=item_data.get("thumbnail"),
                    )
                    items.append(item)

                end_time = datetime.now()
                took_ms = int((end_time - start_time).total_seconds() * 1000)

                search_result = SearchResult(
                    items=items,
                    total=result_dict.get("total_count", len(items)),
                    page=page or 1,
                    per_page=page_size or 10,
                    query=query,
                    took_ms=took_ms,
                    engines_used=result_dict.get("engines_used", engines or []),
                    has_more=(page or 1) * (page_size or 10)
                    < result_dict.get("total_count", 0),
                )

                return Result(success=True, data=search_result, took_ms=took_ms)

            except Exception as e:
                return Result(
                    success=False,
                    error=Error(
                        code="SearchClient.search", message=f"Search failed: {e}"
                    ),
                )

    def search_streaming(
        self,
        query: str,
        callback: Callable[[Dict[str, Any]], None],
        page: Optional[int] = 1,
        page_size: Optional[int] = 10,
        engines: Optional[List[str]] = None,
        include_deepweb: Optional[bool] = False,
    ) -> Result[Dict[str, Any]]:
        """
        流式搜索 - 每个引擎完成时立即调用回调函数

        Args:
            query: 搜索关键词
            callback: 回调函数，签名为 callback(result_dict)
            page: 页码
            page_size: 每页大小
            engines: 指定引擎列表
            include_deepweb: 是否包含深网搜索

        Returns:
            Result[Dict]: 最终聚合的搜索结果
        """
        if not self._is_connected or not self._core_client:
            return Result(
                success=False,
                error=Error(
                    code="SearchClient.search_streaming", message="Client not connected"
                ),
            )

        try:
            result = self._core_client.search_streaming(
                query,
                callback,
                page,
                page_size,
                engines,
                include_deepweb,
            )
            return Result(success=True, data=result)
        except Exception as e:
            return Result(
                success=False,
                error=Error(
                    code="SearchClient.search_streaming",
                    message=f"Streaming search failed: {e}",
                ),
            )

    def clear_cache(self) -> Result[bool]:
        """清除所有缓存"""
        if not self._is_connected or not self._core_client:
            return Result(
                success=False,
                error=Error(
                    code="SearchClient.clear_cache", message="Client not connected"
                ),
            )

        try:
            self._core_client.clear_cache()
            return Result(success=True, data=True)
        except Exception as e:
            return Result(
                success=False,
                error=Error(
                    code="SearchClient.clear_cache", message=f"Clear cache failed: {e}"
                ),
            )

    def list_engines(self) -> Result[List[str]]:
        """列出所有可用的搜索引擎"""
        if not self._is_connected or not self._core_client:
            return Result(
                success=False,
                error=Error(
                    code="SearchClient.list_engines", message="Client not connected"
                ),
            )

        try:
            engines = self._core_client.list_engines()
            return Result(success=True, data=engines)
        except Exception as e:
            return Result(
                success=False,
                error=Error(
                    code="SearchClient.list_engines",
                    message=f"List engines failed: {e}",
                ),
            )

    def get_stats(self) -> Result[Dict[str, Any]]:
        """获取搜索统计信息"""
        if not self._is_connected or not self._core_client:
            return Result(
                success=False,
                error=Error(
                    code="SearchClient.get_stats", message="Client not connected"
                ),
            )

        try:
            stats_dict = self._core_client.get_stats()
            # 计算缓存命中率
            if stats_dict.get("cache_hits", 0) + stats_dict.get("cache_misses", 0) > 0:
                stats_dict["cache_hit_rate"] = stats_dict["cache_hits"] / (
                    stats_dict["cache_hits"] + stats_dict["cache_misses"]
                )
            else:
                stats_dict["cache_hit_rate"] = 0.0

            return Result(success=True, data=stats_dict)
        except Exception as e:
            return Result(
                success=False,
                error=Error(
                    code="SearchClient.get_stats", message=f"Get stats failed: {e}"
                ),
            )
