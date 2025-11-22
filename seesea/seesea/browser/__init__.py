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
Browser automation package for SeeSea

This package provides comprehensive browser automation support for search engines
that require JavaScript rendering. It includes:

- Base classes for implementing custom browser engines
- Specialized engines for specific websites (e.g., Xinhua News)
- High-level client for easy integration
- Type-safe interfaces with full type annotations

Architecture:
    browser/
    ├── __init__.py      # Package exports and convenience imports
    ├── base.py          # Base classes and interfaces
    └── xinhua.py        # Xinhua News engine implementation

Usage Patterns:

1. Using the high-level client (recommended):
    >>> from seesea.browser import BrowserEngineClient, XinhuaEngine
    >>> 
    >>> client = BrowserEngineClient()
    >>> results = await client.execute_search(
    ...     XinhuaEngine,
    ...     url="https://so.news.cn/#search/0/AI/1",
    ...     actions=[{"type": "wait", "ms": 3000}],
    ...     params={"query": "AI", "max_results": 20}
    ... )

2. Using engine directly with context manager:
    >>> from seesea.browser import XinhuaEngine, BrowserConfig
    >>> 
    >>> config = BrowserConfig(headless=True, stealth=True)
    >>> async with XinhuaEngine(config) as engine:
    ...     results = await engine.search_xinhua("科技", page=1)

3. Creating custom engines:
    >>> from seesea.browser import BaseBrowserEngine, BrowserConfig
    >>> 
    >>> class MyEngine(BaseBrowserEngine):
    ...     async def extract_data(self, page, params):
    ...         # Custom extraction logic
    ...         elements = await page.locator("a.result").all()
    ...         return [
    ...             {"title": await e.text_content(), "url": await e.get_attribute("href")}
    ...             for e in elements
    ...         ]

Performance Considerations:
- Browser instances are created lazily and reused when possible
- Context managers ensure proper resource cleanup
- Singleton browser instance per engine (configurable)
- Efficient deduplication using sets
- Selector strategies ordered by reliability

Type Safety:
All public APIs include complete type annotations for improved IDE support
and type checking with mypy.
"""

from .base import (
    BrowserConfig,
    BaseBrowserEngine,
    BrowserEngineClient,
    SearchResultItem,
    BrowserActionDict,
    PLAYWRIGHT_AVAILABLE,
)

from .xinhua import (
    XinhuaEngine,
    create_xinhua_callback,
    create_xinhua_callback_sync,
    XINHUA_SELECTORS,
    DEFAULT_USER_AGENT,
)


# Convenience aliases for backward compatibility
BrowserEngine = BaseBrowserEngine
xinhua_search_callback = create_xinhua_callback


__all__ = [
    # Base classes and types
    'BrowserConfig',
    'BaseBrowserEngine',
    'BrowserEngineClient',
    'SearchResultItem',
    'BrowserActionDict',
    
    # Convenience aliases
    'BrowserEngine',
    
    # Xinhua engine
    'XinhuaEngine',
    'create_xinhua_callback',
    'xinhua_search_callback',
    
    # Constants
    'PLAYWRIGHT_AVAILABLE',
    'XINHUA_SELECTORS',
    'DEFAULT_USER_AGENT',
]


__version__ = '0.1.0'
__author__ = 'SeeSea Team'
