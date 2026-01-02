//! API 测试工具模块
//!
//! 提供通用的测试工具函数和辅助结构，用于简化 API 测试的编写和维护。

use seesea_api::api::types::*;
use serde_json::Value;
use std::time::{Duration, Instant};

/// 测试数据生成器
pub struct TestDataGenerator;

impl TestDataGenerator {
    /// 生成有效的搜索请求
    pub fn create_valid_search_request(query: &str) -> ApiSearchRequest {
        ApiSearchRequest {
            query: Some(query.to_string()),
            _q: None,
            engine_count: Some(1),
            page: 1,
            page_size: 10,
            language: Some("en".to_string()),
            region: Some("us".to_string()),
            safe_search: Some("moderate".to_string()),
            time_range: None,
            engines: None,
            include_deepweb: false,
        }
    }

    /// 生成无效的搜索请求
    pub fn create_invalid_search_request() -> ApiSearchRequest {
        ApiSearchRequest {
            query: None,
            _q: None,
            engine_count: None,
            page: 0,      // 无效页码
            page_size: 0, // 无效页面大小
            language: None,
            region: None,
            safe_search: None,
            time_range: None,
            engines: None,
            include_deepweb: false,
        }
    }

    /// 生成完整的搜索响应
    pub fn create_search_response(query: &str, result_count: usize) -> ApiSearchResponse {
        let mut results = Vec::new();

        for i in 0..result_count {
            results.push(ApiSearchResultItem {
                title: format!("Result {} for {}", i + 1, query),
                url: format!("https://example.com/result/{}", i),
                description: Some(format!(
                    "This is search result number {} for query: {}",
                    i + 1,
                    query
                )),
                engine: "google".to_string(),
                score: Some(0.95 - (i as f64 * 0.05)),
                published_date: Some("2024-01-15".to_string()),
            });
        }

        ApiSearchResponse {
            query: query.to_string(),
            results,
            total_count: result_count * 2, // 模拟更多结果
            page: 1,
            page_size: result_count as u32,
            engines_used: vec!["google".to_string()],
            query_time_ms: 150,
            cached: false,
        }
    }

    /// 生成错误响应
    pub fn create_error_response(code: &str, message: &str) -> ApiErrorResponse {
        ApiErrorResponse {
            code: code.to_string(),
            message: message.to_string(),
            details: Some("Test error details".to_string()),
        }
    }

    /// 生成健康检查响应
    pub fn create_health_response(status: &str, version: &str) -> ApiHealthResponse {
        ApiHealthResponse {
            status: status.to_string(),
            version: version.to_string(),
            available_engines: 5,
            total_engines: 8,
        }
    }

    /// 生成统计响应
    pub fn create_stats_response() -> ApiStatsResponse {
        ApiStatsResponse {
            total_searches: 12345,
            cache_hits: 9258,
            cache_misses: 3087,
            cache_hit_rate: 0.75,
            engine_failures: 12,
            timeouts: 5,
        }
    }
}

/// JSON 验证工具
pub struct JsonValidator;

impl JsonValidator {
    /// 验证 JSON 结构是否有效
    pub fn is_valid_json(json_str: &str) -> bool {
        serde_json::from_str::<Value>(json_str).is_ok()
    }

    /// 验证搜索请求 JSON
    pub fn validate_search_request_json(json_str: &str) -> Result<bool, String> {
        let value: Value =
            serde_json::from_str(json_str).map_err(|e| format!("Invalid JSON: {}", e))?;

        // 检查必需字段
        if let Some(obj) = value.as_object() {
            if !obj.contains_key("query") && !obj.contains_key("q") {
                return Err("Missing query field".to_string());
            }

            if let Some(page) = obj.get("page").and_then(|v| v.as_i64()) {
                if page < 1 {
                    return Err("Page must be >= 1".to_string());
                }
            }

            if let Some(page_size) = obj.get("page_size").and_then(|v| v.as_i64()) {
                if page_size < 1 || page_size > 100 {
                    return Err("Page size must be between 1 and 100".to_string());
                }
            }
        }

        Ok(true)
    }

    /// 验证搜索响应 JSON
    pub fn validate_search_response_json(json_str: &str) -> Result<bool, String> {
        let value: Value =
            serde_json::from_str(json_str).map_err(|e| format!("Invalid JSON: {}", e))?;

        if let Some(obj) = value.as_object() {
            let required_fields = ["query", "results", "total_count", "page", "page_size"];
            for field in required_fields {
                if !obj.contains_key(field) {
                    return Err(format!("Missing field: {}", field));
                }
            }
        }

        Ok(true)
    }

    /// 验证错误响应 JSON
    pub fn validate_error_response_json(json_str: &str) -> Result<bool, String> {
        let value: Value =
            serde_json::from_str(json_str).map_err(|e| format!("Invalid JSON: {}", e))?;

        if let Some(obj) = value.as_object() {
            let required_fields = ["code", "message"];
            for field in required_fields {
                if !obj.contains_key(field) {
                    return Err(format!("Missing field: {}", field));
                }
            }
        }

        Ok(true)
    }
}

/// 性能测试工具
pub struct PerformanceTester {
    start_time: Instant,
    request_count: usize,
    success_count: usize,
    error_count: usize,
    response_times: Vec<u64>,
}

impl PerformanceTester {
    /// 创建新的性能测试器
    pub fn new() -> Self {
        Self {
            start_time: Instant::now(),
            request_count: 0,
            success_count: 0,
            error_count: 0,
            response_times: Vec::new(),
        }
    }

    /// 记录成功的请求
    pub fn record_success(&mut self, response_time: Duration) {
        self.request_count += 1;
        self.success_count += 1;
        self.response_times.push(response_time.as_millis() as u64);
    }

    /// 记录失败的请求
    pub fn record_error(&mut self) {
        self.request_count += 1;
        self.error_count += 1;
    }

    /// 获取性能统计
    pub fn get_stats(&self) -> PerformanceStats {
        let total_time = self.start_time.elapsed();

        let avg_response_time = if !self.response_times.is_empty() {
            self.response_times.iter().sum::<u64>() as f64 / self.response_times.len() as f64
        } else {
            0.0
        };

        let min_response_time = self.response_times.first().copied().unwrap_or(0);
        let max_response_time = self.response_times.last().copied().unwrap_or(0);

        let mut sorted_times = self.response_times.clone();
        sorted_times.sort_unstable();

        let p95_index = (sorted_times.len() as f64 * 0.95) as usize;
        let p95_response_time = sorted_times.get(p95_index).copied().unwrap_or(0) as f64;

        let throughput = self.request_count as f64 / total_time.as_secs_f64();
        let success_rate = (self.success_count as f64 / self.request_count as f64) * 100.0;
        let error_rate = (self.error_count as f64 / self.request_count as f64) * 100.0;

        PerformanceStats {
            total_requests: self.request_count,
            successful_requests: self.success_count,
            failed_requests: self.error_count,
            total_time_ms: total_time.as_millis() as u64,
            average_response_time_ms: avg_response_time,
            min_response_time_ms: min_response_time,
            max_response_time_ms: max_response_time,
            p95_response_time_ms: p95_response_time,
            throughput_rps: throughput,
            success_rate_percent: success_rate,
            error_rate_percent: error_rate,
        }
    }

    /// 重置测试器
    pub fn reset(&mut self) {
        self.start_time = Instant::now();
        self.request_count = 0;
        self.success_count = 0;
        self.error_count = 0;
        self.response_times.clear();
    }
}

/// 性能统计信息
#[derive(Debug, Clone)]
pub struct PerformanceStats {
    pub total_requests: usize,
    pub successful_requests: usize,
    pub failed_requests: usize,
    pub total_time_ms: u64,
    pub average_response_time_ms: f64,
    pub min_response_time_ms: u64,
    pub max_response_time_ms: u64,
    pub p95_response_time_ms: f64,
    pub throughput_rps: f64,
    pub success_rate_percent: f64,
    pub error_rate_percent: f64,
}

/// 安全测试工具
pub struct SecurityTester;

impl SecurityTester {
    /// 测试 SQL 注入攻击
    pub fn test_sql_injection(payload: &str) -> bool {
        let sql_patterns = [
            r"(?i)(union\s+select|select\s+.*from|insert\s+into|delete\s+from|drop\s+table)",
            r"(?i)(';|--|/\*|\*/)",
            r"(?i)(or\s+1=1|and\s+1=1)",
            r"(?i)(exec\s*\(|execute\s*\()",
        ];

        for pattern in &sql_patterns {
            if regex::Regex::new(pattern).unwrap().is_match(payload) {
                return true;
            }
        }
        false
    }

    /// 测试 XSS 攻击
    pub fn test_xss(payload: &str) -> bool {
        let xss_patterns = [
            r"<script[^>]*>.*?</script>",
            r"javascript:",
            r"on\w+\s*=",
            r"<iframe[^>]*>",
            r"<svg[^>]*on\w+",
            r"<img[^>]*on\w+",
        ];

        for pattern in &xss_patterns {
            if regex::Regex::new(pattern).unwrap().is_match(payload) {
                return true;
            }
        }
        false
    }

    /// 测试路径遍历攻击
    pub fn test_path_traversal(payload: &str) -> bool {
        let traversal_patterns = [
            r"\.\./",
            r"\.\.\\",
            r"%2e%2e%2f",
            r"%2e%2e%5c",
            r"%252e%252e%252f",
        ];

        for pattern in &traversal_patterns {
            if regex::Regex::new(pattern).unwrap().is_match(payload) {
                return true;
            }
        }
        false
    }

    /// 测试命令注入攻击
    pub fn test_command_injection(payload: &str) -> bool {
        let command_patterns = [
            r"[;&|]",
            r"`.*`",
            r"\$\(.*\)",
            r"\$\{.*\}",
            r"(?:cat|ls|pwd|whoami|id|uname|netstat|ipconfig)",
        ];

        for pattern in &command_patterns {
            if regex::Regex::new(pattern).unwrap().is_match(payload) {
                return true;
            }
        }
        false
    }

    /// 清理输入字符串
    pub fn sanitize_input(input: &str) -> String {
        let mut cleaned = input.to_string();

        // 移除 HTML 标签
        cleaned = regex::Regex::new(r"<[^>]*>")
            .unwrap()
            .replace_all(&cleaned, "")
            .to_string();

        // 移除 JavaScript
        cleaned = regex::Regex::new(r"(?i)javascript:")
            .unwrap()
            .replace_all(&cleaned, "")
            .to_string();

        // 移除 SQL 关键字
        let sql_keywords = [
            "union", "select", "insert", "delete", "drop", "exec", "execute",
        ];
        for keyword in &sql_keywords {
            let pattern = format!(r"(?i){}", regex::escape(keyword));
            cleaned = regex::Regex::new(&pattern)
                .unwrap()
                .replace_all(&cleaned, "")
                .to_string();
        }

        // 限制长度
        if cleaned.len() > 1000 {
            cleaned.truncate(1000);
        }

        cleaned.trim().to_string()
    }
}

/// 测试辅助函数
pub mod helpers {
    use super::*;

    /// 创建测试用的搜索请求
    pub fn create_test_search_request(query: &str) -> ApiSearchRequest {
        TestDataGenerator::create_valid_search_request(query)
    }

    /// 创建测试用的搜索响应
    pub fn create_test_search_response(query: &str, result_count: usize) -> ApiSearchResponse {
        TestDataGenerator::create_search_response(query, result_count)
    }

    /// 验证响应时间是否在合理范围内
    pub fn assert_response_time_reasonable(duration: Duration, max_ms: u64) {
        assert!(
            duration.as_millis() <= max_ms as u128,
            "Response time {}ms exceeds maximum {}ms",
            duration.as_millis(),
            max_ms
        );
    }

    /// 验证搜索结果的完整性
    pub fn assert_search_response_complete(response: &ApiSearchResponse) {
        assert!(!response.query.is_empty(), "Query should not be empty");
        assert!(
            response.results.len() <= response.page_size as usize,
            "Results count should not exceed page size"
        );
        // Note: total_count is usize, so it's always >= 0
        assert!(response.page >= 1, "Page should be >= 1");
        assert!(response.page_size >= 1, "Page size should be >= 1");
        assert!(
            !response.engines_used.is_empty(),
            "Engines used should not be empty"
        );
        // Note: query_time_ms is u64, so it's always >= 0
    }

    /// 生成随机查询字符串
    pub fn generate_random_query(length: usize) -> String {
        use rand::Rng;
        const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789 ";
        let mut rng = rand::thread_rng();

        (0..length)
            .map(|_| {
                let idx = rng.gen_range(0..CHARSET.len());
                CHARSET[idx] as char
            })
            .collect()
    }

    /// 生成随机 URL
    pub fn generate_random_url() -> String {
        format!("https://example.com/{}", generate_random_query(10).trim())
    }

    /// 模拟网络延迟
    pub async fn simulate_network_delay(ms: u64) {
        tokio::time::sleep(Duration::from_millis(ms)).await;
    }

    /// 模拟 API 响应延迟
    pub async fn simulate_api_response_delay() {
        // 模拟 50-200ms 的响应延迟
        let delay = 50 + (rand::random::<u64>() % 150);
        simulate_network_delay(delay).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_data_generator_creates_valid_request() {
        let request = TestDataGenerator::create_valid_search_request("test query");
        assert_eq!(request.get_query().unwrap(), "test query");
        assert_eq!(request.page, 1);
        assert_eq!(request.page_size, 10);
    }

    #[test]
    fn test_data_generator_creates_invalid_request() {
        let request = TestDataGenerator::create_invalid_search_request();
        assert!(request.get_query().is_err());
        assert_eq!(request.page, 0);
        assert_eq!(request.page_size, 0);
    }

    #[test]
    fn test_json_validator_validates_search_request() {
        let valid_json = r#"{"query": "test", "page": 1, "page_size": 10}"#;
        assert!(JsonValidator::validate_search_request_json(valid_json).is_ok());

        let invalid_json = r#"{"page": 0, "page_size": 10}"#;
        assert!(JsonValidator::validate_search_request_json(invalid_json).is_err());
    }

    #[test]
    fn test_security_tester_detects_sql_injection() {
        assert!(SecurityTester::test_sql_injection(
            "'; DROP TABLE users; --"
        ));
        assert!(SecurityTester::test_sql_injection("' OR 1=1"));
        assert!(!SecurityTester::test_sql_injection("normal query"));
    }

    #[test]
    fn test_security_tester_detects_xss() {
        assert!(SecurityTester::test_xss("<script>alert('xss')</script>"));
        assert!(SecurityTester::test_xss("javascript:alert(1)"));
        assert!(!SecurityTester::test_xss("normal text"));
    }

    #[test]
    fn test_helpers_generate_random_query() {
        let query = helpers::generate_random_query(20);
        assert_eq!(query.len(), 20);
        assert!(!query.trim().is_empty());
    }
}
