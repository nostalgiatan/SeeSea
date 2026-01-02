# 浏览器引擎模块

浏览器引擎模块提供Python与Rust的浏览器自动化反向API接口，支持通过Python Playwright回调函数执行浏览器操作。

## 核心类

### PyBrowserConfig

浏览器配置类，用于配置浏览器引擎的各种参数。

#### 构造函数

```python
config = PyBrowserConfig(
    headless=True,              # 是否无头模式
    stealth=True,               # 是否启用隐身模式
    browser_type="chromium",    # 浏览器类型
    user_agent=None,            # 自定义User-Agent
    viewport_width=1920,       # 视口宽度
    viewport_height=1080       # 视口高度
)
```

#### 参数说明

**headless** (bool, 可选): 是否以无头模式运行浏览器，默认为True

**stealth** (bool, 可选): 是否启用隐身模式，默认为True

**browser_type** (str, 可选): 浏览器类型，支持以下值：
- `"chromium"`: Chromium浏览器（默认）
- `"firefox"`: Firefox浏览器
- `"webkit"`: WebKit浏览器

**user_agent** (str, 可选): 自定义User-Agent字符串，默认为None

**viewport_width** (int, 可选): 浏览器视口宽度，默认为1920

**viewport_height** (int, 可选): 浏览器视口高度，默认为1080

#### 属性

- `headless`: 无头模式状态
- `stealth`: 隐身模式状态
- `browser_type`: 浏览器类型
- `user_agent`: 自定义User-Agent
- `viewport_width`: 视口宽度
- `viewport_height`: 视口高度

#### 示例

```python
# 创建基本配置
config = PyBrowserConfig()

# 创建自定义配置
config = PyBrowserConfig(
    headless=False,                    # 显示浏览器窗口
    stealth=False,                     # 不启用隐身模式
    browser_type="firefox",            # 使用Firefox
    user_agent="Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
    viewport_width=1366,               # 设置视口宽度
    viewport_height=768               # 设置视口高度
)

# 修改配置属性
config.headless = True
config.viewport_width = 1920
```

### PyBrowserEngineClient

浏览器引擎客户端类，提供统一的浏览器操作接口。

#### 构造函数

```python
client = PyBrowserEngineClient()
```

创建浏览器引擎客户端实例。

#### 方法

##### register_playwright()

注册Playwright回调函数。

**函数签名:**
```python
register_playwright(callback: callable) -> None
```

**参数:**
- `callback` (callable): Python异步函数，接收参数字典并返回提取的数据

**说明:**
该函数用于注册Python端的Playwright回调函数。回调函数应该是一个异步函数，接收包含URL、操作列表、选择器和配置的字典参数，并返回提取的数据字典。

**回调函数签名:**
```python
async def playwright_callback(args: dict) -> dict:
    # args包含以下键值：
    # - url: 目标URL
    # - actions: 操作列表
    # - selectors: 选择器映射
    # - config: PyBrowserConfig实例
    
    # 返回提取的数据字典
    return {"data": "extracted_data"}
```

**示例:**
```python
async def my_playwright_callback(args):
    url = args["url"]
    actions = args["actions"]
    selectors = args["selectors"]
    config = args["config"]
    
    # 这里使用Playwright执行浏览器操作
    # 返回提取的数据
    return {"title": "页面标题", "content": "页面内容"}

client.register_playwright(my_playwright_callback)
```

##### execute()

执行浏览器操作。

**函数签名:**
```python
execute(
    url: str,
    actions: List[Dict[str, str]],
    selectors: Dict[str, str],
    config: PyBrowserConfig
) -> Dict[str, Any]
```

**参数:**
- `url` (str): 目标URL
- `actions` (List[Dict[str, str]]): 操作列表，每个操作是一个字典
- `selectors` (Dict[str, str]): 选择器映射，键为选择器名称，值为CSS选择器字符串
- `config` (PyBrowserConfig): 浏览器配置对象

**返回值:**
- `Dict[str, Any]`: 提取的数据字典

**异常:**
- `RuntimeError`: 当Playwright回调函数未注册时抛出

**操作列表格式:**
```python
actions = [
    {
        "type": "navigate",           # 操作类型
        "selector": "",              # 选择器（可选）
        "value": "",                 # 值（可选）
        "timeout": "30000"           # 超时时间（毫秒）
    },
    {
        "type": "click",
        "selector": "button.submit",
        "timeout": "5000"
    },
    {
        "type": "wait",
        "selector": "div.content",
        "timeout": "2000"
    },
    {
        "type": "extract",
        "selector": "title",
        "value": "text"
    }
]
```

**选择器映射格式:**
```python
selectors = {
    "title": "title",
    "content": "div.content",
    "button": "button.submit",
    "link": "a.external"
}
```

**示例:**
```python
# 创建配置
config = PyBrowserConfig(headless=False)

# 定义操作
actions = [
    {"type": "navigate", "timeout": "30000"},
    {"type": "wait", "selector": "div.main", "timeout": "2000"},
    {"type": "extract", "selector": "h1", "value": "text"},
    {"type": "extract", "selector": "div.content", "value": "text"}
]

# 定义选择器
selectors = {
    "title": "h1",
    "content": "div.content",
    "main": "div.main"
}

# 执行浏览器操作
result = client.execute(
    url="https://example.com",
    actions=actions,
    selectors=selectors,
    config=config
)

print(f"提取的数据: {result}")
```

##### is_registered()

检查是否已注册Playwright回调函数。

**函数签名:**
```python
is_registered() -> bool
```

**返回值:**
- `bool`: 如果已注册回调函数返回True，否则返回False

**示例:**
```python
client = PyBrowserEngineClient()
print(client.is_registered())  # False

client.register_playwright(my_callback)
print(client.is_registered())  # True
```

## 使用示例

### 基本使用

```python
from seesea_python_bindings import PyBrowserEngineClient, PyBrowserConfig

# 创建客户端
client = PyBrowserEngineClient()

# 定义Playwright回调函数
async def playwright_callback(args):
    url = args["url"]
    actions = args["actions"]
    selectors = args["selectors"]
    config = args["config"]
    
    # 这里使用Playwright执行实际的浏览器操作
    # 返回提取的数据
    return {
        "title": "示例页面标题",
        "content": "示例页面内容",
        "links": ["https://example.com/link1", "https://example.com/link2"]
    }

# 注册回调函数
client.register_playwright(playwright_callback)

# 创建配置
config = PyBrowserConfig(
    headless=False,
    browser_type="chromium",
    viewport_width=1920,
    viewport_height=1080
)

# 定义操作
actions = [
    {"type": "navigate", "timeout": "30000"},
    {"type": "wait", "selector": "body", "timeout": "1000"},
    {"type": "extract", "selector": "title", "value": "text"}
]

# 定义选择器
selectors = {
    "title": "title",
    "body": "body",
    "links": "a[href]"
}

# 执行浏览器操作
result = client.execute(
    url="https://example.com",
    actions=actions,
    selectors=selectors,
    config=config
)

print(f"提取结果: {result}")
```

### 高级使用

```python
# 创建多个配置的客户端
client = PyBrowserEngineClient()

# 移动端配置
mobile_config = PyBrowserConfig(
    headless=True,
    browser_type="chromium",
    user_agent="Mozilla/5.0 (iPhone; CPU iPhone OS 14_0 like Mac OS X)",
    viewport_width=375,
    viewport_height=667
)

# 桌面端配置
desktop_config = PyBrowserConfig(
    headless=False,
    browser_type="firefox",
    viewport_width=1920,
    viewport_height=1080
)

# 复杂的浏览器操作序列
complex_actions = [
    {"type": "navigate", "timeout": "30000"},
    {"type": "wait", "selector": "div.loading", "timeout": "2000"},
    {"type": "click", "selector": "button.accept-cookies", "timeout": "1000"},
    {"type": "wait", "selector": "div.content", "timeout": "3000"},
    {"type": "scroll", "selector": "body", "value": "bottom", "timeout": "2000"},
    {"type": "extract", "selector": "h1", "value": "text"},
    {"type": "extract", "selector": "div.article-content", "value": "text"},
    {"type": "extract", "selector": "img", "value": "src"}
]

# 复杂选择器映射
complex_selectors = {
    "title": "h1",
    "content": "div.article-content",
    "images": "img",
    "loading": "div.loading",
    "cookie_button": "button.accept-cookies",
    "main_content": "div.content"
}

# 执行复杂操作
result = client.execute(
    url="https://news-site.com/article",
    actions=complex_actions,
    selectors=complex_selectors,
    config=desktop_config
)

print(f"文章数据: {result}")
```

## 错误处理

```python
from seesea_python_bindings import PyBrowserEngineClient, PyBrowserConfig

client = PyBrowserEngineClient()

# 未注册回调函数时执行操作会抛出异常
try:
    config = PyBrowserConfig()
    result = client.execute(
        url="https://example.com",
        actions=[],
        selectors={},
        config=config
    )
except RuntimeError as e:
    print(f"错误: {e}")
    # 输出: 错误: Playwright callback not registered. Call register_playwright() first.

# 检查注册状态
if not client.is_registered():
    print("请先注册Playwright回调函数")
```

## 注意事项

1. **回调函数注册**: 在执行任何浏览器操作之前，必须先注册Playwright回调函数

2. **异步处理**: Playwright回调函数应该是异步函数，使用async/await语法

3. **超时设置**: 为每个操作设置合理的超时时间，避免长时间等待

4. **选择器准确性**: 确保CSS选择器准确匹配目标元素，避免操作失败

5. **错误处理**: 在回调函数中妥善处理可能出现的异常和错误情况

6. **资源清理**: 浏览器操作完成后，确保正确清理资源，避免内存泄漏

7. **并发限制**: 避免同时执行过多的浏览器实例，合理控制并发数量