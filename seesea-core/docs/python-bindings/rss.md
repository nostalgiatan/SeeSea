# RSS模块文档

## 概述

RSS模块提供了RSS源聚合、订阅管理、内容解析和智能排名功能。该模块基于 Rust 的异步RSS接口，通过 PyO3 绑定到 Python，支持多种RSS格式和模板化订阅。

## 核心类

### PyRssClient

RSS客户端类，提供统一的RSS源访问和管理功能。

#### 构造函数

```python
client = PyRssClient()
```

创建RSS客户端实例，自动初始化：
- Tokio运行时
- HTTP客户端
- RSS缓存
- 模板目录

#### 方法

##### fetch_feed()

获取RSS源内容。

**参数：**
- `url` (str): RSS源URL
- `max_items` (int, 可选): 最大项目数量
- `filter_keywords` (List[str], 可选): 过滤关键词列表

**返回值：**
字典对象，包含以下字段：
- `meta`: 元数据字典：
  - `title`: Feed标题
  - `link`: Feed链接
  - `description`: Feed描述
  - `language`: 语言
- `items`: 项目列表，每个项目包含：
  - `title`: 标题
  - `link`: 链接
  - `description`: 描述/摘要
  - `author`: 作者
  - `pub_date`: 发布日期
  - `content`: 完整内容
  - `categories`: 分类列表

**示例：**
```python
feed = client.fetch_feed(
    url="https://news.ycombinator.com/rss",
    max_items=20,
    filter_keywords=["AI", "machine learning"]
)

print(f"Feed标题: {feed['meta']['title']}")
for item in feed['items']:
    print(f"标题: {item['title']}")
    print(f"链接: {item['link']}")
    print(f"发布日期: {item['pub_date']}")
```

##### parse_feed()

解析RSS源内容（从字符串）。

**参数：**
- `content` (str): RSS内容字符串

**返回值：**
与 fetch_feed() 方法格式相同的Feed字典

**示例：**
```python
with open('feed.xml', 'r', encoding='utf-8') as f:
    content = f.read()

feed = client.parse_feed(content)
print(f"解析到 {len(feed['items'])} 个项目")
```

##### list_templates()

列出可用的RSS模板。

**返回值：**
模板名称列表（List[str]）

**示例：**
```python
templates = client.list_templates()
print(f"可用模板: {templates}")
```

##### add_from_template()

从模板添加RSS源。

**参数：**
- `template_name` (str): 模板名称
- `categories` (List[str], 可选): 分类列表

**返回值：**
添加的RSS源数量（int）

**示例：**
```python
count = client.add_from_template(
    template_name="tech_news",
    categories=["AI", "Programming", "Security"]
)
print(f"添加了 {count} 个RSS源")
```

##### create_ranking()

创建RSS榜单 - 基于关键词对RSS项目进行评分和排名。

**参数：**
- `feed_urls` (List[str]): RSS源URL列表
- `keywords` (List[tuple]): 关键词及权重列表 [(keyword, weight), ...]
- `min_score` (float, 可选): 最小评分阈值，默认为 0.0
- `max_results` (int, 可选): 最大结果数，默认为 100

**返回值：**
字典对象，包含：
- `name`: 榜单名称
- `total_items`: 总项目数量
- `timestamp`: 时间戳
- `items`: 排序后的项目列表，每个项目包含：
  - `title`: 标题
  - `link`: 链接
  - `description`: 描述
  - `pub_date`: 发布日期
  - `score`: 评分
  - `matched_keywords`: 匹配的关键词列表

**示例：**
```python
# 定义关键词和权重
keywords = [
    ("人工智能", 2.0),
    ("机器学习", 1.8),
    ("深度学习", 1.5),
    ("AI", 2.0),
    ("neural network", 1.2),
    ("GPT", 1.0)
]

# RSS源列表
feed_urls = [
    "https://techcrunch.com/feed/",
    "https://www.wired.com/feed/rss",
    "https://venturebeat.com/feed/"
]

# 创建榜单
ranking = client.create_ranking(
    feed_urls=feed_urls,
    keywords=keywords,
    min_score=0.5,
    max_results=50
)

print(f"榜单名称: {ranking['name']}")
print(f"总项目数: {ranking['total_items']}")

# 显示前10个高评分项目
for i, item in enumerate(ranking['items'][:10]):
    print(f"{i+1}. {item['title']} (评分: {item['score']:.2f})")
    print(f"   匹配关键词: {item['matched_keywords']}")
    print(f"   链接: {item['link']}")
```

## RSS数据类型

### RssFeed

完整的RSS源结构：

```python
{
    "meta": {
        "title": str,           # Feed标题
        "link": str,            # Feed链接
        "description": str,     # Feed描述
        "language": str,        # 语言
        "copyright": str,         # 版权信息
        "last_build_date": str, # 最后构建日期
        "pub_date": str,        # 发布日期
        "image": {              # 图片信息（可选）
            "url": str,
            "title": str,
            "link": str,
            "width": int,
            "height": int
        }
    },
    "items": [{
        "title": str,           # 标题
        "link": str,            # 链接
        "description": str,     # 描述
        "author": str,          # 作者
        "pub_date": str,        # 发布日期
        "content": str,         # 完整内容
        "categories": [str],    # 分类列表
        "guid": str,            # 唯一标识符
        "enclosures": [{       # 附件列表
            "url": str,
            "mime_type": str,
            "length": int
        }],
        "custom_fields": {}    # 自定义字段
    }]
}
```

### RssFeedQuery

RSS查询参数：

```python
{
    "url": str,                 # Feed URL
    "max_items": int,           # 最大项目数
    "filter_keywords": [str],   # 过滤关键词
    "after_date": str           # 日期过滤（只获取此日期之后的项目）
}
```

## 缓存机制

RSS模块实现了智能缓存：

1. **全局缓存实例**: 复用缓存资源，避免重复创建
2. **Feed缓存**: 按URL分别缓存RSS源内容
3. **模板缓存**: 缓存模板化的RSS源配置
4. **自动过期**: 基于TTL的自动缓存过期

## 模板系统

RSS模块支持模板化订阅：

### 模板结构
```json
{
    "name": "tech_news",
    "description": "科技新闻RSS源集合",
    "feeds": [
        {
            "name": "TechCrunch",
            "url": "https://techcrunch.com/feed/",
            "categories": ["startup", "venture"]
        },
        {
            "name": "Wired",
            "url": "https://www.wired.com/feed/rss",
            "categories": ["security", "culture"]
        }
    ]
}
```

### 使用模板
```python
# 按分类添加
count = client.add_from_template(
    "tech_news",
    categories=["AI", "security"]
)

# 添加所有源
count = client.add_from_template("tech_news")
```

## 排名算法

### 关键词评分

基于关键词匹配的项目评分算法：

1. **精确匹配**: 关键词完全匹配，获得完整权重
2. **部分匹配**: 关键词部分匹配，获得部分权重
3. **语义匹配**: 基于语义相似度的匹配
4. **位置权重**: 关键词在标题、描述中的位置影响权重

### 评分公式

```
score = Σ(keyword_weight × match_strength × position_factor)
```

其中：
- `keyword_weight`: 关键词权重
- `match_strength`: 匹配强度（0.0-1.0）
- `position_factor`: 位置因子（标题 > 描述 > 内容）

## 错误处理

RSS模块会抛出以下 Python 异常：

- `PyRuntimeError`: 运行时错误，如网络失败、解析错误、模板加载失败等
- `PyValueError`: 参数值错误，如无效的URL、格式错误的内容等
- `PyIOError`: IO错误，如文件读写失败

## 性能优化

### 1. 并发获取

支持多个RSS源的并发获取：
```python
# 并发获取多个源
feeds = []
for url in feed_urls:
    feed = client.fetch_feed(url, max_items=20)
    feeds.append(feed)
```

### 2. 缓存利用

合理利用缓存减少网络请求：
```python
# 获取缓存统计
from seesea_core import PyCacheInterface
cache = PyCacheInterface()
stats = cache.get_stats()
print(f"RSS缓存命中率: {stats['hit_rate']:.2%}")
```

### 3. 过滤优化

在服务器端进行关键词过滤：
```python
# 服务器端过滤，减少数据传输
feed = client.fetch_feed(
    url,
    filter_keywords=["AI", "machine learning"],
    max_items=50
)
```

## 使用场景

### 1. 新闻聚合
```python
# 聚合多个新闻源
news_sources = [
    "https://rss.cnn.com/rss/edition.rss",
    "https://feeds.bbci.co.uk/news/rss.xml",
    "https://www.reuters.com/rss/topNews.xml"
]

all_items = []
for url in news_sources:
    feed = client.fetch_feed(url, max_items=10)
    all_items.extend(feed['items'])

# 按日期排序
all_items.sort(key=lambda x: x['pub_date'], reverse=True)
```

### 2. 技术趋势监控
```python
# 监控技术趋势关键词
tech_keywords = [
    ("blockchain", 1.5),
    ("cryptocurrency", 1.3),
    ("NFT", 1.2),
    ("DeFi", 1.4)
]

tech_feeds = [
    "https://cointelegraph.com/rss",
    "https://coindesk.com/feed"
]

ranking = client.create_ranking(
    feed_urls=tech_feeds,
    keywords=tech_keywords,
    max_results=20
)

# 输出热门文章
for item in ranking['items'][:5]:
    print(f"[{item['score']:.1f}] {item['title']}")
```

### 3. 内容推荐
```python
def get_personalized_feed(user_interests, rss_urls):
    """基于用户兴趣生成个性化Feed"""
    
    # 创建基于兴趣的关键词权重
    keywords = [(interest, 1.0) for interest in user_interests]
    
    # 生成排名
    ranking = client.create_ranking(
        feed_urls=rss_urls,
        keywords=keywords,
        min_score=0.3,  # 降低阈值获取更多内容
        max_results=30
    )
    
    return ranking['items']

# 用户兴趣
user_interests = ["python", "data science", "machine learning"]

# RSS源
rss_urls = [
    "https://realpython.com/atom.xml",
    "https://towardsdatascience.com/feed",
    "https://blog.keras.io/feeds/all.atom.xml"
]

# 获取个性化内容
personalized = get_personalized_feed(user_interests, rss_urls)
```

## 注意事项

1. **频率限制**: 注意RSS源的访问频率限制，避免被封禁
2. **缓存策略**: 合理利用缓存，减少对源站的压力
3. **错误处理**: 处理网络异常、解析错误等情况
4. **内容版权**: 尊重内容版权，合理使用RSS内容
5. **数据格式**: 不同RSS源可能使用不同的格式和标准
6. **编码问题**: 注意处理不同编码的RSS内容

## 相关模块

- [缓存模块](cache.md): RSS数据缓存
- [网络模块](network.md): HTTP客户端和网络请求
- [配置模块](config.md): RSS源配置管理
- [搜索模块](search.md): RSS内容搜索