// Copyright (C) 2025 nostalgiatan
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

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
