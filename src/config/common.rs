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

//! SeeSea 配置通用类型定义
//!
//! 定义了跨模块使用的通用类型

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 引擎加载模式
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EngineLoadingMode {
    /// 全局模式 - 加载所有可用引擎
    Global,
    /// 设置模式 - 只加载配置中指定的引擎
    Settings,
}

/// 安全搜索级别
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SafeSearchLevel {
    /// 不启用安全搜索
    None = 0,
    /// 中等安全搜索
    Moderate = 1,
    /// 严格安全搜索
    Strict = 2,
}

/// 代理类型
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProxyType {
    /// HTTP 代理
    Http,
    /// HTTPS 代理
    Https,
    /// SOCKS4 代理
    Socks4,
    /// SOCKS5 代理
    Socks5,
}

/// TLS 指纹保护级别
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FingerprintLevel {
    /// 不启用指纹保护
    None,
    /// 基础指纹混淆
    Basic,
    /// 高级指纹随机化
    Advanced,
    /// 最大程度保护
    Maximum,
}

/// 请求时序策略
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TimingStrategy {
    /// 无随机化
    None,
    /// 轻度随机化
    Light,
    /// 中等随机化
    Medium,
    /// 高度随机化
    Heavy,
}

/// 引擎类型
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EngineType {
    /// 在线搜索引擎
    Online,
    /// 离线搜索引擎
    Offline,
    /// 在线词典引擎
    OnlineDictionary,
    /// 在线货币转换引擎
    OnlineCurrency,
    /// 在线 URL 搜索引擎
    OnlineUrlSearch,
    /// API 引擎
    Api,
    /// 自定义引擎
    Custom,
}

/// 认证类型
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AuthType {
    /// 无认证
    None,
    /// API 密钥认证
    ApiKey,
    /// JWT 认证
    Jwt,
    /// 基础认证
    Basic,
}

/// 日志级别
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    /// 错误级别
    Error,
    /// 警告级别
    Warn,
    /// 信息级别
    Info,
    /// 调试级别
    Debug,
    /// 跟踪级别
    Trace,
}

/// 日志格式
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogFormat {
    /// 简单格式
    Simple,
    /// 完整格式
    Full,
    /// JSON 格式
    Json,
    /// Compact 格式
    Compact,
}

/// 日志输出
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogOutput {
    /// 标准输出
    Stdout,
    /// 标准错误
    Stderr,
    /// 文件输出
    File,
    /// 同时输出到标准输出和文件
    Both,
}

/// 基础的引擎配置结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaseEngineConfig {
    /// 引擎名称
    pub name: String,
    /// 引擎类型
    pub engine_type: EngineType,
    /// 是否启用
    pub enabled: bool,
    /// 权重
    pub weight: f32,
    /// 超时时间（秒）
    pub timeout: Option<u64>,
    /// 支持的分类
    pub categories: Vec<String>,
    /// 支持的语言
    pub languages: Vec<String>,
    /// 自定义参数
    pub custom_params: HashMap<String, serde_json::Value>,
}

/// 配置验证结果
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ConfigValidationResult {
    /// 是否验证通过
    pub is_valid: bool,
    /// 验证错误列表
    pub errors: Vec<String>,
    /// 验证警告列表
    pub warnings: Vec<String>,
}

impl ConfigValidationResult {
    /// 创建成功的验证结果
    pub fn success() -> Self {
        Self {
            is_valid: true,
            errors: Vec::new(),
            warnings: Vec::new(),
        }
    }
    
    /// 创建有效的验证结果（别名，兼容性）
    pub fn valid() -> Self {
        Self::success()
    }

    /// 创建失败的验证结果
    pub fn failure(errors: Vec<String>) -> Self {
        Self {
            is_valid: false,
            errors,
            warnings: Vec::new(),
        }
    }

    /// 添加错误
    pub fn add_error(&mut self, error: String) {
        self.errors.push(error);
        self.is_valid = false;
    }

    /// 添加警告
    pub fn add_warning(&mut self, warning: String) {
        self.warnings.push(warning);
    }

    /// 检查是否有错误
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    /// 检查是否有警告
    pub fn has_warnings(&self) -> bool {
        !self.warnings.is_empty()
    }
}