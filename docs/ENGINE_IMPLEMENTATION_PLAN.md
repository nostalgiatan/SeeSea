# 搜索引擎实现计划

## 目标
对照 SearXNG 项目，为 SeeSea 添加缺失的搜索引擎实现，确保每个引擎的请求和解析都一一对应。

## 核心引擎列表 (11个)

| # | 引擎 | 快捷码 | 状态 | Rust实现文件 | SearXNG参考 |
|---|------|--------|------|-------------|------------|
| 1 | Bing | `bi` | ✅ 已实现 | `bing.rs` | `engines/bing.py` |
| 2 | DuckDuckGo | `ddg` | ✅ 已实现 | `duckduckgo.rs` | `engines/duckduckgo.py` |
| 3 | Brave | `br` | ✅ 已实现 | `brave.rs` | `engines/brave.py` |
| 4 | Startpage | `sp` | ✅ 已实现 | `startpage.rs` | `engines/startpage.py` |
| 5 | 360搜索 | `360so` | ❌ 待实现 | - | `engines/360search.py` |
| 6 | Wikipedia | `wp` | ❌ 待实现 | - | `engines/wikipedia.py` |
| 7 | Wikidata | `wd` | ❌ 待实现 | - | `engines/wikidata.py` |
| 8 | GitHub | `gh` | ❌ 待实现 | - | `engines/github.py` |
| 9 | Stack Overflow | `st` | ❌ 待实现 | - | `engines/stackoverflow.py` |
| 10 | Unsplash | `us` | ❌ 待实现 | - | `engines/unsplash.py` |

## 实现优先级

### 高优先级 (P0)
1. **360search** - 中国本土搜索引擎
2. **Wikipedia** - 常用百科
3. **GitHub** - 开发者必备

### 中优先级 (P1)
4. **Stack Overflow** - 开发问答
5. **Wikidata** - 知识库

### 低优先级 (P2)
6. **Unsplash** - 图片资源

## 实现步骤

每个引擎的实现需要遵循以下步骤：

### 1. 研究 SearXNG 实现
- [ ] 阅读 Python 引擎代码
- [ ] 理解请求参数构建
- [ ] 理解 HTML/JSON 解析逻辑
- [ ] 记录关键的 headers 和 cookies
- [ ] 理解机器人检测机制

### 2. 创建 Rust 实现
- [ ] 创建 `src/search/engines/{engine_name}.rs`
- [ ] 实现 `SearchEngine` trait
- [ ] 实现 `RequestResponseEngine` trait
- [ ] `request()` - 构建请求参数
- [ ] `fetch()` - 发送 HTTP 请求
- [ ] `response()` - 解析响应

### 3. 对比验证
- [ ] URL 格式一致
- [ ] Headers 完全匹配
- [ ] 表单字段顺序一致
- [ ] HTML/JSON 选择器正确
- [ ] 结果字段映射正确

### 4. 测试
- [ ] 单元测试
- [ ] 集成测试
- [ ] 实际搜索测试

## 360search 实现计划

### SearXNG 参考
- 文件: `src/python/searx/engines/360search.py`
- 类型: HTML 解析
- URL: `https://www.so.com/`

### 关键要点
```python
# Request URL
url = 'https://www.so.com/s'

# Query parameters
params = {
    'q': query,
    'pn': pageno,  # page number
}

# Result parsing
results_xpath = '//ul[@class="result"]/li'
title_xpath = './/h3/a'
url_xpath = './/h3/a/@href'
content_xpath = './/p[@class="res-desc"]'
```

### 实现任务
- [ ] 创建 `360search.rs`
- [ ] 实现基本搜索
- [ ] 实现分页
- [ ] 解析结果
- [ ] 测试验证

## Wikipedia 实现计划

### SearXNG 参考
- 文件: `src/python/searx/engines/wikipedia.py`
- 类型: REST API (JSON)
- URL: `https://{lang}.wikipedia.org/api/rest_v1/page/summary/{title}`

### 关键要点
```python
# API endpoint
rest_v1_summary_url = 'https://{wiki_netloc}/api/rest_v1/page/summary/{title}'

# Language support
# Use Accept-Language header for LanguageConverter

# Result parsing (JSON)
item['title'] = result['title']
item['url'] = result['content_urls']['desktop']['page']
item['content'] = result['extract']
item['thumbnail'] = result.get('thumbnail', {}).get('source')
```

### 实现任务
- [ ] 创建 `wikipedia.rs`
- [ ] 实现多语言支持
- [ ] 解析 JSON 响应
- [ ] 处理缩略图
- [ ] infobox 显示类型

## GitHub 实现计划

### SearXNG 参考
- 文件: `src/python/searx/engines/github.py`
- 类型: GitHub API
- URL: `https://api.github.com/search/repositories`

### 关键要点
```python
# API endpoint
search_url = 'https://api.github.com/search/repositories?q={query}'

# Headers
headers = {
    'Accept': 'application/vnd.github.v3+json'
}

# Result parsing (JSON)
for item in results['items']:
    result = {
        'title': item['full_name'],
        'url': item['html_url'],
        'content': item.get('description'),
        'metadata': {
            'stars': item['stargazers_count'],
            'forks': item['forks_count'],
        }
    }
```

### 实现任务
- [ ] 创建 `github.rs`
- [ ] GitHub API 认证（可选）
- [ ] 解析 JSON 响应
- [ ] 显示 stars/forks
- [ ] 代码搜索变体

## Stack Overflow 实现计划

### SearXNG 参考
- 文件: `src/python/searx/engines/stackoverflow.py`
- 类型: Stack Exchange API
- URL: `https://api.stackexchange.com/2.2/search`

### 关键要点
```python
# API endpoint
base_url = 'https://api.stackexchange.com/2.2/search'

# Parameters
params = {
    'order': 'desc',
    'sort': 'relevance',
    'intitle': query,
    'site': 'stackoverflow'
}

# Result parsing
for item in data['items']:
    result = {
        'title': item['title'],
        'url': item['link'],
        'content': extract_text(item.get('body')),
        'metadata': {
            'score': item['score'],
            'answers': item['answer_count']
        }
    }
```

### 实现任务
- [ ] 创建 `stackoverflow.rs`
- [ ] Stack Exchange API
- [ ] 解析 JSON
- [ ] 显示 score/answers
- [ ] 处理 HTML 内容

## Wikidata 实现计划

### SearXNG 参考
- 文件: `src/python/searx/engines/wikidata.py`
- 类型: Wikidata API
- URL: `https://www.wikidata.org/w/api.php`

### 关键要点
```python
# API endpoint
url = 'https://www.wikidata.org/w/api.php'

# Parameters
params = {
    'action': 'wbsearchentities',
    'search': query,
    'language': language,
    'format': 'json'
}

# Result parsing
for item in data['search']:
    result = {
        'title': item['label'],
        'url': item['concepturi'],
        'content': item.get('description')
    }
```

### 实现任务
- [ ] 创建 `wikidata.rs`
- [ ] Wikidata API
- [ ] 多语言支持
- [ ] 实体解析

## Unsplash 实现计划

### SearXNG 参考
- 文件: `src/python/searx/engines/unsplash.py`
- 类型: Unsplash API
- URL: `https://unsplash.com/napi/search/photos`

### 关键要点
```python
# API endpoint
url = 'https://unsplash.com/napi/search/photos'

# Parameters
params = {
    'query': query,
    'per_page': 20,
    'page': pageno
}

# Result parsing
for item in results['results']:
    result = {
        'title': item['description'] or item['alt_description'],
        'url': item['links']['html'],
        'img_src': item['urls']['regular'],
        'thumbnail_src': item['urls']['thumb'],
        'content': f"by {item['user']['name']}"
    }
```

### 实现任务
- [ ] 创建 `unsplash.rs`
- [ ] 图片搜索
- [ ] 缩略图处理
- [ ] 作者信息

## 通用实现模板

每个引擎的 Rust 实现应遵循以下模板：

```rust
use async_trait::async_trait;
use std::collections::HashMap;
use std::error::Error;

use crate::derive::{...};
use crate::net::client::HttpClient;

pub struct {Engine}Engine {
    info: EngineInfo,
    client: HttpClient,
}

impl {Engine}Engine {
    pub fn new() -> Self {
        // 初始化
    }
    
    fn parse_results(data: &str) -> Result<Vec<SearchResultItem>, ...> {
        // 参照 SearXNG 的解析逻辑
    }
}

#[async_trait]
impl SearchEngine for {Engine}Engine {
    fn info(&self) -> &EngineInfo {
        &self.info
    }

    async fn search(&self, query: &SearchQuery) -> Result<SearchResult, ...> {
        // 调用 RequestResponseEngine
    }
}

#[async_trait]
impl RequestResponseEngine for {Engine}Engine {
    type Response = String; // 或 serde_json::Value

    fn request(&self, query: &str, params: &mut RequestParams) -> Result<(), ...> {
        // 参照 SearXNG 的 request() 函数
        // 设置 URL, headers, data, cookies
    }

    async fn fetch(&self, params: &RequestParams) -> Result<Self::Response, ...> {
        // 发送 HTTP 请求
    }

    fn response(&self, resp: Self::Response) -> Result<Vec<SearchResultItem>, ...> {
        // 参照 SearXNG 的 response() 函数
        // 解析 HTML 或 JSON
    }
}
```

## 验证清单

每个引擎实现完成后需要验证：

- [ ] URL 格式与 SearXNG 一致
- [ ] HTTP 方法 (GET/POST) 正确
- [ ] Headers 完整
- [ ] Cookies 正确
- [ ] 查询参数顺序（如果重要）
- [ ] 表单数据顺序（如果重要）
- [ ] HTML 选择器精确
- [ ] JSON 字段路径正确
- [ ] 结果字段映射完整
- [ ] 分页逻辑正确
- [ ] 错误处理完善
- [ ] 编译通过
- [ ] 测试通过

## 时间估算

每个引擎的实现时间：
- 简单引擎 (API based): 2-4 小时
- 中等引擎 (HTML parsing): 4-6 小时  
- 复杂引擎 (特殊逻辑): 6-8 小时

总计估算: 30-40 小时

## 参考资源

- SearXNG 源代码: `src/python/searx/engines/`
- SearXNG 文档: https://docs.searxng.org/
- 各引擎官方 API 文档
- 你的项目已实现引擎: `src/search/engines/`
