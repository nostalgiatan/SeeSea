# SeeSea 搜索引擎系统使用指南

SeeSea 搜索引擎系统现已全面升级，支持基于 raming 系统的事件驱动架构，实现了完整的 Python-Rust 引擎通信机制。

## 🚀 核心特性

- **自动注册机制**：Python 引擎通过元类自动注册到 Rust 端
- **事件驱动通信**：使用 raming 共享内存和事件系统实现跨语言通信
- **无回调依赖**：完全基于事件通知，无需 Python 回调函数
- **统一管理**：EngineManager 统一管理 Rust 和 Python 引擎

## 📋 架构概览

```
Python 端                     Rust 端
┌─────────────────────┐       ┌─────────────────────┐
│  BaseSearchEngine   │       │  EngineManager      │
│  (元类自动注册)     │◄─────►│  (统一管理)         │
├─────────────────────┤       ├─────────────────────┤
│  - MockSearchEngine │       │  - BingEngine       │
│  - DemoEngine       │       │  - BaiduEngine      │
│  - NewsEngine       │       │  - PythonEngineProxy│
│  - ErrorTestEngine  │       │  - ...              │
└─────────────────────┘       └─────────────────────┘
          │                             │
          └───────────raming系统──────────┘
               (共享内存 + 事件通知)
```

## 🛠️ 使用方法

### 1. 创建自定义搜索引擎

```python
# 方法1: 继承 BaseSearchEngine（自动注册）
from seesea import engines

class MyCustomEngine(engines.BaseSearchEngine):
    # 引擎基础信息
    engine_name = "my_custom"
    engine_type = "web"
    description = "我的自定义搜索引擎"
    version = "1.0.0"
    author = "Your Name"

    # 引擎能力配置
    supports_pagination = True
    max_page_size = 30
    default_page_size = 10

    def search(self, query: str, page: int = 1, page_size: int = 10, **kwargs):
        # 实现您的搜索逻辑
        results = []
        # ... 搜索逻辑 ...

        return {
            "success": True,
            "results": results,
            "total_results": len(results),
            "elapsed_ms": 100
        }

# 引擎会自动注册到系统！
```

### 2. 使用装饰器注册引擎

```python
from seesea import engines

@engines.register_engine_class(
    name="decorated_engine",
    engine_type="news",
    description="使用装饰器的引擎",
    supports_pagination=True,
    max_page_size=20
)
class DecoratedEngine(engines.BaseSearchEngine):
    def search(self, query: str, page: int = 1, page_size: int = 10, **kwargs):
        # 实现搜索逻辑
        return {"success": True, "results": []}
```

### 3. Rust 端使用

```rust
use seesea_search::{EngineManager, EngineMode};
use seesea_derive::SearchQuery;

#[tokio::main]
async fn main() {
    // 创建引擎管理器（会自动启动 Python 引擎监听器）
    let manager = EngineManager::new(EngineMode::Global, vec![]);

    // 创建搜索查询
    let query = SearchQuery {
        query: "Rust programming".to_string(),
        page: 1,
        page_size: 10,
        language: None,
        region: None,
    };

    // 搜索所有引擎（包括 Python 引擎）
    let results = manager.search_with_python_engines(&query).await;

    // 查看所有可用引擎
    let all_engines = manager.get_all_available_engines().await;
    println!("可用引擎: {:?}", all_engines);

    // 处理搜索结果
    for (engine_name, result) in results {
        match result {
            Ok(search_result) => {
                println!("引擎 {} 返回 {} 条结果", engine_name, search_result.items.len());
            }
            Err(error) => {
                println!("引擎 {} 出错: {}", engine_name, error);
            }
        }
    }
}
```

## 🎯 示例引擎

系统自带了几个示例引擎供参考：

1. **MockSearchEngine** (`mock_search`)：模拟搜索引擎，返回测试数据
2. **DemoSearchEngine** (`demo_engine`)：演示引擎，展示装饰器用法
3. **NewsSearchEngine** (`news_mock`)：新闻搜索引擎示例
4. **ErrorTestEngine** (`error_test`)：错误测试引擎

```python
# 导入示例引擎
import seesea.engines.examples

# 引擎会自动注册，可以直接在 Rust 端使用
```

## 🔧 事件和内存区域命名规范

### Python → Rust 通信
- **引擎注册事件**: `engine_register`
- **引擎信息内存区域**: `engine_info_{engine_name}`

### 搜索过程通信
- **搜索请求内存区域**: `search_request_{request_id}`
- **搜索响应内存区域**: `search_response_{request_id}`
- **搜索请求事件**: `search_request`
- **搜索响应事件**: `search_response`

## 📊 工作流程

1. **引擎注册阶段**：
   - Python 创建 `engine_info_{name}` 内存区域
   - 写入引擎信息到共享内存
   - 发布 `engine_register` 事件
   - Rust 监听事件，读取并注册引擎
   - Rust 清理临时内存区域

2. **搜索执行阶段**：
   - Rust 创建 `search_request_{id}` 内存区域
   - 写入搜索参数到共享内存
   - 发布 `search_request` 事件
   - Python 引擎监听事件，处理请求
   - Python 创建 `search_response_{id}` 区域
   - Python 写入结果并发布 `search_response` 事件
   - Rust 读取结果并清理内存区域

## 🛡️ 错误处理

系统提供了完善的错误处理机制：

```python
class MyEngine(engines.BaseSearchEngine):
    def search(self, query: str, **kwargs):
        try:
            # 搜索逻辑
            results = self.do_search(query)
            return {
                "success": True,
                "results": results
            }
        except Exception as e:
            return {
                "success": False,
                "error": str(e),
                "results": []
            }
```

## 📈 性能优化

1. **共享内存通信**：避免数据序列化开销
2. **事件驱动架构**：异步非阻塞通信
3. **全局实例管理**：避免重复初始化
4. **内存自动清理**：防止内存泄漏

## 🔍 调试和监控

```python
# 查看引擎状态
from seesea import engines

status = engines.get_search_engine_status()
print("引擎系统状态：", status)

# 查看已注册的引擎
registered_engines = engines.list_search_engines()
print("已注册引擎：", registered_engines)
```

---

**注意事项**：
- 引擎名称必须唯一
- 必须实现 `search` 方法
- 搜索结果必须符合指定格式
- 推荐使用元类自动注册机制
- 系统会自动处理跨语言通信细节