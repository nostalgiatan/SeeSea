"""
Example usage of the refactored browser module

This example demonstrates the proper usage of the new browser package
structure with base classes and specialized engines.
"""

import asyncio
from seesea.browser import (
    BrowserConfig,
    BrowserEngineClient,
    XinhuaEngine,
    xinhua_search_callback,
)


async def example_direct_engine_usage():
    """
    Example 1: Using XinhuaEngine directly with context manager
    
    This is the most efficient approach for multiple searches with the same engine.
    """
    print("\n=== Example 1: Direct Engine Usage ===\n")
    
    # Create configuration
    config = BrowserConfig(
        headless=True,
        stealth=True,
        timeout=30000
    )
    
    # Use engine with context manager (automatically handles cleanup)
    async with XinhuaEngine(config) as engine:
        # Search for multiple queries using the same engine instance
        queries = ["科技创新", "人工智能", "新能源"]
        
        for query in queries:
            print(f"Searching for: {query}")
            results = await engine.search_xinhua(query, page=1, max_results=5)
            
            print(f"Found {len(results)} results:")
            for i, item in enumerate(results, 1):
                print(f"  {i}. {item['title'][:60]}...")
                print(f"     {item['url']}")
            print()
            
            # Clear cache between different queries
            engine.clear_cache()


async def example_client_usage():
    """
    Example 2: Using BrowserEngineClient
    
    This approach is convenient when you want automatic engine lifecycle management.
    """
    print("\n=== Example 2: Client Usage ===\n")
    
    # Create client with configuration
    config = BrowserConfig(headless=True, stealth=True)
    client = BrowserEngineClient(config)
    
    # Execute search with automatic engine creation and cleanup
    results = await client.execute_search(
        XinhuaEngine,
        url="https://so.news.cn/#search/0/%E7%A7%91%E6%8A%80/1",
        actions=[{"type": "wait", "ms": 3000}],
        params={"query": "科技", "max_results": 5}
    )
    
    print(f"Found {len(results)} results:")
    for i, item in enumerate(results, 1):
        print(f"  {i}. {item['title'][:60]}...")


async def example_callback_for_rust():
    """
    Example 3: Using callback function (for Rust integration)
    
    This demonstrates the callback that will be called from Rust side.
    """
    print("\n=== Example 3: Callback for Rust Integration ===\n")
    
    params = {
        "query": "新华网",
        "page": 1,
        "page_size": 5,
        "category": "0"
    }
    
    result = await xinhua_search_callback(params)
    
    print(f"Search completed in {result['elapsed_ms']}ms")
    print(f"Found {len(result['results'])} results:")
    
    for i, item in enumerate(result['results'], 1):
        print(f"  {i}. {item['title'][:60]}...")


async def example_with_retry():
    """
    Example 4: Using retry mechanism for robustness
    
    Demonstrates the built-in retry mechanism with exponential backoff.
    """
    print("\n=== Example 4: Search with Retry ===\n")
    
    config = BrowserConfig(headless=True)
    
    async with XinhuaEngine(config) as engine:
        try:
            # This will retry up to 3 times with exponential backoff
            results = await engine.search_with_retry(
                query="区块链",
                page=1,
                max_results=5,
                max_retries=3,
                retry_delay=1.0
            )
            
            print(f"Successfully found {len(results)} results after retries")
            for i, item in enumerate(results, 1):
                print(f"  {i}. {item['title'][:60]}...")
        
        except Exception as e:
            print(f"Search failed after all retries: {e}")


async def example_custom_engine():
    """
    Example 5: Creating a custom engine by extending BaseBrowserEngine
    
    This shows how to create engines for other websites.
    """
    print("\n=== Example 5: Custom Engine ===\n")
    
    from seesea.browser.base import BaseBrowserEngine, Page, SearchResultItem
    from typing import Dict, List, Any
    
    class CustomEngine(BaseBrowserEngine):
        """Example custom engine for a hypothetical website"""
        
        async def extract_data(
            self,
            page: Page,
            params: Dict[str, Any]
        ) -> List[SearchResultItem]:
            """Extract data using custom selectors"""
            results = []
            
            # Example: Extract all article links
            elements = await page.locator("article a").all()
            
            for element in elements:
                title = await element.text_content()
                url = await element.get_attribute("href")
                
                if title and url:
                    results.append({
                        "title": title.strip(),
                        "url": url,
                        "snippet": ""
                    })
            
            return results
    
    print("Custom engine created successfully!")
    print("This engine can be used similarly to XinhuaEngine")


async def main():
    """Run all examples"""
    print("=" * 60)
    print("SeeSea Browser Module - Examples")
    print("=" * 60)
    
    # Note: These examples require Playwright to be installed
    # Install with: pip install playwright && playwright install chromium
    
    try:
        # Run examples
        await example_direct_engine_usage()
        await example_client_usage()
        await example_callback_for_rust()
        await example_with_retry()
        await example_custom_engine()
        
        print("\n" + "=" * 60)
        print("All examples completed successfully!")
        print("=" * 60)
    
    except ImportError as e:
        print(f"\nError: {e}")
        print("\nPlease install Playwright:")
        print("  pip install playwright")
        print("  playwright install chromium")
    
    except Exception as e:
        print(f"\nError running examples: {e}")
        import traceback
        traceback.print_exc()


if __name__ == "__main__":
    asyncio.run(main())
