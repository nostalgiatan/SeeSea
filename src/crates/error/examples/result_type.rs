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

//! # Result 类型示例
//!
//! 演示如何在函数中使用 Result 类型进行错误处理

use error::{Error, ErrorInfo, Result};

/// 配置错误
#[derive(Debug, Error)]
enum ConfigError {
    /// 配置文件未找到
    #[error("配置文件未找到: {path}")]
    NotFound { path: String },
    
    /// 配置格式错误
    #[error("配置格式错误: {reason}")]
    InvalidFormat { reason: String },
    
    /// 必需字段缺失
    #[error("必需字段缺失: {field}")]
    MissingField { field: String },
}

/// 配置结构体
struct Config {
    host: String,
    port: u16,
    timeout: u64,
}

/// 加载配置文件
///
/// # 参数
///
/// * `path` - 配置文件路径
///
/// # 返回
///
/// * `Result<Config>` - 成功返回配置对象，失败返回错误信息
fn load_config(path: &str) -> Result<Config> {
    // 验证路径
    if path.is_empty() {
        return Err(ErrorInfo::new(
            400,
            "配置文件路径不能为空".to_string()
        ));
    }
    
    // 模拟加载配置
    if path == "/invalid/path" {
        return Err(ErrorInfo::with_source(
            404,
            "配置加载失败".to_string(),
            ConfigError::NotFound {
                path: path.to_string(),
            }
        ));
    }
    
    // 成功返回配置
    Ok(Config {
        host: "localhost".to_string(),
        port: 8080,
        timeout: 30,
    })
}

/// 验证配置
///
/// # 参数
///
/// * `config` - 配置对象引用
///
/// # 返回
///
/// * `Result<()>` - 验证成功返回 Ok(())，失败返回错误信息
fn validate_config(config: &Config) -> Result<()> {
    if config.host.is_empty() {
        return Err(ErrorInfo::with_source(
            400,
            "配置验证失败".to_string(),
            ConfigError::MissingField {
                field: "host".to_string(),
            }
        ));
    }
    
    if config.port == 0 {
        return Err(ErrorInfo::with_source(
            400,
            "配置验证失败".to_string(),
            ConfigError::InvalidFormat {
                reason: "端口号不能为0".to_string(),
            }
        ));
    }
    
    Ok(())
}

/// 初始化应用程序
///
/// # 参数
///
/// * `config_path` - 配置文件路径
///
/// # 返回
///
/// * `Result<()>` - 初始化成功返回 Ok(())，失败返回错误信息
fn init_app(config_path: &str) -> Result<()> {
    // 加载配置
    let config = load_config(config_path)?;
    
    // 验证配置
    validate_config(&config)?;
    
    println!("应用程序初始化成功:");
    println!("  主机: {}", config.host);
    println!("  端口: {}", config.port);
    println!("  超时: {}秒", config.timeout);
    
    Ok(())
}

fn main() {
    println!("=== Result 类型示例 ===\n");
    
    // 示例 1: 成功场景
    println!("示例 1: 成功加载配置");
    match load_config("/etc/app.conf") {
        Ok(config) => {
            println!("  配置加载成功");
            println!("  主机: {}", config.host);
            println!("  端口: {}", config.port);
            println!("  超时: {}秒", config.timeout);
        }
        Err(e) => {
            println!("  错误 [{}]: {}", e.code(), e.message());
        }
    }
    println!();
    
    // 示例 2: 路径为空
    println!("示例 2: 空路径错误");
    match load_config("") {
        Ok(_) => println!("  不应该执行到这里"),
        Err(e) => {
            println!("  错误 [{}]: {}", e.code(), e.message());
        }
    }
    println!();
    
    // 示例 3: 文件未找到
    println!("示例 3: 文件未找到错误");
    match load_config("/invalid/path") {
        Ok(_) => println!("  不应该执行到这里"),
        Err(e) => {
            println!("  错误 [{}]: {}", e.code(), e.message());
            if let Some(source) = e.source() {
                println!("  源错误: {}", source);
            }
        }
    }
    println!();
    
    // 示例 4: 使用 ? 操作符
    println!("示例 4: 应用程序初始化");
    match init_app("/etc/app.conf") {
        Ok(()) => println!("  ✓ 初始化完成"),
        Err(e) => println!("  ✗ 初始化失败: {}", e),
    }
    println!();
    
    println!("示例 5: 初始化失败");
    match init_app("") {
        Ok(()) => println!("  不应该执行到这里"),
        Err(e) => println!("  ✗ 初始化失败: {}", e),
    }
    println!();
    
    println!("=== 示例完成 ===");
}
