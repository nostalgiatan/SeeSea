"""
SeeSea SDK 基础客户端抽象类
"""

from abc import ABC, abstractmethod
from typing import Any, Dict, Optional, Generic, TypeVar
from contextlib import asynccontextmanager, contextmanager

from ..seesea_types.common_types import Result, Error, Status, Timestamp

T = TypeVar("T")


class BaseClient(ABC, Generic[T]):
    """
    SeeSea 基础客户端抽象类

    所有 SeeSea 客户端都应该继承此类，提供统一的接口结构。
    """

    def __init__(self, config: Optional[Dict[str, Any]] = None):
        """初始化客户端"""
        self._config = config or {}
        self._is_connected = False
        self._last_error: Optional[Error] = None
        self._status = Status.PENDING

    @property
    def config(self) -> Dict[str, Any]:
        """获取配置"""
        return self._config.copy()

    @property
    def is_connected(self) -> bool:
        """检查连接状态"""
        return self._is_connected

    @property
    def status(self) -> Status:
        """获取客户端状态"""
        return self._status

    @property
    def last_error(self) -> Optional[Error]:
        """获取最后一次错误"""
        return self._last_error

    @abstractmethod
    def connect(self) -> Result[bool]:
        """连接到服务"""
        pass

    @abstractmethod
    def disconnect(self) -> Result[bool]:
        """断开连接"""
        pass

    @abstractmethod
    def health_check(self) -> Result[Dict[str, Any]]:
        """健康检查"""
        pass

    @abstractmethod
    def get_info(self) -> Result[Dict[str, Any]]:
        """获取客户端信息"""
        pass

    def _set_error(self, error: Error) -> None:
        """设置错误状态"""
        self._last_error = error
        self._status = Status.FAILED

    def _clear_error(self) -> None:
        """清除错误状态"""
        self._last_error = None
        if self._status == Status.FAILED:
            self._status = Status.SUCCESS

    @contextmanager
    def _handle_operation(self, operation_name: str):
        """操作上下文管理器"""
        self._status = Status.RUNNING
        try:
            yield
            self._clear_error()
        except Exception as e:
            error = Error(
                code=f"{self.__class__.__name__}.{operation_name}",
                message=str(e),
                timestamp=Timestamp.now().value,
            )
            self._set_error(error)
            raise

    def __enter__(self):
        """上下文管理器入口"""
        result = self.connect()
        if not result.success:
            raise RuntimeError(f"Failed to connect: {result.error}")
        return self

    def __exit__(self, exc_type, exc_val, exc_tb):
        """上下文管理器出口"""
        self.disconnect()


class AsyncBaseClient(ABC, Generic[T]):
    """
    SeeSea 异步基础客户端抽象类
    """

    def __init__(self, config: Optional[Dict[str, Any]] = None):
        """初始化异步客户端"""
        self._config = config or {}
        self._is_connected = False
        self._last_error: Optional[Error] = None
        self._status = Status.PENDING

    @property
    def config(self) -> Dict[str, Any]:
        """获取配置"""
        return self._config.copy()

    @property
    def is_connected(self) -> bool:
        """检查连接状态"""
        return self._is_connected

    @property
    def status(self) -> Status:
        """获取客户端状态"""
        return self._status

    @property
    def last_error(self) -> Optional[Error]:
        """获取最后一次错误"""
        return self._last_error

    @abstractmethod
    async def connect(self) -> Result[bool]:
        """异步连接到服务"""
        pass

    @abstractmethod
    async def disconnect(self) -> Result[bool]:
        """异步断开连接"""
        pass

    @abstractmethod
    async def health_check(self) -> Result[Dict[str, Any]]:
        """异步健康检查"""
        pass

    @abstractmethod
    async def get_info(self) -> Result[Dict[str, Any]]:
        """异步获取客户端信息"""
        pass

    def _set_error(self, error: Error) -> None:
        """设置错误状态"""
        self._last_error = error
        self._status = Status.FAILED

    def _clear_error(self) -> None:
        """清除错误状态"""
        self._last_error = None
        if self._status == Status.FAILED:
            self._status = Status.SUCCESS

    @asynccontextmanager
    async def _handle_operation(self, operation_name: str):
        """异步操作上下文管理器"""
        self._status = Status.RUNNING
        try:
            yield
            self._clear_error()
        except Exception as e:
            error = Error(
                code=f"{self.__class__.__name__}.{operation_name}",
                message=str(e),
                timestamp=Timestamp.now().value,
            )
            self._set_error(error)
            raise

    async def __aenter__(self):
        """异步上下文管理器入口"""
        result = await self.connect()
        if not result.success:
            raise RuntimeError(f"Failed to connect: {result.error}")
        return self

    async def __aexit__(self, exc_type, exc_val, exc_tb):
        """异步上下文管理器出口"""
        await self.disconnect()
