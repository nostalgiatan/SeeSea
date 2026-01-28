// Copyright (C) 2025 nostalgiatan
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

//! 数据处理模块 - 清洗、转换、筛选
//!
//! 职责：
//! 1. 数据清洗 - 处理缺失值、异常值
//! 2. 数据转换 - 统一字段名称、类型转换
//! 3. 数据筛选 - 按代码、名称等条件筛选
//! 4. 数据聚合 - 合并、计算衍生指标

use crate::error::{StockError, StockResult};
use serde_json::{Value, json};

/// 数据处理器
pub struct StockProcessor;

impl StockProcessor {
    /// 创建新的处理器
    pub fn new() -> Self {
        Self
    }

    /// 处理股票列表数据 - 标准化字段
    pub fn process_stock_list(&self, data: Value) -> StockResult<Value> {
        let stocks = data
            .as_array()
            .ok_or_else(|| StockError::ParseError("数据不是数组".into()))?;

        let processed: Vec<Value> = stocks
            .iter()
            .filter_map(|stock| self.normalize_stock_item(stock))
            .collect();

        Ok(Value::Array(processed))
    }

    /// 标准化单个股票数据项
    fn normalize_stock_item(&self, item: &Value) -> Option<Value> {
        let obj = item.as_object()?;

        // 尝试多种可能的字段名
        let code = self.extract_field(obj, &["代码", "code", "股票代码", "symbol"]);
        let name = self.extract_field(obj, &["名称", "name", "股票名称"]);

        code.as_ref()?;

        // 构建标准化的数据结构
        let mut normalized = serde_json::Map::new();

        // 必填字段
        normalized.insert("code".to_string(), json!(code));
        normalized.insert("name".to_string(), json!(name.unwrap_or_default()));

        // 行情字段 (可选)
        if let Some(v) = self.extract_number(obj, &["最新价", "price", "现价"]) {
            normalized.insert("price".to_string(), json!(v));
        }
        if let Some(v) = self.extract_number(obj, &["涨跌幅", "change_pct", "pct_chg"]) {
            normalized.insert("change_pct".to_string(), json!(v));
        }
        if let Some(v) = self.extract_number(obj, &["涨跌额", "change", "change_amt"]) {
            normalized.insert("change".to_string(), json!(v));
        }
        if let Some(v) = self.extract_number(obj, &["成交量", "volume", "vol"]) {
            normalized.insert("volume".to_string(), json!(v));
        }
        if let Some(v) = self.extract_number(obj, &["成交额", "amount", "turnover"]) {
            normalized.insert("amount".to_string(), json!(v));
        }
        if let Some(v) = self.extract_number(obj, &["最高", "high"]) {
            normalized.insert("high".to_string(), json!(v));
        }
        if let Some(v) = self.extract_number(obj, &["最低", "low"]) {
            normalized.insert("low".to_string(), json!(v));
        }
        if let Some(v) = self.extract_number(obj, &["今开", "open"]) {
            normalized.insert("open".to_string(), json!(v));
        }
        if let Some(v) = self.extract_number(obj, &["昨收", "pre_close"]) {
            normalized.insert("pre_close".to_string(), json!(v));
        }

        Some(Value::Object(normalized))
    }

    /// 从对象中提取字段值（尝试多个可能的键名）
    fn extract_field(&self, obj: &serde_json::Map<String, Value>, keys: &[&str]) -> Option<String> {
        for key in keys {
            if let Some(v) = obj.get(*key) {
                if let Some(s) = v.as_str() {
                    return Some(s.to_string());
                }
                // 尝试转换数字为字符串
                if let Some(n) = v.as_i64() {
                    return Some(format!("{:06}", n));
                }
            }
        }
        None
    }

    /// 从对象中提取数字值
    fn extract_number(&self, obj: &serde_json::Map<String, Value>, keys: &[&str]) -> Option<f64> {
        for key in keys {
            if let Some(v) = obj.get(*key) {
                if let Some(n) = v.as_f64() {
                    return Some(n);
                }
                if let Some(n) = v.as_i64() {
                    return Some(n as f64);
                }
                // 尝试解析字符串
                if let Some(s) = v.as_str()
                    && let Ok(n) = s.parse::<f64>()
                {
                    return Some(n);
                }
            }
        }
        None
    }

    /// 按股票代码搜索
    pub fn search_by_code(&self, data: &Value, code: &str) -> StockResult<Vec<Value>> {
        let stocks = data
            .as_array()
            .ok_or_else(|| StockError::ParseError("数据不是数组".into()))?;

        let code_lower = code.to_lowercase();
        let results: Vec<Value> = stocks
            .iter()
            .filter(|stock| {
                if let Some(obj) = stock.as_object()
                    && let Some(stock_code) = self.extract_field(obj, &["code", "代码", "股票代码"])
                {
                    stock_code.to_lowercase().contains(&code_lower)
                } else {
                    false
                }
            })
            .cloned()
            .collect();

        Ok(results)
    }

    /// 按股票名称搜索
    pub fn search_by_name(&self, data: &Value, name: &str) -> StockResult<Vec<Value>> {
        let stocks = data
            .as_array()
            .ok_or_else(|| StockError::ParseError("数据不是数组".into()))?;

        let name_lower = name.to_lowercase();
        let results: Vec<Value> = stocks
            .iter()
            .filter(|stock| {
                if let Some(obj) = stock.as_object()
                    && let Some(stock_name) = self.extract_field(obj, &["name", "名称", "股票名称"])
                {
                    stock_name.to_lowercase().contains(&name_lower)
                } else {
                    false
                }
            })
            .cloned()
            .collect();

        Ok(results)
    }

    /// 综合搜索（代码或名称）
    pub fn search(
        &self,
        data: &Value,
        keyword: &str,
        limit: Option<usize>,
    ) -> StockResult<Vec<Value>> {
        let stocks = data
            .as_array()
            .ok_or_else(|| StockError::ParseError("数据不是数组".into()))?;

        let keyword_lower = keyword.to_lowercase();
        let mut results: Vec<Value> = stocks
            .iter()
            .filter(|stock| {
                if let Some(obj) = stock.as_object() {
                    let code_match = self
                        .extract_field(obj, &["code", "代码", "股票代码"])
                        .map(|c| c.to_lowercase().contains(&keyword_lower))
                        .unwrap_or(false);
                    let name_match = self
                        .extract_field(obj, &["name", "名称", "股票名称"])
                        .map(|n| n.to_lowercase().contains(&keyword_lower))
                        .unwrap_or(false);
                    return code_match || name_match;
                }
                false
            })
            .cloned()
            .collect();

        // 排序：精确匹配优先，代码匹配优先
        results.sort_by(|a, b| {
            let a_code = a.get("code").and_then(|v| v.as_str()).unwrap_or("");
            let b_code = b.get("code").and_then(|v| v.as_str()).unwrap_or("");

            // 精确匹配代码排最前
            let a_exact = a_code.to_lowercase() == keyword_lower;
            let b_exact = b_code.to_lowercase() == keyword_lower;

            if a_exact && !b_exact {
                std::cmp::Ordering::Less
            } else if !a_exact && b_exact {
                std::cmp::Ordering::Greater
            } else {
                // 代码开头匹配优先
                let a_starts = a_code.to_lowercase().starts_with(&keyword_lower);
                let b_starts = b_code.to_lowercase().starts_with(&keyword_lower);
                if a_starts && !b_starts {
                    std::cmp::Ordering::Less
                } else if !a_starts && b_starts {
                    std::cmp::Ordering::Greater
                } else {
                    std::cmp::Ordering::Equal
                }
            }
        });

        // 限制结果数量
        if let Some(max) = limit {
            results.truncate(max);
        }

        Ok(results)
    }

    /// 按股票代码精确获取单只股票
    pub fn get_by_code(&self, data: &Value, code: &str) -> StockResult<Option<Value>> {
        let stocks = data
            .as_array()
            .ok_or_else(|| StockError::ParseError("数据不是数组".into()))?;

        // 标准化代码（补零到6位）
        let normalized_code = format!("{:0>6}", code.trim_start_matches('0'));
        let code_variants = vec![
            code.to_string(),
            normalized_code.clone(),
            code.trim_start_matches('0').to_string(),
        ];

        for stock in stocks {
            if let Some(obj) = stock.as_object()
                && let Some(stock_code) = self.extract_field(obj, &["code", "代码", "股票代码"])
            {
                for variant in &code_variants {
                    if stock_code == *variant || format!("{:0>6}", stock_code) == normalized_code {
                        return Ok(Some(stock.clone()));
                    }
                }
            }
        }

        Ok(None)
    }

    /// 处理板块数据
    pub fn process_board_list(&self, data: Value) -> StockResult<Value> {
        let boards = data
            .as_array()
            .ok_or_else(|| StockError::ParseError("数据不是数组".into()))?;

        let processed: Vec<Value> = boards
            .iter()
            .filter_map(|board| {
                let obj = board.as_object()?;
                let mut normalized = serde_json::Map::new();

                // 板块名称
                if let Some(name) = self.extract_field(obj, &["板块名称", "name", "板块"]) {
                    normalized.insert("name".to_string(), json!(name));
                } else {
                    return None;
                }

                // 板块代码
                if let Some(code) = self.extract_field(obj, &["板块代码", "code"]) {
                    normalized.insert("code".to_string(), json!(code));
                }

                // 涨跌幅
                if let Some(v) = self.extract_number(obj, &["涨跌幅", "change_pct"]) {
                    normalized.insert("change_pct".to_string(), json!(v));
                }

                // 成交额
                if let Some(v) = self.extract_number(obj, &["总成交额", "amount"]) {
                    normalized.insert("amount".to_string(), json!(v));
                }

                Some(Value::Object(normalized))
            })
            .collect();

        Ok(Value::Array(processed))
    }

    /// 处理指数数据
    pub fn process_index_list(&self, data: Value) -> StockResult<Value> {
        // 指数数据格式与股票类似
        self.process_stock_list(data)
    }

    /// 处理涨跌停数据
    pub fn process_limit_pool(&self, data: Value) -> StockResult<Value> {
        let stocks = data
            .as_array()
            .ok_or_else(|| StockError::ParseError("数据不是数组".into()))?;

        let processed: Vec<Value> = stocks
            .iter()
            .filter_map(|stock| {
                let obj = stock.as_object()?;
                let mut normalized = serde_json::Map::new();

                // 基础信息
                if let Some(code) = self.extract_field(obj, &["代码", "code"]) {
                    normalized.insert("code".to_string(), json!(code));
                } else {
                    return None;
                }

                if let Some(name) = self.extract_field(obj, &["名称", "name"]) {
                    normalized.insert("name".to_string(), json!(name));
                }

                // 涨停特有字段
                if let Some(v) = self.extract_number(obj, &["涨停价", "limit_price"]) {
                    normalized.insert("limit_price".to_string(), json!(v));
                }
                if let Some(v) = self.extract_number(obj, &["封单额", "seal_amount"]) {
                    normalized.insert("seal_amount".to_string(), json!(v));
                }
                if let Some(v) = self.extract_number(obj, &["连板数", "streak"]) {
                    normalized.insert("streak".to_string(), json!(v));
                }
                if let Some(v) = self.extract_field(obj, &["首次涨停时间", "first_time"]) {
                    normalized.insert("first_time".to_string(), json!(v));
                }
                if let Some(v) = self.extract_field(obj, &["最后涨停时间", "last_time"]) {
                    normalized.insert("last_time".to_string(), json!(v));
                }
                if let Some(v) = self.extract_field(obj, &["涨停原因类别", "reason"]) {
                    normalized.insert("reason".to_string(), json!(v));
                }

                Some(Value::Object(normalized))
            })
            .collect();

        Ok(Value::Array(processed))
    }
}

impl Default for StockProcessor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search() {
        let processor = StockProcessor::new();
        let data = json!([
            {"code": "000001", "name": "平安银行"},
            {"code": "600000", "name": "浦发银行"},
            {"code": "000002", "name": "万科A"},
        ]);

        let results = processor.search(&data, "银行", Some(10)).unwrap();
        assert_eq!(results.len(), 2);

        let results = processor.search(&data, "000001", Some(10)).unwrap();
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_get_by_code() {
        let processor = StockProcessor::new();
        let data = json!([
            {"code": "000001", "name": "平安银行"},
            {"code": "600000", "name": "浦发银行"},
        ]);

        let result = processor.get_by_code(&data, "000001").unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap()["name"], "平安银行");

        let result = processor.get_by_code(&data, "1").unwrap();
        assert!(result.is_some()); // 应该能匹配到 000001
    }
}
