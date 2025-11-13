# Transaction - 事务管理系统

一个高性能的 Rust 事务管理系统，参考 Python Transaction Manager 的设计理念，提供完整的事务支持。

## 特性

- **零隐式转换**: 所有转换都是显式的，确保最高性能
- **事务管理**: 支持事务的开始、提交、回滚和撤销
- **线程安全**: 使用 Arc 和 Mutex 确保线程安全
- **唯一事务ID**: 使用时间戳和 UUID 生成唯一的事务ID
- **调用记录**: 记录所有函数调用及其参数
- **错误处理**: 使用 error 模块进行统一错误处理
- **过程宏支持**: 提供装饰器风格的宏简化使用
- **测试驱动**: 所有代码都有完整的测试覆盖
- **零警告编译**: 所有代码编译时无警告

## 核心概念

### Transaction

事务对象，包含：
- 唯一的事务ID（时间戳 + UUID）
- 创建事务的线程ID
- 函数调用列表
- 执行和撤销功能

### TransactionManager

单例模式的事务管理器，负责：
- 管理当前活跃事务
- 维护事务历史记录
- 处理事务提交和回滚
- 支持事务查询和撤销

### Proxy

代理对象，用于：
- 延迟函数执行
- 记录函数调用
- 支持方法链式调用

### Callable

可调用对象trait，定义：
- `execute()`: 执行操作
- `undo()`: 撤销操作
- `name()`: 获取操作名称

## 使用示例

### 基本使用

```rust
use transaction::{TransactionManager, Callable};
use std::sync::Arc;

// 定义可调用对象
struct MyOperation {
    // ... 字段
}

impl Callable for MyOperation {
    fn execute(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // 执行操作
        Ok(())
    }

    fn undo(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // 撤销操作
        Ok(())
    }

    fn name(&self) -> &str {
        "my_operation"
    }
}

fn main() {
    // 创建事务管理器
    let tm = TransactionManager::new();

    // 开始事务
    tm.begin_transaction().unwrap();

    // 添加操作
    tm.add_call(Arc::new(MyOperation { /* ... */ })).unwrap();

    // 提交事务
    tm.commit_transaction().unwrap();
}
```

### 回滚事务

```rust
let tm = TransactionManager::new();

tm.begin_transaction().unwrap();
tm.add_call(Arc::new(MyOperation { /* ... */ })).unwrap();

// 回滚事务（操作不会被执行）
tm.rollback_transaction().unwrap();
```

### 撤销已提交的事务

```rust
let tm = TransactionManager::new();

tm.begin_transaction().unwrap();
let tx_id = tm.current_transaction().unwrap().transaction_id().to_string();

tm.add_call(Arc::new(MyOperation { /* ... */ })).unwrap();
tm.commit_transaction().unwrap();

// 撤销事务（调用 undo 方法）
tm.undo_transaction(&tx_id).unwrap();
```

### 使用代理

```rust
use transaction::{TransactionManager, Proxy};

let tm = TransactionManager::new();
tm.begin_transaction().unwrap();

let proxy = Proxy::new(tm.clone());
proxy.record_call(Arc::new(MyOperation { /* ... */ })).unwrap();

tm.commit_transaction().unwrap();
```

## API 文档

完整的中文 API 文档，包括：
- 模块级文档
- 类型文档
- 函数文档
- 示例代码
- 使用说明

生成文档：
```bash
cd src/crates/transaction
cargo doc --open
```

## 编译和测试

### 构建

```bash
cd src/crates/transaction
cargo build
cargo build --release
```

### 测试

```bash
cd src/crates/transaction
cargo test
```

> **注意**: 由于使用单例模式，集成测试可能在并行运行时相互干扰。如果遇到测试失败，请使用单线程运行：
> ```bash
> cargo test -- --test-threads=1
> ```

测试覆盖：
- 22 个单元测试
- 7 个集成测试
- 7 个文档测试
- 总计 36 个测试

### 运行示例

```bash
cd src/crates/transaction
cargo run --example basic
cargo run --example bank_transfer
```

## 设计原则

1. **显式优于隐式**: 所有转换和操作都是显式的
2. **性能第一**: 避免不必要的分配和复制
3. **类型安全**: 利用 Rust 的类型系统确保正确性
4. **错误处理**: 使用 Result 类型明确错误处理
5. **文档完备**: 所有公开 API 都有详细的中文文档
6. **测试驱动**: 所有功能都有对应的测试

## 性能特点

1. **零成本抽象**: 利用 Rust 的零成本抽象特性
2. **单例模式**: 使用 LazyLock 实现线程安全的单例
3. **最小锁粒度**: 使用细粒度的锁减少竞争
4. **显式转换**: 避免隐式转换的开销
5. **无额外分配**: 事务对象仅在创建时分配一次

## 与 Python 实现的对比

| 特性 | Python | Rust |
|------|--------|------|
| 类型安全 | 运行时检查 | 编译时检查 |
| 性能 | 解释执行 | 原生代码 |
| 并发 | GIL 限制 | 无锁并发 |
| 内存安全 | GC | 所有权系统 |
| 错误处理 | 异常 | Result 类型 |

## 项目结构

```
transaction/
├── Cargo.toml              # 项目配置
├── README.md               # 使用文档
├── src/
│   ├── lib.rs              # 库入口
│   ├── error.rs            # 错误定义
│   ├── transaction.rs      # 事务结构
│   ├── manager.rs          # 事务管理器
│   └── proxy.rs            # 代理对象
├── tests/
│   └── integration_tests.rs # 集成测试
└── examples/
    ├── basic.rs            # 基本示例
    └── bank_transfer.rs    # 银行转账示例
```

## 依赖项

- `error`: 错误处理框架（内部依赖）
- `tokio`: 异步运行时
- `uuid`: UUID 生成
- `transaction-derive`: 过程宏支持

## 未来扩展

1. 支持异步事务
2. 添加事务隔离级别
3. 支持分布式事务
4. 添加事务性能监控
5. 支持事务持久化

## 许可证

遵循项目根目录的许可证。
