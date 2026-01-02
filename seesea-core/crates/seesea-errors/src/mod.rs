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

//! # 错误处理模块
//!
//! 错误处理模块是 SeeSea 的核心组件之一，提供统一的错误处理机制，支持多种错误类型和详细的错误信息。
//!
//! ## 模块架构
//!
//! 错误处理模块采用分层设计，主要包含以下核心组件：
//!
//! - **base**：基础错误类型定义，包括错误信息、类型、严重程度和类别
//! - **network**：网络相关错误，如连接失败、超时等
//! - **search**：搜索相关错误，如引擎不可用、搜索失败等
//! - **parse**：解析相关错误，如 JSON 解析失败、XML 解析失败等
//! - **validation**：验证相关错误，如参数验证失败、配置验证失败等
//! - **io**：IO 相关错误，如文件读写失败、目录不存在等
//! - **permission**：权限相关错误，如访问被拒绝、权限不足等
//! - **configuration**：配置相关错误，如配置文件不存在、配置项无效等
//! - **database**：数据库相关错误，如连接失败、查询失败等
//! - **business**：业务逻辑相关错误，如业务规则违反、状态错误等
//! - **system**：系统相关错误，如内存不足、系统调用失败等
//!
//! ## 错误分类
//!
//! SeeSea 的错误分为以下几个主要类别：
//!
//! - **ErrorCategory**：错误类别，如网络、搜索、解析等
//! - **ErrorKind**：具体错误类型，如连接失败、超时、解析错误等
//! - **ErrorSeverity**：错误严重程度，如调试、信息、警告、错误、致命等
//!
//! ## 核心功能
//!
//! - **结构化错误信息**：提供详细的错误信息，包括错误代码、描述、位置等
//! - **错误链**：支持错误嵌套，便于追踪错误根源
//! - **错误转换**：支持不同错误类型之间的转换
//! - **错误格式化**：支持多种错误格式化方式
//! - **错误日志**：便于日志记录和监控
//!
//! ## 错误处理流程
//!
//! 1. 当发生错误时，使用对应的错误创建函数创建错误实例
//! 2. 错误实例包含详细的错误信息和上下文
//! 3. 错误通过 Result 类型向上传播
//! 4. 最终在适当的位置处理或记录错误
//!
//! ## 使用示例
//!
//! ```rust
//! use seesea_errors::{network_error, ErrorCategory, ErrorSeverity};
//!
//! fn example_function() -> Result<(), Box<dyn std::error::Error>> {
//!     // 模拟网络错误
//!     return Err(network_error("连接失败").into());
//! }
//!
//! // 处理错误
//! match example_function() {
//!     Ok(result) => println!("成功: {:?}", result),
//!     Err(error) => {
//!         println!("错误: {}", error);
//!     }
//! }
//! ```

/// 基础错误类型模块，定义核心错误结构和枚举
pub mod base;

/// 网络相关错误模块
pub mod network;

/// 搜索相关错误模块
pub mod search;

/// 解析相关错误模块
pub mod parse;

/// 验证相关错误模块
pub mod validation;

/// IO 相关错误模块
pub mod io;

/// 权限相关错误模块
pub mod permission;

/// 配置相关错误模块
pub mod configuration;

/// 数据库相关错误模块
pub mod database;

/// 业务逻辑相关错误模块
pub mod business;

/// 系统相关错误模块
pub mod system;

// 重新导出核心错误类型，方便外部使用

/// 核心错误类型和结果类型
pub use base::{
    ErrorCategory, // 错误类别枚举
    ErrorInfo,     // 详细的错误信息结构
    ErrorKind,     // 具体错误类型枚举
    ErrorSeverity, // 错误严重程度枚举
    Result,        // 结果类型别名，简化错误处理
};

// 导出所有错误创建函数，方便外部使用

/// 网络错误创建函数
pub use network::{
    // 网络错误常量
    BAD_GATEWAY,
    BAD_REQUEST,
    CONNECTION_REFUSED,
    CONNECTION_TIMEOUT,
    DNS_RESOLVE_FAILED,
    FORBIDDEN,
    GATEWAY_TIMEOUT,
    HTTP_CLIENT_ERROR_BASE,
    HTTP_ERROR,
    HTTP_SERVER_ERROR_BASE,
    INTERNAL_SERVER_ERROR,
    INVALID_RESPONSE,
    METHOD_NOT_ALLOWED,
    NETWORK_ERROR_BASE,
    NETWORK_UNREACHABLE,
    NOT_FOUND,
    PROXY_ERROR,
    REQUEST_CANCELLED,
    SERVICE_UNAVAILABLE,
    SSL_ERROR,
    TOO_MANY_REDIRECTS,
    TOO_MANY_REQUESTS,
    UNAUTHORIZED,
    bad_gateway,
    bad_request,
    connection_refused,
    connection_timeout,
    dns_resolve_failed,
    forbidden,
    gateway_timeout,
    http_error,
    internal_server_error,
    invalid_response,
    method_not_allowed,
    network_error,
    network_unreachable,
    not_found,
    proxy_error,
    request_cancelled,
    service_unavailable as network_service_unavailable,
    ssl_error,
    too_many_redirects,
    too_many_requests,
    unauthorized as network_unauthorized,
};

/// 搜索错误创建函数
pub use search::{
    ENGINE_ERROR,
    // 搜索错误常量
    ENGINE_UNAVAILABLE,
    INVALID_QUERY,
    INVALID_SEARCH_SCOPE,
    RESULT_PARSE_FAILED,
    SEARCH_DEPTH_TOO_LARGE,
    SEARCH_ERROR_BASE,
    SEARCH_RATE_LIMITED,
    SEARCH_TIMEOUT,
    UNSUPPORTED_SEARCH_TYPE,
    ZERO_RESULTS,
    engine_error,
    engine_unavailable,
    invalid_query,
    invalid_search_scope,
    result_parse_failed,
    search_depth_too_large,
    search_rate_limited,
    search_timeout,
    unsupported_search_type,
    zero_results,
};

/// 解析错误创建函数
pub use parse::{
    csv_parse_error, html_parse_error, invalid_field_type, invalid_field_value, invalid_format,
    json_parse_error, missing_field, parse_error, parse_timeout, xml_parse_error, yaml_parse_error,
};

/// 验证错误创建函数
pub use validation::{
    // 验证错误常量
    DUPLICATE_VALUE,
    EMPTY_FIELD,
    FIELD_TOO_LONG,
    FIELD_TOO_SHORT,
    INVALID_DATE,
    INVALID_EMAIL,
    INVALID_ENUM_VALUE,
    INVALID_NUMBER,
    INVALID_URL,
    UNSUPPORTED_PARAMETER,
    VALIDATION_ERROR_BASE,
    duplicate_value,
    empty_field,
    field_too_long,
    field_too_short,
    invalid_date,
    invalid_email,
    invalid_enum_value,
    invalid_number,
    invalid_url,
    out_of_range,
    unsupported_parameter,
    validation_error,
};

/// IO 错误创建函数
pub use io::{
    directory_create_failed, directory_not_found, disk_full, file_not_found, file_open_failed,
    file_permission_denied, file_read_failed, file_too_large, file_write_failed, invalid_path,
    io_error,
};

/// 权限错误创建函数
pub use permission::{
    access_denied, account_disabled, account_locked, insufficient_role, invalid_credentials,
    invalid_token, missing_token, permission_denied, permission_error, token_expired, unauthorized,
};

/// 配置错误创建函数
pub use configuration::{
    config_conflict, config_file_not_found, config_parse_failed, config_permission_error,
    config_type_error, config_validation_failed, config_value_out_of_range,
    config_version_mismatch, configuration_error, invalid_config_value, missing_config_item,
};

/// 数据库错误创建函数
pub use database::{
    column_not_found, connection_failed, database_error, database_full, database_locked,
    duplicate_key, foreign_key_violation, invalid_sql, query_failed, table_not_found,
    transaction_failed,
};

/// 业务逻辑错误创建函数
pub use business::{
    business_error, business_rule_violation, concurrency_error, dependency_failed,
    internal_business_error, invalid_state, operation_not_allowed, quota_exceeded,
    rate_limit_exceeded, resource_not_found, validation_failed,
};

/// 系统错误创建函数
pub use system::{
    configuration_error as system_configuration_error, internal_system_error, resource_exhausted,
    resource_leak, service_unavailable, system_error, system_overload, system_timeout,
};
