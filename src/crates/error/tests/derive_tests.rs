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

//! # Error 派生宏集成测试
//!
//! 测试 `#[derive(Error)]` 宏的各种使用场景

use error::{Error, ErrorKind, ErrorInfo};

/// 测试单元变体
#[derive(Debug, Error)]
enum SimpleError {
    #[error("简单错误")]
    Simple,
    
    #[error("另一个简单错误")]
    AnotherSimple,
}

#[test]
fn test_simple_error() {
    let err = SimpleError::Simple;
    
    // 测试 Display
    assert_eq!(format!("{}", err), "简单错误");
    
    // 测试 ErrorKind
    assert_eq!(err.error_code(), 1);
    assert_eq!(err.error_message(), "简单错误");
    
    let err2 = SimpleError::AnotherSimple;
    assert_eq!(format!("{}", err2), "另一个简单错误");
    assert_eq!(err2.error_code(), 2);
}

/// 测试元组变体
#[derive(Debug, Error)]
#[allow(dead_code)]
enum TupleError {
    #[error("IO错误: {0}")]
    Io(String),
    
    #[error("解析错误，行 {0}，列 {1}")]
    Parse(usize, usize),
    
    #[error("网络错误")]
    Network(String, u16),
}

#[test]
fn test_tuple_error() {
    let err = TupleError::Io("文件未找到".to_string());
    assert_eq!(format!("{}", err), "IO错误: 文件未找到");
    assert_eq!(err.error_code(), 1);
    
    let err2 = TupleError::Parse(10, 5);
    assert_eq!(format!("{}", err2), "解析错误，行 10，列 5");
    assert_eq!(err2.error_code(), 2);
    
    let err3 = TupleError::Network("localhost".to_string(), 8080);
    assert_eq!(format!("{}", err3), "网络错误");
    assert_eq!(err3.error_code(), 3);
}

/// 测试结构体变体
#[derive(Debug, Error)]
#[allow(dead_code)]
enum StructError {
    #[error("文件错误: {path}")]
    File { path: String },
    
    #[error("数据库错误: {message} (代码: {code})")]
    Database { message: String, code: i32 },
    
    #[error("配置错误")]
    Config { key: String, value: String },
}

#[test]
fn test_struct_error() {
    let err = StructError::File {
        path: "/tmp/test.txt".to_string(),
    };
    assert_eq!(format!("{}", err), "文件错误: /tmp/test.txt");
    assert_eq!(err.error_code(), 1);
    
    let err2 = StructError::Database {
        message: "连接失败".to_string(),
        code: -1,
    };
    assert_eq!(format!("{}", err2), "数据库错误: 连接失败 (代码: -1)");
    assert_eq!(err2.error_code(), 2);
    
    let err3 = StructError::Config {
        key: "timeout".to_string(),
        value: "30".to_string(),
    };
    assert_eq!(format!("{}", err3), "配置错误");
    assert_eq!(err3.error_code(), 3);
}

/// 测试混合变体
#[derive(Debug, Error)]
#[allow(dead_code)]
enum MixedError {
    #[error("未知错误")]
    Unknown,
    
    #[error("简单消息: {0}")]
    Simple(String),
    
    #[error("详细错误: {msg}")]
    Detailed { msg: String },
}

#[test]
fn test_mixed_error() {
    let err1 = MixedError::Unknown;
    assert_eq!(format!("{}", err1), "未知错误");
    assert_eq!(err1.error_code(), 1);
    
    let err2 = MixedError::Simple("测试".to_string());
    assert_eq!(format!("{}", err2), "简单消息: 测试");
    assert_eq!(err2.error_code(), 2);
    
    let err3 = MixedError::Detailed {
        msg: "详细信息".to_string(),
    };
    assert_eq!(format!("{}", err3), "详细错误: 详细信息");
    assert_eq!(err3.error_code(), 3);
}

/// 测试错误链
#[derive(Debug, Error)]
#[allow(dead_code)]
enum ChainError {
    #[error("顶层错误")]
    Top,
    
    #[error("中间层错误: {0}")]
    Middle(String),
}

#[test]
fn test_error_chain() {
    let source = ChainError::Middle("根本原因".to_string());
    let error = ErrorInfo::with_source(500, "包装错误".to_string(), source);
    
    assert_eq!(error.code(), 500);
    assert_eq!(error.message(), "包装错误");
    
    let source_error = error.source().unwrap();
    assert_eq!(source_error.error_code(), 2);
    assert_eq!(source_error.error_message(), "中间层错误: 根本原因");
}

/// 测试实际应用场景
#[derive(Debug, Error)]
#[allow(dead_code)]
enum AppError {
    #[error("配置文件未找到: {0}")]
    ConfigNotFound(String),
    
    #[error("配置解析失败: {path}")]
    ConfigParseFailed { path: String },
    
    #[error("数据库连接失败")]
    DatabaseConnection,
    
    #[error("用户认证失败: {username}")]
    AuthenticationFailed { username: String },
    
    #[error("权限不足")]
    PermissionDenied,
}

#[test]
fn test_app_error_scenarios() {
    // 场景1: 配置文件未找到
    let err = AppError::ConfigNotFound("/etc/app.conf".to_string());
    assert_eq!(format!("{}", err), "配置文件未找到: /etc/app.conf");
    assert_eq!(err.error_code(), 1);
    
    // 场景2: 配置解析失败
    let err = AppError::ConfigParseFailed {
        path: "/etc/app.conf".to_string(),
    };
    assert_eq!(format!("{}", err), "配置解析失败: /etc/app.conf");
    assert_eq!(err.error_code(), 2);
    
    // 场景3: 数据库连接失败
    let err = AppError::DatabaseConnection;
    assert_eq!(format!("{}", err), "数据库连接失败");
    assert_eq!(err.error_code(), 3);
    
    // 场景4: 用户认证失败
    let err = AppError::AuthenticationFailed {
        username: "admin".to_string(),
    };
    assert_eq!(format!("{}", err), "用户认证失败: admin");
    assert_eq!(err.error_code(), 4);
    
    // 场景5: 权限不足
    let err = AppError::PermissionDenied;
    assert_eq!(format!("{}", err), "权限不足");
    assert_eq!(err.error_code(), 5);
}

/// 测试返回类型为 Result 的函数
fn read_config(path: &str) -> error::Result<String> {
    if path.is_empty() {
        return Err(ErrorInfo::new(400, "路径不能为空".to_string()));
    }
    Ok("配置内容".to_string())
}

#[test]
fn test_result_type_usage() {
    let result = read_config("/etc/config");
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "配置内容");
    
    let result = read_config("");
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.code(), 400);
    assert_eq!(err.message(), "路径不能为空");
}

/// 测试 Debug trait
#[test]
fn test_debug_trait() {
    let err = SimpleError::Simple;
    let debug_str = format!("{:?}", err);
    assert!(debug_str.contains("Simple"));
}
