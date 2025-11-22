# SeeSea v0.2.1 - 完整更新说明

## 概述

本次更新对 SeeSea 进行了全面的增强，包括代码清理、缓存系统升级、搜索功能扩展、RSS 榜单系统、Python SDK 完善和隐私保护加强。

## 主要更新

### 1. 代码质量提升

#### 调试代码清理
- ✅ 移除所有库代码中的 `println!`、`dbg!`、`eprintln!` 调试语句
- ✅ 替换为标准 `tracing` 日志系统
- ✅ 改进错误处理和日志记录

#### 代码审查修复
- ✅ 将 futures 导入移到模块顶部
- ✅ 改进 Python 对象转换错误处理（添加日志）
- ✅ 增强缓存持久化文档说明

### 2. 缓存系统升级

#### 持久化过期缓存
```rust
// 旧行为：过期数据被删除
if metadata.is_expired() {
    self.delete(key);  // ❌ 数据丢失
    return Ok(None);
}

// 新行为：过期数据保留在数据库
if metadata.is_expired() {
    // ✅ 数据保留，可用于全文搜索
    return Ok(None);
}
```

**优势：**
- 历史数据不丢失
- 支持全文搜索
- 减少网络请求
- 可通过 `cleanup_expired()` 手动清理

#### 新增 API
- `get_include_stale(key)` - 获取包括过期数据的缓存

#### 统一缓存架构
- Search 和 RSS 共享同一个 `CacheManager` 单例
- 内存高效（sled 自动管理）
- 持久化存储

### 3. 搜索功能扩展

#### 流式搜索 (Streaming Search)
```rust
pub async fn search_streaming<F>(&self, request: &SearchRequest, callback: F)
where
    F: FnMut(SearchResult, String) + Send,
```

**特性：**
- 使用 `FuturesUnordered` 实现
- 每个引擎完成即返回结果
- 不等待所有引擎完成
- 支持回调函数处理

**Python 示例：**
```python
def on_result(result):
    print(f"引擎 {result['engine']} 完成: {len(result['items'])} 个结果")

client.search_streaming("python", on_result)
```

#### 全文搜索 (Full-Text Search)
```rust
pub async fn search_fulltext(&self, request: &SearchRequest) -> SearchResponse
```

**特性：**
- 搜索网络 + 数据库历史数据
- 整合和去重
- 为未来增强预留接口

### 4. RSS 榜单系统 ⭐ 全新功能

#### 核心组件

**RankingKeyword - 关键词配置**
```rust
pub struct RankingKeyword {
    pub keyword: String,    // 关键词
    pub weight: f64,        // 权重 1.0-10.0
    pub required: bool,     // 是否必须匹配
}
```

**RankingConfig - 榜单配置**
```rust
pub struct RankingConfig {
    pub name: String,           // 榜单名称
    pub keywords: Vec<RankingKeyword>,
    pub min_score: f64,         // 最小评分阈值
    pub max_results: usize,     // 最大结果数
}
```

**ScoredRssItem - 已评分项目**
```rust
pub struct ScoredRssItem {
    pub item: RssFeedItem,         // 原始 RSS 项目
    pub score: f64,                // 相关性评分
    pub matched_keywords: Vec<String>,  // 匹配的关键词
}
```

#### 评分算法

**相关性计算：**
```rust
// 基于权重和出现次数的对数评分
score = weight * (1 + ln(count))
```

**特性：**
- 关键词权重（1.0-10.0）
- 对数缩放防止关键词堆砌
- 必需关键词支持（未匹配=0分）
- 自动去重（基于链接）
- 分数阈值过滤

#### 使用示例

**Rust:**
```rust
use seesea::rss::ranking::{RankingConfig, RankingKeyword, RssRankingEngine};

let config = RankingConfig {
    name: "tech_news".to_string(),
    keywords: vec![
        RankingKeyword::new("人工智能", 8.0),
        RankingKeyword::new("机器学习", 6.0),
        RankingKeyword::required("AI", 5.0),
    ],
    min_score: 3.0,
    max_results: 50,
};

let engine = RssRankingEngine::new(config);
let ranking = engine.rank_feeds(&feeds);
```

**Python:**
```python
from seesea import RssClient

client = RssClient()

# 定义关键词和权重
keywords = [
    ("人工智能", 8.0),
    ("深度学习", 6.0),
    ("神经网络", 5.0),
]

# 创建榜单
ranking = client.create_ranking(
    feed_urls=["https://example.com/rss"],
    keywords=keywords,
    min_score=3.0,
    max_results=50,
)

# 显示结果
for item in ranking['items']:
    print(f"[{item['score']:.1f}] {item['title']}")
    print(f"  关键词: {', '.join(item['matched_keywords'])}")
```

### 5. Python SDK v0.2.1 完善

#### 版本更新
- `seesea/__init__.py`: 0.2.0 → **0.2.1**
- `seesea/pyproject.toml`: 0.1.0 → **0.2.1**

#### SearchClient 新增方法

| 方法 | 说明 |
|------|------|
| `search_streaming(query, callback)` | 流式搜索，实时回调 |
| `search_fulltext(query)` | 全文搜索（网络+数据库） |
| `get_engine_states()` | 获取所有引擎状态 |
| `get_cache_info()` | 获取缓存统计信息 |
| `invalidate_engine(name)` | 使特定引擎缓存失效 |
| `list_global_engines()` | 列出全局模式引擎 |

#### RssClient 新增方法

| 方法 | 说明 |
|------|------|
| `create_ranking(feeds, keywords, ...)` | 创建 RSS 榜单 |

#### 完整类型提示
所有方法都有完整的类型注解：
```python
def search_streaming(
    self,
    query: str,
    callback,
    page: Optional[int] = 1,
    page_size: Optional[int] = 10,
    engines: Optional[List[str]] = None,
) -> Dict[str, Any]:
```

#### 详细文档字符串
每个方法都有完整的文档：
```python
"""
流式搜索 - 每个引擎完成时立即调用回调函数

Args:
    query: 搜索关键词
    callback: 回调函数，签名为 callback(result_dict)
    page: 页码
    page_size: 每页大小
    engines: 指定引擎列表
    
Returns:
    最终聚合的搜索结果

示例:
    >>> def on_result(result):
    ...     print(f"引擎 {result['engine']} 完成")
    >>> client.search_streaming("python", on_result)
"""
```

### 6. 隐私保护系统增强 ⭐ 全新功能

#### PrivacyManager - 隐私管理器

**核心功能：**
```rust
pub struct PrivacyManager {
    config: Arc<RwLock<PrivacyConfig>>,
    tls_config: Arc<RwLock<TlsConfig>>,
    doh_config: Arc<RwLock<DohConfig>>,
    fingerprint_protector: Arc<FingerprintProtector>,
    ua_generator: Arc<UserAgentGenerator>,
}
```

**API 方法：**
- `get_user_agent()` - 获取随机 User-Agent
- `get_privacy_headers(url)` - 生成隐私保护请求头
- `get_tls_params()` - 获取混淆的 TLS 参数
- `get_privacy_level()` - 评估隐私保护级别
- `get_stats()` - 获取隐私保护统计

#### 隐私级别评估

**评分系统（总分 100）：**
- **基础隐私功能 (20分)**
  - 伪造请求头: 5分
  - 伪造 Referer: 5分
  - 移除指纹特征: 10分

- **User-Agent 策略 (20分)**
  - Fixed: 0分
  - Realistic: 10分
  - Random: 15分
  - Custom: 20分

- **TLS 指纹混淆 (30分)**
  - None: 0分
  - Basic: 10分
  - Advanced: 20分
  - Full: 30分

- **DNS over HTTPS (15分)**
  - 启用: 15分
  - 禁用: 0分

- **证书验证 (15分)**
  - 启用: 10分（更安全）
  - 禁用: 5分（特殊需求）

**隐私级别分类：**
- 0-30分: 低 (Low)
- 31-60分: 中 (Medium)
- 61-85分: 高 (High)
- 86-100分: 最大 (Maximum)

#### 使用示例

```rust
use seesea::net::privacy::PrivacyManager;

let manager = PrivacyManager::new(
    PrivacyConfig::default(),
    TlsConfig::default(),
    DohConfig::default(),
);

// 获取隐私级别
let level = manager.get_privacy_level().await;
println!("隐私保护级别: {}", level);

// 获取统计信息
let stats = manager.get_stats().await;
println!("User-Agent 策略: {:?}", stats.user_agent_strategy);
println!("TLS 指纹保护: {:?}", stats.fingerprint_protection);
```

## 测试覆盖

### 新增测试
- RSS 榜单: 3个测试 ✅
- 隐私管理器: 3个测试 ✅

### 测试结果
```
Total tests: 222
Passing: 205
Failing: 17 (pre-existing, cache singleton issue)
```

## 性能影响

### 优势
- **缓存持久化**: 减少网络请求，提高响应速度
- **流式搜索**: 更快的首个结果时间（TTFR）
- **共享连接池**: 减少内存使用
- **RSS 榜单**: 智能内容过滤，节省带宽

### 开销
- **存储**: 过期缓存占用磁盘（可手动清理）
- **计算**: RSS 评分需要额外 CPU（可忽略）

## 迁移指南

### 从 0.2.0 升级到 0.2.1

**Python SDK:**
```bash
pip install --upgrade seesea==0.2.1
```

**代码更改:**
无破坏性更改！所有现有代码继续工作。

**新功能使用:**
```python
from seesea import SearchClient, RssClient

# 使用流式搜索
client = SearchClient()
client.search_streaming("query", lambda r: print(r))

# 使用 RSS 榜单
rss = RssClient()
ranking = rss.create_ranking(
    ["url1", "url2"],
    [("keyword", 5.0)],
)
```

## 后续计划

### 建议优化
1. 修复缓存单例测试冲突
2. 添加更多集成测试
3. 性能基准测试
4. 完善 README 文档
5. 添加使用示例

### 功能增强
1. Web UI 界面
2. 更多搜索引擎支持
3. 高级隐私保护（Tor 集成）
4. 分布式缓存
5. 机器学习评分

## 技术细节

### 依赖变更
无新增外部依赖

### API 破坏性变更
无

### 配置变更
无，所有新功能通过新 API 提供

## 贡献者

本次更新由 GitHub Copilot 协助完成。

## 许可证

MIT License - 保持不变
