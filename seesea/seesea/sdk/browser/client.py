"""
SeeSea 浏览器引擎客户端

提供统一的浏览器自动化接口，基于PyBrowserEngineClient实现完整功能。
"""

from typing import Dict, Any, Optional

from ..base import BaseClient
from ..seesea_types.common_types import Result, Error

try:
    from seesea_core import PyBrowserEngineClient, PyBrowserConfig

    _CORE_AVAILABLE = True
except ImportError:
    _CORE_AVAILABLE = False


class BrowserClient(BaseClient[Dict[str, Any]]):
    """
    SeeSea 浏览器引擎客户端

    提供高层次的浏览器自动化接口，支持动态内容渲染和页面交互。
    继承自BaseClient，提供统一的客户端接口。

    主要功能:
    - 页面加载
    - JavaScript执行
    - 截图
    - Cookie管理

    示例:
        >>> client = BrowserClient()
        >>> with client:
        ...     result = client.load_page("https://example.com")
        ...     if result.success:
        ...         print("页面加载成功")
    """

    def __init__(self, config: Optional[Dict[str, Any]] = None):
        """初始化浏览器客户端"""
        super().__init__(config)
        self._client: Optional[PyBrowserEngineClient] = None

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
            browser_config = PyBrowserConfig()
            self._client = PyBrowserEngineClient(browser_config)
        except Exception as e:
            error = Error(
                code="BROWSER_INIT_FAILED",
                message=f"浏览器初始化失败: {str(e)}",
                details={"error": str(e)},
            )
            self._error = error

    def load_page(self, url: str, wait_for: Optional[str] = None) -> "Result[str]":
        """
        加载页面

        参数:
            url: 页面URL
            wait_for: 等待的CSS选择器

        返回:
            Result[str]: 页面HTML
        """
        if self._client is None:
            return Result.failure_result(self._error)

        try:
            html = self._client.load_page(url, wait_for or "")
            return Result.success_result(html)
        except Exception as e:
            return Result.failure_result(
                Error(
                    code="LOAD_PAGE_FAILED",
                    message=f"页面加载失败: {str(e)}",
                    details={"error": str(e)},
                )
            )

    def execute_script(self, script: str) -> "Result[Any]":
        """
        执行JavaScript

        参数:
            script: JavaScript代码

        返回:
            Result[Any]: 执行结果
        """
        if self._client is None:
            return Result.failure_result(self._error)

        try:
            result = self._client.execute_script(script)
            return Result.success_result(result)
        except Exception as e:
            return Result.failure_result(
                Error(
                    code="EXECUTE_SCRIPT_FAILED",
                    message=f"脚本执行失败: {str(e)}",
                    details={"error": str(e)},
                )
            )

    def screenshot(self, path: str) -> "Result[bool]":
        """
        截图

        参数:
            path: 保存路径

        返回:
            Result[bool]: 操作结果
        """
        if self._client is None:
            return Result.failure_result(self._error)

        try:
            self._client.screenshot(path)
            return Result.success_result(True)
        except Exception as e:
            return Result.failure_result(
                Error(
                    code="SCREENSHOT_FAILED",
                    message=f"截图失败: {str(e)}",
                    details={"error": str(e)},
                )
            )

    def get_cookies(self) -> "Result[Dict[str, str]]":
        """
        获取Cookies

        返回:
            Result[Dict[str, str]]: Cookies字典
        """
        if self._client is None:
            return Result.failure_result(self._error)

        try:
            cookies = self._client.get_cookies()
            return Result.success_result(cookies)
        except Exception as e:
            return Result.failure_result(
                Error(
                    code="GET_COOKIES_FAILED",
                    message=f"获取Cookies失败: {str(e)}",
                    details={"error": str(e)},
                )
            )

    def set_cookies(self, cookies: Dict[str, str]) -> "Result[bool]":
        """
        设置Cookies

        参数:
            cookies: Cookies字典

        返回:
            Result[bool]: 操作结果
        """
        if self._client is None:
            return Result.failure_result(self._error)

        try:
            self._client.set_cookies(cookies)
            return Result.success_result(True)
        except Exception as e:
            return Result.failure_result(
                Error(
                    code="SET_COOKIES_FAILED",
                    message=f"设置Cookies失败: {str(e)}",
                    details={"error": str(e)},
                )
            )

    def close(self) -> "Result[bool]":
        """
        关闭浏览器

        返回:
            Result[bool]: 操作结果
        """
        if self._client is None:
            return Result.failure_result(self._error)

        try:
            self._client.close()
            return Result.success_result(True)
        except Exception as e:
            return Result.failure_result(
                Error(
                    code="CLOSE_BROWSER_FAILED",
                    message=f"关闭浏览器失败: {str(e)}",
                    details={"error": str(e)},
                )
            )
