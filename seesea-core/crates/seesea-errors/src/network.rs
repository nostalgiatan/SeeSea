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

//! 网络相关错误定义
//!
//! 包含网络请求、连接、超时等相关错误的定义和创建函数。

use crate::{ErrorInfo, ErrorSeverity};
use error::ErrorCategory;

/// 网络错误码常量
///
/// 网络错误码范围：1000-1999
pub const NETWORK_ERROR_BASE: u32 = 1000;
pub const CONNECTION_TIMEOUT: u32 = NETWORK_ERROR_BASE + 1;
pub const CONNECTION_REFUSED: u32 = NETWORK_ERROR_BASE + 2;
pub const DNS_RESOLVE_FAILED: u32 = NETWORK_ERROR_BASE + 3;
pub const INVALID_RESPONSE: u32 = NETWORK_ERROR_BASE + 4;
pub const SSL_ERROR: u32 = NETWORK_ERROR_BASE + 5;
pub const HTTP_ERROR: u32 = NETWORK_ERROR_BASE + 6;
pub const NETWORK_UNREACHABLE: u32 = NETWORK_ERROR_BASE + 7;
pub const PROXY_ERROR: u32 = NETWORK_ERROR_BASE + 8;
pub const TOO_MANY_REDIRECTS: u32 = NETWORK_ERROR_BASE + 9;
pub const REQUEST_CANCELLED: u32 = NETWORK_ERROR_BASE + 10;

/// HTTP 客户端错误码范围：1100-1199
pub const HTTP_CLIENT_ERROR_BASE: u32 = 1100;
pub const BAD_REQUEST: u32 = HTTP_CLIENT_ERROR_BASE; // 400
pub const UNAUTHORIZED: u32 = HTTP_CLIENT_ERROR_BASE + 1; // 401
pub const FORBIDDEN: u32 = HTTP_CLIENT_ERROR_BASE + 3; // 403
pub const NOT_FOUND: u32 = HTTP_CLIENT_ERROR_BASE + 4; // 404
pub const METHOD_NOT_ALLOWED: u32 = HTTP_CLIENT_ERROR_BASE + 5; // 405
pub const TOO_MANY_REQUESTS: u32 = HTTP_CLIENT_ERROR_BASE + 9; // 429

/// HTTP 服务器错误码范围：1200-1299
pub const HTTP_SERVER_ERROR_BASE: u32 = 1200;
pub const INTERNAL_SERVER_ERROR: u32 = HTTP_SERVER_ERROR_BASE; // 500
pub const BAD_GATEWAY: u32 = HTTP_SERVER_ERROR_BASE + 2; // 502
pub const SERVICE_UNAVAILABLE: u32 = HTTP_SERVER_ERROR_BASE + 3; // 503
pub const GATEWAY_TIMEOUT: u32 = HTTP_SERVER_ERROR_BASE + 4; // 504

/// 创建通用网络错误
///
/// # 参数
/// - `message`: 错误消息
///
/// # 返回
/// 包含指定消息的网络错误对象
pub fn network_error(message: &str) -> ErrorInfo {
    ErrorInfo::new(NETWORK_ERROR_BASE, format!("Network Error: {message}"))
        .with_category(ErrorCategory::Network)
        .with_severity(ErrorSeverity::Error)
}

/// 创建连接超时错误
///
/// # 参数
/// - `host`: 连接的主机名或IP地址
///
/// # 返回
/// 包含连接超时信息的错误对象
pub fn connection_timeout(host: &str) -> ErrorInfo {
    ErrorInfo::new(CONNECTION_TIMEOUT, format!("连接到 {host} 超时"))
        .with_category(ErrorCategory::Network)
        .with_severity(ErrorSeverity::Error)
}

/// 创建连接被拒绝错误
///
/// # 参数
/// - `host`: 连接的主机名或IP地址
///
/// # 返回
/// 包含连接被拒绝信息的错误对象
pub fn connection_refused(host: &str) -> ErrorInfo {
    ErrorInfo::new(CONNECTION_REFUSED, format!("连接到 {host} 被拒绝"))
        .with_category(ErrorCategory::Network)
        .with_severity(ErrorSeverity::Error)
}

/// 创建DNS解析失败错误
///
/// # 参数
/// - `domain`: 无法解析的域名
///
/// # 返回
/// 包含DNS解析失败信息的错误对象
pub fn dns_resolve_failed(domain: &str) -> ErrorInfo {
    ErrorInfo::new(DNS_RESOLVE_FAILED, format!("无法解析域名: {domain}"))
        .with_category(ErrorCategory::Network)
        .with_severity(ErrorSeverity::Error)
}

/// 创建无效响应错误
///
/// # 参数
/// - `host`: 发送无效响应的主机
///
/// # 返回
/// 包含无效响应信息的错误对象
pub fn invalid_response(host: &str) -> ErrorInfo {
    ErrorInfo::new(INVALID_RESPONSE, format!("从 {host} 收到无效响应"))
        .with_category(ErrorCategory::Network)
        .with_severity(ErrorSeverity::Error)
}

/// 创建SSL错误
///
/// # 参数
/// - `message`: SSL错误的详细信息
///
/// # 返回
/// 包含SSL错误信息的错误对象
pub fn ssl_error(message: &str) -> ErrorInfo {
    ErrorInfo::new(SSL_ERROR, format!("SSL错误: {message}"))
        .with_category(ErrorCategory::Network)
        .with_severity(ErrorSeverity::Error)
}

/// 创建HTTP错误
///
/// # 参数
/// - `status`: HTTP状态码
/// - `message`: 错误消息
///
/// # 返回
/// 包含HTTP错误信息的错误对象
pub fn http_error(status: u16, message: &str) -> ErrorInfo {
    ErrorInfo::new(HTTP_ERROR, format!("HTTP错误 {status}: {message}"))
        .with_category(ErrorCategory::Network)
        .with_severity(ErrorSeverity::Error)
}

/// 创建网络不可达错误
///
/// # 参数
/// - `host`: 不可达的主机
///
/// # 返回
/// 包含网络不可达信息的错误对象
pub fn network_unreachable(host: &str) -> ErrorInfo {
    ErrorInfo::new(NETWORK_UNREACHABLE, format!("网络不可达: {host}"))
        .with_category(ErrorCategory::Network)
        .with_severity(ErrorSeverity::Error)
}

/// 创建代理错误
///
/// # 参数
/// - `message`: 代理错误的详细信息
///
/// # 返回
/// 包含代理错误信息的错误对象
pub fn proxy_error(message: &str) -> ErrorInfo {
    ErrorInfo::new(PROXY_ERROR, format!("代理错误: {message}"))
        .with_category(ErrorCategory::Network)
        .with_severity(ErrorSeverity::Error)
}

/// 创建重定向次数过多错误
///
/// # 参数
/// - `url`: 重定向的URL
///
/// # 返回
/// 包含重定向次数过多信息的错误对象
pub fn too_many_redirects(url: &str) -> ErrorInfo {
    ErrorInfo::new(TOO_MANY_REDIRECTS, format!("重定向次数过多: {url}"))
        .with_category(ErrorCategory::Network)
        .with_severity(ErrorSeverity::Error)
}

/// 创建请求取消错误
///
/// # 参数
/// - `message`: 请求取消的原因
///
/// # 返回
/// 包含请求取消信息的错误对象
pub fn request_cancelled(message: &str) -> ErrorInfo {
    ErrorInfo::new(REQUEST_CANCELLED, format!("请求取消: {message}"))
        .with_category(ErrorCategory::Network)
        .with_severity(ErrorSeverity::Warning)
}

/// 创建400 Bad Request错误
///
/// # 参数
/// - `message`: 错误消息
///
/// # 返回
/// 包含指定消息的400错误对象
pub fn bad_request(message: &str) -> ErrorInfo {
    ErrorInfo::new(BAD_REQUEST, format!("Bad Request: {message}"))
        .with_category(ErrorCategory::Network)
        .with_severity(ErrorSeverity::Error)
}

/// 创建401 Unauthorized错误
///
/// # 参数
/// - `message`: 错误消息
///
/// # 返回
/// 包含指定消息的401错误对象
pub fn unauthorized(message: &str) -> ErrorInfo {
    ErrorInfo::new(UNAUTHORIZED, format!("Unauthorized: {message}"))
        .with_category(ErrorCategory::Network)
        .with_severity(ErrorSeverity::Error)
}

/// 创建403 Forbidden错误
///
/// # 参数
/// - `message`: 错误消息
///
/// # 返回
/// 包含指定消息的403错误对象
pub fn forbidden(message: &str) -> ErrorInfo {
    ErrorInfo::new(FORBIDDEN, format!("Forbidden: {message}"))
        .with_category(ErrorCategory::Network)
        .with_severity(ErrorSeverity::Error)
}

/// 创建404 Not Found错误
///
/// # 参数
/// - `message`: 错误消息
///
/// # 返回
/// 包含指定消息的404错误对象
pub fn not_found(message: &str) -> ErrorInfo {
    ErrorInfo::new(NOT_FOUND, format!("Not Found: {message}"))
        .with_category(ErrorCategory::Network)
        .with_severity(ErrorSeverity::Error)
}

/// 创建405 Method Not Allowed错误
///
/// # 参数
/// - `message`: 错误消息
///
/// # 返回
/// 包含指定消息的405错误对象
pub fn method_not_allowed(message: &str) -> ErrorInfo {
    ErrorInfo::new(METHOD_NOT_ALLOWED, format!("Method Not Allowed: {message}"))
        .with_category(ErrorCategory::Network)
        .with_severity(ErrorSeverity::Error)
}

/// 创建429 Too Many Requests错误
///
/// # 参数
/// - `message`: 错误消息
///
/// # 返回
/// 包含指定消息的429错误对象
pub fn too_many_requests(message: &str) -> ErrorInfo {
    ErrorInfo::new(TOO_MANY_REQUESTS, format!("Too Many Requests: {message}"))
        .with_category(ErrorCategory::Network)
        .with_severity(ErrorSeverity::Error)
}

/// 创建500 Internal Server Error错误
///
/// # 参数
/// - `message`: 错误消息
///
/// # 返回
/// 包含指定消息的500错误对象
pub fn internal_server_error(message: &str) -> ErrorInfo {
    ErrorInfo::new(
        INTERNAL_SERVER_ERROR,
        format!("Internal Server Error: {message}"),
    )
    .with_category(ErrorCategory::Network)
    .with_severity(ErrorSeverity::Error)
}

/// 创建502 Bad Gateway错误
///
/// # 参数
/// - `message`: 错误消息
///
/// # 返回
/// 包含指定消息的502错误对象
pub fn bad_gateway(message: &str) -> ErrorInfo {
    ErrorInfo::new(BAD_GATEWAY, format!("Bad Gateway: {message}"))
        .with_category(ErrorCategory::Network)
        .with_severity(ErrorSeverity::Error)
}

/// 创建503 Service Unavailable错误
///
/// # 参数
/// - `message`: 错误消息
///
/// # 返回
/// 包含指定消息的503错误对象
pub fn service_unavailable(message: &str) -> ErrorInfo {
    ErrorInfo::new(
        SERVICE_UNAVAILABLE,
        format!("Service Unavailable: {message}"),
    )
    .with_category(ErrorCategory::Network)
    .with_severity(ErrorSeverity::Error)
}

/// 创建504 Gateway Timeout错误
///
/// # 参数
/// - `message`: 错误消息
///
/// # 返回
/// 包含指定消息的504错误对象
pub fn gateway_timeout(message: &str) -> ErrorInfo {
    ErrorInfo::new(GATEWAY_TIMEOUT, format!("Gateway Timeout: {message}"))
        .with_category(ErrorCategory::Network)
        .with_severity(ErrorSeverity::Error)
}
