# 引擎定制指南

## 概述

SeeSea 支持在 Rust 和 Python 中创建自定义搜索引擎。本指南涵盖了两种方法。

## Python 自定义引擎

### 基本引擎结构

在 `seesea/seesea/browser/` 中创建一个 Python 文件，包含一个回调函数：

```python
# seesea/seesea/browser/my_engine.py

from typing import Dict, Any, List

# 引擎元数据
ENGINE_TYPE = "general"  # 或 "news", "image" 等
ENGINE_DESCRIPTION = "我的自定义搜索引擎"
ENGINE_CATEGORIES = ["general", "tech"]

def create_my_engine_callback_sync(query_dict: Dict[str, Any]) -> List[Dict[str, Any]]:
    """
    搜索回调函数。
    
    参数：
        query_dict: 包含以下内容的字典：
            - query: str - 搜索查询
            - page: int - 页码
            - page_size: int - 每页结果数
            - language: Optional[str]
            - region: Optional[str]
    
    返回：
        结果字典列表，每个字典包含：
            - title: str
            - url: str
            - content: str
            - score: float (0.0-1.0)
            - display_url: Optional[str]
            - site_name: Optional[str]
    """
    query = query_dict.get("query", "")
    page = query_dict.get("page", 1)
    
    # 在这里实现你的搜索逻辑
    results = []
    
    # 示例结果
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

### 回调函数参数

`query_dict` 参数包含：

| 键 | 类型 | 描述 |
|-----|------|------|
| `query` | `str` | 搜索关键词 |
| `page` | `int` | 页码（从1开始） |
| `page_size` | `int` | 每页结果数 |
| `language` | `Optional[str]` | 语言代码 |
| `region` | `Optional[str]` | 地区代码 |
| `safe_search` | `int` | 安全搜索级别 (0, 1, 2) |

### 返回格式

每个结果字典必须包含：

| 键 | 类型 | 必须 | 描述 |
|-----|------|------|------|
| `title` | `str` | 是 | 结果标题 |
| `url` | `str` | 是 | 结果 URL |
| `content` | `str` | 是 | 内容/描述 |
| `score` | `float` | 是 | 相关性得分 (0.0-1.0) |
| `display_url` | `str` | 否 | 显示 URL |
| `site_name` | `str` | 否 | 站点名称 |

### 注册

`seesea/seesea/browser/` 中的引擎在导入时会自动注册：

```python
from seesea import SearchClient

client = SearchClient()
# 你的自定义引擎现在可用
engines = client.list_engines()
print(engines)  # 包含 'my_engine'
```

### 使用浏览器自动化的高级示例

```python
# seesea/seesea/browser/playwright_engine.py

from playwright.sync_api import sync_playwright
from typing import Dict, Any, List

ENGINE_TYPE = "general"
ENGINE_DESCRIPTION = "自定义 Playwright 引擎"
ENGINE_CATEGORIES = ["general"]

def create_playwright_engine_callback_sync(query_dict: Dict[str, Any]) -> List[Dict[str, Any]]:
    """使用 Playwright 浏览器自动化进行搜索。"""
    query = query_dict.get("query", "")
    
    results = []
    
    with sync_playwright() as p:
        browser = p.chromium.launch(headless=True)
        page = browser.new_page()
        
        # 导航和抓取
        page.goto(f"https://example.com/search?q={query}")
        
        # 提取结果
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

## Rust 自定义引擎

### 引擎特性实现

在 `src/engines/` 中创建一个 Rust 文件：

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
        // 实现搜索逻辑
        let url = format!("https://api.example.com/search?q={}", query.query);
        let response = self.client.get(&url).send().await?;
        let data: serde_json::Value = response.json().await?;
        
        // 解析响应
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

### 注册引擎

在 `src/engines/mod.rs` 中：

```rust
pub mod my_engine;

use crate::derive::engine::SearchEngine;
use std::sync::Arc;

pub fn get_all_engines() -> Vec<Arc<dyn SearchEngine>> {
    vec![
        Arc::new(my_engine::MyEngine::new()),
        // ... 其他引擎
    ]
}
```

## 最佳实践

### Python 引擎

1. **保持简单**：返回基本的字典结构
2. **处理错误**：使用 try-except 包装，并在失败时返回空列表
3. **尊重速率限制**：必要时添加延迟
4. **缓存结果**：考虑对重复查询进行缓存
5. **类型提示**：使用类型提示以获得更好的 IDE 支持

### Rust 引擎

1. **错误处理**：正确使用 `Result` 类型
2. **Async/await**：优先使用异步操作进行 I/O
3. **连接池**：重用 HTTP 客户端
4. **超时处理**：设置合理的超时时间
5. **测试**：为解析逻辑编写单元测试

## 测试自定义引擎

### Python

```python
# 测试你的引擎
def test_my_engine():
    from seesea.seesea.browser.my_engine import create_my_engine_callback_sync
    
    results = create_my_engine_callback_sync({
        "query": "test",
        "page": 1,
        "page_size": 10
    })
    
    assert len(results) > 0
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

## 示例

请参阅 `examples/` 目录获取完整的工作示例。

## 故障排除

**引擎未注册：**
- 检查文件名是否匹配 `create_{name}_callback_sync` 模式
- 确保文件位于 `seesea/seesea/browser/` 目录中
- 验证没有语法错误

**导入错误：**
- 安装所需依赖
- 检查 Python 路径

**Rust 编译错误：**
- 运行 `cargo check` 获取详细信息
- 验证特性实现是否完整

## 参考

- [搜索用法指南](./SEARCH_USAGE.md)
- [类型系统](./TYPE_SYSTEM.md)
- [全文搜索指南](./fulltext-search-guide.md)