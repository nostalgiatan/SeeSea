# 热点趋势模块

热点趋势模块提供多平台热点数据获取功能，支持知乎、微博、百度等主流平台的热点信息聚合。

## 核心类

### PyHotTrendClient

热点数据客户端类，提供统一的热点数据获取接口。

#### 构造函数

```python
client = PyHotTrendClient(max_concurrency=10)
```

创建热点数据客户端实例。

**参数：**
- `max_concurrency` (int, 可选): 最大并发请求数，默认为10

#### 方法

##### fetch_platform()

获取单个平台的热点数据。

**参数：**
- `platform_id` (str): 平台ID，支持以下平台：
  - `"zhihu"`: 知乎热榜
  - `"weibo"`: 微博热搜
  - `"baidu"`: 百度热搜
  - `"douyin"`: 抖音热点
  - `"bilibili"`: 哔哩哔哩热门
  - `"github"`: GitHub趋势
  - `"hackernews"`: Hacker News
  - `"reddit"`: Reddit热门

**返回值：**
- `dict`: 平台热点数据，包含以下字段：
  - `platform_id` (str): 平台ID
  - `platform_name` (str): 平台名称
  - `status` (str): 响应状态（"success" 或 "cache"）
  - `items` (list): 热点项列表，每个项包含：
    - `title` (str): 热点标题
    - `url` (str): 热点链接
    - `mobile_url` (str, 可选): 移动端链接
    - `rank` (int, 可选): 排名
    - `hot_value` (str, 可选): 热度值
    - `hot_index` (int, 可选): 热度指数
    - `source` (str, 可选): 来源
    - `publish_time` (str, 可选): 发布时间

**示例：**
```python
try:
    result = client.fetch_platform("zhihu")
    print(f"平台: {result['platform_name']}")
    print(f"状态: {result['status']}")
    
    for item in result['items']:
        print(f"{item['rank']}. {item['title']}")
        print(f"   链接: {item['url']}")
        if 'hot_value' in item:
            print(f"   热度: {item['hot_value']}")
except ValueError as e:
    print(f"平台ID无效: {e}")
except RuntimeError as e:
    print(f"获取数据失败: {e}")
```

##### fetch_all_platforms()

获取所有支持平台的热点数据。

**参数：**
无

**返回值：**
- `list`: 所有平台的热点数据列表，每个元素格式与`fetch_platform`返回值相同

**示例：**
```python
all_trends = client.fetch_all_platforms()

for platform_data in all_trends:
    print(f"\n=== {platform_data['platform_name']} ===")
    
    # 显示前5个热点
    for i, item in enumerate(platform_data['items'][:5]):
        print(f"{i+1}. {item['title']}")
        if 'hot_value' in item:
            print(f"   热度: {item['hot_value']}")
```

##### list_platforms()

列出所有支持的平台。

**参数：**
无

**返回值：**
- `dict`: 平台ID到平台名称的映射字典

**示例：**
```python
platforms = client.list_platforms()

print("支持的平台:")
for platform_id, platform_name in platforms.items():
    print(f"- {platform_id}: {platform_name}")
```

## 支持的平台

### 中文平台
- **知乎 (zhihu)**: 知乎热榜，包含热门问题和讨论
- **微博 (weibo)**: 微博热搜，实时热点话题
- **百度 (baidu)**: 百度热搜，搜索热点排行
- **抖音 (douyin)**: 抖音热点，短视频热门内容
- **哔哩哔哩 (bilibili)**: B站热门，视频平台热点

### 国际平台
- **GitHub (github)**: GitHub趋势，开源项目热度
- **Hacker News (hackernews)**: 技术新闻聚合
- **Reddit (reddit)**: Reddit热门，社区讨论热点

## 热点数据结构

### 热点项 (HotTrendItem)
```python
{
    "title": "热点标题",                    # 热点标题
    "url": "https://example.com",         # 热点链接
    "mobile_url": "https://m.example.com", # 移动端链接（可选）
    "rank": 1,                            # 排名（可选）
    "hot_value": "100万",                 # 热度值（可选）
    "hot_index": 95,                      # 热度指数（可选）
    "source": "知乎",                     # 来源（可选）
    "publish_time": "2024-01-01 12:00"    # 发布时间（可选）
}
```

### 平台结果 (HotTrendResult)
```python
{
    "platform_id": "zhihu",               # 平台ID
    "platform_name": "知乎",                # 平台名称
    "status": "success",                    # 响应状态
    "items": [...]                          # 热点项列表
}
```

## 缓存机制

### 缓存策略
- **内存缓存**: 热点数据在内存中缓存5分钟
- **状态标识**: 返回数据包含缓存状态（"cache" 或 "success"）
- **自动更新**: 缓存过期后自动重新获取

### 缓存状态
```python
if result['status'] == 'cache':
    print("数据来自缓存")
elif result['status'] == 'success':
    print("数据已更新")
```

## 错误处理

### 平台错误
```python
try:
    result = client.fetch_platform("invalid_platform")
except ValueError as e:
    print(f"无效的平台ID: {e}")
```

### 网络错误
```python
try:
    result = client.fetch_platform("zhihu")
except RuntimeError as e:
    if "network" in str(e).lower():
        print("网络连接失败")
    elif "timeout" in str(e).lower():
        print("请求超时")
    else:
        print(f"获取数据失败: {e}")
```

## 使用场景

### 热点监控
```python
import time

def monitor_hot_trends():
    client = PyHotTrendClient()
    
    while True:
        # 获取知乎热榜
        zhihu_trends = client.fetch_platform("zhihu")
        
        # 分析热点变化
        current_titles = [item['title'] for item in zhihu_trends['items']]
        
        # 保存或处理热点数据
        save_trends(current_titles)
        
        # 每10分钟检查一次
        time.sleep(600)
```

### 多平台聚合
```python
def aggregate_trends():
    client = PyHotTrendClient()
    
    # 获取所有平台数据
    all_trends = client.fetch_all_platforms()
    
    # 聚合分析
    trend_analysis = {}
    
    for platform_data in all_trends:
        platform_name = platform_data['platform_name']
        
        for item in platform_data['items'][:10]:  # 只分析前10个
            title = item['title']
            
            if title not in trend_analysis:
                trend_analysis[title] = {
                    'platforms': [],
                    'total_heat': 0
                }
            
            trend_analysis[title]['platforms'].append(platform_name)
            
            if 'hot_value' in item:
                trend_analysis[title]['total_heat'] += parse_heat_value(item['hot_value'])
    
    # 找出跨平台热点
    cross_platform = {
        title: data for title, data in trend_analysis.items()
        if len(data['platforms']) > 1
    }
    
    return cross_platform
```

### 趋势分析
```python
def analyze_trending_topics():
    client = PyHotTrendClient()
    
    # 获取GitHub趋势
    github_trends = client.fetch_platform("github")
    
    # 提取技术关键词
    tech_keywords = []
    for item in github_trends['items']:
        # 简单的关键词提取（实际应用中可使用NLP技术）
        words = item['title'].lower().split()
        tech_keywords.extend([w for w in words if len(w) > 3])
    
    # 统计词频
    from collections import Counter
    keyword_counts = Counter(tech_keywords)
    
    # 返回热门技术
    return keyword_counts.most_common(10)
```

## 性能优化

### 并发控制
```python
# 高并发场景
client = PyHotTrendClient(max_concurrency=20)

# 低并发场景（避免被封IP）
client = PyHotTrendClient(max_concurrency=5)
```

### 缓存利用
```python
def get_cached_trends(platform_id):
    result = client.fetch_platform(platform_id)
    
    if result['status'] == 'cache':
        # 使用缓存数据
        return result
    else:
        # 新获取的数据，可以立即使用
        return result
```

## 最佳实践

1. **错误处理**: 始终处理可能的网络和平台错误
2. **缓存利用**: 检查缓存状态，避免重复请求
3. **并发控制**: 根据网络环境调整并发数
4. **数据清洗**: 对获取的热点数据进行清洗和标准化
5. **频率控制**: 避免过于频繁的请求，合理设置检查间隔

## 相关模块

- [搜索模块](search.md): 结合热点数据进行搜索优化
- [RSS模块](rss.md): 与RSS源结合进行内容聚合
- [缓存模块](cache.md): 缓存热点数据提高性能
- [配置模块](config.md): 热点功能相关配置