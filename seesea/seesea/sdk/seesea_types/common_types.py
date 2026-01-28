"""
通用类型定义
"""

from typing import Dict, Optional, Any, Literal, TypeVar, Generic
from dataclasses import dataclass
from datetime import datetime
from enum import Enum

T = TypeVar("T")

__all__ = [
    "T",
    "Result",
    "Error",
    "Status",
    "Timestamp",
    "PageInfo",
    "HealthCheck",
    "ResourceUsage",
    "LogLevel",
    "LogEntry",
]


@dataclass
class Result(Generic[T]):
    """通用结果类型"""

    success: bool
    data: Optional[T] = None
    error: Optional["Error"] = None
    message: Optional[str] = None
    took_ms: Optional[int] = None

    @classmethod
    def success_result(
        cls, data: T, message: Optional[str] = None, took_ms: Optional[int] = None
    ) -> "Result[T]":
        """创建成功结果"""
        return cls(success=True, data=data, message=message, took_ms=took_ms)

    @classmethod
    def failure_result(
        cls, error: "Error", took_ms: Optional[int] = None
    ) -> "Result[T]":
        """创建失败结果"""
        return cls(success=False, error=error, took_ms=took_ms)


@dataclass
class Error:
    """错误类型"""

    code: str
    message: str
    details: Optional[Dict[str, Any]] = None
    timestamp: Optional[datetime] = None
    source: Optional[str] = None

    def __post_init__(self):
        if self.timestamp is None:
            self.timestamp = datetime.now()


class Status(Enum):
    """状态枚举"""

    PENDING = "pending"
    RUNNING = "running"
    SUCCESS = "success"
    FAILED = "failed"
    CANCELLED = "cancelled"
    TIMEOUT = "timeout"


@dataclass
class Timestamp:
    """时间戳"""

    value: datetime
    timezone: Optional[str] = None
    format: str = "%Y-%m-%d %H:%M:%S"

    @classmethod
    def now(cls) -> "Timestamp":
        return cls(value=datetime.now())

    @classmethod
    def from_unix(cls, unix_timestamp: float) -> "Timestamp":
        return cls(value=datetime.fromtimestamp(unix_timestamp))

    def to_unix(self) -> float:
        return self.value.timestamp()

    def __str__(self) -> str:
        return self.value.strftime(self.format)


@dataclass
class PageInfo:
    """分页信息"""

    page: int
    per_page: int
    total: int
    total_pages: int
    has_next: bool
    has_prev: bool


@dataclass
class HealthCheck:
    """健康检查"""

    service: str
    status: Status
    timestamp: Timestamp
    response_time_ms: int
    details: Optional[Dict[str, Any]] = None


@dataclass
class ResourceUsage:
    """资源使用情况"""

    cpu_percent: float
    memory_percent: float
    disk_percent: float
    network_in_mbps: float
    network_out_mbps: float
    active_connections: int
    timestamp: Timestamp


LogLevel = Literal["DEBUG", "INFO", "WARN", "ERROR", "CRITICAL"]


@dataclass
class LogEntry:
    """日志条目"""

    level: LogLevel
    message: str
    timestamp: Timestamp
    source: str
    context: Optional[Dict[str, Any]] = None
