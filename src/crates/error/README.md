# Error - 零依赖的错误处理框架

这是一个完全自主实现的、零外部依赖的强大错误处理框架，提供自定义属性装饰器和自动宏支持。

## 特性

- **零外部依赖**: 不依赖 `thiserror` 或 `anyhow` 等外部库
- **自定义装饰器**: 通过 `#[derive(Error)]` 自动为错误枚举实现错误处理能力
- **无隐式转换**: 所有转换都是显式的，确保最高性能
- **完整的错误信息**: 支持错误码、错误消息、错误源等
- **错误链**: 支持错误的嵌套和追踪
- **错误上下文**: 在错误传播过程中添加上下文信息
- **错误严重程度**: 支持 Debug、Info、Warning、Error、Critical 五级分类
- **错误类别**: 支持 IO、Network、Parse、Validation 等 10 种错误类别
- **全面的测试**: 所有代码都经过全面测试（30+ 测试）
- **完善的文档**: 中文API文档和注释

## 快速开始

### 安装

在您的 `Cargo.toml` 中添加:

```toml
[dependencies]
error = { path = "path/to/error" }
```

### 基本使用

```rust
use error::{Error, ErrorKind};

#[derive(Debug, Error)]
enum MyError {
    #[error("IO错误: {0}")]
    Io(String),
    
    #[error("解析错误: {msg}")]
    Parse { msg: String },
    
    #[error("未知错误")]
    Unknown,
}

fn main() {
    let err = MyError::Io("文件未找到".to_string());
    println!("{}", err); // 输出: IO错误: 文件未找到
    println!("错误码: {}", err.error_code()); // 输出: 错误码: 1
}
```

## 使用示例

### 元组变体

```rust
use error::Error;

#[derive(Debug, Error)]
enum FileError {
    #[error("文件未找到: {0}")]
    NotFound(String),
    
    #[error("读取错误，行 {0}，列 {1}")]
    ReadError(usize, usize),
}

let err = FileError::NotFound("/tmp/test.txt".to_string());
println!("{}", err); // 输出: 文件未找到: /tmp/test.txt
```

### 结构体变体

```rust
use error::Error;

#[derive(Debug, Error)]
enum DatabaseError {
    #[error("连接失败: {host}:{port}")]
    ConnectionFailed { host: String, port: u16 },
    
    #[error("查询错误: {query} - {reason}")]
    QueryError { query: String, reason: String },
}

let err = DatabaseError::ConnectionFailed {
    host: "localhost".to_string(),
    port: 5432,
};
println!("{}", err); // 输出: 连接失败: localhost:5432
```

### 单元变体

```rust
use error::Error;

#[derive(Debug, Error)]
enum SimpleError {
    #[error("权限被拒绝")]
    PermissionDenied,
    
    #[error("超时")]
    Timeout,
}

let err = SimpleError::PermissionDenied;
println!("{}", err); // 输出: 权限被拒绝
```

### 错误链

```rust
use error::{Error, ErrorInfo, ErrorKind};

#[derive(Debug, Error)]
enum InnerError {
    #[error("内部错误: {0}")]
    Internal(String),
}

let inner = InnerError::Internal("数据库连接失败".to_string());
let outer = ErrorInfo::with_source(
    500,
    "服务不可用".to_string(),
    inner
);

println!("{}", outer);
// 输出:
// [错误][其他错误][错误码: 500] 服务不可用
//   由以下错误引起: 内部错误: 数据库连接失败
```

### 错误上下文

```rust
use error::ErrorInfo;

let error = ErrorInfo::new(404, "文件未找到".to_string())
    .with_context("尝试读取配置文件: /etc/app/config.toml".to_string())
    .with_context("在应用初始化阶段".to_string());

println!("{}", error);
// 输出:
// [错误][其他错误][错误码: 404] 文件未找到
//   上下文: 尝试读取配置文件: /etc/app/config.toml
//   上下文: 在应用初始化阶段
```

### 错误严重程度和类别

```rust
use error::{ErrorInfo, ErrorSeverity, ErrorCategory};

let error = ErrorInfo::new(403, "访问被拒绝".to_string())
    .with_severity(ErrorSeverity::Warning)
    .with_category(ErrorCategory::Permission)
    .with_context("用户尝试访问受限资源".to_string());

println!("{}", error);
// 输出:
// [警告][权限错误][错误码: 403] 访问被拒绝
//   上下文: 用户尝试访问受限资源

// 判断错误严重程度
if error.is_warning() {
    println!("这是一个警告");
}

// 根据严重程度进行不同处理
match error.severity() {
    ErrorSeverity::Critical => println!("严重错误，需要立即处理！"),
    ErrorSeverity::Error => println!("错误，需要处理"),
    ErrorSeverity::Warning => println!("警告，需要注意"),
    _ => {}
}
```

### 复杂错误示例

```rust
use error::{ErrorInfo, ErrorSeverity, ErrorCategory};

// 底层IO错误
let io_error = ErrorInfo::new(1001, "无法打开文件".to_string())
    .with_severity(ErrorSeverity::Error)
    .with_category(ErrorCategory::Io);

// 包装为解析错误
let parse_error = ErrorInfo::with_source(2001, "JSON解析失败".to_string(), io_error)
    .with_context("解析用户配置".to_string())
    .with_severity(ErrorSeverity::Error)
    .with_category(ErrorCategory::Parse);

// 包装为应用错误
let app_error = ErrorInfo::with_source(3001, "应用启动失败".to_string(), parse_error)
    .with_context("初始化应用程序".to_string())
    .with_context("主函数入口".to_string())
    .with_severity(ErrorSeverity::Critical)
    .with_category(ErrorCategory::System);

println!("{}", app_error);
// 显示完整的错误链和所有上下文信息
```

### 返回结果类型

```rust
use error::{Error, ErrorInfo, Result};

#[derive(Debug, Error)]
enum ConfigError {
    #[error("配置文件未找到: {0}")]
    NotFound(String),
    
    #[error("配置格式错误: {0}")]
    InvalidFormat(String),
}

fn load_config(path: &str) -> Result<String> {
    if path.is_empty() {
        return Err(ErrorInfo::new(400, "路径不能为空".to_string()));
    }
    Ok("配置内容".to_string())
}

match load_config("/etc/app.conf") {
    Ok(content) => println!("配置加载成功: {}", content),
    Err(e) => println!("错误 [{}]: {}", e.code(), e.message()),
}
```

## API 文档

查看完整的 API 文档:

```bash
cargo doc --open
```

## 核心 API

### ErrorKind trait

所有自定义错误类型都应该实现此trait，以提供统一的错误处理接口。

```rust
pub trait ErrorKind: fmt::Debug + fmt::Display {
    fn error_code(&self) -> u32;
    fn error_message(&self) -> String;
    fn source(&self) -> Option<&dyn ErrorKind>;
}
```

### ErrorInfo 结构体

封装了错误的详细信息，包括错误码、消息、源错误、上下文、严重程度和类别等。

主要方法：
- `new(code, message)` - 创建新的错误信息
- `with_source(code, message, source)` - 创建带源错误的错误信息
- `with_context(context)` - 链式添加上下文信息
- `add_context(&mut self, context)` - 添加上下文信息
- `with_severity(severity)` - 设置错误严重程度
- `with_category(category)` - 设置错误类别
- `is_critical()` - 判断是否为严重错误
- `is_warning()` - 判断是否为警告

### ErrorSeverity 枚举

错误严重程度分级：
- `Debug` - 调试级别
- `Info` - 信息级别
- `Warning` - 警告级别
- `Error` - 错误级别（默认）
- `Critical` - 严重级别

严重程度支持排序和比较。

### ErrorCategory 枚举

错误类别分类：
- `Io` - IO 错误
- `Network` - 网络错误
- `Parse` - 解析错误
- `Validation` - 验证错误
- `Permission` - 权限错误
- `Configuration` - 配置错误
- `Database` - 数据库错误
- `Business` - 业务逻辑错误
- `System` - 系统错误
- `Other` - 其他错误（默认）

```rust
pub struct ErrorInfo {
    code: u32,
    message: String,
    source: Option<Box<dyn ErrorKind>>,
}
```

### Result 类型别名

使用 `ErrorInfo` 作为错误类型的 Result 别名，简化函数签名。

```rust
pub type Result<T> = std::result::Result<T, ErrorInfo>;
```

## 测试

运行所有测试:

```bash
cargo test
```

运行特定测试:

```bash
cargo test --test derive_tests
```

## 设计原则

1. **零依赖**: 除了必需的过程宏依赖（`syn`、`quote`、`proc-macro2`）外，不依赖任何外部crate
2. **显式转换**: 所有错误转换都是显式的，避免隐式转换带来的性能开销
3. **类型安全**: 利用Rust的类型系统确保错误处理的正确性
4. **性能优先**: 避免不必要的分配和克隆
5. **易用性**: 通过过程宏简化错误定义，减少样板代码

## 许可证

本项目采用 CC0 1.0 Universal 许可证。

## 贡献

欢迎提交问题和拉取请求！
