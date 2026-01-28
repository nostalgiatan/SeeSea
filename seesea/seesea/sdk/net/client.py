"""
SeeSea 网络客户端

提供统一的HTTP请求接口，基于PyNetClient实现完整功能。
"""

from typing import Dict, Any, Optional, Union
from pathlib import Path

from ..base import BaseClient
from ..seesea_types.common_types import Result, Error

try:
    from seesea_core import PyNetClient

    _CORE_AVAILABLE = True
except ImportError:
    _CORE_AVAILABLE = False


class NetClient(BaseClient[Dict[str, Any]]):
    """
    SeeSea 网络客户端

    提供高层次的HTTP请求接口，支持GET、POST请求和文件操作。
    继承自BaseClient，提供统一的客户端接口。

    主要功能:
    - HTTP GET请求
    - HTTP POST请求
    - 文件下载
    - 文件上传
    - 代理支持
    - TLS配置

    示例:
        >>> client = NetClient()
        >>> with client:
        ...     result = client.get("https://example.com")
        ...     if result.success:
        ...         print(result.data)
    """

    def __init__(self, config: Optional[Dict[str, Any]] = None):
        """初始化网络客户端"""
        super().__init__(config)
        self._client: Optional[PyNetClient] = None

        if not _CORE_AVAILABLE:
            error = Error(
                code="CORE_NOT_AVAILABLE",
                message="SeeSea核心模块不可用",
                details={"error": "请确保已正确安装seesea_core"},
            )
            self._error = error
            return

        try:
            self._client = PyNetClient()
        except Exception as e:
            error = Error(
                code="NET_CLIENT_INIT_FAILED",
                message=f"网络客户端初始化失败: {str(e)}",
                details={"error": str(e)},
            )
            self._error = error

    def get(
        self,
        url: str,
        headers: Optional[Dict[str, str]] = None,
        timeout: Optional[int] = None,
        proxy: Optional[str] = None,
    ) -> "Result[Dict[str, Any]]":
        """
        执行HTTP GET请求

        参数:
            url: 请求URL
            headers: 请求头
            timeout: 超时时间（秒）
            proxy: 代理地址

        返回:
            Result[Dict[str, Any]]: 响应数据
        """
        if self._client is None:
            return Result.failure_result(self._error)

        try:
            response = self._client.get(url, headers or {}, timeout or 30, proxy or "")
            return Result.success_result(response)
        except Exception as e:
            return Result.failure_result(
                Error(
                    code="GET_REQUEST_FAILED",
                    message=f"GET请求失败: {str(e)}",
                    details={"error": str(e)},
                )
            )

    def post(
        self,
        url: str,
        data: Optional[Union[Dict[str, Any], str]] = None,
        json_data: Optional[Dict[str, Any]] = None,
        headers: Optional[Dict[str, str]] = None,
        timeout: Optional[int] = None,
        proxy: Optional[str] = None,
    ) -> "Result[Dict[str, Any]]":
        """
        执行HTTP POST请求

        参数:
            url: 请求URL
            data: 表单数据
            json_data: JSON数据
            headers: 请求头
            timeout: 超时时间（秒）
            proxy: 代理地址

        返回:
            Result[Dict[str, Any]]: 响应数据
        """
        if self._client is None:
            return Result.failure_result(self._error)

        try:
            if json_data:
                import json

                data = json.dumps(json_data)

            response = self._client.post(
                url, data or "", headers or {}, timeout or 30, proxy or ""
            )
            return Result.success_result(response)
        except Exception as e:
            return Result.failure_result(
                Error(
                    code="POST_REQUEST_FAILED",
                    message=f"POST请求失败: {str(e)}",
                    details={"error": str(e)},
                )
            )

    def download_file(
        self,
        url: str,
        save_path: Union[str, Path],
        headers: Optional[Dict[str, str]] = None,
        timeout: Optional[int] = None,
        proxy: Optional[str] = None,
    ) -> "Result[str]":
        """
        下载文件

        参数:
            url: 下载URL
            save_path: 保存路径
            headers: 请求头
            timeout: 超时时间（秒）
            proxy: 代理地址

        返回:
            Result[str]: 保存的文件路径
        """
        if self._client is None:
            return Result.failure_result(self._error)

        try:
            save_path = Path(save_path)
            save_path.parent.mkdir(parents=True, exist_ok=True)

            self._client.get_file(
                url, str(save_path), headers or {}, timeout or 300, proxy or ""
            )
            return Result.success_result(str(save_path))
        except Exception as e:
            return Result.failure_result(
                Error(
                    code="DOWNLOAD_FAILED",
                    message=f"文件下载失败: {str(e)}",
                    details={"error": str(e)},
                )
            )

    def upload_file(
        self,
        url: str,
        file_path: Union[str, Path],
        field_name: str = "file",
        data: Optional[Dict[str, Any]] = None,
        headers: Optional[Dict[str, str]] = None,
        timeout: Optional[int] = None,
        proxy: Optional[str] = None,
    ) -> "Result[Dict[str, Any]]":
        """
        上传文件

        参数:
            url: 上传URL
            file_path: 文件路径
            field_name: 表单字段名
            data: 附加表单数据
            headers: 请求头
            timeout: 超时时间（秒）
            proxy: 代理地址

        返回:
            Result[Dict[str, Any]]: 响应数据
        """
        if self._client is None:
            return Result.failure_result(self._error)

        try:
            file_path = Path(file_path)
            if not file_path.exists():
                return Result.failure_result(
                    Error(
                        code="FILE_NOT_FOUND",
                        message=f"文件不存在: {file_path}",
                        details={"error": str(file_path)},
                    )
                )

            response = self._client.post_file(
                url,
                str(file_path),
                field_name,
                data or {},
                headers or {},
                timeout or 300,
                proxy or "",
            )
            return Result.success_result(response)
        except Exception as e:
            return Result.failure_result(
                Error(
                    code="UPLOAD_FAILED",
                    message=f"文件上传失败: {str(e)}",
                    details={"error": str(e)},
                )
            )
