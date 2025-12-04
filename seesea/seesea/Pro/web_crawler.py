# Copyright (C) 2025 nostalgiatan
#
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU Affero General Public License as published
# by the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
#
# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU Affero General Public License for more details.
#
# You should have received a copy of the GNU Affero General Public License
# along with this program.  If not, see <https://www.gnu.org/licenses/>.

"""
Web Crawler module for SeeSea Pro

This module provides a high-level interface for crawling web pages using Playwright,
waiting for DOM content to load, and returning the rendered HTML.

Features:
- Asynchronous web crawling using Playwright
- Configurable browser settings
- Automatic resource management
- Support for DOM content loaded waiting
- Headless and non-headless modes

Example:
    >>> from seesea.Pro import WebCrawler
    >>> crawler = WebCrawler()
    >>> html = await crawler.crawl("https://example.com")
    >>> print(html[:100])  # Print first 100 characters of HTML
"""

from typing import Optional, Dict, Any
from ..browser.base import BrowserConfig, PLAYWRIGHT_AVAILABLE

if PLAYWRIGHT_AVAILABLE:
    pass


class WebCrawler:
    """
    Web Crawler class for crawling web pages and returning rendered HTML

    This class provides a simple interface for crawling web pages using Playwright,
    waiting for DOM content to load, and returning the rendered HTML.

    Attributes:
        config (BrowserConfig): Configuration for browser instances
        playwright (Optional[Playwright]): Playwright instance
        browser (Optional[Browser]): Browser instance

    Example:
        >>> crawler = WebCrawler(
        ...     headless=True,
        ...     browser_type="chromium",
        ...     timeout=30000
        ... )
        >>> html = await crawler.crawl("https://example.com")
    """

    def __init__(
        self,
        headless: bool = True,
        browser_type: str = "chromium",
        user_agent: Optional[str] = None,
        viewport_width: int = 1920,
        viewport_height: int = 1080,
        timeout: int = 30000,
    ) -> None:
        """
        Initialize the web crawler

        Args:
            headless (bool): Run browser in headless mode (default: True)
            browser_type (str): Browser type - "chromium", "firefox", or "webkit" (default: "chromium")
            user_agent (Optional[str]): Custom user agent string (default: None)
            viewport_width (int): Browser viewport width in pixels (default: 1920)
            viewport_height (int): Browser viewport height in pixels (default: 1080)
            timeout (int): Default timeout for operations in milliseconds (default: 30000)

        Raises:
            RuntimeError: If Playwright is not installed
        """
        if not PLAYWRIGHT_AVAILABLE:
            raise RuntimeError(
                "Playwright is not installed. Install with: pip install playwright && playwright install chromium"
            )

        # 使用与xinhua.py相同的默认用户代理，更接近真实浏览器
        DEFAULT_USER_AGENT = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36"

        self.config = BrowserConfig(
            headless=headless,
            stealth=False,  # 关闭stealth模式，使用正常浏览器配置
            browser_type=browser_type,
            user_agent=user_agent or DEFAULT_USER_AGENT,  # 使用默认用户代理如果没有提供
            viewport_width=viewport_width,
            viewport_height=viewport_height,
            timeout=timeout,
        )
        self._playwright: Optional[Any] = None
        self._browser: Optional[Any] = None

    async def __aenter__(self) -> "WebCrawler":
        """
        Async context manager entry

        Returns:
            Self for use in async with statements
        """
        await self._init_playwright()
        return self

    async def __aexit__(self, exc_type: Any, exc_val: Any, exc_tb: Any) -> None:
        """
        Async context manager exit

        Args:
            exc_type: Exception type
            exc_val: Exception value
            exc_tb: Exception traceback
        """
        await self.close()

    async def _init_playwright(self) -> None:
        """
        Initialize Playwright and browser instance

        Raises:
            RuntimeError: If browser fails to start
        """
        if self._playwright is None:
            from playwright.async_api import async_playwright

            self._playwright = await async_playwright().start()

        # 确保 playwright 实例已初始化
        assert self._playwright is not None

        # Select browser type
        if self.config.browser_type == "firefox":
            browser_launcher = self._playwright.firefox
        elif self.config.browser_type == "webkit":
            browser_launcher = self._playwright.webkit
        else:  # chromium (default)
            browser_launcher = self._playwright.chromium

            # Launch browser with configuration
            launch_options: Dict[str, Any] = {
                "headless": self.config.headless,
            }

            self._browser = await browser_launcher.launch(**launch_options)

    async def close(self) -> None:
        """
        Close the browser instance and cleanup resources

        This method ensures all browser resources are properly released.
        Safe to call multiple times.
        """
        if self._browser:
            await self._browser.close()
            self._browser = None

        if self._playwright:
            await self._playwright.stop()
            self._playwright = None

    async def crawl(
        self, url: str, wait_until: str = "domcontentloaded", additional_wait: int = 0
    ) -> str:
        """
        Crawl a web page and return the rendered HTML

        Args:
            url (str): URL of the web page to crawl
            wait_until (str): When to consider navigation succeeded. Options:
                - "domcontentloaded": When the DOM content has loaded (default)
                - "load": When the page has fully loaded
                - "networkidle": When there are no network connections for at least 500ms
            additional_wait (int): Additional wait time in milliseconds after navigation
                to allow JavaScript to render content (default: 0)

        Returns:
            str: Rendered HTML of the web page

        Raises:
            Exception: If crawling fails
        """
        if not self._browser:
            await self._init_playwright()

        if not self._browser:
            raise RuntimeError("Browser is not initialized")

        # Create context with viewport
        context = await self._browser.new_context(
            viewport={
                "width": self.config.viewport_width,
                "height": self.config.viewport_height,
            },
            user_agent=self.config.user_agent,
        )

        # Apply stealth if enabled
        if self.config.stealth:
            # Basic stealth: hide webdriver property
            await context.add_init_script(
                """
                Object.defineProperty(navigator, 'webdriver', {
                    get: () => undefined
                });
            """
            )

        page = await context.new_page()

        try:
            # Navigate to the URL with specified wait condition
            await page.goto(
                url,
                wait_until=wait_until,
                timeout=self.config.timeout,
            )

            # Add additional wait time if specified (for SPA rendering)
            if additional_wait > 0:
                await page.wait_for_timeout(additional_wait)

            # Get the rendered HTML
            html = await page.content()

            # Ensure the result is a string
            return str(html)
        finally:
            await page.close()
            await context.close()

    @classmethod
    async def crawl_page(
        cls,
        url: str,
        headless: bool = True,
        browser_type: str = "chromium",
        timeout: int = 30000,
        wait_until: str = "domcontentloaded",
        additional_wait: int = 0,
        user_agent: Optional[str] = None,
    ) -> str:
        """
        Class method for one-time crawling of a web page

        This method provides a convenient way to crawl a single page without
        having to manage the crawler instance lifecycle manually.

        Args:
            url (str): URL of the web page to crawl
            headless (bool): Run browser in headless mode (default: True)
            browser_type (str): Browser type (default: "chromium")
            timeout (int): Operation timeout in milliseconds (default: 30000)
            wait_until (str): When to consider navigation succeeded (default: "domcontentloaded")
            additional_wait (int): Additional wait time in milliseconds after navigation (default: 0)
            user_agent (Optional[str]): Custom user agent string (default: None)

        Returns:
            str: Rendered HTML of the web page

        Example:
            >>> html = await WebCrawler.crawl_page("https://example.com")
            >>> print(html[:100])
        """
        async with cls(
            headless=headless, browser_type=browser_type, timeout=timeout, user_agent=user_agent
        ) as crawler:
            return await crawler.crawl(url, wait_until=wait_until, additional_wait=additional_wait)


__all__ = ["WebCrawler"]
