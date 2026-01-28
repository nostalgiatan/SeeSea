"""
SeeSea 缓存客户端

提供统一的缓存管理接口，基于PyCache实现完整功能。
"""

from typing import Dict, Any, Optional, List

from ..base.client import BaseClient
from ..seesea_types.common_types import Result, Error, Timestamp

try:
    from seesea_core import PyCacheInterface

    _CORE_AVAILABLE = True
except ImportError:
    _CORE_AVAILABLE = False


class CacheClient(BaseClient[Dict[str, Any]]):
    """
    SeeSea 缓存客户端

    提供高层次的缓存管理接口，支持数据存储、检索和统计。
    继承自BaseClient，提供统一的客户端接口。

    主要功能:
    - 键值存储
    - TTL管理
    - 缓存统计
    - 批量操作

    示例:
        >>> client = CacheClient()
        >>> result = client.connect()
        >>> if result.success:
        ...     client.set("key", "value", ttl=3600)
        ...     value = client.get("key")
    """

    def __init__(self, config: Optional[Dict[str, Any]] = None):
        """初始化缓存客户端"""
        super().__init__(config)
        self._cache: Optional[PyCacheInterface] = None
        self._scope_name: str = "default"

        if not _CORE_AVAILABLE:
            error = Error(
                code="CORE_NOT_AVAILABLE",
                message="SeeSea核心模块不可用",
                details={
                    "error": "请确保已正确安装seesea_core",
                },
            )
            self._set_error(error)
            return

    def connect(self) -> Result[bool]:
        """连接到缓存服务"""
        with self._handle_operation("connect"):
            if not _CORE_AVAILABLE:
                return Result.failure_result(
                    Error(
                        code="CacheClient.connect",
                        message="Core modules not available",
                    )
                )

            try:
                self._cache = PyCacheInterface()
                self._is_connected = True
                return Result.success_result(True)
            except Exception as e:
                return Result.failure_result(
                    Error(
                        code="CacheClient.connect",
                        message=f"Failed to initialize cache: {e}",
                    )
                )

    def disconnect(self) -> Result[bool]:
        """断开缓存服务连接"""
        with self._handle_operation("disconnect"):
            self._cache = None
            self._is_connected = False
            return Result.success_result(True)

    def health_check(self) -> Result[Dict[str, Any]]:
        """缓存服务健康检查"""
        if not self._is_connected or not self._cache:
            return Result.failure_result(
                Error(
                    code="CacheClient.health_check",
                    message="Client not connected",
                )
            )

        try:
            return Result.success_result(
                {
                    "status": "healthy",
                    "timestamp": Timestamp.now().value,
                    "scope": self._scope_name,
                }
            )
        except Exception as e:
            return Result.failure_result(
                Error(
                    code="CacheClient.health_check",
                    message=f"Health check failed: {e}",
                )
            )

    def get_info(self) -> Result[Dict[str, Any]]:
        """获取缓存客户端信息"""
        info = {
            "client_type": "CacheClient",
            "version": "0.1.0",
            "core_available": _CORE_AVAILABLE,
            "connected": self._is_connected,
            "scope": self._scope_name,
        }

        return Result.success_result(info)

    def _get_scope_cache(self):
        """获取指定作用域的缓存访问器"""
        if not self._cache:
            return None
        return self._cache.scope(self._scope_name)

    def set(
        self,
        key: str,
        value: Any,
        ttl: Optional[int] = None,
    ) -> Result[bool]:
        """
        设置缓存值

        参数:
            key: 缓存键
            value: 缓存值
            ttl: 生存时间（秒），None表示永不过期

        返回:
            Result[bool]: 操作结果
        """
        scope_cache = self._get_scope_cache()
        if scope_cache is None:
            return Result.failure_result(
                self.last_error
                if self.last_error
                else Error(
                    code="CACHE_SET_FAILED",
                    message="缓存未初始化",
                )
            )

        try:
            import pickle

            serialized_value = pickle.dumps(value)
            scope_cache.set(key, serialized_value, ttl)
            return Result.success_result(True)
        except Exception as e:
            return Result.failure_result(
                Error(
                    code="CACHE_SET_FAILED",
                    message=f"设置缓存失败: {str(e)}",
                    details={"error": str(e)},
                )
            )

    def get(self, key: str) -> Result[Any]:
        """
        获取缓存值

        参数:
            key: 缓存键

        返回:
            Result[Any]: 缓存值
        """
        scope_cache = self._get_scope_cache()
        if scope_cache is None:
            return Result.failure_result(
                self.last_error
                if self.last_error
                else Error(
                    code="CACHE_GET_FAILED",
                    message="缓存未初始化",
                )
            )

        try:
            value = scope_cache.get(key)
            if value is None:
                return Result.failure_result(
                    Error(
                        code="CACHE_MISS",
                        message="缓存未命中",
                        details={
                            "error": f"键 '{key}' 不存在或已过期",
                        },
                    )
                )
            import pickle

            deserialized_value = pickle.loads(value)
            return Result.success_result(deserialized_value)
        except Exception as e:
            return Result.failure_result(
                Error(
                    code="CACHE_GET_FAILED",
                    message=f"获取缓存失败: {str(e)}",
                    details={"error": str(e)},
                )
            )

    def delete(self, key: str) -> Result[bool]:
        """
        删除缓存值

        参数:
            key: 缓存键

        返回:
            Result[bool]: 操作结果
        """
        scope_cache = self._get_scope_cache()
        if scope_cache is None:
            return Result.failure_result(
                self.last_error
                if self.last_error
                else Error(
                    code="CACHE_DELETE_FAILED",
                    message="缓存未初始化",
                )
            )

        try:
            scope_cache.delete(key)
            return Result.success_result(True)
        except Exception as e:
            return Result.failure_result(
                Error(
                    code="CACHE_DELETE_FAILED",
                    message=f"删除缓存失败: {str(e)}",
                    details={"error": str(e)},
                )
            )

    def exists(self, key: str) -> Result[bool]:
        """
        检查缓存是否存在

        参数:
            key: 缓存键

        返回:
            Result[bool]: 是否存在
        """
        scope_cache = self._get_scope_cache()
        if scope_cache is None:
            return Result.failure_result(
                self.last_error
                if self.last_error
                else Error(
                    code="CACHE_CHECK_FAILED",
                    message="缓存未初始化",
                )
            )

        try:
            exists = scope_cache.exists(key)
            return Result.success_result(exists)
        except Exception as e:
            return Result.failure_result(
                Error(
                    code="CACHE_CHECK_FAILED",
                    message=f"检查缓存失败: {str(e)}",
                    details={"error": str(e)},
                )
            )

    def ttl(self, key: str) -> Result[int]:
        """
        获取缓存剩余生存时间

        参数:
            key: 缓存键

        返回:
            Result[int]: 剩余秒数，-1表示永不过期，-2表示键不存在
        """
        scope_cache = self._get_scope_cache()
        if scope_cache is None:
            return Result.failure_result(
                self.last_error
                if self.last_error
                else Error(
                    code="CACHE_TTL_FAILED",
                    message="缓存未初始化",
                )
            )

        try:
            value = scope_cache.get(key)
            if value is None:
                return Result.success_result(-2)
            return Result.success_result(-1)  # 暂时返回 -1，实际实现需要支持 TTL 查询
        except Exception as e:
            return Result.failure_result(
                Error(
                    code="CACHE_TTL_FAILED",
                    message=f"获取TTL失败: {str(e)}",
                    details={"error": str(e)},
                )
            )

    def keys(self, pattern: str = "*") -> Result[List[str]]:
        """
        列出匹配模式的缓存键

        参数:
            pattern: 匹配模式，支持通配符

        返回:
            Result[List[str]]: 键列表
        """
        # PyScopeCache 不直接支持 keys 方法，返回空列表
        return Result.success_result([])

    def mset(self, mapping: Dict[str, Any]) -> Result[bool]:
        """
        批量设置缓存值

        参数:
            mapping: 键值对字典

        返回:
            Result[bool]: 操作结果
        """
        scope_cache = self._get_scope_cache()
        if scope_cache is None:
            return Result.failure_result(
                self.last_error
                if self.last_error
                else Error(
                    code="CACHE_MSET_FAILED",
                    message="缓存未初始化",
                )
            )

        try:
            import pickle

            for key, value in mapping.items():
                serialized_value = pickle.dumps(value)
                scope_cache.set(key, serialized_value, None)
            return Result.success_result(True)
        except Exception as e:
            return Result.failure_result(
                Error(
                    code="CACHE_MSET_FAILED",
                    message=f"批量设置失败: {str(e)}",
                    details={"error": str(e)},
                )
            )

    def mget(self, keys: List[str]) -> Result[List[Any]]:
        """
        批量获取缓存值

        参数:
            keys: 键列表

        返回:
            Result[List[Any]]: 值列表
        """
        scope_cache = self._get_scope_cache()
        if scope_cache is None:
            return Result.failure_result(
                self.last_error
                if self.last_error
                else Error(
                    code="CACHE_MGET_FAILED",
                    message="缓存未初始化",
                )
            )

        try:
            import pickle

            values = []
            for key in keys:
                value = scope_cache.get(key)
                if value is not None:
                    deserialized_value = pickle.loads(value)
                    values.append(deserialized_value)
                else:
                    values.append(None)
            return Result.success_result(values)
        except Exception as e:
            return Result.failure_result(
                Error(
                    code="CACHE_MGET_FAILED",
                    message=f"批量获取失败: {str(e)}",
                    details={"error": str(e)},
                )
            )

    def clear(self) -> Result[bool]:
        """
        清空所有缓存

        返回:
            Result[bool]: 操作结果
        """
        if not self._cache:
            return Result.failure_result(
                self.last_error
                if self.last_error
                else Error(
                    code="CACHE_CLEAR_FAILED",
                    message="缓存未初始化",
                )
            )

        try:
            self._cache.clear_all()
            return Result.success_result(True)
        except Exception as e:
            return Result.failure_result(
                Error(
                    code="CACHE_CLEAR_FAILED",
                    message=f"清空缓存失败: {str(e)}",
                    details={"error": str(e)},
                )
            )

    def get_stats(self) -> Result[Dict[str, Any]]:
        """
        获取缓存统计信息

        返回:
            Result[Dict[str, Any]]: 缓存统计
        """
        if not self._cache:
            return Result.failure_result(
                self.last_error
                if self.last_error
                else Error(
                    code="CACHE_STATS_FAILED",
                    message="缓存未初始化",
                )
            )

        try:
            stats = self._cache.get_stats()
            return Result.success_result(stats)
        except Exception as e:
            return Result.failure_result(
                Error(
                    code="CACHE_STATS_FAILED",
                    message=f"获取统计失败: {str(e)}",
                    details={"error": str(e)},
                )
            )
