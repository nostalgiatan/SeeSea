"""
SeeSea 日期页面客户端

提供统一的日期页面管理接口，基于PyDatePage和PyDatePageObjectPool实现完整功能。
"""

from typing import Dict, Any, Optional, List

from ..base import BaseClient
from ..seesea_types.common_types import Result, Error

try:
    from seesea_core import PyDatePage, PyDatePageObjectPool

    _CORE_AVAILABLE = True
except ImportError:
    _CORE_AVAILABLE = False


class DatePageClient(BaseClient[Dict[str, Any]]):
    """
    SeeSea 日期页面客户端

    提供高层次的日期页面管理接口，支持页面创建、获取和对象池管理。
    继承自BaseClient，提供统一的客户端接口。

    主要功能:
    - 日期页面创建
    - 页面内容获取
    - 对象池管理
    - 批量操作

    示例:
        >>> client = DatePageClient()
        >>> with client:
        ...     result = client.create_page("2024-01-01", "content")
        ...     if result.success:
        ...         print("页面已创建")
    """

    def __init__(self, config: Optional[Dict[str, Any]] = None):
        """初始化日期页面客户端"""
        super().__init__(config)
        self._page: Optional[PyDatePage] = None
        self._pool: Optional[PyDatePageObjectPool] = None

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
            self._page = PyDatePage()
            self._pool = PyDatePageObjectPool()
        except Exception as e:
            error = Error(
                code="DATE_PAGE_INIT_FAILED",
                message=f"日期页面初始化失败: {str(e)}",
                details={"error": str(e)},
            )
            self._error = error

    def create_page(
        self, date: str, content: str, metadata: Optional[Dict[str, Any]] = None
    ) -> "Result[str]":
        """
        创建日期页面

        参数:
            date: 日期字符串（YYYY-MM-DD）
            content: 页面内容
            metadata: 元数据

        返回:
            Result[str]: 页面ID
        """
        if self._page is None:
            return Result.failure_result(self._error)

        try:
            page_id = self._page.create(date, content, metadata or {})
            return Result.success_result(page_id)
        except Exception as e:
            return Result.failure_result(
                Error(
                    code="CREATE_PAGE_FAILED",
                    message=f"创建页面失败: {str(e)}",
                    details={"error": str(e)},
                )
            )

    def get_page(self, date: str) -> "Result[Dict[str, Any]]":
        """
        获取日期页面

        参数:
            date: 日期字符串（YYYY-MM-DD）

        返回:
            Result[Dict[str, Any]]: 页面数据
        """
        if self._page is None:
            return Result.failure_result(self._error)

        try:
            page = self._page.get(date)
            return Result.success_result(page)
        except Exception as e:
            return Result.failure_result(
                Error(
                    code="GET_PAGE_FAILED",
                    message=f"获取页面失败: {str(e)}",
                    details={"error": str(e)},
                )
            )

    def update_page(
        self, date: str, content: str, metadata: Optional[Dict[str, Any]] = None
    ) -> "Result[bool]":
        """
        更新日期页面

        参数:
            date: 日期字符串（YYYY-MM-DD）
            content: 页面内容
            metadata: 元数据

        返回:
            Result[bool]: 操作结果
        """
        if self._page is None:
            return Result.failure_result(self._error)

        try:
            self._page.update(date, content, metadata or {})
            return Result.success_result(True)
        except Exception as e:
            return Result.failure_result(
                Error(
                    code="UPDATE_PAGE_FAILED",
                    message=f"更新页面失败: {str(e)}",
                    details={"error": str(e)},
                )
            )

    def delete_page(self, date: str) -> "Result[bool]":
        """
        删除日期页面

        参数:
            date: 日期字符串（YYYY-MM-DD）

        返回:
            Result[bool]: 操作结果
        """
        if self._page is None:
            return Result.failure_result(self._error)

        try:
            self._page.delete(date)
            return Result.success_result(True)
        except Exception as e:
            return Result.failure_result(
                Error(
                    code="DELETE_PAGE_FAILED",
                    message=f"删除页面失败: {str(e)}",
                    details={"error": str(e)},
                )
            )

    def list_pages(
        self, start_date: Optional[str] = None, end_date: Optional[str] = None
    ) -> "Result[List[Dict[str, Any]]]":
        """
        列出日期页面

        参数:
            start_date: 开始日期（YYYY-MM-DD）
            end_date: 结束日期（YYYY-MM-DD）

        返回:
            Result[List[Dict[str, Any]]]: 页面列表
        """
        if self._page is None:
            return Result.failure_result(self._error)

        try:
            pages = self._page.list(start_date or "", end_date or "")
            return Result.success_result(pages)
        except Exception as e:
            return Result.failure_result(
                Error(
                    code="LIST_PAGES_FAILED",
                    message=f"列出页面失败: {str(e)}",
                    details={"error": str(e)},
                )
            )

    def acquire_page(self, date: str) -> "Result[Dict[str, Any]]":
        """
        从对象池获取页面

        参数:
            date: 日期字符串（YYYY-MM-DD）

        返回:
            Result[Dict[str, Any]]: 页面数据
        """
        if self._pool is None:
            return Result.failure_result(self._error)

        try:
            page = self._pool.acquire(date)
            return Result.success_result(page)
        except Exception as e:
            return Result.failure_result(
                Error(
                    code="ACQUIRE_PAGE_FAILED",
                    message=f"获取页面失败: {str(e)}",
                    details={"error": str(e)},
                )
            )

    def release_page(self, date: str) -> "Result[bool]":
        """
        释放页面到对象池

        参数:
            date: 日期字符串（YYYY-MM-DD）

        返回:
            Result[bool]: 操作结果
        """
        if self._pool is None:
            return Result.failure_result(self._error)

        try:
            self._pool.release(date)
            return Result.success_result(True)
        except Exception as e:
            return Result.failure_result(
                Error(
                    code="RELEASE_PAGE_FAILED",
                    message=f"释放页面失败: {str(e)}",
                    details={"error": str(e)},
                )
            )

    def get_pool_stats(self) -> "Result[Dict[str, Any]]":
        """
        获取对象池统计

        返回:
            Result[Dict[str, Any]]: 统计信息
        """
        if self._pool is None:
            return Result.failure_result(self._error)

        try:
            stats = self._pool.get_stats()
            return Result.success_result(stats)
        except Exception as e:
            return Result.failure_result(
                Error(
                    code="GET_POOL_STATS_FAILED",
                    message=f"获取统计失败: {str(e)}",
                    details={"error": str(e)},
                )
            )
