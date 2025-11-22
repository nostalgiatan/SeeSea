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

//! 搜索结果处理 trait

use crate::derive::types::*;
use std::error::Error;
use async_trait::async_trait;

/// 结果解析器 trait
#[async_trait]
pub trait ResultParser {
    /// 解析响应为搜索结果
    async fn parse(&self, response: &str, query: &SearchQuery) -> Result<SearchResult, Box<dyn Error + Send + Sync>>;

    /// 解析单个结果项
    fn parse_item(&self, raw: &serde_json::Value) -> Result<SearchResultItem, Box<dyn Error + Send + Sync>> {
        Ok(SearchResultItem {
            title: self.extract_title(raw)?,
            url: self.extract_url(raw)?,
            content: self.extract_content(raw)?,
            display_url: self.extract_display_url(raw).ok(),
            site_name: self.extract_site_name(raw).ok(),
            score: self.extract_score(raw).unwrap_or(0.0),
            result_type: self.extract_result_type(raw).unwrap_or(ResultType::Web),
            thumbnail: self.extract_thumbnail(raw).ok(),
            published_date: self.extract_published_date(raw).ok(),
            metadata: self.extract_metadata(raw)?,
            template: None, // 默认无特殊模板
        })
    }

    /// 提取标题
    fn extract_title(&self, raw: &serde_json::Value) -> Result<String, Box<dyn Error + Send + Sync>>;

    /// 提取URL
    fn extract_url(&self, raw: &serde_json::Value) -> Result<String, Box<dyn Error + Send + Sync>>;

    /// 提取内容
    fn extract_content(&self, raw: &serde_json::Value) -> Result<String, Box<dyn Error + Send + Sync>>;

    /// 提取显示URL
    fn extract_display_url(&self, raw: &serde_json::Value) -> Result<String, Box<dyn Error + Send + Sync>>;

    /// 提取网站名称
    fn extract_site_name(&self, raw: &serde_json::Value) -> Result<String, Box<dyn Error + Send + Sync>>;

    /// 提取评分
    fn extract_score(&self, raw: &serde_json::Value) -> Result<f64, Box<dyn Error + Send + Sync>>;

    /// 提取结果类型
    fn extract_result_type(&self, raw: &serde_json::Value) -> Result<ResultType, Box<dyn Error + Send + Sync>>;

    /// 提取缩略图
    fn extract_thumbnail(&self, raw: &serde_json::Value) -> Result<String, Box<dyn Error + Send + Sync>>;

    /// 提取发布日期
    fn extract_published_date(&self, raw: &serde_json::Value) -> Result<chrono::DateTime<chrono::Utc>, Box<dyn Error + Send + Sync>>;

    /// 提取元数据
    fn extract_metadata(&self, raw: &serde_json::Value) -> Result<std::collections::HashMap<String, String>, Box<dyn Error + Send + Sync>>;
}

/// 结果过滤器 trait
pub trait ResultFilter {
    /// 过滤结果
    fn filter(&self, results: &mut Vec<SearchResultItem>) -> Result<(), Box<dyn Error>>;

    /// 过滤重复结果
    fn deduplicate(&self, results: &mut Vec<SearchResultItem>) {
        let mut seen_urls = std::collections::HashSet::new();
        results.retain(|item| {
            let normalized_url = self.normalize_url(&item.url);
            seen_urls.insert(normalized_url)
        });
    }

    /// 过滤低质量结果
    fn filter_low_quality(&self, results: &mut Vec<SearchResultItem>, min_score: f64) {
        results.retain(|item| item.score >= min_score);
    }

    /// 过滤特定域名
    fn filter_domains(&self, results: &mut Vec<SearchResultItem>, blocked_domains: &[String]) {
        results.retain(|item| {
            !blocked_domains.iter().any(|domain| {
                item.url.contains(domain) ||
                item.display_url.as_ref().map_or(false, |url| url.contains(domain))
            })
        });
    }

    /// 规范化URL
    fn normalize_url(&self, url: &str) -> String {
        // 移除协议、www、尾部斜杠等
        url.to_lowercase()
            .replace("https://", "")
            .replace("http://", "")
            .replace("www.", "")
            .trim_end_matches('/')
            .to_string()
    }

    /// 限制结果数量
    fn limit_results(&self, results: &mut Vec<SearchResultItem>, limit: usize) {
        if results.len() > limit {
            results.truncate(limit);
        }
    }
}

/// 结果排序 trait
pub trait ResultSorter {
    /// 排序结果
    fn sort(&self, results: &mut Vec<SearchResultItem>);

    /// 按评分排序
    fn sort_by_score(&self, results: &mut Vec<SearchResultItem>) {
        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    }

    /// 按相关性排序
    fn sort_by_relevance(&self, results: &mut Vec<SearchResultItem>, query: &str) {
        results.sort_by(|a, b| {
            let score_a = self.calculate_relevance(a, query);
            let score_b = self.calculate_relevance(b, query);
            score_b.partial_cmp(&score_a).unwrap_or(std::cmp::Ordering::Equal)
        });
    }

    /// 计算相关性评分
    fn calculate_relevance(&self, item: &SearchResultItem, query: &str) -> f64 {
        let query_lower = query.to_lowercase();
        let query_terms: Vec<&str> = query_lower.split_whitespace().collect();
        let title_lower = item.title.to_lowercase();
        let content_lower = item.content.to_lowercase();

        let mut score = 0.0;

        for term in query_terms {
            // 标题匹配权重更高
            if title_lower.contains(term) {
                if title_lower.starts_with(term) {
                    score += 2.0; // 开头匹配
                } else {
                    score += 1.0; // 包含匹配
                }
            }

            // 内容匹配
            if content_lower.contains(term) {
                score += 0.5;
            }
        }

        // URL匹配
        if item.url.to_lowercase().contains(query) {
            score += 1.5;
        }

        score
    }

    /// 多因子排序
    fn sort_by_multiple_factors(&self, results: &mut Vec<SearchResultItem>, query: &str) {
        results.sort_by(|a, b| {
            let score_a = self.calculate_combined_score(a, query);
            let score_b = self.calculate_combined_score(b, query);
            score_b.partial_cmp(&score_a).unwrap_or(std::cmp::Ordering::Equal)
        });
    }

    /// 计算综合评分
    fn calculate_combined_score(&self, item: &SearchResultItem, query: &str) -> f64 {
        let relevance = self.calculate_relevance(item, query);
        let original_score = item.score;
        let freshness_bonus = self.calculate_freshness_bonus(&item.published_date);

        relevance * 0.6 + original_score * 0.3 + freshness_bonus * 0.1
    }

    /// 计算时效性加分
    fn calculate_freshness_bonus(&self, published_date: &Option<chrono::DateTime<chrono::Utc>>) -> f64 {
        match published_date {
            Some(date) => {
                let now = chrono::Utc::now();
                let days_old = (now - *date).num_days();
                if days_old < 7 {
                    1.0
                } else if days_old < 30 {
                    0.5
                } else if days_old < 365 {
                    0.2
                } else {
                    0.0
                }
            }
            None => 0.0,
        }
    }
}

/// 结果增强 trait
pub trait ResultEnhancer {
    /// 增强结果
    fn enhance(&self, results: &mut Vec<SearchResultItem>) -> Result<(), Box<dyn Error>>;

    /// 添加网站图标
    fn add_favicons(&self, results: &mut Vec<SearchResultItem>) -> Result<(), Box<dyn Error>> {
        for item in results.iter_mut() {
            if let Some(domain) = self.extract_domain(&item.url) {
                let favicon_url = format!("https://www.google.com/s2/favicons?domain={}&sz=32", domain);
                item.metadata.insert("favicon".to_string(), favicon_url);
            }
        }
        Ok(())
    }

    /// 提取域名
    fn extract_domain(&self, url: &str) -> Option<String> {
        if let Ok(parsed) = url::Url::parse(url) {
            Some(parsed.host_str()?.to_string())
        } else {
            None
        }
    }

    /// 添加语言检测
    fn add_language_detection(&self, results: &mut Vec<SearchResultItem>) -> Result<(), Box<dyn Error>> {
        for item in results.iter_mut() {
            // 简单的语言检测逻辑
            let language = self.detect_language(&item.title, &item.content);
            item.metadata.insert("language".to_string(), language);
        }
        Ok(())
    }

    /// 检测语言
    fn detect_language(&self, title: &str, content: &str) -> String {
        let text = format!("{} {}", title, content);

        // 简单的字符统计检测
        let chinese_chars = text.chars().filter(|c| is_chinese_char(*c)).count();
        let total_chars = text.chars().count();

        if total_chars > 0 && (chinese_chars as f64 / total_chars as f64) > 0.3 {
            "zh".to_string()
        } else {
            "en".to_string() // 默认英语
        }
    }

    /// 添加页面信息
    fn add_page_info(&self, results: &mut Vec<SearchResultItem>) -> Result<(), Box<dyn Error>> {
        for item in results.iter_mut() {
            if let Ok(parsed) = url::Url::parse(&item.url) {
                item.metadata.insert("scheme".to_string(), parsed.scheme().to_string());
                item.metadata.insert("host".to_string(), parsed.host_str().unwrap_or("").to_string());
                if let Some(port) = parsed.port() {
                    item.metadata.insert("port".to_string(), port.to_string());
                }
                if let Some(path) = parsed.path_segments() {
                    item.metadata.insert("path_depth".to_string(), path.count().to_string());
                }
            }
        }
        Ok(())
    }
}

/// 结果格式化 trait
pub trait ResultFormatter {
    /// 格式化结果
    fn format(&self, results: &[SearchResultItem]) -> Result<String, Box<dyn Error>>;

    /// 格式化为JSON
    fn to_json(&self, results: &[SearchResultItem]) -> Result<String, Box<dyn Error>> {
        serde_json::to_string(results).map_err(Into::into)
    }

    /// 格式化为HTML
    fn to_html(&self, results: &[SearchResultItem]) -> Result<String, Box<dyn Error>> {
        let mut html = String::from("<div class=\"search-results\">");

        for item in results {
            html.push_str(&format!(r#"
<div class="result-item">
    <h3 class="title"><a href="{url}">{title}</a></h3>
    <div class="url">{display_url}</div>
    <div class="content">{content}</div>
    <div class="meta">Score: {score:.2}</div>
</div>"#,
                url = html_escape::encode_text(&item.url),
                title = html_escape::encode_text(&item.title),
                display_url = item.display_url.as_deref().unwrap_or(&item.url),
                content = html_escape::encode_text(&item.content),
                score = item.score
            ));
        }

        html.push_str("</div>");
        Ok(html)
    }

    /// 格式化为纯文本
    fn to_text(&self, results: &[SearchResultItem]) -> Result<String, Box<dyn Error>> {
        let mut text = String::new();

        for (i, item) in results.iter().enumerate() {
            text.push_str(&format!(
                "{}. {}\n   {}\n   {}\n   Score: {:.2}\n\n",
                i + 1,
                item.title,
                item.url,
                item.content,
                item.score
            ));
        }

        Ok(text)
    }
}

/// 辅助函数：判断是否为中文字符
fn is_chinese_char(c: char) -> bool {
    matches!(c,
        '\u{4e00}'..='\u{9fff}' | // CJK Unified Ideographs
        '\u{3400}'..='\u{4dbf}' | // CJK Unified Ideographs Extension A
        '\u{20000}'..='\u{2a6df}' | // CJK Unified Ideographs Extension B
        '\u{2a700}'..='\u{2b73f}' | // CJK Unified Ideographs Extension C
        '\u{2b740}'..='\u{2b81f}' | // CJK Unified Ideographs Extension D
        '\u{2b820}'..='\u{2ceaf}' | // CJK Unified Ideographs Extension E
        '\u{2ceb0}'..='\u{2ebef}' | // CJK Unified Ideographs Extension F
        '\u{3000}'..='\u{303f}' |   // CJK Symbols and Punctuation
        '\u{ff00}'..='\u{ffef}'     // Halfwidth and Fullwidth Forms
    )
}