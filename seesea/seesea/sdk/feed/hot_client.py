"""
SeeSea 热点趋势客户端

提供统一的热点数据获取接口，继承自BaseClient。
基于现有PyHotTrendClient实现完整功能。
"""

from typing import Dict, List, Optional, Any
from datetime import datetime

from ..base import BaseClient
from ..seesea_types.common_types import Result, Error

try:
    from seesea_core import PyHotTrendClient

    _CORE_AVAILABLE = True
except ImportError:
    _CORE_AVAILABLE = False


class HotTrendClient(BaseClient[Dict[str, Any]]):
    """
    SeeSea 热点趋势客户端

    提供高层次的热点数据获取功能，支持多平台热点数据聚合。
    继承自BaseClient，提供统一的客户端接口。

    主要功能:
    - 单平台热点数据获取
    - 多平台热点数据聚合
    - 平台列表管理
    - 并发数据获取
    - 数据格式化和过滤

    示例:
        >>> client = HotTrendClient()
        >>> with client:
        ...     result = client.fetch_platform("weibo")
        ...     if result.success:
        ...         hot_data = result.data
        ...         print(f"获取到 {len(hot_data['items'])} 条热点")
    """

    def __init__(
        self, config: Optional[Dict[str, Any]] = None, max_concurrency: int = 10
    ):
        """
        初始化热点趋势客户端

        Args:
            config: 客户端配置
            max_concurrency: 最大并发数 (默认: 10)
        """
        super().__init__(config)
        self.max_concurrency = max_concurrency
        self._core_client: Optional[PyHotTrendClient] = None

        if not _CORE_AVAILABLE:
            error = Error(
                code="HotTrendClient.init",
                message="seesea_core not available. Please install SeeSea core modules.",
                source="HotTrendClient",
            )
            self._set_error(error)

    def connect(self) -> Result[bool]:
        """连接到热点趋势服务"""
        with self._handle_operation("connect"):
            if not _CORE_AVAILABLE:
                return Result(
                    success=False,
                    error=Error(
                        code="HotTrendClient.connect",
                        message="Core modules not available",
                    ),
                )

            try:
                self._core_client = PyHotTrendClient(self.max_concurrency)
                self._is_connected = True
                return Result(success=True, data=True)
            except Exception as e:
                return Result(
                    success=False,
                    error=Error(
                        code="HotTrendClient.connect",
                        message=f"Failed to initialize core client: {e}",
                    ),
                )

    def disconnect(self) -> Result[bool]:
        """断开热点趋势服务连接"""
        with self._handle_operation("disconnect"):
            self._core_client = None
            self._is_connected = False
            return Result(success=True, data=True)

    def health_check(self) -> Result[Dict[str, Any]]:
        """热点趋势服务健康检查"""
        if not self._is_connected or not self._core_client:
            return Result(
                success=False,
                error=Error(
                    code="HotTrendClient.health_check", message="Client not connected"
                ),
            )

        try:
            platforms = self._core_client.list_platforms()
            health_data = {
                "status": "healthy",
                "core_available": _CORE_AVAILABLE,
                "available_platforms": platforms,
                "platforms_count": len(platforms),
                "max_concurrency": self.max_concurrency,
            }
            return Result(success=True, data=health_data)
        except Exception as e:
            return Result(
                success=False,
                error=Error(
                    code="HotTrendClient.health_check",
                    message=f"Health check failed: {e}",
                ),
            )

    def get_info(self) -> Result[Dict[str, Any]]:
        """获取热点趋势客户端信息"""
        info = {
            "client_type": "HotTrendClient",
            "version": "0.1.0",
            "core_available": _CORE_AVAILABLE,
            "connected": self._is_connected,
            "max_concurrency": self.max_concurrency,
        }

        if self._is_connected and self._core_client:
            try:
                platforms = self._core_client.list_platforms()
                info["available_platforms"] = platforms
                info["platforms_count"] = len(platforms)
            except Exception:
                info["platforms_count"] = 0

        return Result(success=True, data=info)

    def fetch_platform(self, platform_id: str) -> Result[Dict[str, Any]]:
        """
        获取单个平台的热点数据

        Args:
            platform_id: 平台ID

        Returns:
            Result[Dict]: 平台热点数据
        """
        if not self._is_connected or not self._core_client:
            return Result(
                success=False,
                error=Error(
                    code="HotTrendClient.fetch_platform", message="Client not connected"
                ),
            )

        if not platform_id:
            return Result(
                success=False,
                error=Error(
                    code="HotTrendClient.fetch_platform",
                    message="platform_id cannot be empty",
                ),
            )

        with self._handle_operation("fetch_platform"):
            try:
                start_time = datetime.now()

                result = self._core_client.fetch_platform(platform_id)

                # 确保结果包含items字段
                if "items" not in result:
                    result["items"] = []

                end_time = datetime.now()
                took_ms = int((end_time - start_time).total_seconds() * 1000)

                return Result(success=True, data=result, took_ms=took_ms)

            except Exception as e:
                return Result(
                    success=False,
                    error=Error(
                        code="HotTrendClient.fetch_platform",
                        message=f"Failed to fetch platform data: {e}",
                    ),
                )

    def fetch_all_platforms(self) -> Result[List[Dict[str, Any]]]:
        """
        获取所有支持平台的热点数据

        Returns:
            Result[List[Dict]]: 所有平台热点数据列表
        """
        if not self._is_connected or not self._core_client:
            return Result(
                success=False,
                error=Error(
                    code="HotTrendClient.fetch_all_platforms",
                    message="Client not connected",
                ),
            )

        with self._handle_operation("fetch_all_platforms"):
            try:
                start_time = datetime.now()

                results = self._core_client.fetch_all_platforms()

                end_time = datetime.now()
                took_ms = int((end_time - start_time).total_seconds() * 1000)

                return Result(success=True, data=results, took_ms=took_ms)

            except Exception as e:
                return Result(
                    success=False,
                    error=Error(
                        code="HotTrendClient.fetch_all_platforms",
                        message=f"Failed to fetch all platforms data: {e}",
                    ),
                )

    def fetch_multiple_platforms(
        self, platform_ids: List[str]
    ) -> Result[List[Dict[str, Any]]]:
        """
        批量获取多个平台的热点数据

        Args:
            platform_ids: 平台ID列表

        Returns:
            Result[List[Dict]]: 多个平台热点数据列表
        """
        if not self._is_connected or not self._core_client:
            return Result(
                success=False,
                error=Error(
                    code="HotTrendClient.fetch_multiple_platforms",
                    message="Client not connected",
                ),
            )

        if not platform_ids:
            return Result(
                success=False,
                error=Error(
                    code="HotTrendClient.fetch_multiple_platforms",
                    message="platform_ids cannot be empty",
                ),
            )

        with self._handle_operation("fetch_multiple_platforms"):
            try:
                start_time = datetime.now()

                results = []
                for platform_id in platform_ids:
                    platform_result = self.fetch_platform(platform_id)
                    if platform_result.success and platform_result.data:
                        results.append(platform_result.data)
                    else:
                        # 记录错误但继续处理其他平台
                        error_data = {
                            "platform_id": platform_id,
                            "error": (
                                platform_result.error.message
                                if platform_result.error
                                else "Unknown error"
                            ),
                            "items": [],
                        }
                        results.append(error_data)

                end_time = datetime.now()
                took_ms = int((end_time - start_time).total_seconds() * 1000)

                return Result(success=True, data=results, took_ms=took_ms)

            except Exception as e:
                return Result(
                    success=False,
                    error=Error(
                        code="HotTrendClient.fetch_multiple_platforms",
                        message=f"Failed to fetch multiple platforms data: {e}",
                    ),
                )

    def list_platforms(self) -> Result[Dict[str, str]]:
        """
        获取所有支持的平台列表

        Returns:
            Result[Dict[str, str]]: 平台ID到平台名称的映射
        """
        if not self._is_connected or not self._core_client:
            return Result(
                success=False,
                error=Error(
                    code="HotTrendClient.list_platforms", message="Client not connected"
                ),
            )

        try:
            platforms = self._core_client.list_platforms()
            return Result(success=True, data=platforms)
        except Exception as e:
            return Result(
                success=False,
                error=Error(
                    code="HotTrendClient.list_platforms",
                    message=f"Failed to list platforms: {e}",
                ),
            )

    def get_platform_info(self, platform_id: str) -> Result[Dict[str, Any]]:
        """获取单个平台的信息"""
        platforms_result = self.list_platforms()
        if not platforms_result.success:
            return Result(success=False, error=platforms_result.error)

        platforms = platforms_result.data
        if not platforms or platform_id not in platforms:
            return Result(
                success=False,
                error=Error(
                    code="HotTrendClient.get_platform_info",
                    message=f"Platform '{platform_id}' not found",
                ),
            )

        platform_info = {
            "platform_id": platform_id,
            "platform_name": platforms[platform_id],
            "available": True,
        }
        return Result(success=True, data=platform_info)

    def search_platforms(self, query: str) -> Result[Dict[str, str]]:
        """根据查询字符串搜索平台"""
        platforms_result = self.list_platforms()
        if not platforms_result.success:
            return Result(success=False, error=platforms_result.error)

        platforms = platforms_result.data
        if not platforms:
            return Result(success=True, data={})

        query_lower = query.lower()

        filtered_platforms = {
            pid: name
            for pid, name in platforms.items()
            if query_lower in pid.lower() or query_lower in name.lower()
        }

        return Result(success=True, data=filtered_platforms)
