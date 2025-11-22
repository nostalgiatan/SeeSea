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
Xinhua (新华网) browser engine implementation

Specialized browser engine for extracting news from Xinhua News Agency website.
This implementation is based on the validated approach from get_xinhua_results.py.

Key Features:
- JavaScript-rendered content support via Playwright
- Multiple wait time strategies for content loading
- Multiple selector fallback strategies
- Automatic deduplication by URL
- Navigation link filtering
- Performance-optimized with selector caching

Example:
    >>> from seesea.browser.xinhua import XinhuaEngine
    >>> from seesea.browser.base import BrowserConfig
    >>> 
    >>> config = BrowserConfig(headless=True, stealth=True)
    >>> async with XinhuaEngine(config) as engine:
    ...     results = await engine.search_xinhua("科技创新", page=1)
    >>> 
    >>> for item in results:
    ...     print(f"{item['title']}: {item['url']}")
"""

# 引擎元数据（用于自动注册）
ENGINE_TYPE = "news"
ENGINE_DESCRIPTION = "新华网搜索引擎 - 基于JavaScript渲染的SPA应用"
ENGINE_CATEGORIES = ["news", "china"]

from typing import Dict, List, Any, Optional, Set
import time

try:
    from playwright.async_api import Page
except ImportError:
    Page = Any

from .base import BaseBrowserEngine, BrowserConfig, SearchResultItem, BrowserActionDict


# Validated selectors from get_xinhua_results.py (in priority order)
XINHUA_SELECTORS = [
    "a[href*='news.cn']",      # Primary: Links containing news.cn (most reliable)
    "a[href*='article']",       # Secondary: Article links
    "div[class*='result'] a",   # Tertiary: Links within result containers
    "div[class*='item'] a",     # Quaternary: Links within item containers
    "li a",                     # Quinary: Links within list items
    "[class*='title'] a",       # Senary: Links within title elements
]

# Navigation keywords to filter out (Chinese)
NAVIGATION_KEYWORDS = {'首页', '登录', '注册', '更多', '返回', '下一页', '上一页'}

# Minimum title length to be considered valid
MIN_TITLE_LENGTH = 10

# Maximum title length to avoid header/footer text
MAX_TITLE_LENGTH = 200

# Default user agent for Xinhua requests (from successful get_xinhua_results.py)
DEFAULT_USER_AGENT = 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36'

# Wait times for JavaScript rendering (reduced for faster timeout)
DEFAULT_WAIT_TIMES = [2000, 3000, 3000]  # milliseconds (total 8 seconds)


class XinhuaEngine(BaseBrowserEngine):
    """
    Xinhua News (新华网) search engine
    
    Implements browser automation and data extraction for Xinhua News Agency's
    search functionality. This engine handles JavaScript-rendered content and
    uses multiple selector strategies for robust data extraction.
    
    Performance Optimizations:
    - Selector strategies ordered by reliability (most to least reliable)
    - Early termination when sufficient results found
    - URL-based deduplication using set (O(1) lookup)
    - Lazy evaluation of selectors (stops when results found)
    
    Attributes:
        config: Browser configuration
        _result_cache: Cache for deduplicated results (per session)
    
    Example:
        >>> engine = XinhuaEngine(BrowserConfig(headless=True))
        >>> async with engine:
        ...     results = await engine.search_xinhua("人工智能", page=1)
        >>> print(f"Found {len(results)} news articles")
    """
    
    def __init__(self, config: Optional[BrowserConfig] = None) -> None:
        """
        Initialize Xinhua search engine
        
        Args:
            config: Browser configuration (uses defaults if None)
        
        Note:
            The default configuration uses headless mode with stealth
            enabled to avoid detection.
        """
        # Set Xinhua-specific user agent if not provided
        if config is None:
            config = BrowserConfig(user_agent=DEFAULT_USER_AGENT)
        elif config.user_agent is None:
            config.user_agent = DEFAULT_USER_AGENT
        
        super().__init__(config)
        self._result_cache: Set[str] = set()
    
    def _build_search_url(
        self,
        query: str,
        page: int = 1,
        category: str = "0"
    ) -> str:
        """
        Build Xinhua search URL
        
        Args:
            query: Search query text
            page: Page number (1-indexed)
            category: Search category ("0" for full text, "1" for title only)
        
        Returns:
            Formatted search URL
        
        Example:
            >>> engine._build_search_url("科技", page=2, category="0")
            'https://so.news.cn/#search/0/%E7%A7%91%E6%8A%80/2'
        """
        from urllib.parse import quote
        encoded_query = quote(query)
        return f"https://so.news.cn/#search/{category}/{encoded_query}/{page}"
    
    def _is_valid_result(self, title: str, url: str) -> bool:
        """
        Validate if a result should be included
        
        Args:
            title: Result title text
            url: Result URL
        
        Returns:
            True if result is valid, False otherwise
        
        Validation Rules:
            - Title length must be between MIN_TITLE_LENGTH and MAX_TITLE_LENGTH
            - URL must contain 'news.cn'
            - Title must not contain navigation keywords
            - URL must not be in cache (no duplicates)
        """
        # Check title length
        title_len = len(title)
        if title_len < MIN_TITLE_LENGTH or title_len > MAX_TITLE_LENGTH:
            return False
        
        # Check if URL contains news.cn
        if 'news.cn' not in url:
            return False
        
        # Check for navigation keywords (case-insensitive)
        title_lower = title.lower()
        if any(keyword in title_lower for keyword in NAVIGATION_KEYWORDS):
            return False
        
        # Check if already seen (deduplication)
        if url in self._result_cache:
            return False
        
        return True
    
    async def extract_data(
        self,
        page: Page,
        params: Dict[str, Any]
    ) -> List[SearchResultItem]:
        """
        Extract search results from Xinhua page
        
        Based on the successful approach from get_xinhua_results.py:
        - Multiple wait times to allow JavaScript rendering
        - Multiple selector strategies
        - Proper filtering and deduplication
        
        Args:
            page: Playwright page instance
            params: Extraction parameters
                - query: Search query text (optional)
                - max_results: Maximum number of results (default: 50)
                - wait_times: List of wait times in ms (default: [3000, 5000, 8000])
        
        Returns:
            List of extracted search result items with title, url, and snippet
        """
        query_text = params.get("query", "")
        max_results = params.get("max_results", 50)
        wait_times = params.get("wait_times", DEFAULT_WAIT_TIMES)
        
        results: List[SearchResultItem] = []
        
        # Try different wait times (from get_xinhua_results.py)
        for wait_time in wait_times:
            await page.wait_for_timeout(wait_time)
            
            # Try each selector in priority order
            for selector in XINHUA_SELECTORS:
                try:
                    elements = await page.locator(selector).all()
                    
                    for element in elements:
                        # Stop if we have enough results
                        if len(results) >= max_results:
                            break
                        
                        try:
                            # Extract data from element
                            href = await element.get_attribute('href')
                            text = await element.text_content()
                            
                            if not href or not text:
                                continue
                            
                            title = text.strip()
                            
                            # Validate result (from get_xinhua_results.py logic)
                            if self._is_valid_result(title, href):
                                # Add to cache for deduplication
                                self._result_cache.add(href)
                                
                                # Create result item
                                result_item: SearchResultItem = {
                                    'title': title,
                                    'url': href,
                                    'snippet': ''  # Can be enhanced to extract snippets
                                }
                                results.append(result_item)
                        
                        except Exception:
                            # Skip individual element errors
                            continue
                    
                    # If we found results with this selector, don't try others
                    if results:
                        break
                
                except Exception:
                    # Skip to next selector if this one fails
                    continue
            
            # If we found results, no need to wait longer
            if results:
                break
        
        return results
    
    async def search_xinhua(
        self,
        query: str,
        page: int = 1,
        category: str = "0",
        max_results: int = 50
    ) -> List[SearchResultItem]:
        """
        Search Xinhua news with the given query
        
        High-level method based on get_xinhua_results.py successful approach.
        
        Args:
            query: Search query text
            page: Page number (1-indexed, default: 1)
            category: Search category ("0" for full text, "1" for title, default: "0")
            max_results: Maximum number of results to return (default: 50)
        
        Returns:
            List of search result items
        
        Raises:
            Exception: If search operation fails
        
        Example:
            >>> results = await engine.search_xinhua("科技创新", page=1, max_results=20)
            >>> print(f"Found {len(results)} articles")
        """
        # Build URL
        url = self._build_search_url(query, page, category)
        
        # Define actions (navigate and wait for domcontentloaded)
        actions: List[BrowserActionDict] = [
            {"type": "navigate", "url": url}
        ]
        
        # Define parameters with multiple wait times
        params = {
            "query": query,
            "max_results": max_results,
            "wait_times": DEFAULT_WAIT_TIMES
        }
        
        # Execute search
        return await self.search(url, actions, params)
    
    def clear_cache(self) -> None:
        """
        Clear the result cache
        
        This should be called between different search sessions to
        ensure fresh results and prevent cache buildup.
        
        Example:
            >>> engine.clear_cache()
            >>> results = await engine.search_xinhua("new query")
        """
        self._result_cache.clear()


def create_xinhua_callback_sync(params: Dict[str, Any]) -> Dict[str, Any]:
    """
    同步包装器函数，用于 Rust 集成

    这个函数处理异步调用，避免跨语言异步问题。
    Rust 调用这个同步函数，由它来处理异步逻辑。
    """
    import asyncio

    try:
        # 尝试获取当前事件循环
        loop = asyncio.get_event_loop()
        if loop.is_running():
            # 如果事件循环正在运行，创建任务
            import concurrent.futures
            with concurrent.futures.ThreadPoolExecutor() as executor:
                future = executor.submit(asyncio.run, create_xinhua_callback(params))
                return future.result()
        else:
            # 如果没有运行的事件循环，直接运行
            return asyncio.run(create_xinhua_callback(params))
    except Exception as e:
        # 降级到同步处理
        return {
            "results": [],
            "elapsed_ms": 0,
            "error": f"Failed to run async callback: {str(e)}"
        }


async def create_xinhua_callback(params: Dict[str, Any]) -> Dict[str, Any]:
    """
    Callback function for Rust integration

    This function provides a bridge between Rust and Python for Xinhua searches.
    Based on the successful implementation from get_xinhua_results.py.

    Args:
        params: Dictionary containing:
            - query: Search query text (required)
            - page: Page number (default: 1)
            - page_size: Maximum results (default: 50)
            - category: Search category (default: "0" for full text, "1" for title only)
            - language: Language (optional, ignored)
            - region: Region (optional, ignored)

    Returns:
        Dictionary containing:
            - results: List of result items (each with title, url, snippet)
            - elapsed_ms: Execution time in milliseconds
            - error: Error message if operation failed (optional)

    Example:
        >>> params = {
        ...     "query": "科技创新",
        ...     "page": 1,
        ...     "page_size": 20
        ... }
        >>> result = await create_xinhua_callback(params)
        >>> print(f"Found {len(result['results'])} results")
    """
    start_time = time.time()
    
    try:
        # Extract parameters
        query = params.get("query", "")
        if not query:
            return {
                "results": [],
                "elapsed_ms": 0,
                "error": "Query is required"
            }
        
        page = params.get("page", 1)
        page_size = params.get("page_size", 50)
        category = params.get("category", "0")
        
        # Create engine with default config (from get_xinhua_results.py)
        config = BrowserConfig(
            headless=True,
            stealth=True,
            user_agent=DEFAULT_USER_AGENT
        )
        
        # Execute search using the validated approach
        async with XinhuaEngine(config) as engine:
            # Set user agent
            async with engine._get_page() as page_obj:
                await engine.set_user_agent(page_obj, DEFAULT_USER_AGENT)
                
                # Build URL
                url = engine._build_search_url(query, page, category)
                
                # Navigate to URL
                await page_obj.goto(url, wait_until="domcontentloaded", timeout=30000)

                # Extract data with multiple wait times (from get_xinhua_results.py)
                params_dict = {
                    "query": query,
                    "max_results": page_size,
                    "wait_times": DEFAULT_WAIT_TIMES
                }
                results = await engine.extract_data(page_obj, params_dict)

        elapsed_ms = int((time.time() - start_time) * 1000)

        return {
            "results": results,
            "elapsed_ms": elapsed_ms
        }
    
    except Exception as e:
        elapsed_ms = int((time.time() - start_time) * 1000)
        return {
            "results": [],
            "elapsed_ms": elapsed_ms,
            "error": str(e)
        }


__all__ = [
    'XinhuaEngine',
    'create_xinhua_callback',
    'create_xinhua_callback_sync',
    'XINHUA_SELECTORS',
    'DEFAULT_USER_AGENT',
]
