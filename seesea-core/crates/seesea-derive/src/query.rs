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

//! 搜索查询处理 trait

use crate::types::*;
use seesea_errors::{empty_field, field_too_long};
use std::error::Error;

/// 查询构建器 trait
pub trait QueryBuilder {
    /// 构建基础查询
    fn build_base_query(&self, query: &str, engine_type: EngineType) -> SearchQuery {
        SearchQuery {
            query: query.to_string(),
            engine_type,
            language: None,
            region: None,
            page_size: 10,
            page: 1,
            safe_search: SafeSearchLevel::Moderate,
            time_range: None,
            params: std::collections::HashMap::new(),
        }
    }

    /// 设置语言偏好
    fn with_language(self, _language: impl Into<String>) -> Self
    where
        Self: Sized,
    {
        // 在具体实现中设置语言
        self
    }

    /// 设置地区偏好
    fn with_region(self, _region: impl Into<String>) -> Self
    where
        Self: Sized,
    {
        // 在具体实现中设置地区
        self
    }

    /// 设置分页
    fn with_pagination(self, _page: usize, _page_size: usize) -> Self
    where
        Self: Sized,
    {
        // 在具体实现中设置分页
        self
    }

    /// 设置每页大小
    fn with_page_size(self, _page_size: usize) -> Self
    where
        Self: Sized,
    {
        // 在具体实现中设置每页大小
        self
    }

    /// 设置安全搜索级别
    fn with_safe_search(self, _level: SafeSearchLevel) -> Self
    where
        Self: Sized,
    {
        // 在具体实现中设置安全搜索
        self
    }

    /// 设置时间范围
    fn with_time_range(self, _range: TimeRange) -> Self
    where
        Self: Sized,
    {
        // 在具体实现中设置时间范围
        self
    }

    /// 添加自定义参数
    fn with_param(self, _key: impl Into<String>, _value: impl Into<String>) -> Self
    where
        Self: Sized,
    {
        // 在具体实现中添加参数
        self
    }

    /// 构建最终的查询
    fn build(self) -> SearchQuery;
}

/// 查询预处理 trait
pub trait QueryPreprocessor {
    /// 预处理查询
    fn preprocess(&self, query: &mut SearchQuery) -> Result<(), Box<dyn Error>>;

    /// 清理查询字符串
    fn clean_query(&self, query: &str) -> String {
        query
            .trim()
            .chars()
            .filter(|c| c.is_ascii_alphanumeric() || c.is_whitespace() || "-+\"".contains(*c))
            .collect::<String>()
            .split_whitespace()
            .collect::<Vec<&str>>()
            .join(" ")
    }

    /// 转义特殊字符
    fn escape_special_chars(&self, query: &str) -> String {
        query
            .replace('"', "\\\"")
            .replace("'", "\\'")
            .replace("\\", "\\\\")
    }
}

/// 查询优化 trait
pub trait QueryOptimizer {
    /// 优化查询
    fn optimize(&self, query: &mut SearchQuery) -> Result<(), Box<dyn Error>> {
        // 调整页面大小
        self.optimize_page_size(query, 100);

        // 设置默认语言（可选）
        self.set_default_language(query, "en");

        // 设置默认地区（可选）
        self.set_default_region(query, "us");

        // 移除无效参数
        self.remove_invalid_params(query);

        Ok(())
    }

    /// 调整页面大小
    fn optimize_page_size(&self, query: &mut SearchQuery, max_size: usize) {
        if query.page_size > max_size {
            query.page_size = max_size;
        } else if query.page_size == 0 {
            query.page_size = 10;
        }
    }

    /// 设置默认语言
    fn set_default_language(&self, query: &mut SearchQuery, default: &str) {
        if query.language.is_none() {
            query.language = Some(default.to_string());
        }
    }

    /// 设置默认地区
    fn set_default_region(&self, query: &mut SearchQuery, default: &str) {
        if query.region.is_none() {
            query.region = Some(default.to_string());
        }
    }

    /// 移除无效参数
    fn remove_invalid_params(&self, query: &mut SearchQuery) {
        query.params.retain(|_, value| !value.is_empty());
    }

    /// 限制最大页码
    fn limit_max_page(&self, query: &mut SearchQuery, max_page: usize) {
        if query.page > max_page {
            query.page = max_page;
        } else if query.page == 0 {
            query.page = 1;
        }
    }
}

/// 查询验证 trait
pub trait QueryValidator {
    /// 验证查询
    fn validate(&self, query: &SearchQuery) -> Result<(), ValidationError>;

    /// 验证查询字符串
    fn validate_query_string(&self, query: &str) -> Result<(), ValidationError> {
        if query.trim().is_empty() {
            let error_info = empty_field("query");
            return Err(ValidationError {
                code: "EMPTY_QUERY".to_string(),
                message: error_info.message().to_string(),
                field: Some("query".to_string()),
            });
        }

        if query.len() > 1000 {
            let error_info = field_too_long("query", 1000, query.len());
            return Err(ValidationError {
                code: "QUERY_TOO_LONG".to_string(),
                message: error_info.message().to_string(),
                field: Some("query".to_string()),
            });
        }

        // 检查是否包含潜在的恶意内容
        if self.contains_malicious_content(query) {
            return Err(ValidationError {
                code: "MALICIOUS_CONTENT".to_string(),
                message: "包含潜在的恶意内容".to_string(),
                field: Some("query".to_string()),
            });
        }

        Ok(())
    }

    /// 验证分页参数
    fn validate_pagination(&self, page: usize, page_size: usize) -> Result<(), ValidationError> {
        if page < 1 {
            return Err(ValidationError {
                code: "INVALID_PAGE_NUMBER".to_string(),
                message: "页码无效，必须大于0".to_string(),
                field: Some("page".to_string()),
            });
        }

        if !(1..=100).contains(&page_size) {
            return Err(ValidationError {
                code: "INVALID_PAGE_SIZE".to_string(),
                message: "页面大小无效，必须在1-100之间".to_string(),
                field: Some("page_size".to_string()),
            });
        }

        Ok(())
    }

    /// 检查恶意内容
    fn contains_malicious_content(&self, query: &str) -> bool {
        let malicious_patterns = [
            "<script",
            "</script>",
            "javascript:",
            "data:",
            "vbscript:",
            "onload=",
            "onerror=",
            "onclick=",
            "onmouseover=",
        ];

        let lower_query = query.to_lowercase();
        malicious_patterns
            .iter()
            .any(|&pattern| lower_query.contains(pattern))
    }
}

/// 查询转换 trait
pub trait QueryTransformer {
    /// 转换查询格式
    fn transform(&self, query: &SearchQuery, target_format: &str)
    -> Result<String, Box<dyn Error>>;

    /// 转换为URL参数
    fn to_url_params(&self, query: &SearchQuery) -> String {
        let mut params = Vec::new();

        params.push(format!("q={}", urlencoding::encode(&query.query)));

        if let Some(lang) = &query.language {
            params.push(format!("lang={}", urlencoding::encode(lang)));
        }

        if let Some(region) = &query.region {
            params.push(format!("region={}", urlencoding::encode(region)));
        }

        params.push(format!("num={}", query.page_size));
        params.push(format!("start={}", (query.page - 1) * query.page_size));

        if let Some(time_range) = query.time_range {
            params.push(format!("time_range={time_range:?}"));
        }

        for (key, value) in &query.params {
            params.push(format!(
                "{}={}",
                urlencoding::encode(key),
                urlencoding::encode(value)
            ));
        }

        params.join("&")
    }

    /// 转换为JSON
    fn to_json(&self, query: &SearchQuery) -> Result<String, Box<dyn Error>> {
        serde_json::to_string(query).map_err(Into::into)
    }

    /// 从JSON解析查询
    fn parse_json(&self, json: &str) -> Result<SearchQuery, Box<dyn Error>> {
        serde_json::from_str(json).map_err(Into::into)
    }
}
