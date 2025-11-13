# derive 模块

## 概述

`derive` 模块是搜索引擎的**抽象骨架结构**，提供核心的 trait 定义和类型系统。

本模块**只包含抽象定义**，不包含具体实现。

## 模块结构

- `error.rs` - 错误类型定义
- `types.rs` - 核心数据类型（SearchQuery, SearchResult, EngineInfo 等）
- `engine.rs` - 搜索引擎 trait 定义（4 个抽象 trait）
- `result.rs` - 结果处理 trait 定义
- `query.rs` - 查询处理 trait 定义
- `macros.rs` - 便利宏
- `mod.rs` - 模块导出

## 核心 Trait

### SearchEngine
基础搜索引擎接口，定义核心方法：
- `info()` - 获取引擎信息
- `search()` - 执行搜索
- `is_available()` - 检查可用性
- `health_check()` - 健康检查
- `validate_query()` - 验证查询

### ConfigurableEngine
可配置的搜索引擎接口

### CacheableEngine  
支持缓存的搜索引擎接口（抽象）

### RetryableEngine
支持重试的搜索引擎接口（抽象）

## 使用方式

具体的搜索引擎实现应该在其他模块中进行，例如：

```rust
use crate::derive::*;
use async_trait::async_trait;

pub struct MyEngine {
    // 你的具体实现字段
}

#[async_trait]
impl SearchEngine for MyEngine {
    fn info(&self) -> &EngineInfo {
        // 实现
    }
    
    async fn search(&self, query: &SearchQuery) -> Result<SearchResult> {
        // 实现
    }
}
```

## 注意

- ❌ 不要在此模块添加 HTTP 客户端
- ❌ 不要在此模块添加缓存实现
- ❌ 不要在此模块添加速率限制
- ❌ 不要在此模块添加具体的引擎实现

这些具体实现应该在各自的模块中完成。
