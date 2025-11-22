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

//! 语义相似度模块
//!
//! 提供基于向量的语义相似度计算功能

use std::collections::HashMap;
use serde::{Deserialize, Serialize};

/// 简单的TF-IDF向量化器
#[derive(Debug, Clone)]
pub struct SimpleVectorizer {
    /// 词汇表
    #[allow(dead_code)]
    vocabulary: HashMap<String, usize>,
    /// IDF权重
    idf_weights: HashMap<String, f64>,
}

impl SimpleVectorizer {
    /// 创建新的向量化器
    pub fn new() -> Self {
        Self {
            vocabulary: HashMap::new(),
            idf_weights: HashMap::new(),
        }
    }

    /// 对文本进行分词（简单空格分割）
    fn tokenize(&self, text: &str) -> Vec<String> {
        text.to_lowercase()
            .split_whitespace()
            .map(|s| s.trim_matches(|c: char| !c.is_alphanumeric()))
            .filter(|s| !s.is_empty() && s.len() > 1)
            .map(|s| s.to_string())
            .collect()
    }

    /// 将文本转换为向量
    pub fn vectorize(&self, text: &str) -> Vec<f64> {
        let tokens = self.tokenize(text);
        let mut term_freq: HashMap<String, usize> = HashMap::new();
        
        // 计算词频
        for token in &tokens {
            *term_freq.entry(token.clone()).or_insert(0) += 1;
        }

        // 创建固定大小的向量
        let mut vector = vec![0.0; 100]; // 使用简单的hash映射到100维
        
        for (term, freq) in term_freq {
            let hash = self.hash_term(&term);
            let index = hash % 100;
            let tf = freq as f64 / tokens.len() as f64;
            let idf = self.idf_weights.get(&term).copied().unwrap_or(1.0);
            vector[index] += tf * idf;
        }

        // 归一化
        self.normalize(&mut vector);
        vector
    }

    /// 简单hash函数
    fn hash_term(&self, term: &str) -> usize {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        term.hash(&mut hasher);
        hasher.finish() as usize
    }

    /// 向量归一化
    fn normalize(&self, vector: &mut Vec<f64>) {
        let norm: f64 = vector.iter().map(|x| x * x).sum::<f64>().sqrt();
        if norm > 0.0 {
            for val in vector.iter_mut() {
                *val /= norm;
            }
        }
    }

    /// 计算余弦相似度
    pub fn cosine_similarity(&self, vec1: &[f64], vec2: &[f64]) -> f64 {
        if vec1.len() != vec2.len() {
            return 0.0;
        }

        let dot_product: f64 = vec1.iter().zip(vec2.iter()).map(|(a, b)| a * b).sum();
        dot_product.clamp(0.0, 1.0)
    }
}

impl Default for SimpleVectorizer {
    fn default() -> Self {
        Self::new()
    }
}

/// 查询向量和元数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryVector {
    /// 查询文本
    pub query: String,
    /// 向量表示
    pub vector: Vec<f64>,
    /// 时间戳
    pub timestamp: u64,
}

impl QueryVector {
    /// 创建新的查询向量
    pub fn new(query: String, vector: Vec<f64>) -> Self {
        use std::time::{SystemTime, UNIX_EPOCH};
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        Self {
            query,
            vector,
            timestamp,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vectorizer_creation() {
        let vectorizer = SimpleVectorizer::new();
        assert!(vectorizer.vocabulary.is_empty());
    }

    #[test]
    fn test_vectorize() {
        let vectorizer = SimpleVectorizer::new();
        let vec = vectorizer.vectorize("rust programming language");
        assert_eq!(vec.len(), 100);
    }

    #[test]
    fn test_cosine_similarity() {
        let vectorizer = SimpleVectorizer::new();
        let vec1 = vectorizer.vectorize("rust programming");
        let vec2 = vectorizer.vectorize("rust programming language");
        let vec3 = vectorizer.vectorize("python data science");
        
        let sim1 = vectorizer.cosine_similarity(&vec1, &vec2);
        let sim2 = vectorizer.cosine_similarity(&vec1, &vec3);
        
        // Similar queries should have higher similarity
        assert!(sim1 > sim2);
    }

    #[test]
    fn test_identical_queries() {
        let vectorizer = SimpleVectorizer::new();
        let vec1 = vectorizer.vectorize("test query");
        let vec2 = vectorizer.vectorize("test query");
        
        let sim = vectorizer.cosine_similarity(&vec1, &vec2);
        assert!((sim - 1.0).abs() < 0.01); // Should be very close to 1.0
    }
}
