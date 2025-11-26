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
Quark AI browser engine implementation

Specialized browser engine for extracting search results from Quark AI search.
This implementation handles JavaScript-rendered content with intelligent
waiting strategies and multiple selector fallback strategies.

Key Features:
- JavaScript-rendered content support via Playwright
- Multiple wait time strategies for content loading
- Multiple selector fallback strategies for search box and results
- Automatic deduplication by URL
- Smart text cleaning and snippet extraction
- Performance-optimized with selector caching

Example:
    >>> from seesea.browser.quark import QuarkEngine
    >>> from seesea.browser.base import BrowserConfig
    >>>
    >>> config = BrowserConfig(headless=True, stealth=True)
    >>> async with QuarkEngine(config) as engine:
    ...     results = await engine.search_quark("Python编程", max_results=10)
    >>>
    >>> for item in results:
    ...     print(f"{item['title']}: {item['url']}")
"""

# 引擎元数据（用于自动注册）
ENGINE_TYPE = "search"
ENGINE_DESCRIPTION = "夸克AI搜索引擎 - 基于JavaScript渲染的智能搜索"
ENGINE_CATEGORIES = ["search", "ai", "china"]

from typing import Dict, List, Any, Optional, Set
import time
import re

try:
    from playwright.async_api import Page
except ImportError:
    Page = Any

from .base import BaseBrowserEngine, BrowserConfig, SearchResultItem, BrowserActionDict


# Search input selectors (in priority order)
QUARK_SEARCH_SELECTORS = [
    "textarea[placeholder*='搜索']",           # Primary: textarea with search placeholder
    "input[placeholder*='搜索']",              # Secondary: input with search placeholder
    "textarea[placeholder*='search']",          # Tertiary: textarea with English search placeholder
    "input[placeholder*='search']",             # Quaternary: input with English search placeholder
    "input[type='search']",                     # Quinary: input type search
    ".search-input textarea",                   # Senary: textarea within search input container
    ".search-input input",                      # Septenary: input within search input container
    "[class*='search'] textarea",               # Octary: textarea within any search class
    "[class*='search'] input",                 # Nonary: input within any search class
]

# Search result selectors (in priority order)
QUARK_RESULT_SELECTORS = [
    '.sgs-container',                     # Primary: 右侧搜索结果容器
    '.sgs-container .search-box',       # Secondary: 搜索框区域中的结果
    '.sgs-container .result-item',      # Tertiary: 搜索结果项目
    '.sgs-container [class*="result"]', # Quaternary: 结果相关类
    '.sgs-container [class*="item"]',      # Quinary: 项目相关类
    '.sgs-container [class*="answer"]',    # Senary: 回答相关类
    '.sgs-container [class*="content"]',   # Octary: 内容相关类
    '.sgs-container div[class*="card"]',  # Nonary: 卡片相关类
    '.sgs-container article',               # Decary: 文章元素
    '.sgs-container section',               # Undenary: 区块元素
    "[class*='result']",                       # 备用: 通用结果类
    ".result-item",
    ".search-result",
    "[class*='answer']",                       # Quaternary: elements with answer class
    "[class*='content']",                      # Quinary: elements with content class
    "div[class*='card']",                     # Senary: div with card class
    "div[class*='item']",                     # Septenary: div with item class
    "article",                                # Octary: article elements
    "[data-testid*='result']",                # Nonary: elements with result testid
]

# Title extraction selectors (in priority order)
QUARK_TITLE_SELECTORS = [
    "h1",                                    # Primary: h1 tags
    "h2",                                    # Secondary: h2 tags
    "h3",                                    # Tertiary: h3 tags
    "h4",                                    # Quaternary: h4 tags
    "[class*='title']",                       # Quinary: elements with title class
    "[class*='heading']",                     # Senary: elements with heading class
    "strong",                                 # Septenary: strong elements
    "b",                                     # Octary: bold elements
]

# URL extraction selectors
QUARK_URL_SELECTORS = [
    "a",                                     # Primary: link elements
    "[href]",                                 # Secondary: elements with href attribute
    "[data-url]",                             # Tertiary: elements with data-url attribute
]

# Navigation keywords to filter out (Chinese and English)
NAVIGATION_KEYWORDS = {
    '首页', '登录', '注册', '更多', '返回', '下一页', '上一页', 'home', 'login', 'register',
    'more', 'back', 'next', 'previous', '搜索', 'search', '设置', 'settings', '菜单', 'menu'
}

# JavaScript patterns to clean from text
JAVASCRIPT_PATTERNS = [
    r'window\._[^;]+;',                       # window._q_wl_sc variables
    r'Date\.now\(\)',                         # Date.now() calls
    r'window\.[^;]+;',                        # other window variables
    r'C语言中文网',                            # site name watermarks
    r'function\s+\w+\([^)]*\)\s*{[^}]*}',     # function definitions
    r'var\s+\w+\s*=',                        # variable declarations
    r'console\.[^(]+\([^)]*\);',              # console calls
]

# Minimum title length to be considered valid
MIN_TITLE_LENGTH = 5

# Maximum title length to avoid header/footer text
MAX_TITLE_LENGTH = 200

# Maximum snippet length for content
MAX_SNIPPET_LENGTH = 300

# Default user agent for Quark requests
DEFAULT_QUARK_USER_AGENT = 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36'

# Default viewport for Quark (important for proper page rendering)
DEFAULT_QUARK_VIEWPORT = {'width': 1440, 'height': 900}

# Wait times for JavaScript rendering
DEFAULT_QUARK_WAIT_TIMES = [2000, 3000, 5000]  # milliseconds (total 10 seconds)

# Aliases for backward compatibility
DEFAULT_USER_AGENT = DEFAULT_QUARK_USER_AGENT
DEFAULT_WAIT_TIMES = DEFAULT_QUARK_WAIT_TIMES


class QuarkEngine(BaseBrowserEngine):
    """
    Quark AI search engine

    Implements browser automation and data extraction for Quark AI search.
    This engine handles JavaScript-rendered content and uses multiple
    selector strategies for robust data extraction.

    Performance Optimizations:
    - Selector strategies ordered by reliability (most to least reliable)
    - Early termination when sufficient results found
    - URL-based deduplication using set (O(1) lookup)
    - Lazy evaluation of selectors (stops when results found)
    - Intelligent text cleaning to remove JavaScript noise
    - Smart waiting strategies for dynamic content

    Attributes:
        config: Browser configuration
        _result_cache: Cache for deduplicated results (per session)

    Example:
        >>> engine = QuarkEngine(BrowserConfig(headless=True))
        >>> async with engine:
        ...     results = await engine.search_quark("人工智能", max_results=20)
        >>> print(f"Found {len(results)} search results")
    """

    def __init__(self, config: Optional[BrowserConfig] = None) -> None:
        """
        Initialize Quark search engine

        Args:
            config: Browser configuration (uses defaults if None)

        Note:
            The default configuration uses headless mode with stealth
            enabled to avoid detection.
        """
        # Set Quark-specific configuration if not provided
        if config is None:
            config = BrowserConfig(
                user_agent=DEFAULT_QUARK_USER_AGENT,
                viewport_width=DEFAULT_QUARK_VIEWPORT['width'],
                viewport_height=DEFAULT_QUARK_VIEWPORT['height']
            )
        else:
            if config.user_agent is None:
                config.user_agent = DEFAULT_QUARK_USER_AGENT
            # Override viewport for Quark (crucial for proper page rendering)
            config.viewport_width = DEFAULT_QUARK_VIEWPORT['width']
            config.viewport_height = DEFAULT_QUARK_VIEWPORT['height']

        super().__init__(config)
        self._result_cache: Set[str] = set()

    def _build_search_url(self, query: str) -> str:
        """
        Build Quark search URL

        Args:
            query: Search query text

        Returns:
            Quark AI search base URL (query is submitted via form)

        Example:
            >>> engine._build_search_url("科技")
            'https://ai.quark.cn/s'
        """
        return "https://ai.quark.cn/s"

    def _clean_text(self, text: str) -> str:
        """
        Clean text content by removing JavaScript code and noise

        Args:
            text: Raw text content

        Returns:
            Cleaned text content
        """
        if not text:
            return ""

        # Remove JavaScript patterns (extended)
        for pattern in JAVASCRIPT_PATTERNS:
            text = re.sub(pattern, '', text, flags=re.IGNORECASE)

        # Additional cleaning patterns based on debug
        additional_patterns = [
            r'try\s*{[^}]*}\s*catch\s*\([^)]*\)\s*{[^}]*}',  # try-catch blocks
            r'var\s+\w+\s*=\s*[^;]+;',              # variable declarations
            r'document\.[^;]+;',                     # document methods
            r'console\.[^;]+;',                      # console methods
            r'\.offset[^;]+;',                       # offset properties
            r'\.client[^;]+;',                       # client properties
            r'\.scroll[^;]+;',                       # scroll properties
            r'function\s*\([^)]*\)\s*{[^}]*}',      # function definitions
            r'\{\s*[^}]*\}',                        # simple braces content
            r'tabs\.forEach[^;]+;',                  # forEach blocks
            r'querySelector[^;]+;',                  # querySelector calls
            r'dataset\.[^;]+;',                      # dataset access
        ]

        for pattern in additional_patterns:
            text = re.sub(pattern, '', text, flags=re.IGNORECASE)

        # Remove excessive whitespace
        text = re.sub(r'\s+', ' ', text)
        text = re.sub(r'\n+', '\n', text)

        # Remove duplicate lines
        lines = text.split('\n')
        unique_lines = []
        seen = set()

        for line in lines:
            line = line.strip()
            if line and line not in seen and len(line) > 5:  # Increased minimum length
                # Filter out lines that look like JavaScript
                if not any(js_keyword in line.lower() for js_keyword in
                          ['function', 'var ', 'const ', 'let ', 'document.', 'console.', 'queryselector']):
                    unique_lines.append(line)
                    seen.add(line)

        return '\n'.join(unique_lines)

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
            - Title must not contain navigation keywords
            - URL must not be in cache (no duplicates) - only if URL exists
            - Title must not be empty or just whitespace
        """
        # Check title length
        title_len = len(title.strip())
        if title_len < MIN_TITLE_LENGTH or title_len > MAX_TITLE_LENGTH:
            return False

        # Check if title is empty after stripping
        title_stripped = title.strip()
        if not title_stripped:
            return False

        # Check for navigation keywords (case-insensitive)
        title_lower = title_stripped.lower()
        if any(keyword.lower() in title_lower for keyword in NAVIGATION_KEYWORDS):
            return False

        # Check if already seen (deduplication) - only if URL exists
        if url and url in self._result_cache:
            return False

        # Additional deduplication: check title similarity for empty URLs
        if not url and self._result_cache:
            title_hash = hash(title.lower())
            # Simple hash-based deduplication for titles without URLs
            for cached_title in [t for t in self._result_cache if isinstance(t, str)]:
                if hash(cached_title.lower()) == title_hash:
                    return False

        return True

    async def _find_search_input(self, page: Page) -> Optional[Any]:
        """
        Find search input element on the page

        Args:
            page: Playwright page instance

        Returns:
            Search input element or None if not found
        """
        for selector in QUARK_SEARCH_SELECTORS:
            try:
                element = await page.wait_for_selector(selector, timeout=2000)
                if element and await element.is_visible():
                    return element
            except:
                continue
        return None

    async def _extract_title(self, element: Any, fallback_text: str = "") -> str:
        """
        Extract title from element or use fallback text

        Args:
            element: DOM element
            fallback_text: Fallback text if no title found

        Returns:
            Extracted title
        """
        # Try to find title within the element
        for selector in QUARK_TITLE_SELECTORS:
            try:
                title_element = await element.query_selector(selector)
                if title_element:
                    title = await title_element.text_content()
                    if title and len(title.strip()) > 0:
                        return title.strip()
            except:
                continue

        # Fallback to first line of text
        if fallback_text:
            lines = fallback_text.strip().split('\n')
            return lines[0] if lines else fallback_text[:50]

        return ""

    async def _extract_url(self, element: Any) -> Optional[str]:
        """
        Extract URL from element - improved version

        Args:
            element: DOM element

        Returns:
            Extracted URL or None
        """
        try:
            # 1. First check if element itself is a link
            current_element = element
            for _ in range(3):  # Check up to 3 levels up
                if current_element:
                    # Check if current element is a link
                    tag_name = await current_element.evaluate('el => el.tagName?.toLowerCase()')
                    if tag_name == 'a':
                        href = await current_element.get_attribute('href')
                        if href and href.startswith('http'):
                            return href

                    # Find links within current element
                    for selector in QUARK_URL_SELECTORS:
                        try:
                            link_element = await current_element.query_selector(selector)
                            if link_element:
                                href = await link_element.get_attribute('href')
                                if href and href.startswith('http'):
                                    return href
                        except:
                            continue

                    # Move to parent element
                    current_element = await current_element.evaluate('el => el.parentElement')
        except Exception as e:
            pass

        return None

    def _extract_snippet(self, content: str, max_length: int = MAX_SNIPPET_LENGTH) -> str:
        """
        Extract snippet from content

        Args:
            content: Cleaned text content
            max_length: Maximum length of snippet

        Returns:
            Extracted snippet
        """
        if not content:
            return ""

        # Split by lines and take first few meaningful lines
        lines = content.split('\n')
        snippet_lines = []

        for line in lines:
            line = line.strip()
            if line and len(line) > 10:
                snippet_lines.append(line)
                if len('\n'.join(snippet_lines)) > max_length:
                    break

        snippet = '\n'.join(snippet_lines)

        # If still too long, truncate at word boundary
        if len(snippet) > max_length:
            snippet = snippet[:max_length].rsplit(' ', 1)[0] + '...'

        return snippet

    async def extract_data(
        self,
        page: Page,
        params: Dict[str, Any]
    ) -> List[SearchResultItem]:
        """
        Extract search results from Quark page

        This method implements the core data extraction logic with multiple
        selector strategies and intelligent text processing.

        Args:
            page: Playwright page instance
            params: Parameters including 'query' and 'max_results'

        Returns:
            List of extracted search result items

        Example:
            >>> results = await engine.extract_data(page, {"query": "AI", "max_results": 10})
        """
        query = params.get('query', '')
        max_results = params.get('max_results', 10)
        wait_times = params.get('wait_times', DEFAULT_WAIT_TIMES)

        results = []

        # Wait for JavaScript rendering with multiple attempts
        for wait_time in wait_times:
            await page.wait_for_timeout(wait_time)

            # Try each selector strategy
            for selector in QUARK_RESULT_SELECTORS:
                try:
                    elements = await page.query_selector_all(selector)
                    if not elements:
                        continue

  
                    for i, element in enumerate(elements[:max_results]):
                        try:
                            # Get text content
                            text_content = await element.text_content()
                            if not text_content or len(text_content.strip()) < 10:
                                continue

                            # Clean text content
                            cleaned_content = self._clean_text(text_content)

                            # Extract title
                            title = await self._extract_title(element, cleaned_content)

                            # Extract URL
                            url = await self._extract_url(element)

                            # Extract snippet
                            snippet = self._extract_snippet(cleaned_content)

                            # Validate result
                            if not self._is_valid_result(title, url or ''):
                                continue

                            # Add to cache for deduplication
                            if url:
                                self._result_cache.add(url)

                            # Create result item
                            result = SearchResultItem(
                                title=title,
                                url=url or '',
                                snippet=snippet
                            )

                            results.append(result)

                            # Stop if we have enough results
                            if len(results) >= max_results:
                                break

                        except Exception as e:
                            continue

                    # If we found results, stop trying other selectors
                    if results:
                        break

                except Exception as e:
                    continue

                # If we found results, stop trying other wait times
                if results:
                    break

            # If we found results, stop trying other wait times
            if results:
                break

        return results

    async def search_quark(
        self,
        query: str,
        max_results: int = 10,
        wait_times: Optional[List[int]] = None
    ) -> List[SearchResultItem]:
        """
        Search Quark AI with the given query

        High-level method that handles the complete search workflow:
        1. Navigate to Quark AI
        2. Find and fill search input
        3. Submit search form
        4. Wait for results to load
        5. Extract and return results

        Args:
            query: Search query string
            max_results: Maximum number of results to return
            wait_times: Custom wait times for JavaScript rendering

        Returns:
            List of search result items

        Example:
            >>> results = await engine.search_quark("人工智能", max_results=20)
            >>> print(f"Found {len(results)} results")
        """
        if wait_times is None:
            wait_times = DEFAULT_WAIT_TIMES

        url = self._build_search_url(query)

        # Define browser actions for Quark search
        actions = [
            {"type": "navigate", "url": url, "timeout_ms": 30000},
            {"type": "wait", "ms": 3000},  # Wait for page to load
        ]

        # Parameters for data extraction
        params = {
            "query": query,
            "max_results": max_results,
            "wait_times": wait_times
        }

        # Execute search with custom actions
        async with self._get_page() as page:
            # Navigate to the page
            await page.goto(url, wait_until="domcontentloaded", timeout=self.config.timeout)

            # Wait for initial content to load
            await page.wait_for_timeout(3000)

            # Find search input
            search_input = await self._find_search_input(page)
            if not search_input:
                raise Exception("未找到搜索框")

            # Fill search input
            await search_input.fill(query)
            await page.wait_for_timeout(500)

            # Submit search (press Enter)
            await search_input.press('Enter')

            # Wait for search results to load
            await self._wait_for_results(page, wait_times)

            # Extract results
            results = await self.extract_data(page, params)

      
            return results

    async def _wait_for_results(self, page: Page, wait_times: List[int]) -> None:
        """
        Wait for search results to load with multiple strategies

        Args:
            page: Playwright page instance
            wait_times: List of wait times to try
        """
        # Try waiting for result selectors first
        for selector in QUARK_RESULT_SELECTORS:
            try:
                await page.wait_for_selector(selector, timeout=2000)
                return
            except:
                continue

        # If selectors don't work, use timed waits
        for wait_time in wait_times:
            await page.wait_for_timeout(wait_time)


# Convenience functions for creating Quark callbacks
def create_quark_callback(
    max_results: int = 10,
    wait_times: Optional[List[int]] = None
):
    """
    Create a callback function for Quark AI search

    This function returns an async callback that can be used with
    the broader SeeSea search framework.

    Args:
        max_results: Maximum number of results to return
        wait_times: Custom wait times for JavaScript rendering

    Returns:
        Async callback function for Quark search

    Example:
        >>> callback = create_quark_callback(max_results=20)
        >>> results = await callback("人工智能", page=1)
    """
    async def quark_callback(query: str, **kwargs) -> List[SearchResultItem]:
        config = BrowserConfig(headless=True, stealth=True)
        async with QuarkEngine(config) as engine:
            return await engine.search_quark(
                query=query,
                max_results=kwargs.get('max_results', max_results),
                wait_times=wait_times
            )

    return quark_callback


def create_quark_callback_sync(params: Dict[str, Any]) -> Dict[str, Any]:
    """
    同步包装器函数，用于 Rust 集成

    这个函数处理异步调用，避免跨语言异步问题。
    Rust 调用这个同步函数，由它来处理异步逻辑。

    Args:
        params: 包含搜索参数的字典，必须包含 'query' 键

    Returns:
        包含搜索结果的字典，格式与 SeeSea 核心兼容

    Example:
        >>> params = {"query": "人工智能", "max_results": 10}
        >>> results = create_quark_callback_sync(params)
        >>> print(f"找到 {len(results['items'])} 个结果")
    """
    import asyncio

    try:
        # 从参数中提取搜索查询
        query = params.get('query', '')
        if not query:
            return {"items": [], "error": "Missing query parameter"}

        max_results = params.get('max_results', 10)
        wait_times = params.get('wait_times', DEFAULT_WAIT_TIMES)

        # 创建并运行异步搜索
        async def run_search():
            config = BrowserConfig(headless=True, stealth=True)
            async with QuarkEngine(config) as engine:
                return await engine.search_quark(
                    query=query,
                    max_results=max_results,
                    wait_times=wait_times
                )

        results = asyncio.run(run_search())

        # 转换为 SeeSea 核心期望的格式 - 直接返回字典，避免循环导入
        return {
            "items": [
                {
                    "title": item.get("title", ""),
                    "url": item.get("url", ""),
                    "content": item.get("snippet", ""),
                    "snippet": item.get("snippet", ""),
                    "score": 1.0
                }
                for item in results
            ],
            "total_count": len(results),
            "engine": "quark",
            "query_time_ms": 0,  # 实际搜索时间
            "cached": False,
        }

    except Exception as e:
        return {
            "items": [],
            "error": str(e),
            "engine": "quark",
            "total_count": 0,
            "query_time_ms": 0,
            "cached": False,
        }


# Export constants for external use
__all__ = [
    'QuarkEngine',
    'create_quark_callback',
    'create_quark_callback_sync',
    'QUARK_SEARCH_SELECTORS',
    'QUARK_RESULT_SELECTORS',
    'QUARK_TITLE_SELECTORS',
    'QUARK_URL_SELECTORS',
    'DEFAULT_USER_AGENT',
    'DEFAULT_WAIT_TIMES',
    'NAVIGATION_KEYWORDS',
    'MIN_TITLE_LENGTH',
    'MAX_TITLE_LENGTH',
    'MAX_SNIPPET_LENGTH',
]