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

//! 布隆过滤器实现
//!
//! 用于防止缓存穿透的概率型数据结构

use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::hash::Hasher;
use std::marker::PhantomData;

/// 布隆过滤器配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BloomFilterConfig {
    /// 预计元素数量
    pub expected_elements: u64,
    /// 期望的误判率
    pub false_positive_rate: f64,
}

impl Default for BloomFilterConfig {
    fn default() -> Self {
        Self {
            expected_elements: 1_000_000,
            false_positive_rate: 0.01, // 1% 误判率
        }
    }
}

/// 布隆过滤器
///
/// 用于高效判断一个元素是否可能存在于集合中
/// - 空间效率高，使用位数组和多个哈希函数
/// - 查找速度快，时间复杂度为O(k)，k为哈希函数数量
/// - 存在一定的误判率（假阳性），但不会漏判（假阴性）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BloomFilter<T> {
    /// 位数组
    bits: Vec<bool>,
    /// 哈希函数数量
    num_hashes: usize,
    /// 预计元素数量
    expected_elements: u64,
    /// 实际元素数量
    count: u64,
    /// 元素类型
    _marker: PhantomData<T>,
}

impl<T> BloomFilter<T> {
    /// 创建新的布隆过滤器
    ///
    /// # 参数
    ///
    /// * `config` - 布隆过滤器配置
    pub fn new(config: BloomFilterConfig) -> Self {
        let expected_elements = config.expected_elements;
        let false_positive_rate = config.false_positive_rate;

        // 计算最优位数组大小和哈希函数数量
        let (bits, num_hashes) = Self::calculate_parameters(expected_elements, false_positive_rate);

        Self {
            bits: vec![false; bits],
            num_hashes,
            expected_elements,
            count: 0,
            _marker: PhantomData,
        }
    }

    /// 计算布隆过滤器的最优参数
    ///
    /// # 参数
    ///
    /// * `expected_elements` - 预计元素数量
    /// * `false_positive_rate` - 期望的误判率
    ///
    /// # 返回值
    ///
    /// 返回 (位数组大小, 哈希函数数量)
    fn calculate_parameters(expected_elements: u64, false_positive_rate: f64) -> (usize, usize) {
        // 计算最优位数组大小: m = -n * ln(p) / (ln(2)^2)
        let m = (-(expected_elements as f64) * false_positive_rate.ln() / (2.0_f64.ln().powi(2)))
            .ceil() as usize;

        // 计算最优哈希函数数量: k = (m / n) * ln(2)
        let k = ((m as f64 / expected_elements as f64) * 2.0_f64.ln()).round() as usize;

        (m, k)
    }

    /// 计算元素的哈希值
    ///
    /// # 参数
    ///
    /// * `item` - 要计算哈希的元素
    ///
    /// # 返回值
    ///
    /// 返回哈希值
    fn hash(&self, item: &T) -> u64
    where
        T: std::hash::Hash,
    {
        let mut hasher = DefaultHasher::new();
        item.hash(&mut hasher);
        hasher.finish()
    }

    /// 获取元素在位数组中的多个位置
    ///
    /// # 参数
    ///
    /// * `item` - 要计算位置的元素
    ///
    /// # 返回值
    ///
    /// 返回多个哈希位置
    fn get_positions(&self, item: &T) -> Vec<usize>
    where
        T: std::hash::Hash,
    {
        let mut positions = Vec::with_capacity(self.num_hashes);
        let hash = self.hash(item);

        // 使用双重哈希技术生成多个哈希值
        for i in 0..self.num_hashes {
            let position = (hash
                .wrapping_add((i as u64).wrapping_mul(hash.wrapping_add(i as u64))))
                % self.bits.len() as u64;
            positions.push(position as usize);
        }

        positions
    }

    /// 向布隆过滤器中添加元素
    ///
    /// # 参数
    ///
    /// * `item` - 要添加的元素
    pub fn add(&mut self, item: &T)
    where
        T: std::hash::Hash,
    {
        let positions = self.get_positions(item);

        // 设置对应位置的位为true
        for &pos in &positions {
            self.bits[pos] = true;
        }

        self.count += 1;
    }

    /// 检查元素是否可能存在于布隆过滤器中
    ///
    /// # 参数
    ///
    /// * `item` - 要检查的元素
    ///
    /// # 返回值
    ///
    /// 返回 true 表示元素可能存在，false 表示元素一定不存在
    pub fn contains(&self, item: &T) -> bool
    where
        T: std::hash::Hash,
    {
        let positions = self.get_positions(item);

        // 检查所有位置是否都为true
        positions.iter().all(|&pos| self.bits[pos])
    }

    /// 获取布隆过滤器的实际元素数量
    pub fn count(&self) -> u64 {
        self.count
    }

    /// 获取布隆过滤器的预计元素数量
    pub fn expected_elements(&self) -> u64 {
        self.expected_elements
    }

    /// 获取布隆过滤器的当前误判率
    pub fn current_false_positive_rate(&self) -> f64 {
        // 当前误判率: (1 - e^(-kn/m))^k
        let k = self.num_hashes as f64;
        let n = self.count as f64;
        let m = self.bits.len() as f64;

        (1.0 - (-k * n / m).exp()).powf(k)
    }

    /// 清空布隆过滤器
    pub fn clear(&mut self) {
        self.bits.fill(false);
        self.count = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bloom_filter_basic() {
        let config = BloomFilterConfig {
            expected_elements: 1000,
            false_positive_rate: 0.01,
        };

        let mut filter = BloomFilter::new(config);

        // 添加元素
        filter.add(&"test1");
        filter.add(&"test2");
        filter.add(&"test3");

        // 检查存在的元素
        assert!(filter.contains(&"test1"));
        assert!(filter.contains(&"test2"));
        assert!(filter.contains(&"test3"));

        // 检查不存在的元素
        // 注意：由于误判率的存在，这里可能会返回true，但概率很低
        let not_exists = filter.contains(&"not_exists");
        println!("False positive: {not_exists}");

        // 检查计数
        assert_eq!(filter.count(), 3);

        // 检查误判率
        let fpr = filter.current_false_positive_rate();
        assert!(fpr <= 0.01);
    }

    #[test]
    fn test_bloom_filter_clear() {
        let config = BloomFilterConfig {
            expected_elements: 1000,
            false_positive_rate: 0.01,
        };

        let mut filter = BloomFilter::new(config);

        // 添加元素
        filter.add(&"test1");
        filter.add(&"test2");

        // 清空过滤器
        filter.clear();

        // 检查元素是否被清空
        // 注意：由于布隆过滤器的特性，清空后可能仍然会返回true
        // 但计数应该为0
        assert_eq!(filter.count(), 0);
    }
}
