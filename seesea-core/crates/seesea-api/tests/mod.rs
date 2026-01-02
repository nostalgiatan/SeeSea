//! SeeSea API 测试模块
//!
//! 提供全面的 API 测试覆盖，包括:
//! - 类型验证和序列化测试
//! - 请求处理逻辑测试
//! - 中间件功能测试
//! - 错误处理测试
//! - 性能和安全测试

pub mod test_api_types;
pub mod test_error_handling;
pub mod test_handlers;
pub mod test_integration;
pub mod test_middleware;
pub mod test_performance;
pub mod test_security;
pub mod test_utils;
