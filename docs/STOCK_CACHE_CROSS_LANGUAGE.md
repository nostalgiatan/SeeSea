# 股票缓存跨语言访问机制

## 📋 概述

SeeSea 股票模块已实现**跨语言缓存共享机制**，Rust 端和 Python 端可以无缝访问同一份缓存数据。

### ✅ 核心特性

1. **全局缓存实例** - Rust 和 Python 共享同一个 Sled 数据库
2. **作用域隔离** - 通过 scope 机制隔离不同类型的数据
3. **自动序列化** - 自动处理 JSON 序列化/反序列化
4. **零配置** - 无需手动指定缓存路径，系统自动管理

---

## 🏗️ 架构设计

### 缓存层级

```
全局缓存实例 (GLOBAL_CACHE_INSTANCE)
├─ Sled 数据库 (系统默认路径)
│  └─ Windows: D:\seesea\cache
│  └─ Linux: /etc/seesea/cache
│  └─ macOS: ~/Library/Caches/seesea
│
├─ 作用域隔离
│  ├─ stock.quote        # 股票行情 (5分钟 TTL)
│  ├─ stock.info         # 股票信息 (24小时 TTL)
│  ├─ stock.financial    # 财务数据 (24小时 TTL)
│  ├─ stock.kline        # K线数据 (12小时 TTL)
│  └─ ...更多作用域
│
└─ 跨语言访问
   ├─ Python: PyCacheInterface.scope(scope_name)
   └─ Rust: CacheInterface::scope(scope_name)
```

### 作用域命名规范

| 作用域 | TTL | 用途 |
|-------|-----|-----|
| `stock.realtime` | 5秒 | 实时数据 |
| `stock.quote` | 5分钟 | 股票行情 |
| `stock.fund_flow` | 5分钟 | 资金流向 |
| `stock.ranking` | 2分钟 | 股票排行榜 |
| `stock.kline.intraday` | 30分钟 | 日内K线 |
| `stock.index` | 30分钟 | 指数数据 |
| `stock.kline` | 12小时 | K线数据 |
| `stock.financial` | 24小时 | 财务数据 |
| `stock.info` | 24小时 | 股票基础信息 |
| `stock.list` | 24小时 | 股票列表 |
| `stock.announcements` | 1小时 | 公告数据 |

---

## 💻 使用示例

### Python 端写入

```python
from seesea.stock import initialize_stock_service, get_stock_service
from seesea.stock.cache import CacheScope, CacheTTL

# 1. 初始化服务（自动使用全局缓存）
initialize_stock_service()
service = get_stock_service()
cache = service._cache

# 2. 写入股票数据
stock_data = {
    "code": "000001",
    "name": "平安银行",
    "price": 12.34,
    "change_pct": 2.5
}

await cache.set(
    scope=CacheScope.STOCK_QUOTE,
    key="000001",
    data=stock_data,
    ttl=CacheTTL.QUOTE  # 5分钟
)
```

### Rust 端读取

```rust
use seesea_core::CacheInterface;
use serde::Deserialize;

#[derive(Deserialize)]
struct StockQuote {
    code: String,
    name: String,
    price: f64,
    change_pct: f64,
}

async fn get_stock(code: &str) -> Result<StockQuote> {
    // 1. 创建缓存接口（自动使用全局实例）
    let cache = CacheInterface::new(config)?;

    // 2. 获取股票行情作用域
    let quote_cache = cache.scope("stock.quote");

    // 3. 读取数据
    let data = quote_cache.get(code)?
        .ok_or("股票不存在")?;

    // 4. 反序列化
    let quote: StockQuote = serde_json::from_slice(&data)?;

    Ok(quote)
}
```

### Python 端读取

```python
# 方式1: 通过 StockCacheManager
cached = await cache.get(CacheScope.STOCK_QUOTE, "000001")

# 方式2: 直接通过 Rust 缓存接口
from seesea_core import PyCacheInterface

rust_cache = PyCacheInterface()
scope_cache = rust_cache.scope("stock.quote")
raw_bytes = scope_cache.get("000001")
data = json.loads(raw_bytes.decode('utf-8'))
```

---

## 🔧 技术实现

### 全局缓存实例机制

**Rust 端** (seesea-core/crates/seesea-python-bindings/src/py_cache.rs:80-95):

```rust
pub static GLOBAL_CACHE_INSTANCE: Lazy<Arc<CacheInterface>> = Lazy::new(|| {
    let cache_dir = get_cache_dir();
    let config = CacheImplConfig {
        db_path: cache_dir.to_string_lossy().to_string(),
        ..Default::default()
    };

    Arc::new(CacheInterface::new(config).unwrap())
});

#[pymethods]
impl PyCacheInterface {
    #[new]
    pub fn new(...) -> PyResult<Self> {
        // 无论传入什么参数，都使用全局实例
        Ok(Self {
            cache: GLOBAL_CACHE_INSTANCE.clone(),
        })
    }
}
```

**Python 端** (seesea/seesea/stock/cache.py:162-243):

```python
class StockCacheManager:
    def __init__(self):
        """自动使用全局缓存实例"""
        self._cache: Optional[Any] = None
        self._initialized = False

    def initialize_sync(self) -> bool:
        if CACHE_AVAILABLE:
            # 创建实例，内部自动指向全局缓存
            self._cache = PyCacheInterface()
            self._initialized = True
        return True
```

### 作用域访问

```python
# Python 端
scope_cache = cache_interface.scope("stock.quote")
scope_cache.set("000001", data, ttl_seconds=300)
value = scope_cache.get("000001")
```

```rust
// Rust 端
let scope_cache = cache.scope("stock.quote");
scope_cache.set("000001".to_string(), data, Some(Duration::from_secs(300)))?;
let value = scope_cache.get("000001")?;
```

---

## 🧪 测试验证

### 运行 Python 测试

```bash
cd /root/SeeSea
python examples/stock_cache_cross_language_test.py
```

**预期输出**:
```
🧪 股票缓存跨语言访问测试
=================================================
📦 步骤1: 初始化股票服务...
✅ 股票缓存初始化成功（使用全局缓存实例，支持跨语言访问）

✍️  步骤2: Python 端写入股票数据...
✅ 已写入股票数据到作用域: stock.quote

🔍 步骤3: Python 端读取验证...
✅ Python 端读取成功

🦀 步骤4: 通过 Rust 缓存接口访问...
✅ Rust 缓存接口读取成功
✅ 数据一致性验证通过！

🔐 步骤5: 测试作用域隔离...
✅ 作用域隔离验证通过！

✅ 所有测试通过！跨语言缓存访问功能正常！
```

### 运行 Rust 测试

```bash
cd /root/SeeSea
cargo run --example stock_cache_rust_access
```

---

## 📊 性能优化

### 缓存分层策略

```
L1: 内存缓存 (Python 端)
 └─> 超高频数据，亚毫秒访问

L2: Sled 数据库 (共享)
 └─> 持久化存储，毫秒级访问
```

### TTL 分级

| 级别 | TTL | 适用场景 |
|------|-----|---------|
| 实时 | 5秒 | 实时行情 |
| 短期 | 5分钟 | 行情、资金流向 |
| 中期 | 30分钟 | 日内K线、指数 |
| 长期 | 12小时 | 历史K线 |
| 持久 | 24小时 | 基础信息、财务数据 |

---

## 🛡️ 注意事项

### ✅ 优点

1. **零配置** - 自动使用系统默认路径
2. **线程安全** - Sled 数据库内置并发控制
3. **自动清理** - 过期数据自动删除
4. **作用域隔离** - 不同数据类型互不干扰

### ⚠️ 限制

1. **路径固定** - 不支持自定义缓存路径（按设计）
2. **序列化开销** - JSON 序列化有性能开销
3. **内存占用** - Python 端额外维护内存缓存

### 🔐 最佳实践

1. **使用作用域** - 始终通过作用域访问，避免键冲突
2. **设置合理TTL** - 根据数据特性设置过期时间
3. **批量操作** - 使用批量接口提升性能
4. **错误处理** - 缓存失败应降级到数据源

---

## 🔍 调试技巧

### 查看缓存内容

```python
from seesea_core import PyCacheInterface

cache = PyCacheInterface()
stats = cache.get_stats()
print(f"命中率: {stats['hit_rate']:.2%}")
print(f"总键数: {stats['total_keys']}")
```

### 清空指定作用域

```python
scope_cache = cache.scope("stock.quote")
# 注意：当前 PyScopeCache 没有 clear 方法，需要手动删除所有键
```

### 监控缓存大小

```bash
# Linux
du -sh /etc/seesea/cache

# Windows
dir D:\seesea\cache
```

---

## 📚 相关文档

- [缓存系统设计](./CACHE_ARCHITECTURE.md)
- [作用域机制详解](./CACHE_SCOPE.md)
- [股票模块API](./STOCK_API.md)

---

## 🤝 贡献

如有问题或建议，请提交 Issue 或 Pull Request。

**维护者**: SeeSea Team
**更新日期**: 2026-01-02
