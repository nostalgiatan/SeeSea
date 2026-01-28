"""
API相关类型定义
"""

from typing import Dict, List, Optional, Any, Union, Literal
from dataclasses import dataclass
from datetime import datetime

HttpMethodType = Literal["GET", "POST", "PUT", "DELETE", "PATCH", "HEAD", "OPTIONS"]
HttpStatusCode = Literal[200, 201, 400, 401, 403, 404, 500, 502, 503, 504]

__all__ = [
    "HttpMethodType",
    "HttpStatusCode",
    "ApiRequest",
    "ApiResponse",
    "ApiEndpoint",
    "ApiConfig",
    "HttpStatus",
]


@dataclass
class ApiRequest:
    """API请求"""

    method: HttpMethodType
    path: str
    headers: Optional[Dict[str, str]] = None
    params: Optional[Dict[str, Any]] = None
    body: Optional[Union[str, bytes, Dict[str, Any]]] = None
    timeout_ms: Optional[int] = None


@dataclass
class ApiResponse:
    """API响应"""

    status_code: HttpStatusCode
    headers: Dict[str, str]
    body: Union[str, bytes, Dict[str, Any]]
    content_type: str
    took_ms: int
    timestamp: datetime


@dataclass
class ApiEndpoint:
    """API端点配置"""

    path: str
    method: HttpMethodType
    handler: str
    auth_required: bool = False
    rate_limit: Optional[int] = None
    timeout_ms: int = 30000
    description: Optional[str] = None


@dataclass
class ApiConfig:
    """API服务配置"""

    host: str = "0.0.0.0"
    port: int = 8080
    workers: int = 4
    max_connections: int = 1000
    request_timeout_ms: int = 30000
    enable_cors: bool = True
    enable_gzip: bool = True
    log_requests: bool = True
    endpoints: Optional[List[ApiEndpoint]] = None


@dataclass
class HttpStatus:
    """HTTP状态"""

    code: HttpStatusCode
    message: str
    success: bool
