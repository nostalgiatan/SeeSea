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

//! 搜索结果标准化
//!
//! 对搜索结果进行基本的清理和标准化

use super::utils::time_extractor::{TimeSource, extract_time, extract_time_from_url};
use chrono::{DateTime, Utc};
use scraper::{Html, Selector};
use seesea_derive::{SearchResult, SearchResultItem};
use std::collections::HashSet;

/// 清理文本
pub fn clean_text(text: &str, max_length: usize) -> String {
    // 1. 移除HTML标签
    let mut cleaned = String::with_capacity(text.len());
    let mut in_tag = false;

    for c in text.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => cleaned.push(c),
            _ => {}
        }
    }

    // 2. 移除多余空白
    cleaned = cleaned.split_whitespace().collect::<Vec<_>>().join(" ");

    // 3. HTML 实体解码
    cleaned = html_escape::decode_html_entities(&cleaned).to_string();

    // 4. 截断
    if cleaned.len() > max_length {
        let truncated: String = cleaned.chars().take(max_length - 3).collect();
        format!("{truncated}...")
    } else {
        cleaned
    }
}

/// 从HTML中提取时间
///
/// 尝试从HTML中提取发布时间
///
/// # 参数
///
/// * `html` - 要提取时间的HTML文本
///
/// # 返回
///
/// 提取的时间，或None
pub fn extract_time_from_html(html: &str) -> Option<DateTime<Utc>> {
    let document = Html::parse_document(html);

    // 尝试从meta标签提取时间
    let meta_selectors = [
        "meta[property='article:published_time']",
        "meta[name='article:published_time']",
        "meta[property='og:pubdate']",
        "meta[name='pubdate']",
        "meta[property='og:updated_time']",
        "meta[name='updated_time']",
        "meta[name='date']",
        "meta[property='article:modified_time']",
    ];

    for selector_str in meta_selectors {
        if let Ok(selector) = Selector::parse(selector_str) {
            for elem in document.select(&selector) {
                if let Some(content) = elem.value().attr("content") {
                    let result = extract_time(content, TimeSource::MetaTag);
                    if result.datetime.is_some() {
                        return result.datetime;
                    }
                }
            }
        }
    }

    // 尝试从时间标签提取
    let time_selectors = [
        "time",
        "span[class*='time']",
        "span[class*='date']",
        "div[class*='time']",
        "div[class*='date']",
    ];

    for selector_str in time_selectors {
        if let Ok(selector) = Selector::parse(selector_str) {
            for elem in document.select(&selector) {
                let text = elem.text().collect::<String>().trim().to_string();
                if !text.is_empty() {
                    let result = extract_time(&text, TimeSource::ResultCard);
                    if result.datetime.is_some() {
                        return result.datetime;
                    }
                }

                // 尝试从datetime属性提取
                if let Some(datetime) = elem.value().attr("datetime") {
                    let result = extract_time(datetime, TimeSource::ResultCard);
                    if result.datetime.is_some() {
                        return result.datetime;
                    }
                }
            }
        }
    }

    None
}

/// 从内容中提取时间
///
/// 尝试从文本内容中提取发布时间
///
/// # 参数
///
/// * `content` - 要提取时间的文本内容
///
/// # 返回
///
/// 提取的时间，或None
pub fn extract_time_from_content(content: &str) -> Option<DateTime<Utc>> {
    let result = extract_time(content, TimeSource::Content);
    result.datetime
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

    // 提取时间
    let mut extracted_time = None;

    // 1. 尝试从URL提取时间
    if let Some(time) = extract_time_from_url(&item.url) {
        extracted_time = Some(time);
    }

    // 2. 尝试从内容提取时间
    if extracted_time.is_none()
        && let Some(time) = extract_time_from_content(&item.content)
    {
        extracted_time = Some(time);
    }

    // 设置提取的时间
    item.published_date = extracted_time;
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
    standardize_items(&mut result.items);
}

/// 标准化搜索结果项
pub fn standardize_items(items: &mut Vec<SearchResultItem>) {
    // 1. 标准化每个项
    for item in &mut *items {
        standardize_item(item);
    }

    // 2. 去重
    deduplicate_by_url(items);
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
