"""
SeeSea 配置辅助模块

提供统一的配置辅助函数。
"""

from pathlib import Path
from ..seesea_types.common_types import Result, Error

try:
    from seesea_core import (
        init_config as _init_config,
        get_cache_dir as _get_cache_dir,
        get_data_dir as _get_data_dir,
        get_config_dir as _get_config_dir,
        get_log_dir as _get_log_dir,
    )

    _CORE_AVAILABLE = True
except ImportError:
    _CORE_AVAILABLE = False


class ConfigHelper:
    """
    SeeSea 配置辅助器

    提供配置初始化和路径查询功能。

    主要功能:
    - 初始化配置
    - 获取缓存目录
    - 获取数据目录
    - 获取配置目录
    - 获取日志目录

    示例:
        >>> ConfigHelper.init()
        >>> cache_dir = ConfigHelper.get_cache_dir()
        >>> print(f"缓存目录: {cache_dir}")
    """

    @staticmethod
    def init() -> "Result[bool]":
        """
        初始化配置

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
            _init_config()
            return Result.success_result(True)
        except Exception as e:
            return Result.failure_result(
                Error(
                    code="INIT_CONFIG_FAILED",
                    message=f"初始化配置失败: {str(e)}",
                    details={"error": str(e)},
                )
            )

    @staticmethod
    def get_cache_dir() -> "Result[Path]":
        """
        获取缓存目录

        返回:
            Result[Path]: 缓存目录路径
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
            cache_dir = _get_cache_dir()
            return Result.success_result(Path(cache_dir))
        except Exception as e:
            return Result.failure_result(
                Error(
                    code="GET_CACHE_DIR_FAILED",
                    message=f"获取缓存目录失败: {str(e)}",
                    details={"error": str(e)},
                )
            )

    @staticmethod
    def get_data_dir() -> "Result[Path]":
        """
        获取数据目录

        返回:
            Result[Path]: 数据目录路径
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
            data_dir = _get_data_dir()
            return Result.success_result(Path(data_dir))
        except Exception as e:
            return Result.failure_result(
                Error(
                    code="GET_DATA_DIR_FAILED",
                    message=f"获取数据目录失败: {str(e)}",
                    details={"error": str(e)},
                )
            )

    @staticmethod
    def get_config_dir() -> "Result[Path]":
        """
        获取配置目录

        返回:
            Result[Path]: 配置目录路径
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
            config_dir = _get_config_dir()
            return Result.success_result(Path(config_dir))
        except Exception as e:
            return Result.failure_result(
                Error(
                    code="GET_CONFIG_DIR_FAILED",
                    message=f"获取配置目录失败: {str(e)}",
                    details={"error": str(e)},
                )
            )

    @staticmethod
    def get_log_dir() -> "Result[Path]":
        """
        获取日志目录

        返回:
            Result[Path]: 日志目录路径
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
            log_dir = _get_log_dir()
            return Result.success_result(Path(log_dir))
        except Exception as e:
            return Result.failure_result(
                Error(
                    code="GET_LOG_DIR_FAILED",
                    message=f"获取日志目录失败: {str(e)}",
                    details={"error": str(e)},
                )
            )

    @staticmethod
    def ensure_dirs() -> "Result[bool]":
        """
        确保所有目录存在

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
            dirs = [
                ConfigHelper.get_cache_dir(),
                ConfigHelper.get_data_dir(),
                ConfigHelper.get_config_dir(),
                ConfigHelper.get_log_dir(),
            ]

            for dir_result in dirs:
                if dir_result.success and dir_result.data:
                    dir_result.data.mkdir(parents=True, exist_ok=True)

            return Result.success_result(True)
        except Exception as e:
            return Result.failure_result(
                Error(
                    code="ENSURE_DIRS_FAILED",
                    message=f"确保目录失败: {str(e)}",
                    details={"error": str(e)},
                )
            )
