"""
SeeSea SDK 基础模块

提供所有客户端的基础抽象类和通用功能。
"""

from .client import BaseClient, AsyncBaseClient

__all__ = [
    "BaseClient",
    "AsyncBaseClient",
]
