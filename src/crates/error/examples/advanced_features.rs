// Copyright 2025 nostalgiatan
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! # 高级错误处理功能示例
//!
//! 展示 error 模块的高级功能，包括错误上下文、严重程度和类别

use error::{Error, ErrorCategory, ErrorInfo, ErrorKind, ErrorSeverity};

/// 应用程序错误类型
#[derive(Debug, Error)]
enum AppError {
    /// 配置文件错误
    #[error("配置文件错误: {0}")]
    ConfigError(String),

    /// 数据库连接错误
    #[error("数据库连接失败: {host}:{port}")]
    DatabaseError { host: String, port: u16 },

    /// 权限不足
    #[error("权限不足，需要管理员权限")]
    PermissionDenied,
}

/// 模拟读取配置文件
fn read_config(path: &str) -> error::Result<String> {
    // 模拟底层IO错误
    let io_error = ErrorInfo::new(1001, format!("无法打开文件: {}", path))
        .with_severity(ErrorSeverity::Error)
        .with_category(ErrorCategory::Io);

    // 包装为高级错误并添加上下文
    Err(ErrorInfo::with_source(2001, "读取配置文件失败".to_string(), io_error)
        .with_context(format!("文件路径: {}", path))
        .with_context("应用初始化阶段".to_string())
        .with_severity(ErrorSeverity::Critical)
        .with_category(ErrorCategory::Configuration))
}

/// 模拟数据库连接
fn connect_database(host: &str, port: u16) -> error::Result<String> {
    // 模拟网络错误
    let net_error = ErrorInfo::new(3001, format!("无法连接到 {}:{}", host, port))
        .with_severity(ErrorSeverity::Error)
        .with_category(ErrorCategory::Network);

    Err(ErrorInfo::with_source(4001, "数据库初始化失败".to_string(), net_error)
        .with_context("尝试连接主数据库".to_string())
        .with_severity(ErrorSeverity::Critical)
        .with_category(ErrorCategory::Database))
}

/// 模拟权限检查
fn check_permission(user: &str) -> error::Result<()> {
    if user != "admin" {
        return Err(ErrorInfo::new(5001, format!("用户 '{}' 无权限执行此操作", user))
            .with_context("需要管理员权限".to_string())
            .with_severity(ErrorSeverity::Warning)
            .with_category(ErrorCategory::Permission));
    }
    Ok(())
}

/// 模拟业务逻辑错误
fn process_data(data: &str) -> error::Result<String> {
    if data.is_empty() {
        return Err(ErrorInfo::new(6001, "数据为空".to_string())
            .with_context("处理用户输入".to_string())
            .with_severity(ErrorSeverity::Warning)
            .with_category(ErrorCategory::Validation));
    }
    Ok(data.to_uppercase())
}

fn main() {
    println!("=== Error 模块高级功能示例 ===\n");

    // 示例 1: 带上下文的配置错误
    println!("1. 配置文件错误（带上下文和严重程度）:");
    match read_config("/etc/app/config.toml") {
        Ok(_) => println!("成功"),
        Err(e) => {
            println!("{}", e);
            println!("\n错误分析:");
            println!("  - 错误码: {}", e.code());
            println!("  - 严重程度: {}", e.severity());
            println!("  - 类别: {}", e.category());
            println!("  - 是否严重: {}", e.is_critical());
            println!("  - 上下文数量: {}", e.context().len());
        }
    }

    println!("\n{}", "=".repeat(60));

    // 示例 2: 数据库连接错误
    println!("\n2. 数据库连接错误:");
    match connect_database("localhost", 5432) {
        Ok(_) => println!("成功"),
        Err(e) => {
            println!("{}", e);
            println!("\n错误分析:");
            println!("  - 严重程度: {}", e.severity());
            println!("  - 类别: {}", e.category());
        }
    }

    println!("\n{}", "=".repeat(60));

    // 示例 3: 权限错误
    println!("\n3. 权限检查（警告级别）:");
    match check_permission("guest") {
        Ok(_) => println!("权限检查通过"),
        Err(e) => {
            println!("{}", e);
            println!("\n错误分析:");
            println!("  - 是否警告: {}", e.is_warning());
            println!("  - 类别: {}", e.category());
        }
    }

    println!("\n{}", "=".repeat(60));

    // 示例 4: 验证错误
    println!("\n4. 数据验证错误:");
    match process_data("") {
        Ok(result) => println!("处理结果: {}", result),
        Err(e) => {
            println!("{}", e);
            println!("\n错误分析:");
            println!("  - 类别: {}", e.category());
            println!("  - 严重程度: {}", e.severity());
        }
    }

    println!("\n{}", "=".repeat(60));

    // 示例 5: 成功的情况
    println!("\n5. 成功处理数据:");
    match process_data("hello") {
        Ok(result) => println!("处理结果: {}", result),
        Err(e) => println!("错误: {}", e),
    }

    println!("\n{}", "=".repeat(60));

    // 示例 6: 使用派生宏的错误
    println!("\n6. 使用 Error 派生宏:");
    let app_error = AppError::ConfigError("缺少必需字段".to_string());
    println!("错误: {}", app_error);
    println!("错误码: {}", app_error.error_code());
    println!("错误消息: {}", app_error.error_message());
    
    let db_error = AppError::DatabaseError {
        host: "localhost".to_string(),
        port: 5432,
    };
    println!("数据库错误: {}", db_error);
    
    let perm_error = AppError::PermissionDenied;
    println!("权限错误: {}", perm_error);

    println!("\n{}", "=".repeat(60));

    // 示例 7: 复杂的错误链
    println!("\n7. 多层错误链:");
    let io_err = ErrorInfo::new(1, "磁盘读取失败".to_string())
        .with_severity(ErrorSeverity::Error)
        .with_category(ErrorCategory::Io);

    let parse_err = ErrorInfo::with_source(2, "JSON 解析失败".to_string(), io_err)
        .with_context("解析用户配置".to_string())
        .with_severity(ErrorSeverity::Error)
        .with_category(ErrorCategory::Parse);

    let app_err = ErrorInfo::with_source(3, "应用启动失败".to_string(), parse_err)
        .with_context("初始化应用程序".to_string())
        .with_context("主函数入口".to_string())
        .with_severity(ErrorSeverity::Critical)
        .with_category(ErrorCategory::System);

    println!("{}", app_err);

    println!("\n{}", "=".repeat(60));

    // 示例 8: 错误严重程度比较
    println!("\n8. 错误严重程度排序:");
    let severities = vec![
        ErrorSeverity::Debug,
        ErrorSeverity::Info,
        ErrorSeverity::Warning,
        ErrorSeverity::Error,
        ErrorSeverity::Critical,
    ];

    for severity in &severities {
        println!("  {} (级别: {})", severity, *severity as u8);
    }

    println!("\n严重程度排序 (从低到高):");
    let mut sorted = severities.clone();
    sorted.sort();
    for severity in sorted {
        println!("  {}", severity);
    }
}
