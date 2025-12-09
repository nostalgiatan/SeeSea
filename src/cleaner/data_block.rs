// Copyright (C) 2024 SeeSea Authors
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as
// published by the Free Software Foundation, either version 3 of the
// License, or (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

//! 数据块结构定义

use std::sync::Arc;

/// 地图项，存储文本和链接
#[derive(Debug, Clone)]
pub struct MapItem {
    /// 文本
    pub text: Arc<str>,
    /// 链接
    pub url: Arc<str>,
}

/// 额外信息项，存储键和值
#[derive(Debug, Clone)]
pub struct ExtraInfoItem {
    /// 键
    pub key: Arc<str>,
    /// 值
    pub value: Arc<str>,
}

/// 数据块结构
#[derive(Debug, Clone)]
pub struct DataBlock {
    /// 数据块内容
    pub content: Arc<str>,
    /// 起始行号
    pub start_line: usize,
    /// 结束行号
    pub end_line: usize,
    /// 标题相关性得分
    pub title_relevance: f32,
    /// 块内连贯性得分
    pub coherence: f32,
    /// 最终得分
    pub score: f32,
    /// 链接列表
    pub links: Vec<Arc<str>>,
    /// 图片列表
    pub images: Vec<Arc<str>>,
    /// 提取的键值对
    pub extracted_kv: std::collections::HashMap<Arc<str>, Arc<str>>,
    /// 是否有效
    pub is_valid: bool,
    /// 标题向量
    pub title_vector: Option<Vec<f32>>,
    /// 内容向量
    pub content_vector: Option<Vec<f32>>,
    /// 关键词相似度
    pub keyword_similarity: f32,
    /// 地图，存储文本和链接
    pub map: Vec<MapItem>,
    /// 额外信息
    pub extra_info: Vec<ExtraInfoItem>,
}

impl DataBlock {
    /// 创建新的数据块
    pub fn new(content: impl Into<Arc<str>>, start_line: usize, end_line: usize) -> Self {
        Self {
            content: content.into(),
            start_line,
            end_line,
            title_relevance: 0.0,
            coherence: 0.0,
            score: 0.0,
            links: Vec::new(),
            images: Vec::new(),
            extracted_kv: std::collections::HashMap::new(),
            is_valid: true,
            title_vector: None,
            content_vector: None,
            keyword_similarity: 0.0,
            map: Vec::new(),
            extra_info: Vec::new(),
        }
    }

    /// 设置标题向量
    pub fn set_title_vector(&mut self, vector: Vec<f32>) {
        self.title_vector = Some(vector);
    }

    /// 设置内容向量
    pub fn set_content_vector(&mut self, vector: Vec<f32>) {
        self.content_vector = Some(vector);
    }

    /// 设置关键词相似度
    pub fn set_keyword_similarity(&mut self, similarity: f32) {
        self.keyword_similarity = similarity;
    }

    /// 获取向量（如果有）
    pub fn get_vector(&self, title_weight: f32) -> Option<Vec<f32>> {
        if let (Some(title_vec), Some(content_vec)) = (&self.title_vector, &self.content_vector) {
            assert_eq!(title_vec.len(), content_vec.len());

            let mut vector = Vec::with_capacity(title_vec.len());
            for i in 0..title_vec.len() {
                vector.push(title_weight * title_vec[i] + (1.0 - title_weight) * content_vec[i]);
            }

            // 归一化向量
            let norm = vector.iter().map(|&x| x * x).sum::<f32>().sqrt();
            if norm > 0.0 {
                for value in &mut vector {
                    *value /= norm;
                }
            }

            Some(vector)
        } else {
            None
        }
    }

    /// 更新最终得分
    pub fn update_score(&mut self) {
        // 标题相关性权重70%，块内连贯性权重30%
        self.score = self.title_relevance * 0.7 + self.coherence * 0.3;
    }

    /// 添加链接
    pub fn add_link(&mut self, link: impl Into<Arc<str>>) {
        self.links.push(link.into());
    }

    /// 添加图片
    pub fn add_image(&mut self, image: impl Into<Arc<str>>) {
        self.images.push(image.into());
    }

    /// 添加键值对
    pub fn add_kv(&mut self, key: impl Into<Arc<str>>, value: impl Into<Arc<str>>) {
        self.extracted_kv.insert(key.into(), value.into());
    }

    /// 获取标题向量
    pub fn title_vector(&self) -> Option<&Vec<f32>> {
        self.title_vector.as_ref()
    }

    /// 获取内容向量
    pub fn content_vector(&self) -> Option<&Vec<f32>> {
        self.content_vector.as_ref()
    }
}
