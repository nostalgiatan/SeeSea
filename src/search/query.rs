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

//! 查询解析器模块
//!
//! 负责解析和分析搜索查询，识别查询意图、语言、地区等

/// 查询意图
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QueryIntent {
    /// 信息查询
    Informational,
    /// 导航查询
    Navigational,
    /// 交易查询
    Transactional,
    /// 本地查询
    Local,
    /// 新闻查询
    News,
    /// 图片查询
    Image,
    /// 视频查询
    Video,
    /// 代码查询
    Code,
}

/// 查询解析器
pub struct QueryParser {
    /// 是否启用意图识别
    enable_intent_detection: bool,
    /// 是否启用语言检测
    enable_language_detection: bool,
}

impl QueryParser {
    /// 创建新的查询解析器
    pub fn new() -> Self {
        Self {
            enable_intent_detection: true,
            enable_language_detection: true,
        }
    }

    /// 解析查询
    pub fn parse(&self, query: &str) -> ParsedQuery {
        let cleaned = self.normalize(query);
        let intent = if self.enable_intent_detection {
            self.detect_intent(&cleaned)
        } else {
            QueryIntent::Informational
        };
        let language = if self.enable_language_detection {
            self.detect_language(&cleaned)
        } else {
            None
        };

        ParsedQuery {
            original: query.to_string(),
            normalized: cleaned,
            intent,
            language,
            region: None,
            expanded_terms: Vec::new(),
        }
    }

    /// 规范化查询
    fn normalize(&self, query: &str) -> String {
        query.trim().to_lowercase()
    }

    /// 检测查询意图
    fn detect_intent(&self, query: &str) -> QueryIntent {
        if query.contains("site:") {
            return QueryIntent::Navigational;
        }
        if query.contains("buy") || query.contains("price") || query.contains("购买") {
            return QueryIntent::Transactional;
        }
        if query.contains("near me") || query.contains("附近") {
            return QueryIntent::Local;
        }
        if query.contains("news:") || query.starts_with("新闻") {
            return QueryIntent::News;
        }
        if query.contains("image:") || query.contains("图片") {
            return QueryIntent::Image;
        }
        if query.contains("video:") || query.contains("视频") {
            return QueryIntent::Video;
        }
        if query.contains("code:") || query.contains("代码") {
            return QueryIntent::Code;
        }
        QueryIntent::Informational
    }

    /// 检测语言
    fn detect_language(&self, query: &str) -> Option<String> {
        // 简单的语言检测
        if query.chars().any(|c| '\u{4e00}' <= c && c <= '\u{9fff}') {
            return Some("zh".to_string());
        }
        if query.chars().all(|c| c.is_ascii()) {
            return Some("en".to_string());
        }
        None
    }

    /// 扩展查询（添加同义词等）
    pub fn expand(&self, _query: &str) -> Vec<String> {
        // 简化实现，实际应该查询同义词库
        Vec::new()
    }
}

impl Default for QueryParser {
    fn default() -> Self {
        Self::new()
    }
}

/// 解析后的查询
#[derive(Debug, Clone)]
pub struct ParsedQuery {
    /// 原始查询
    pub original: String,
    /// 规范化查询
    pub normalized: String,
    /// 查询意图
    pub intent: QueryIntent,
    /// 语言
    pub language: Option<String>,
    /// 地区
    pub region: Option<String>,
    /// 扩展词汇
    pub expanded_terms: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parser_creation() {
        let parser = QueryParser::new();
        assert!(parser.enable_intent_detection);
        assert!(parser.enable_language_detection);
    }

    #[test]
    fn test_normalize() {
        let parser = QueryParser::new();
        let result = parser.normalize("  Hello World  ");
        assert_eq!(result, "hello world");
    }

    #[test]
    fn test_detect_intent_navigational() {
        let parser = QueryParser::new();
        let intent = parser.detect_intent("site:example.com");
        assert_eq!(intent, QueryIntent::Navigational);
    }

    #[test]
    fn test_detect_intent_transactional() {
        let parser = QueryParser::new();
        let intent = parser.detect_intent("buy shoes");
        assert_eq!(intent, QueryIntent::Transactional);
    }

    #[test]
    fn test_detect_intent_local() {
        let parser = QueryParser::new();
        let intent = parser.detect_intent("restaurant near me");
        assert_eq!(intent, QueryIntent::Local);
    }

    #[test]
    fn test_detect_language_chinese() {
        let parser = QueryParser::new();
        let lang = parser.detect_language("你好世界");
        assert_eq!(lang, Some("zh".to_string()));
    }

    #[test]
    fn test_detect_language_english() {
        let parser = QueryParser::new();
        let lang = parser.detect_language("hello world");
        assert_eq!(lang, Some("en".to_string()));
    }

    #[test]
    fn test_parse_complete() {
        let parser = QueryParser::new();
        let parsed = parser.parse("  Buy Laptop  ");
        assert_eq!(parsed.original, "  Buy Laptop  ");
        assert_eq!(parsed.normalized, "buy laptop");
        assert_eq!(parsed.intent, QueryIntent::Transactional);
        assert_eq!(parsed.language, Some("en".to_string()));
    }

    #[test]
    fn test_parse_chinese() {
        let parser = QueryParser::new();
        let parsed = parser.parse("购买笔记本电脑");
        assert_eq!(parsed.intent, QueryIntent::Transactional);
        assert_eq!(parsed.language, Some("zh".to_string()));
    }
}
