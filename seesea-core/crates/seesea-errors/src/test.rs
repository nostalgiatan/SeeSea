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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ErrorCategory, ErrorSeverity};
use error::ErrorCategory;;

    #[test]
    fn test_network_errors() {
        // 测试连接超时错误
        let error = network::connection_timeout("example.com");
        assert_eq!(error.code(), network::CONNECTION_TIMEOUT);
        assert_eq!(error.category(), ErrorCategory::Network);
        assert_eq!(error.severity(), ErrorSeverity::Error);
        assert!(error.message().contains("example.com"));

        // 测试连接拒绝错误
        let error = network::connection_refused("example.com");
        assert_eq!(error.code(), network::CONNECTION_REFUSED);
        assert!(error.message().contains("被拒绝"));

        // 测试DNS解析失败错误
        let error = network::dns_resolve_failed("invalid-domain.example");
        assert_eq!(error.code(), network::DNS_RESOLVE_FAILED);
        assert!(error.message().contains("无法解析域名"));

        // 测试向后兼容的network_error函数
        let error = network::network_error("测试网络错误");
        assert_eq!(error.code(), network::NETWORK_ERROR_BASE);
    }

    #[test]
    fn test_search_errors() {
        // 测试引擎不可用错误
        let error = seesea_seesea_search::engine_unavailable("bing");
        assert_eq!(error.code(), seesea_seesea_search::ENGINE_UNAVAILABLE);
        assert_eq!(error.category(), ErrorCategory::Search);
        assert!(error.message().contains("bing"));

        // 测试搜索超时错误
        let error = seesea_seesea_search::search_timeout("bing");
        assert_eq!(error.code(), seesea_seesea_search::SEARCH_TIMEOUT);
        assert!(error.message().contains("搜索超时"));

        // 测试零结果错误
        let error = seesea_seesea_search::zero_results("bing");
        assert_eq!(error.code(), seesea_seesea_search::ZERO_RESULTS);
        assert_eq!(error.severity(), ErrorSeverity::Warning);

        // 测试向后兼容的search_error函数
        let error = seesea_seesea_search::search_error("测试搜索错误");
        assert_eq!(error.code(), seesea_seesea_search::SEARCH_ERROR_BASE);
    }

    #[test]
    fn test_parse_errors() {
        // 测试JSON解析错误
        let error = parse::json_parse_error("无效的JSON格式");
        assert_eq!(error.code(), parse::JSON_PARSE_ERROR);
        assert_eq!(error.category(), ErrorCategory::Parse);
        assert!(error.message().contains("JSON解析错误"));

        // 测试XML解析错误
        let error = parse::xml_parse_error("无效的XML格式");
        assert_eq!(error.code(), parse::XML_PARSE_ERROR);
        assert!(error.message().contains("XML解析错误"));

        // 测试HTML解析错误
        let error = parse::html_parse_error("无效的HTML格式");
        assert_eq!(error.code(), parse::HTML_PARSE_ERROR);
        assert!(error.message().contains("HTML解析错误"));
    }

    #[test]
    fn test_validation_errors() {
        // 测试空字段错误
        let error = validation::empty_field("username");
        assert_eq!(error.code(), validation::EMPTY_FIELD);
        assert_eq!(error.category(), ErrorCategory::Validation);
        assert!(error.message().contains("不能为空"));

        // 测试字段太短错误
        let error = validation::field_too_short("password", 8, 6);
        assert_eq!(error.code(), validation::FIELD_TOO_SHORT);
        assert!(error.message().contains("太短"));

        // 测试字段太长错误
        let error = validation::field_too_long("username", 20, 25);
        assert_eq!(error.code(), validation::FIELD_TOO_LONG);
        assert!(error.message().contains("太长"));

        // 测试无效邮箱错误
        let error = validation::invalid_email("invalid-email");
        assert_eq!(error.code(), validation::INVALID_EMAIL);
        assert!(error.message().contains("无效的邮箱地址"));
    }

    #[test]
    fn test_io_errors() {
        // 测试文件不存在错误
        let error = io::file_not_found("/path/to/nonexistent/file.txt");
        assert_eq!(error.code(), io::FILE_NOT_FOUND);
        assert_eq!(error.category(), ErrorCategory::Io);
        assert!(error.message().contains("文件不存在"));

        // 测试文件打开失败错误
        let error = io::file_open_failed("/path/to/file.txt", "权限被拒绝");
        assert_eq!(error.code(), io::FILE_OPEN_FAILED);
        assert!(error.message().contains("无法打开文件"));

        // 测试目录不存在错误
        let error = io::directory_not_found("/path/to/nonexistent/directory");
        assert_eq!(error.code(), io::DIRECTORY_NOT_FOUND);
        assert!(error.message().contains("目录不存在"));
    }

    #[test]
    fn test_permission_errors() {
        // 测试权限被拒绝错误
        let error = permission::permission_denied("resource", "read");
        assert_eq!(error.code(), permission::PERMISSION_DENIED);
        assert_eq!(error.category(), ErrorCategory::Permission);
        assert!(error.message().contains("拒绝访问"));

        // 测试未授权错误
        let error = permission::unauthorized();
        assert_eq!(error.code(), permission::UNAUTHORIZED);
        assert!(error.message().contains("未授权访问"));

        // 测试无效凭证错误
        let error = permission::invalid_credentials();
        assert_eq!(error.code(), permission::INVALID_CREDENTIALS);
        assert!(error.message().contains("无效的用户名或密码"));
    }

    #[test]
    fn test_configuration_errors() {
        // 测试配置文件未找到错误
        let error = configuration::config_file_not_found("/path/to/config.toml");
        assert_eq!(error.code(), configuration::CONFIG_FILE_NOT_FOUND);
        assert_eq!(error.category(), ErrorCategory::Configuration);
        assert!(error.message().contains("配置文件不存在"));

        // 测试配置文件解析失败错误
        let error = configuration::config_parse_failed("/path/to/config.toml", "无效的格式");
        assert_eq!(error.code(), configuration::CONFIG_PARSE_FAILED);
        assert!(error.message().contains("解析失败"));

        // 测试缺少配置项错误
        let error = configuration::missing_config_item("api_key");
        assert_eq!(error.code(), configuration::MISSING_CONFIG_ITEM);
        assert!(error.message().contains("缺少配置项"));
    }

    #[test]
    fn test_database_errors() {
        // 测试数据库连接失败错误
        let error = database::connection_failed("sqlite://test.db", "无法连接");
        assert_eq!(error.code(), database::CONNECTION_FAILED);
        assert_eq!(error.category(), ErrorCategory::Database);
        assert!(error.message().contains("连接失败"));

        // 测试查询失败错误
        let error = database::query_failed("SELECT * FROM users", "表不存在");
        assert_eq!(error.code(), database::QUERY_FAILED);
        assert!(error.message().contains("SQL查询失败"));

        // 测试重复键错误
        let error = database::duplicate_key("users", "email", "test@example.com");
        assert_eq!(error.code(), database::DUPLICATE_KEY);
        assert!(error.message().contains("已存在"));
    }

    #[test]
    fn test_business_errors() {
        // 测试资源未找到错误
        let error = business::resource_not_found("用户", "123");
        assert_eq!(error.code(), business::RESOURCE_NOT_FOUND);
        assert_eq!(error.category(), ErrorCategory::Business);
        assert!(error.message().contains("不存在"));

        // 测试业务规则违反错误
        let error = business::business_rule_violation("年龄限制", "必须年满18岁");
        assert_eq!(error.code(), business::BUSINESS_RULE_VIOLATION);
        assert!(error.message().contains("违反业务规则"));

        // 测试操作不允许错误
        let error = business::operation_not_allowed("删除", "资源正在使用中");
        assert_eq!(error.code(), business::OPERATION_NOT_ALLOWED);
        assert!(error.message().contains("不允许"));
    }

    #[test]
    fn test_system_errors() {
        // 测试资源耗尽错误
        let error = system::resource_exhausted("内存");
        assert_eq!(error.code(), system::RESOURCE_EXHAUSTED);
        assert_eq!(error.category(), ErrorCategory::System);
        assert!(error.message().contains("耗尽"));

        // 测试服务不可用错误
        let error = system::service_unavailable("数据库服务");
        assert_eq!(error.code(), system::SERVICE_UNAVAILABLE);
        assert!(error.message().contains("不可用"));

        // 测试系统超时错误
        let error = system::system_timeout("操作");
        assert_eq!(error.code(), system::SYSTEM_TIMEOUT);
        assert!(error.message().contains("超时"));
    }

    #[test]
    fn test_error_exports() {
        // 测试从mod.rs导出的函数
        let error = network_error("测试网络错误");
        assert_eq!(error.code(), network::NETWORK_ERROR_BASE);

        let error = search_error("测试搜索错误");
        assert_eq!(error.code(), seesea_seesea_search::SEARCH_ERROR_BASE);
    }

    #[test]
    fn test_error_categories() {
        // 测试所有错误类别
        assert_eq!(network::CONNECTION_TIMEOUT / 1000, 1); // 网络错误码范围
        assert_eq!(seesea_seesea_search::ENGINE_UNAVAILABLE / 1000, 2); // 搜索错误码范围
        assert_eq!(parse::JSON_PARSE_ERROR / 1000, 3); // 解析错误码范围
        assert_eq!(validation::EMPTY_FIELD / 1000, 4); // 验证错误码范围
        assert_eq!(io::FILE_NOT_FOUND / 1000, 5); // IO错误码范围
        assert_eq!(permission::PERMISSION_DENIED / 1000, 6); // 权限错误码范围
        assert_eq!(configuration::CONFIG_FILE_NOT_FOUND / 1000, 7); // 配置错误码范围
        assert_eq!(database::CONNECTION_FAILED / 1000, 8); // 数据库错误码范围
        assert_eq!(business::RESOURCE_NOT_FOUND / 1000, 9); // 业务错误码范围
        assert_eq!(system::RESOURCE_EXHAUSTED / 1000, 10); // 系统错误码范围
    }
}
