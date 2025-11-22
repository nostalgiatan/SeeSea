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

//! # 错误链示例
//!
//! 演示如何使用错误链来追踪错误的源头

use error::{Error, ErrorInfo};

/// 底层数据库错误
#[derive(Debug, Error)]
enum DatabaseError {
    /// 连接失败
    #[error("数据库连接失败: {reason}")]
    ConnectionFailed { reason: String },
    
    /// 查询失败
    #[error("数据库查询失败: {query}")]
    QueryFailed { query: String },
}

/// 业务逻辑错误
#[derive(Debug, Error)]
#[allow(dead_code)]
enum BusinessError {
    /// 用户不存在
    #[error("用户不存在: {user_id}")]
    UserNotFound { user_id: u64 },
    
    /// 数据验证失败
    #[error("数据验证失败: {field}")]
    ValidationFailed { field: String },
}

fn main() {
    println!("=== 错误链示例 ===\n");
    
    // 示例 1: 简单错误链
    println!("示例 1: 简单错误链");
    let db_error = DatabaseError::ConnectionFailed {
        reason: "超时".to_string(),
    };
    let wrapped_error = ErrorInfo::with_source(
        500,
        "服务不可用".to_string(),
        db_error
    );
    
    println!("完整错误信息:");
    println!("{}", wrapped_error);
    println!();
    
    // 访问源错误
    if let Some(source) = wrapped_error.source() {
        println!("源错误码: {}", source.error_code());
        println!("源错误消息: {}", source.error_message());
    }
    println!();
    
    // 示例 2: 多层错误链
    println!("示例 2: 多层错误链");
    let query_error = DatabaseError::QueryFailed {
        query: "SELECT * FROM users WHERE id = 123".to_string(),
    };
    let business_error = ErrorInfo::with_source(
        404,
        "用户数据获取失败".to_string(),
        query_error
    );
    
    println!("完整错误信息:");
    println!("{}", business_error);
    println!();
    
    println!("顶层错误:");
    println!("  错误码: {}", business_error.code());
    println!("  错误消息: {}", business_error.message());
    
    if let Some(source) = business_error.source() {
        println!("源错误:");
        println!("  错误码: {}", source.error_code());
        println!("  错误消息: {}", source.error_message());
    }
    println!();
    
    // 示例 3: 使用业务错误作为源
    println!("示例 3: 业务错误链");
    let validation_error = BusinessError::ValidationFailed {
        field: "email".to_string(),
    };
    let api_error = ErrorInfo::with_source(
        400,
        "请求参数无效".to_string(),
        validation_error
    );
    
    println!("完整错误信息:");
    println!("{}", api_error);
    println!();
    
    println!("=== 示例完成 ===");
}
