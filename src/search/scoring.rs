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

//! 搜索结果评分算法
//!
//! 基于 BM25 算法和其他启发式规则进行评分

use crate::derive::{SearchResultItem, SearchQuery};
use std::collections::HashMap;

/// BM25 参数
#[derive(Debug, Clone)]
pub struct BM25Params {
    /// k1 参数：控制词频饱和度 (通常 1.2-2.0)
    pub k1: f64,
    /// b 参数：控制文档长度归一化 (通常 0.75)
    pub b: f64,
}

impl Default for BM25Params {
    fn default() -> Self {
        Self {
            k1: 1.5,
            b: 0.75,
        }
    }
}

/// 评分权重配置
#[derive(Debug, Clone)]
pub struct ScoringWeights {
    /// 标题 BM25 权重
    pub title_bm25: f64,
    /// 内容 BM25 权重
    pub content_bm25: f64,
    /// URL 匹配权重
    pub url_match: f64,
    /// 引擎权威度权重
    pub engine_authority: f64,
    /// 位置权重
    pub position_weight: f64,
}

impl Default for ScoringWeights {
    fn default() -> Self {
        Self {
            title_bm25: 0.40,       // 标题最重要
            content_bm25: 0.30,     // 内容次之
            url_match: 0.10,        // URL 匹配
            engine_authority: 0.15, // 引擎权威度
            position_weight: 0.05,  // 原始排名位置
        }
    }
}

/// 引擎权威度评分
pub fn get_engine_authority(engine_name: &str) -> f64 {
    match engine_name.to_lowercase().as_str() {
        // 国际引擎
        "google" => 1.0,
        "bing" => 0.95,
        "duckduckgo" => 0.90,
        "brave" => 0.88,
        "startpage" => 0.85,
        "qwant" => 0.83,
        "yahoo" => 0.80,
        
        // 中国引擎 (中国模式保留)
        "baidu" => 0.95,
        "search360" => 0.85,
        "sogou" => 0.80,
        
        // 其他引擎
        "yandex" => 0.85,
        "mojeek" => 0.75,
        
        // 专业引擎
        "wikipedia" => 0.95,
        "wikidata" => 0.90,
        "github" => 0.92,
        "stackoverflow" => 0.93,
        "unsplash" => 0.85,
        
        _ => 0.70,
    }
}

/// 分词（简单实现）
pub(crate) fn tokenize(text: &str) -> Vec<String> {
    text.to_lowercase()
        .split(|c: char| !c.is_alphanumeric() && c != '_')
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect()
}

/// 计算词频 (Term Frequency)
fn term_frequency(tokens: &[String]) -> HashMap<String, usize> {
    let mut tf = HashMap::new();
    for token in tokens {
        *tf.entry(token.clone()).or_insert(0) += 1;
    }
    tf
}

/// BM25 评分算法
///
/// BM25 是目前最流行的搜索排名算法之一，被用于 Elasticsearch、Lucene 等
/// 
/// 公式: BM25(D,Q) = Σ IDF(qi) * (f(qi,D) * (k1 + 1)) / (f(qi,D) + k1 * (1 - b + b * |D|/avgdl))
/// 
/// 其中:
/// - D: 文档
/// - Q: 查询
/// - qi: 查询中的词
/// - f(qi,D): qi在D中的频率
/// - |D|: 文档长度
/// - avgdl: 平均文档长度
/// - k1, b: 调节参数
pub(crate) fn bm25_score(
    document: &str,
    query: &str,
    avg_doc_length: f64,
    params: &BM25Params,
) -> f64 {
    let doc_tokens = tokenize(document);
    let query_tokens = tokenize(query);
    
    if doc_tokens.is_empty() || query_tokens.is_empty() {
        return 0.0;
    }
    
    let doc_length = doc_tokens.len() as f64;
    let tf = term_frequency(&doc_tokens);
    
    let mut score = 0.0;
    
    for query_token in &query_tokens {
        if let Some(&freq) = tf.get(query_token) {
            let freq = freq as f64;
            
            // IDF 简化版本（假设文档集合较小）
            let idf = 1.0; // 在单文档评分中简化
            
            // BM25 公式
            let numerator = freq * (params.k1 + 1.0);
            let denominator = freq + params.k1 * (1.0 - params.b + params.b * (doc_length / avg_doc_length));
            
            score += idf * (numerator / denominator);
        }
    }
    
    // 归一化到 0-1
    let max_possible_score = query_tokens.len() as f64 * (params.k1 + 1.0);
    if max_possible_score > 0.0 {
        (score / max_possible_score).min(1.0)
    } else {
        0.0
    }
}

/// 精确匹配加分
pub(crate) fn exact_match_bonus(text: &str, query: &str) -> f64 {
    let text_lower = text.to_lowercase();
    let query_lower = query.to_lowercase();
    
    if text_lower.contains(&query_lower) {
        // 完整查询出现在文本中
        if text_lower == query_lower {
            return 1.0; // 完全匹配
        } else if text_lower.starts_with(&query_lower) {
            return 0.8; // 开头匹配
        } else {
            return 0.5; // 包含匹配
        }
    }
    
    0.0
}

/// URL 相关性评分
pub(crate) fn url_relevance(url: &str, query: &str) -> f64 {
    let url_lower = url.to_lowercase();
    let query_tokens = tokenize(query);
    
    let mut matches = 0;
    for token in &query_tokens {
        if url_lower.contains(token) {
            matches += 1;
        }
    }
    
    if query_tokens.is_empty() {
        0.0
    } else {
        matches as f64 / query_tokens.len() as f64
    }
}

/// 位置评分（原始搜索引擎排名）
pub(crate) fn position_score(position: usize) -> f64 {
    // 对数衰减：前几个结果分数明显更高
    // position 从 0 开始, 加1避免ln(0)
    1.0 / (1.0 + ((position + 1) as f64).ln())
}

/// 计算综合评分
pub(crate) fn calculate_score(
    item: &SearchResultItem,
    query: &SearchQuery,
    engine_name: &str,
    position: usize,
    avg_title_length: f64,
    avg_content_length: f64,
    weights: &ScoringWeights,
    bm25_params: &BM25Params,
) -> f64 {
    // 1. 标题 BM25 评分
    let title_bm25 = bm25_score(&item.title, &query.query, avg_title_length, bm25_params);
    let title_exact = exact_match_bonus(&item.title, &query.query);
    let title_score = (title_bm25 * 0.7 + title_exact * 0.3).min(1.0);
    
    // 2. 内容 BM25 评分
    let content_bm25 = bm25_score(&item.content, &query.query, avg_content_length, bm25_params);
    let content_exact = exact_match_bonus(&item.content, &query.query);
    let content_score = (content_bm25 * 0.8 + content_exact * 0.2).min(1.0);
    
    // 3. URL 相关性
    let url_score = url_relevance(&item.url, &query.query);
    
    // 4. 引擎权威度
    let authority_score = get_engine_authority(engine_name);
    
    // 5. 位置评分
    let pos_score = position_score(position);
    
    // 加权求和
    let final_score = 
        title_score * weights.title_bm25 +
        content_score * weights.content_bm25 +
        url_score * weights.url_match +
        authority_score * weights.engine_authority +
        pos_score * weights.position_weight;
    
    // 确保在 [0, 1] 范围内
    final_score.max(0.0).min(1.0)
}

/// 批量评分
pub fn score_results(
    items: &mut [SearchResultItem],
    query: &SearchQuery,
    engine_name: &str,
    weights: Option<ScoringWeights>,
    bm25_params: Option<BM25Params>,
) {
    if items.is_empty() {
        return;
    }
    
    let weights = weights.unwrap_or_default();
    let bm25_params = bm25_params.unwrap_or_default();
    
    // 计算平均文档长度
    let avg_title_length = items.iter()
        .map(|i| tokenize(&i.title).len())
        .sum::<usize>() as f64 / items.len() as f64;
    
    let avg_content_length = items.iter()
        .map(|i| tokenize(&i.content).len())
        .sum::<usize>() as f64 / items.len() as f64;
    
    // 计算每个结果的评分
    for (position, item) in items.iter_mut().enumerate() {
        item.score = calculate_score(
            item,
            query,
            engine_name,
            position,
            avg_title_length,
            avg_content_length,
            &weights,
            &bm25_params,
        );
    }
}

/// 评分并排序
pub fn score_and_sort_results(
    items: &mut [SearchResultItem],
    query: &SearchQuery,
    engine_name: &str,
    weights: Option<ScoringWeights>,
) {
    score_results(items, query, engine_name, weights, None);
    
    // 按分数降序排序
    items.sort_by(|a, b| {
        b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal)
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tokenize() {
        let tokens = tokenize("Hello, World! This is a test.");
        assert_eq!(tokens, vec!["hello", "world", "this", "is", "a", "test"]);
    }

    #[test]
    fn test_bm25() {
        let params = BM25Params::default();
        
        // 文档包含查询词
        let score1 = bm25_score("rust programming language", "rust", 3.0, &params);
        assert!(score1 > 0.0);
        
        // 文档不包含查询词
        let score2 = bm25_score("python programming", "rust", 3.0, &params);
        assert_eq!(score2, 0.0);
        
        // 多次出现应该分数更高
        let score3 = bm25_score("rust rust rust", "rust", 3.0, &params);
        assert!(score3 > score1);
    }

    #[test]
    fn test_exact_match() {
        assert_eq!(exact_match_bonus("rust programming", "rust programming"), 1.0);
        assert_eq!(exact_match_bonus("rust programming language", "rust"), 0.8);  // starts with "rust"
        assert_eq!(exact_match_bonus("python", "rust"), 0.0);
    }

    #[test]
    fn test_url_relevance() {
        let score = url_relevance("https://www.rust-lang.org/", "rust");
        assert!(score > 0.0);
        
        let score2 = url_relevance("https://www.python.org/", "rust");
        assert_eq!(score2, 0.0);
    }

    #[test]
    fn test_position_score() {
        // 位置越靠前，分数越高
        assert!(position_score(0) > position_score(5));
        assert!(position_score(5) > position_score(10));
    }

    #[test]
    fn test_engine_authority() {
        assert_eq!(get_engine_authority("google"), 1.0);
        assert_eq!(get_engine_authority("baidu"), 0.95); // 中国模式
        assert!(get_engine_authority("unknown") < 1.0);
    }
}

// Include comprehensive scoring tests
#[cfg(test)]
#[path = "scoring_tests.rs"]
mod scoring_tests;
