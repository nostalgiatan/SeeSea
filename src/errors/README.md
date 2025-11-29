# 错误处理系统

SeeSea 项目的错误处理系统提供了结构化、模块化的错误定义和创建函数，便于统一管理和使用错误。

## 设计目标

- 提供清晰的错误分类和错误码
- 便于扩展和维护
- 保持与现有代码的兼容性
- 提供友好的错误消息
- 支持错误链和错误上下文

## 目录结构

```
src/errors/
├── mod.rs          # 统一入口，重新导出所有错误类型和函数
├── network.rs      # 网络相关错误定义
├── search.rs       # 搜索相关错误定义
├── parse.rs        # 解析相关错误定义
├── validation.rs   # 验证相关错误定义
├── io.rs           # IO相关错误定义
├── permission.rs   # 权限相关错误定义
├── configuration.rs # 配置相关错误定义
├── database.rs     # 数据库相关错误定义
├── business.rs     # 业务逻辑相关错误定义
├── system.rs       # 系统相关错误定义
└── test.rs         # 测试用例
```

## 错误码设计

采用分层错误码设计，每个错误类别有独立的错误码范围：

| 错误类别 | 错误码范围 | 模块文件 |
|---------|-----------|---------|
| 网络错误 | 1000-1999 | network.rs |
| 搜索错误 | 2000-2999 | search.rs |
| 解析错误 | 3000-3999 | parse.rs |
| 验证错误 | 4000-4999 | validation.rs |
| IO错误   | 5000-5999 | io.rs |
| 权限错误 | 6000-6999 | permission.rs |
| 配置错误 | 7000-7999 | configuration.rs |
| 数据库错误 | 8000-8999 | database.rs |
| 业务逻辑错误 | 9000-9999 | business.rs |
| 系统错误 | 10000-10999 | system.rs |

## 使用方法

### 1. 导入错误模块

```rust
// 导入所有错误类型和函数
use crate::errors::*;

// 或只导入特定模块
use crate::errors::network;
use crate::errors::search;
```

### 2. 创建错误

```rust
// 使用特定的错误创建函数
let error = network::connection_timeout("example.com");
let error = search::engine_unavailable("bing");

// 使用通用的错误创建函数
let error = network::network_error("网络错误");
let error = search::search_error("搜索错误");
```

### 3. 错误属性

```rust
// 获取错误码
let code = error.code();

// 获取错误消息
let message = error.message();

// 获取错误类别
let category = error.category();

// 获取错误严重性
let severity = error.severity();
```

### 4. 错误链

```rust
// 创建带有源错误的错误
let source_error = network::connection_timeout("example.com");
let error = search::engine_error("bing", &source_error.message());
```

## 示例

### 网络错误示例

```rust
use crate::errors::network;

// 创建连接超时错误
let error = network::connection_timeout("example.com");
assert_eq!(error.code(), network::CONNECTION_TIMEOUT);
assert_eq!(error.category(), ErrorCategory::Network);
assert!(error.message().contains("example.com"));

// 创建DNS解析失败错误
let error = network::dns_resolve_failed("invalid-domain.example");
assert_eq!(error.code(), network::DNS_RESOLVE_FAILED);
assert!(error.message().contains("无法解析域名"));
```

### 搜索错误示例

```rust
use crate::errors::search;

// 创建引擎不可用错误
let error = search::engine_unavailable("bing");
assert_eq!(error.code(), search::ENGINE_UNAVAILABLE);
assert_eq!(error.category(), ErrorCategory::Search);

// 创建搜索超时错误
let error = search::search_timeout("bing");
assert_eq!(error.code(), search::SEARCH_TIMEOUT);
assert!(error.message().contains("搜索超时"));
```

### 验证错误示例

```rust
use crate::errors::validation;

// 创建空字段错误
let error = validation::empty_field("username");
assert_eq!(error.code(), validation::EMPTY_FIELD);
assert!(error.message().contains("不能为空"));

// 创建无效邮箱错误
let error = validation::invalid_email("invalid-email");
assert_eq!(error.code(), validation::INVALID_EMAIL);
assert!(error.message().contains("无效的邮箱地址"));
```

## 向后兼容性

为了保持与现有代码的兼容性，系统保留了原有的 `network_error` 和 `search_error` 函数：

```rust
// 旧代码仍然可以正常工作
let error = crate::error::network_error("网络错误");
let error = crate::error::search_error("搜索错误");
```

## 扩展新的错误类型

要扩展新的错误类型，只需在相应的模块文件中添加新的错误码常量和创建函数，或创建新的模块文件。

### 示例：添加新的网络错误类型

在 `network.rs` 文件中添加：

```rust
// 添加新的错误码常量
pub const NEW_ERROR_TYPE: u32 = NETWORK_ERROR_BASE + 11;

// 添加新的错误创建函数
pub fn new_error_type(param: &str) -> ErrorInfo {
    ErrorInfo::new(NEW_ERROR_TYPE, format!("新的错误类型: {}", param))
        .with_category(ErrorCategory::Network)
        .with_severity(ErrorSeverity::Error)
}
```

## 测试

错误处理系统包含完整的测试用例，位于 `test.rs` 文件中，可以通过以下命令运行测试：

```bash
cargo test --package seesea --lib errors::tests
```

## 最佳实践

1. 优先使用具体的错误创建函数，而不是通用的 `network_error` 或 `search_error` 函数
2. 为错误提供清晰、具体的错误消息
3. 适当设置错误的严重性
4. 对于复杂的错误，可以使用错误链提供更多上下文信息
5. 在添加新的错误类型时，遵循现有的错误码范围和命名约定

## 贡献指南

- 每个错误类别应有独立的模块文件
- 错误码应按类别分组，便于管理和查找
- 错误创建函数应提供清晰的参数和错误消息
- 新的错误类型应添加相应的测试用例
- 保持与现有代码的兼容性
