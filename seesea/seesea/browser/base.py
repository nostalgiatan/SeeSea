# Copyright 2025 nostalgiatan
#
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at
#
#     http://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.

"""
Browser engine base module for SeeSea

Provides the foundational classes and interfaces for browser automation,
supporting JavaScript-rendered search engines through Playwright integration.

This module defines:
- BrowserConfig: Configuration for browser instances
- BaseBrowserEngine: Abstract base class for browser engines
- BrowserEngineClient: High-level client for browser operations

Performance Optimizations:
- Singleton browser instance management
- Context reuse where possible
- Efficient resource cleanup
"""

from abc import ABC, abstractmethod
from typing import Dict, List, Optional, Any, TypedDict
import asyncio
from contextlib import asynccontextmanager

try:
    from playwright.async_api import async_playwright, Browser, Page, Playwright
    PLAYWRIGHT_AVAILABLE = True
except ImportError:
    PLAYWRIGHT_AVAILABLE = False
    async_playwright = None
    Browser = None
    Page = None
    Playwright = None


class SearchResultItem(TypedDict, total=False):
    """Type definition for a search result item"""
    title: str
    url: str
    snippet: str


class BrowserActionDict(TypedDict, total=False):
    """Type definition for browser actions"""
    type: str
    url: str
    selector: str
    text: str
    key: str
    ms: int
    timeout_ms: int
    path: str
    script: str


class BrowserConfig:
    """
    Configuration for browser instances
    
    Attributes:
        headless: Run browser in headless mode (default: True)
        stealth: Enable stealth mode to avoid detection (default: True)
        browser_type: Browser type - "chromium", "firefox", or "webkit" (default: "chromium")
        user_agent: Custom user agent string (default: None)
        viewport_width: Browser viewport width in pixels (default: 1920)
        viewport_height: Browser viewport height in pixels (default: 1080)
        timeout: Default timeout for operations in milliseconds (default: 30000)
    
    Example:
        >>> config = BrowserConfig(
        ...     headless=True,
        ...     stealth=True,
        ...     browser_type="chromium",
        ...     timeout=60000
        ... )
    """
    
    def __init__(
        self,
        headless: bool = True,
        stealth: bool = True,
        browser_type: str = "chromium",
        user_agent: Optional[str] = None,
        viewport_width: int = 1920,
        viewport_height: int = 1080,
        timeout: int = 30000,
    ) -> None:
        """
        Initialize browser configuration
        
        Args:
            headless: Run browser in headless mode
            stealth: Enable stealth mode
            browser_type: Browser type ("chromium", "firefox", "webkit")
            user_agent: Custom user agent string
            viewport_width: Viewport width in pixels
            viewport_height: Viewport height in pixels
            timeout: Default timeout in milliseconds
        """
        self.headless = headless
        self.stealth = stealth
        self.browser_type = browser_type
        self.user_agent = user_agent
        self.viewport_width = viewport_width
        self.viewport_height = viewport_height
        self.timeout = timeout

    def to_dict(self) -> Dict[str, Any]:
        """
        Convert configuration to dictionary
        
        Returns:
            Dictionary representation of configuration
        """
        return {
            "headless": self.headless,
            "stealth": self.stealth,
            "browser_type": self.browser_type,
            "user_agent": self.user_agent,
            "viewport_width": self.viewport_width,
            "viewport_height": self.viewport_height,
            "timeout": self.timeout,
        }


class BaseBrowserEngine(ABC):
    """
    Abstract base class for browser engines
    
    This class provides the foundation for implementing browser-based
    search engines with JavaScript rendering support. Subclasses should
    implement the extract_data method to define engine-specific extraction logic.
    
    Performance Considerations:
    - Browser instances are created on-demand and reused when possible
    - Contexts are created per-operation to maintain isolation
    - Resources are automatically cleaned up using context managers
    
    Example:
        >>> class MyEngine(BaseBrowserEngine):
        ...     async def extract_data(self, page, params):
        ...         elements = await page.locator("a").all()
        ...         return [{"title": await e.text_content()} for e in elements]
    """
    
    def __init__(self, config: Optional[BrowserConfig] = None) -> None:
        """
        Initialize browser engine
        
        Args:
            config: Browser configuration (uses defaults if None)
        
        Raises:
            RuntimeError: If Playwright is not installed
        """
        if not PLAYWRIGHT_AVAILABLE:
            raise RuntimeError(
                "Playwright is not installed. Install with: pip install playwright && playwright install chromium"
            )
        
        self.config = config or BrowserConfig()
        self._playwright: Optional[Playwright] = None
        self._browser: Optional[Browser] = None
    
    async def __aenter__(self) -> 'BaseBrowserEngine':
        """
        Async context manager entry
        
        Returns:
            Self for use in async with statements
        """
        await self.start()
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
    
    async def start(self) -> None:
        """
        Start the browser instance
        
        Initializes Playwright and launches the browser if not already started.
        This method is idempotent - calling it multiple times has no effect.
        
        Raises:
            RuntimeError: If browser fails to start
        """
        if self._playwright is None:
            self._playwright = await async_playwright().start()
            
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
    
    @asynccontextmanager
    async def _get_page(self) -> Any:
        """
        Create a new browser page with proper configuration
        
        This context manager ensures proper cleanup of page resources.
        
        Yields:
            Page: Configured Playwright page instance
        """
        if not self._browser:
            await self.start()
        
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
            await context.add_init_script("""
                Object.defineProperty(navigator, 'webdriver', {
                    get: () => undefined
                });
            """)
        
        page = await context.new_page()
        
        try:
            yield page
        finally:
            await page.close()
            await context.close()
    
    async def set_user_agent(self, page: Page, user_agent: str) -> None:
        """
        Set user agent for the page
        
        Args:
            page: Playwright page instance
            user_agent: User agent string
        """
        await page.set_extra_http_headers({
            'User-Agent': user_agent
        })
    
    async def execute_actions(self, page: Page, actions: List[BrowserActionDict]) -> None:
        """
        Execute a sequence of browser actions
        
        Args:
            page: Playwright page instance
            actions: List of action dictionaries to execute
        
        Raises:
            ValueError: If action type is unknown
        """
        for action in actions:
            await self._execute_action(page, action)
    
    async def _execute_action(self, page: Page, action: BrowserActionDict) -> None:
        """
        Execute a single browser action
        
        Args:
            page: Playwright page instance
            action: Action dictionary with type and parameters
        
        Raises:
            ValueError: If action type is unknown
        """
        action_type = action.get("type")
        
        if action_type == "navigate":
            await page.goto(
                action["url"],
                wait_until="domcontentloaded",
                timeout=action.get("timeout_ms", self.config.timeout)
            )
        
        elif action_type == "wait_selector":
            await page.wait_for_selector(
                action["selector"],
                timeout=action.get("timeout_ms", self.config.timeout)
            )
        
        elif action_type == "click":
            await page.click(action["selector"])
        
        elif action_type == "fill":
            await page.fill(action["selector"], action["text"])
        
        elif action_type == "press":
            await page.keyboard.press(action["key"])
        
        elif action_type == "evaluate":
            await page.evaluate(action["script"])
        
        elif action_type == "wait":
            await asyncio.sleep(action["ms"] / 1000.0)
        
        elif action_type == "wait_for_timeout":
            await page.wait_for_timeout(action.get("ms", 1000))
        
        elif action_type == "screenshot":
            path = action.get("path")
            if path:
                await page.screenshot(path=path)
        
        else:
            raise ValueError(f"Unknown action type: {action_type}")
    
    @abstractmethod
    async def extract_data(
        self,
        page: Page,
        params: Dict[str, Any]
    ) -> List[SearchResultItem]:
        """
        Extract structured data from the page
        
        This method must be implemented by subclasses to define
        engine-specific data extraction logic.
        
        Args:
            page: Playwright page instance
            params: Parameters for data extraction (query, selectors, etc.)
        
        Returns:
            List of extracted search result items
        
        Raises:
            NotImplementedError: If not implemented by subclass
        """
        raise NotImplementedError("Subclasses must implement extract_data")
    
    async def search(
        self,
        url: str,
        actions: List[BrowserActionDict],
        params: Optional[Dict[str, Any]] = None
    ) -> List[SearchResultItem]:
        """
        Execute a search operation
        
        High-level method that orchestrates navigation, actions, and data extraction.
        
        Args:
            url: Target URL to visit
            actions: List of browser actions to perform
            params: Additional parameters for data extraction
        
        Returns:
            List of extracted search result items
        
        Raises:
            Exception: If search operation fails
        """
        async with self._get_page() as page:
            # Execute actions
            await self.execute_actions(page, actions)
            
            # Extract data using engine-specific logic
            results = await self.extract_data(page, params or {})
            
            return results


class BrowserEngineClient:
    """
    High-level client for managing browser engine instances
    
    This client provides a convenient interface for browser operations
    with automatic resource management.
    
    Performance Optimizations:
    - Lazy browser initialization
    - Automatic cleanup of resources
    - Context manager support for efficient resource usage
    
    Example:
        >>> config = BrowserConfig(headless=True)
        >>> client = BrowserEngineClient(config)
        >>> 
        >>> # Using context manager (recommended)
        >>> async with client.get_engine(MyEngine) as engine:
        ...     results = await engine.search(url, actions, params)
        >>> 
        >>> # Direct usage
        >>> results = await client.execute_search(MyEngine, url, actions, params)
    """
    
    def __init__(self, config: Optional[BrowserConfig] = None) -> None:
        """
        Initialize browser engine client
        
        Args:
            config: Browser configuration (uses defaults if None)
        """
        self.config = config or BrowserConfig()
    
    @asynccontextmanager
    async def get_engine(self, engine_class: type) -> Any:
        """
        Get a browser engine instance with automatic cleanup
        
        Args:
            engine_class: Browser engine class to instantiate
        
        Yields:
            Initialized browser engine instance
        
        Example:
            >>> async with client.get_engine(XinhuaEngine) as engine:
            ...     results = await engine.search(url, actions, params)
        """
        engine = engine_class(self.config)
        async with engine:
            yield engine
    
    async def execute_search(
        self,
        engine_class: type,
        url: str,
        actions: List[BrowserActionDict],
        params: Optional[Dict[str, Any]] = None
    ) -> List[SearchResultItem]:
        """
        Execute a search using a specific engine
        
        Convenience method that handles engine lifecycle automatically.
        
        Args:
            engine_class: Browser engine class to use
            url: Target URL to visit
            actions: List of browser actions to perform
            params: Additional parameters for data extraction
        
        Returns:
            List of extracted search result items
        
        Example:
            >>> results = await client.execute_search(
            ...     XinhuaEngine,
            ...     "https://so.news.cn/#search/0/科技/1",
            ...     actions=[{"type": "wait", "ms": 3000}],
            ...     params={"query": "科技"}
            ... )
        """
        async with self.get_engine(engine_class) as engine:
            return await engine.search(url, actions, params)
    
    def is_available(self) -> bool:
        """
        Check if Playwright is available
        
        Returns:
            True if Playwright is installed and available, False otherwise
        """
        return PLAYWRIGHT_AVAILABLE


__all__ = [
    'BrowserConfig',
    'BaseBrowserEngine',
    'BrowserEngineClient',
    'SearchResultItem',
    'BrowserActionDict',
    'PLAYWRIGHT_AVAILABLE',
]
