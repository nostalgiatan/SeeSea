"""
SeeSea 股票调度器模块

提供统一的股票数据调度接口。
"""

from ..seesea_types.common_types import Result, Error

try:
    from seesea_core import (
        stock_start_scheduler as _stock_start_scheduler,
        stock_start_scheduler_with_config as _stock_start_scheduler_with_config,
        stock_stop_scheduler as _stock_stop_scheduler,
    )

    _CORE_AVAILABLE = True
except ImportError:
    _CORE_AVAILABLE = False


class StockScheduler:
    """
    SeeSea 股票调度器

    提供股票数据获取和调度的功能。

    主要功能:
    - 启动调度器
    - 停止调度器
    - 自定义配置

    示例:
        >>> StockScheduler.start()
        >>> # 运行一段时间后
        >>> StockScheduler.stop()
    """

    @staticmethod
    def start() -> "Result[bool]":
        """
        启动股票调度器（默认配置）

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
            _stock_start_scheduler()
            return Result.success_result(True)
        except Exception as e:
            return Result.failure_result(
                Error(
                    code="START_SCHEDULER_FAILED",
                    message=f"启动调度器失败: {str(e)}",
                    details={"error": str(e)},
                )
            )

    @staticmethod
    def start_with_config(
        interval_minutes: int = 60,
        max_retries: int = 3,
        timeout_seconds: int = 60,
        enable_cache: bool = True,
        cache_ttl: int = 3600,
    ) -> "Result[bool]":
        """
        使用自定义配置启动股票调度器

        参数:
            interval_minutes: 调度间隔（分钟）
            max_retries: 最大重试次数
            timeout_seconds: 请求超时时间（秒）
            enable_cache: 是否启用缓存
            cache_ttl: 缓存生存时间（秒）

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
            _stock_start_scheduler_with_config(
                interval_minutes, max_retries, timeout_seconds, enable_cache, cache_ttl
            )
            return Result.success_result(True)
        except Exception as e:
            return Result.failure_result(
                Error(
                    code="START_SCHEDULER_WITH_CONFIG_FAILED",
                    message=f"启动调度器失败: {str(e)}",
                    details={"error": str(e)},
                )
            )

    @staticmethod
    def stop() -> "Result[bool]":
        """
        停止股票调度器

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
            _stock_stop_scheduler()
            return Result.success_result(True)
        except Exception as e:
            return Result.failure_result(
                Error(
                    code="STOP_SCHEDULER_FAILED",
                    message=f"停止调度器失败: {str(e)}",
                    details={"error": str(e)},
                )
            )
