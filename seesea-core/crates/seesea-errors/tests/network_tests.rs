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

//! 网络错误测试
//!
//! 测试网络相关错误的创建、属性验证和功能测试。

use error::ErrorCategory;
use seesea_errors::{
    ErrorSeverity, bad_gateway, connection_refused, connection_timeout, dns_resolve_failed,
    forbidden, gateway_timeout, http_error, internal_server_error, method_not_allowed,
    network_error, network_unreachable, not_found, proxy_error, request_cancelled, ssl_error,
    too_many_redirects, too_many_requests,
};

// 从网络模块单独导入service_unavailable以避免命名冲突
use seesea_errors::network::service_unavailable;

// 从网络模块单独导入unauthorized以避免命名冲突
use seesea_errors::network::unauthorized;

/// 测试基础网络错误创建
#[test]
fn test_network_error_creation() {
    let error = network_error("网络连接失败");

    assert_eq!(error.category(), ErrorCategory::Network);
    assert_eq!(error.severity(), ErrorSeverity::Error);
    assert_eq!(error.code(), 1000); // NETWORK_ERROR_BASE
    assert!(error.message().contains("网络连接失败"));
    assert!(error.context().is_empty());
    assert!(error.source().is_none());
}

/// 测试连接被拒绝错误
#[test]
fn test_connection_refused() {
    let error = connection_refused("127.0.0.1:8080");

    assert_eq!(error.category(), ErrorCategory::Network);
    assert_eq!(error.severity(), ErrorSeverity::Error);
    assert_eq!(error.code(), 1002); // CONNECTION_REFUSED
    assert!(error.message().contains("连接到 127.0.0.1:8080 被拒绝"));
    assert!(error.context().is_empty());
}

/// 测试连接超时错误
#[test]
fn test_connection_timeout() {
    let error = connection_timeout("api.example.com");

    assert_eq!(error.category(), ErrorCategory::Network);
    assert_eq!(error.severity(), ErrorSeverity::Error);
    assert_eq!(error.code(), 1001); // CONNECTION_TIMEOUT
    assert!(error.message().contains("连接到 api.example.com 超时"));
    assert!(error.context().is_empty());
}

/// 测试DNS解析失败错误
#[test]
fn test_dns_resolve_failed() {
    let error = dns_resolve_failed("invalid-domain.xyz");

    assert_eq!(error.category(), ErrorCategory::Network);
    assert_eq!(error.severity(), ErrorSeverity::Error);
    assert_eq!(error.code(), 1003); // DNS_RESOLVE_FAILED
    assert!(error.message().contains("无法解析域名"));
    assert!(error.message().contains("invalid-domain.xyz"));
    assert!(error.context().is_empty());
}

/// 测试网络不可达错误
#[test]
fn test_network_unreachable() {
    let error = network_unreachable("目标网络不可达");

    assert_eq!(error.category(), ErrorCategory::Network);
    assert_eq!(error.severity(), ErrorSeverity::Error);
    assert_eq!(error.code(), 1007); // NETWORK_UNREACHABLE
    assert!(error.message().contains("网络不可达"));
    assert!(error.message().contains("目标网络不可达"));
}

/// 测试HTTP状态码错误
#[test]
fn test_http_error_by_status() {
    // 测试404错误
    let error_404 = http_error(404, "https://example.com/missing");
    assert_eq!(error_404.code(), 1006); // HTTP_ERROR
    assert!(error_404.message().contains("HTTP错误 404"));
    assert!(error_404.message().contains("https://example.com/missing"));

    // 测试500错误
    let error_500 = http_error(500, "https://example.com/error");
    assert_eq!(error_500.code(), 1006); // HTTP_ERROR
    assert!(error_500.message().contains("HTTP错误 500"));
    assert!(error_500.message().contains("https://example.com/error"));

    // 测试403错误
    let error_403 = http_error(403, "https://example.com/forbidden");
    assert_eq!(error_403.code(), 1006); // HTTP_ERROR
    assert!(error_403.message().contains("HTTP错误 403"));
    assert!(
        error_403
            .message()
            .contains("https://example.com/forbidden")
    );

    // 测试401错误
    let error_401 = http_error(401, "https://example.com/unauthorized");
    assert_eq!(error_401.code(), 1006); // HTTP_ERROR
    assert!(error_401.message().contains("HTTP错误 401"));
    assert!(
        error_401
            .message()
            .contains("https://example.com/unauthorized")
    );
}

/// 测试具体的HTTP错误函数
#[test]
fn test_specific_http_errors() {
    // 测试404错误
    let not_found_error = not_found("https://api.example.com/users/123");
    assert_eq!(not_found_error.code(), 1104); // NOT_FOUND
    assert!(not_found_error.message().contains("Not Found"));
    assert!(
        not_found_error
            .message()
            .contains("https://api.example.com/users/123")
    );

    // 测试500错误
    let server_error = internal_server_error("数据库查询失败");
    assert_eq!(server_error.code(), 1200); // INTERNAL_SERVER_ERROR
    assert!(server_error.message().contains("Internal Server Error"));
    assert!(server_error.message().contains("数据库查询失败"));

    // 测试403错误
    let forbidden_error = forbidden("权限不足");
    assert_eq!(forbidden_error.code(), 1103); // FORBIDDEN
    assert!(forbidden_error.message().contains("Forbidden"));
    assert!(forbidden_error.message().contains("权限不足"));

    // 测试401错误
    let unauthorized_error = unauthorized("令牌过期");
    assert_eq!(unauthorized_error.code(), 1101); // UNAUTHORIZED
    assert!(unauthorized_error.message().contains("Unauthorized"));
    assert!(unauthorized_error.message().contains("令牌过期"));
}

/// 测试网关错误
#[test]
fn test_gateway_errors() {
    // 测试502错误
    let bad_gateway_error = bad_gateway("上游服务器无响应");
    assert_eq!(bad_gateway_error.code(), 1202); // BAD_GATEWAY
    assert!(bad_gateway_error.message().contains("Bad Gateway"));
    assert!(bad_gateway_error.message().contains("上游服务器无响应"));

    // 测试504错误
    let gateway_timeout_error = gateway_timeout("网关超时");
    assert_eq!(gateway_timeout_error.code(), 1204); // GATEWAY_TIMEOUT
    assert!(gateway_timeout_error.message().contains("Gateway Timeout"));
    assert!(gateway_timeout_error.message().contains("网关超时"));
}

/// 测试请求方法错误
#[test]
fn test_method_errors() {
    let method_error = method_not_allowed("POST");
    assert_eq!(method_error.code(), 1105); // METHOD_NOT_ALLOWED
    assert!(method_error.message().contains("Method Not Allowed"));
    assert!(method_error.message().contains("POST"));
}

/// 测试SSL错误
#[test]
fn test_ssl_error() {
    let ssl_err = ssl_error("证书验证失败");
    assert_eq!(ssl_err.code(), 1005); // SSL_ERROR
    assert!(ssl_err.message().contains("SSL错误"));
    assert!(ssl_err.message().contains("证书验证失败"));
}

/// 测试代理错误
#[test]
fn test_proxy_error() {
    let proxy_err = proxy_error("代理服务器无响应");
    assert_eq!(proxy_err.code(), 1008); // PROXY_ERROR
    assert!(proxy_err.message().contains("代理错误"));
    assert!(proxy_err.message().contains("代理服务器无响应"));
}

/// 测试重定向错误
#[test]
fn test_redirect_errors() {
    let redirect_err = too_many_redirects("https://example.com");
    assert_eq!(redirect_err.code(), 1009); // TOO_MANY_REDIRECTS
    assert!(redirect_err.message().contains("重定向次数过多"));
    assert!(redirect_err.message().contains("https://example.com"));
}

/// 测试请求取消错误
#[test]
fn test_request_cancelled() {
    let cancelled_err = request_cancelled("用户取消请求");
    assert_eq!(cancelled_err.code(), 1010); // REQUEST_CANCELLED
    assert!(cancelled_err.message().contains("请求取消"));
    assert!(cancelled_err.message().contains("用户取消请求"));
}

/// 测试服务不可用错误
#[test]
fn test_service_unavailable() {
    let unavailable_err = service_unavailable("服务器维护中");
    assert_eq!(unavailable_err.code(), 1203); // SERVICE_UNAVAILABLE
    assert!(unavailable_err.message().contains("Service Unavailable"));
    assert!(unavailable_err.message().contains("服务器维护中"));
}

/// 测试过多请求错误
#[test]
fn test_too_many_requests() {
    let rate_limit_err = too_many_requests("请求频率过高");
    assert_eq!(rate_limit_err.code(), 1109); // TOO_MANY_REQUESTS
    assert!(rate_limit_err.message().contains("Too Many Requests"));
    assert!(rate_limit_err.message().contains("请求频率过高"));
}

/// 测试通用的HTTP错误
#[test]
fn test_generic_http_error() {
    let generic_err = http_error(418, "I'm a teapot");
    assert_eq!(generic_err.code(), 1006); // HTTP_ERROR
    assert!(generic_err.message().contains("418"));
    assert!(generic_err.message().contains("I'm a teapot"));
}

/// 测试错误消息格式
#[test]
fn test_error_message_formats() {
    let error = connection_refused("localhost:8080");
    let message = format!("{}", error);

    // 验证消息包含关键信息
    assert!(message.contains("1002")); // CONNECTION_REFUSED code
    assert!(message.contains("连接到 localhost:8080 被拒绝"));

    // 验证调试格式
    let debug_message = format!("{:?}", error);
    assert!(debug_message.contains("ErrorInfo"));
}

/// 测试错误严重程度的一致性
#[test]
fn test_error_severity_consistency() {
    // 网络错误通常应该是Error或Warning级别
    let connection_err = connection_refused("test:80");
    assert_eq!(connection_err.severity(), ErrorSeverity::Error);

    let timeout_err = connection_timeout("test.com");
    assert_eq!(timeout_err.severity(), ErrorSeverity::Error);

    let server_err = internal_server_error("test");
    assert_eq!(server_err.severity(), ErrorSeverity::Error);

    // 某些错误应该是Warning级别
    let request_err = request_cancelled("用户取消");
    assert_eq!(request_err.severity(), ErrorSeverity::Warning);

    // 频率限制错误应该是Error级别
    let rate_limit_err = too_many_requests("频率限制");
    assert_eq!(rate_limit_err.severity(), ErrorSeverity::Error);
}

/// 测试错误类别的一致性
#[test]
fn test_error_category_consistency() {
    // 所有网络错误都应该有Network类别
    let errors = vec![
        network_error("test"),
        connection_refused("test"),
        connection_timeout("test"),
        dns_resolve_failed("test"),
        network_unreachable("test"),
        not_found("test"),
        internal_server_error("test"),
        forbidden("test"),
        unauthorized("test"),
        bad_gateway("test"),
        gateway_timeout("test"),
        method_not_allowed("GET"),
        ssl_error("test"),
        proxy_error("test"),
        too_many_redirects("test"),
        request_cancelled("test"),
        service_unavailable("test"),
        too_many_requests("test"),
    ];

    for error in errors {
        assert_eq!(error.category(), ErrorCategory::Network);
    }
}
