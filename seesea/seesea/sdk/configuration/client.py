"""
SeeSea 配置客户端

提供统一的配置管理接口，继承自BaseClient。
基于现有PyConfig实现完整功能。
"""

from typing import Dict, Any, Optional

from ..base import BaseClient
from ..seesea_types.common_types import Result, Error

try:
    from seesea_core import PyConfig

    _CORE_AVAILABLE = True
except ImportError:
    _CORE_AVAILABLE = False


class ConfigClient(BaseClient[Dict[str, Any]]):
    """
    SeeSea 配置客户端

    提供高层次的配置管理接口，支持配置读取、修改和持久化。
    继承自BaseClient，提供统一的客户端接口。

    主要功能:
    - 全局配置管理
    - 调试模式控制
    - 结果数量限制
    - 超时设置
    - 配置持久化支持

    示例:
        >>> client = ConfigClient()
        >>> with client:
        ...     result = client.set_debug(True)
        ...     if result.success:
        ...         print("调试模式已启用")
    """

    def __init__(self, config: Optional[Dict[str, Any]] = None):
        """初始化配置客户端"""
        super().__init__(config)
        self._core_config: Optional[PyConfig] = None

        if not _CORE_AVAILABLE:
            error = Error(
                code="ConfigClient.init",
                message="seesea_core not available. Please install SeeSea core modules.",
                source="ConfigClient",
            )
            self._set_error(error)

    def connect(self) -> Result[bool]:
        """连接到配置服务"""
        with self._handle_operation("connect"):
            if not _CORE_AVAILABLE:
                return Result(
                    success=False,
                    error=Error(
                        code="ConfigClient.connect",
                        message="Core modules not available",
                    ),
                )

            try:
                self._core_config = PyConfig()
                self._is_connected = True
                return Result(success=True, data=True)
            except Exception as e:
                return Result(
                    success=False,
                    error=Error(
                        code="ConfigClient.connect",
                        message=f"Failed to initialize core config: {e}",
                    ),
                )

    def disconnect(self) -> Result[bool]:
        """断开配置服务连接"""
        with self._handle_operation("disconnect"):
            self._core_config = None
            self._is_connected = False
            return Result(success=True, data=True)

    def health_check(self) -> Result[Dict[str, Any]]:
        """配置服务健康检查"""
        if not self._is_connected or not self._core_config:
            return Result(
                success=False,
                error=Error(
                    code="ConfigClient.health_check", message="Client not connected"
                ),
            )

        try:
            health_data = {
                "status": "healthy",
                "core_available": _CORE_AVAILABLE,
                "config_loaded": True,
                "current_settings": {
                    "debug": self._core_config.debug,
                    "max_results": self._core_config.max_results,
                    "timeout_seconds": self._core_config.timeout_seconds,
                },
            }
            return Result(success=True, data=health_data)
        except Exception as e:
            return Result(
                success=False,
                error=Error(
                    code="ConfigClient.health_check",
                    message=f"Health check failed: {e}",
                ),
            )

    def get_info(self) -> Result[Dict[str, Any]]:
        """获取配置客户端信息"""
        info = {
            "client_type": "ConfigClient",
            "version": "0.1.0",
            "core_available": _CORE_AVAILABLE,
            "connected": self._is_connected,
        }

        if self._is_connected and self._core_config:
            try:
                info["config"] = {
                    "debug": self._core_config.debug,
                    "max_results": self._core_config.max_results,
                    "timeout_seconds": self._core_config.timeout_seconds,
                }
            except Exception:
                info["config"] = "unavailable"

        return Result(success=True, data=info)

    def get_debug(self) -> Result[bool]:
        """获取调试模式状态"""
        if not self._is_connected or not self._core_config:
            return Result(
                success=False,
                error=Error(
                    code="ConfigClient.get_debug", message="Client not connected"
                ),
            )

        try:
            debug = self._core_config.debug
            return Result(success=True, data=debug)
        except Exception as e:
            return Result(
                success=False,
                error=Error(
                    code="ConfigClient.get_debug", message=f"Failed to get debug: {e}"
                ),
            )

    def set_debug(self, value: bool) -> Result[bool]:
        """设置调试模式"""
        if not self._is_connected or not self._core_config:
            return Result(
                success=False,
                error=Error(
                    code="ConfigClient.set_debug", message="Client not connected"
                ),
            )

        try:
            self._core_config.debug = value
            return Result(success=True, data=True)
        except Exception as e:
            return Result(
                success=False,
                error=Error(
                    code="ConfigClient.set_debug", message=f"Failed to set debug: {e}"
                ),
            )

    def get_max_results(self) -> Result[int]:
        """获取最大结果数"""
        if not self._is_connected or not self._core_config:
            return Result(
                success=False,
                error=Error(
                    code="ConfigClient.get_max_results", message="Client not connected"
                ),
            )

        try:
            max_results = self._core_config.max_results
            return Result(success=True, data=max_results)
        except Exception as e:
            return Result(
                success=False,
                error=Error(
                    code="ConfigClient.get_max_results",
                    message=f"Failed to get max_results: {e}",
                ),
            )

    def set_max_results(self, value: int) -> Result[bool]:
        """设置最大结果数"""
        if not self._is_connected or not self._core_config:
            return Result(
                success=False,
                error=Error(
                    code="ConfigClient.set_max_results", message="Client not connected"
                ),
            )

        if value <= 0:
            return Result(
                success=False,
                error=Error(
                    code="ConfigClient.set_max_results",
                    message="max_results must be greater than 0",
                ),
            )

        try:
            self._core_config.max_results = value
            return Result(success=True, data=True)
        except Exception as e:
            return Result(
                success=False,
                error=Error(
                    code="ConfigClient.set_max_results",
                    message=f"Failed to set max_results: {e}",
                ),
            )

    def get_timeout_seconds(self) -> Result[int]:
        """获取超时时间"""
        if not self._is_connected or not self._core_config:
            return Result(
                success=False,
                error=Error(
                    code="ConfigClient.get_timeout_seconds",
                    message="Client not connected",
                ),
            )

        try:
            timeout_seconds = self._core_config.timeout_seconds
            return Result(success=True, data=timeout_seconds)
        except Exception as e:
            return Result(
                success=False,
                error=Error(
                    code="ConfigClient.get_timeout_seconds",
                    message=f"Failed to get timeout_seconds: {e}",
                ),
            )

    def set_timeout_seconds(self, value: int) -> Result[bool]:
        """设置超时时间"""
        if not self._is_connected or not self._core_config:
            return Result(
                success=False,
                error=Error(
                    code="ConfigClient.set_timeout_seconds",
                    message="Client not connected",
                ),
            )

        if value <= 0:
            return Result(
                success=False,
                error=Error(
                    code="ConfigClient.set_timeout_seconds",
                    message="timeout_seconds must be greater than 0",
                ),
            )

        try:
            self._core_config.timeout_seconds = value
            return Result(success=True, data=True)
        except Exception as e:
            return Result(
                success=False,
                error=Error(
                    code="ConfigClient.set_timeout_seconds",
                    message=f"Failed to set timeout_seconds: {e}",
                ),
            )

    def get_all_config(self) -> Result[Dict[str, Any]]:
        """获取所有配置"""
        if not self._is_connected or not self._core_config:
            return Result(
                success=False,
                error=Error(
                    code="ConfigClient.get_all_config", message="Client not connected"
                ),
            )

        try:
            config = {
                "debug": self._core_config.debug,
                "max_results": self._core_config.max_results,
                "timeout_seconds": self._core_config.timeout_seconds,
            }
            return Result(success=True, data=config)
        except Exception as e:
            return Result(
                success=False,
                error=Error(
                    code="ConfigClient.get_all_config",
                    message=f"Failed to get all config: {e}",
                ),
            )

    def update_config(self, config: Dict[str, Any]) -> Result[bool]:
        """批量更新配置"""
        if not self._is_connected or not self._core_config:
            return Result(
                success=False,
                error=Error(
                    code="ConfigClient.update_config", message="Client not connected"
                ),
            )

        try:
            # 验证配置项
            valid_keys = {"debug", "max_results", "timeout_seconds"}
            for key in config:
                if key not in valid_keys:
                    return Result(
                        success=False,
                        error=Error(
                            code="ConfigClient.update_config",
                            message=f"Invalid config key: {key}",
                        ),
                    )

            # 应用配置
            for key, value in config.items():
                if key == "debug" and isinstance(value, bool):
                    self._core_config.debug = value
                elif key == "max_results" and isinstance(value, int) and value > 0:
                    self._core_config.max_results = value
                elif key == "timeout_seconds" and isinstance(value, int) and value > 0:
                    self._core_config.timeout_seconds = value
                else:
                    return Result(
                        success=False,
                        error=Error(
                            code="ConfigClient.update_config",
                            message=f"Invalid value for {key}: {value}",
                        ),
                    )

            return Result(success=True, data=True)
        except Exception as e:
            return Result(
                success=False,
                error=Error(
                    code="ConfigClient.update_config",
                    message=f"Failed to update config: {e}",
                ),
            )

    def reset_to_defaults(self) -> Result[bool]:
        """重置配置为默认值"""
        try:
            default_config = {"debug": False, "max_results": 50, "timeout_seconds": 10}
            return self.update_config(default_config)
        except Exception as e:
            return Result(
                success=False,
                error=Error(
                    code="ConfigClient.reset_to_defaults",
                    message=f"Failed to reset config: {e}",
                ),
            )
