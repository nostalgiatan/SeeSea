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

//! 搜索结果标准化
//!
//! 对搜索结果进行基本的清理和标准化

use crate::derive::{SearchResultItem, SearchResult};
use std::collections::HashSet;

/// 清理文本
pub fn clean_text(text: &str, max_length: usize) -> String {
    // 1. 移除多余空白
    let cleaned: String = text
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    
    // 2. HTML 实体解码
    let cleaned = html_escape::decode_html_entities(&cleaned).to_string();
    
    // 3. 截断
    if cleaned.len() > max_length {
        let truncated: String = cleaned.chars().take(max_length - 3).collect();
        format!("{}...", truncated)
    } else {
        cleaned
    }
}

/// 标准化单个结果项
pub fn standardize_item(item: &mut SearchResultItem) {
    // 清理标题（最多200字符）
    item.title = clean_text(&item.title, 200);
    
    // 清理内容（最多500字符）
    item.content = clean_text(&item.content, 500);
    
    // 确保 URL 不为空
    if item.url.trim().is_empty() {
        item.url = "#".to_string();
    }
}

/// 简单去重（基于 URL）
pub fn deduplicate_by_url(items: &mut Vec<SearchResultItem>) {
    let mut seen = HashSet::new();
    items.retain(|item| {
        let url_lower = item.url.to_lowercase().trim().to_string();
        seen.insert(url_lower)
    });
}

/// 标准化搜索结果
pub fn standardize_results(result: &mut SearchResult) {
    // 1. 标准化每个项
    for item in &mut result.items {
        standardize_item(item);
    }
    
    // 2. 去重
    deduplicate_by_url(&mut result.items);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clean_text() {
        assert_eq!(clean_text("  hello   world  ", 100), "hello world");
        
        let long = "a".repeat(300);
        let cleaned = clean_text(&long, 100);
        assert!(cleaned.len() <= 103); // 100 + "..."
    }
}
