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

//! # 基本错误处理示例
//!
//! 演示如何使用 `#[derive(Error)]` 宏定义和使用自定义错误类型

use error::{Error, ErrorKind};

/// 应用程序错误枚举
///
/// 定义了应用程序中可能出现的各种错误类型
#[derive(Debug, Error)]
enum AppError {
    /// IO错误，包含错误描述
    #[error("IO错误: {0}")]
    Io(String),
    
    /// 解析错误，包含详细的错误信息
    #[error("解析错误: {msg}")]
    Parse { msg: String },
    
    /// 网络错误，包含主机和端口信息
    #[error("网络错误: 无法连接到 {host}:{port}")]
    Network { host: String, port: u16 },
    
    /// 未知错误
    #[error("未知错误")]
    Unknown,
}

fn main() {
    println!("=== 基本错误处理示例 ===\n");
    
    // 示例 1: IO错误
    println!("示例 1: IO错误");
    let io_error = AppError::Io("文件未找到: /tmp/test.txt".to_string());
    println!("  错误信息: {}", io_error);
    println!("  错误码: {}", io_error.error_code());
    println!("  错误消息: {}", io_error.error_message());
    println!();
    
    // 示例 2: 解析错误
    println!("示例 2: 解析错误");
    let parse_error = AppError::Parse {
        msg: "无效的JSON格式".to_string(),
    };
    println!("  错误信息: {}", parse_error);
    println!("  错误码: {}", parse_error.error_code());
    println!("  错误消息: {}", parse_error.error_message());
    println!();
    
    // 示例 3: 网络错误
    println!("示例 3: 网络错误");
    let network_error = AppError::Network {
        host: "api.example.com".to_string(),
        port: 443,
    };
    println!("  错误信息: {}", network_error);
    println!("  错误码: {}", network_error.error_code());
    println!("  错误消息: {}", network_error.error_message());
    println!();
    
    // 示例 4: 未知错误
    println!("示例 4: 未知错误");
    let unknown_error = AppError::Unknown;
    println!("  错误信息: {}", unknown_error);
    println!("  错误码: {}", unknown_error.error_code());
    println!("  错误消息: {}", unknown_error.error_message());
    println!();
    
    // 示例 5: 使用 Debug trait
    println!("示例 5: Debug 输出");
    println!("  {:?}", io_error);
    println!();
    
    println!("=== 示例完成 ===");
}
