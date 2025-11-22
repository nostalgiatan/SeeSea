# Engine Customization Guide

## Overview

SeeSea supports custom search engines in both Rust and Python. This guide covers both approaches.

## Python Custom Engines

### Basic Engine Structure

Create a Python file in `seesea/seesea/browser/` with a callback function:

```python
# seesea/seesea/browser/my_engine.py

from typing import Dict, Any, List

# Engine metadata
ENGINE_TYPE = "general"  # or "news", "image", etc.
ENGINE_DESCRIPTION = "My Custom Search Engine"
ENGINE_CATEGORIES = ["general", "tech"]

def create_my_engine_callback_sync(query_dict: Dict[str, Any]) -> List[Dict[str, Any]]:
    """
    Search callback function.
    
    Args:
        query_dict: Dictionary containing:
            - query: str - Search query
            - page: int - Page number
            - page_size: int - Results per page
            - language: Optional[str]
            - region: Optional[str]
    
    Returns:
        List of result dictionaries, each containing:
            - title: str
            - url: str
            - content: str
            - score: float (0.0-1.0)
            - display_url: Optional[str]
            - site_name: Optional[str]
    """
    query = query_dict.get("query", "")
    page = query_dict.get("page", 1)
    
    # Implement your search logic here
    results = []
    
    # Example result
    results.append({
        "title": f"Result for {query}",
        "url": "https://example.com/result",
        "content": "Example content",
        "score": 0.95,
        "display_url": "example.com",
        "site_name": "Example Site"
    })
    
    return results
```

### Callback Function Parameters

The `query_dict` parameter contains:

| Key | Type | Description |
|-----|------|-------------|
| `query` | `str` | Search keywords |
| `page` | `int` | Page number (1-indexed) |
| `page_size` | `int` | Results per page |
| `language` | `Optional[str]` | Language code (e.g., "en", "zh") |
| `region` | `Optional[str]` | Region code (e.g., "us", "cn") |
| `safe_search` | `int` | Safe search level (0, 1, 2) |

### Return Format

Each result dictionary must contain:

| Key | Type | Required | Description |
|-----|------|----------|-------------|
| `title` | `str` | Yes | Result title |
| `url` | `str` | Yes | Result URL |
| `content` | `str` | Yes | Content/description |
| `score` | `float` | Yes | Relevance score (0.0-1.0) |
| `display_url` | `str` | No | Display URL |
| `site_name` | `str` | No | Site name |

### Registration

Engines in `seesea/seesea/browser/` are automatically registered on import:

```python
from seesea import SearchClient

client = SearchClient()
# Your custom engine is now available
engines = client.list_engines()
print(engines)  # Includes 'my_engine'
```

### Advanced Example with Browser Automation

```python
# seesea/seesea/browser/playwright_engine.py

from playwright.sync_api import sync_playwright
from typing import Dict, Any, List

ENGINE_TYPE = "general"
ENGINE_DESCRIPTION = "Custom Playwright-based Engine"
ENGINE_CATEGORIES = ["general"]

def create_playwright_engine_callback_sync(query_dict: Dict[str, Any]) -> List[Dict[str, Any]]:
    """Search using Playwright browser automation."""
    query = query_dict.get("query", "")
    
    results = []
    
    with sync_playwright() as p:
        browser = p.chromium.launch(headless=True)
        page = browser.new_page()
        
        # Navigate and scrape
        page.goto(f"https://example.com/search?q={query}")
        
        # Extract results
        for element in page.query_selector_all(".result"):
            title = element.query_selector(".title").inner_text()
            url = element.query_selector("a").get_attribute("href")
            content = element.query_selector(".description").inner_text()
            
            results.append({
                "title": title,
                "url": url,
                "content": content,
                "score": 0.8
            })
        
        browser.close()
    
    return results
```

## Rust Custom Engines

### Engine Trait Implementation

Create a Rust file in `src/engines/`:

```rust
// src/engines/my_engine.rs

use crate::derive::types::{SearchQuery, SearchResult, SearchResultItem, ResultType};
use crate::derive::engine::{SearchEngine, EngineMetadata, EngineCapabilities};
use async_trait::async_trait;

pub struct MyEngine {
    client: reqwest::Client,
}

impl MyEngine {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl SearchEngine for MyEngine {
    fn metadata(&self) -> EngineMetadata {
        EngineMetadata {
            name: "my_engine".to_string(),
            display_name: "My Custom Engine".to_string(),
            description: Some("Custom search engine implementation".to_string()),
            homepage: "https://example.com".to_string(),
            categories: vec!["general".to_string()],
            language_support: vec!["en".to_string(), "zh".to_string()],
        }
    }

    fn capabilities(&self) -> EngineCapabilities {
        EngineCapabilities {
            supports_pagination: true,
            supports_suggestions: false,
            supports_images: false,
            supports_videos: false,
            max_page_size: 50,
        }
    }

    async fn search(
        &self,
        query: &SearchQuery,
    ) -> Result<SearchResult, Box<dyn std::error::Error + Send + Sync>> {
        // Implement search logic
        let url = format!("https://api.example.com/search?q={}", query.query);
        let response = self.client.get(&url).send().await?;
        let data: serde_json::Value = response.json().await?;
        
        // Parse response
        let items = self.parse_results(&data)?;
        
        Ok(SearchResult {
            engine_name: "my_engine".to_string(),
            total_results: Some(items.len()),
            elapsed_ms: 0,
            items,
            pagination: None,
            suggestions: Vec::new(),
            metadata: std::collections::HashMap::new(),
        })
    }
}

impl MyEngine {
    fn parse_results(
        &self,
        data: &serde_json::Value,
    ) -> Result<Vec<SearchResultItem>, Box<dyn std::error::Error + Send + Sync>> {
        let mut items = Vec::new();
        
        if let Some(results) = data["results"].as_array() {
            for result in results {
                items.push(SearchResultItem {
                    title: result["title"].as_str().unwrap_or("").to_string(),
                    url: result["url"].as_str().unwrap_or("").to_string(),
                    content: result["description"].as_str().unwrap_or("").to_string(),
                    score: 0.9,
                    result_type: ResultType::Web,
                    display_url: None,
                    site_name: None,
                    thumbnail: None,
                    published_date: None,
                    template: None,
                    metadata: std::collections::HashMap::new(),
                });
            }
        }
        
        Ok(items)
    }
}
```

### Register the Engine

In `src/engines/mod.rs`:

```rust
pub mod my_engine;

use crate::derive::engine::SearchEngine;
use std::sync::Arc;

pub fn get_all_engines() -> Vec<Arc<dyn SearchEngine>> {
    vec![
        Arc::new(my_engine::MyEngine::new()),
        // ... other engines
    ]
}
```

## Best Practices

### Python Engines

1. **Keep it simple**: Return basic dict structures
2. **Handle errors**: Wrap in try-except and return empty list on failure
3. **Respect rate limits**: Add delays if needed
4. **Cache results**: Consider caching for repeated queries
5. **Type hints**: Use type hints for better IDE support

### Rust Engines

1. **Error handling**: Use `Result` types properly
2. **Async/await**: Prefer async operations for I/O
3. **Connection pooling**: Reuse HTTP clients
4. **Timeout handling**: Set reasonable timeouts
5. **Testing**: Write unit tests for parsing logic

## Testing Custom Engines

### Python

```python
# Test your engine
def test_my_engine():
    from seesea.seesea.browser.my_engine import create_my_engine_callback_sync
    
    results = create_my_engine_callback_sync({
        "query": "test",
        "page": 1,
        "page_size": 10
    })
    
    assert len(results) > 0
    assert "title" in results[0]
    assert "url" in results[0]
```

### Rust

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_my_engine() {
        let engine = MyEngine::new();
        let query = SearchQuery {
            query: "test".to_string(),
            ..Default::default()
        };
        
        let result = engine.search(&query).await;
        assert!(result.is_ok());
    }
}
```

## Examples

See `examples/` directory for complete working examples of custom engines.

## Troubleshooting

**Engine not registered:**
- Check filename matches pattern `create_{name}_callback_sync`
- Ensure file is in `seesea/seesea/browser/`
- Verify no syntax errors

**Import errors:**
- Install required dependencies
- Check Python path

**Rust compilation errors:**
- Run `cargo check` for details
- Verify trait implementation is complete

## Reference

- [Search Usage Guide](./SEARCH_USAGE.md)
- [Type System](./TYPE_SYSTEM.md)
- [Full-text Search Guide](./fulltext-search-guide.md)
