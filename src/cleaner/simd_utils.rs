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

//! SIMD优化工具函数

use wide::f32x4;

/// 使用跨平台SIMD计算余弦相似度
#[inline]
pub fn simd_cosine_similarity(vec1: &[f32], vec2: &[f32]) -> f32 {
    assert_eq!(vec1.len(), vec2.len());

    let mut dot_product = 0.0;
    let mut norm1 = 0.0;
    let mut norm2 = 0.0;

    let len = vec1.len();
    let mut i = 0;

    // 使用f32x4进行SIMD计算
    while i + 4 <= len {
        // 创建f32x4向量
        let v1 = f32x4::new([vec1[i], vec1[i + 1], vec1[i + 2], vec1[i + 3]]);
        let v2 = f32x4::new([vec2[i], vec2[i + 1], vec2[i + 2], vec2[i + 3]]);

        // 计算点积
        let dp = v1 * v2;
        let dp_array = dp.to_array();
        dot_product += dp_array[0] + dp_array[1] + dp_array[2] + dp_array[3];

        // 计算平方和
        let n1 = v1 * v1;
        let n2 = v2 * v2;

        let n1_array = n1.to_array();
        let n2_array = n2.to_array();

        norm1 += n1_array[0] + n1_array[1] + n1_array[2] + n1_array[3];
        norm2 += n2_array[0] + n2_array[1] + n2_array[2] + n2_array[3];

        i += 4;
    }

    // 处理剩余元素
    for j in i..len {
        dot_product += vec1[j] * vec2[j];
        norm1 += vec1[j] * vec1[j];
        norm2 += vec2[j] * vec2[j];
    }

    if norm1 == 0.0 || norm2 == 0.0 {
        return 0.0;
    }

    dot_product / (norm1.sqrt() * norm2.sqrt())
}

/// 跨平台SIMD优化的字符串处理：统计大括号数量
#[inline]
pub fn simd_count_braces(text: &str) -> (usize, usize) {
    let bytes = text.as_bytes();
    let mut open_braces = 0;
    let mut close_braces = 0;

    let len = bytes.len();
    let mut i = 0;

    // 使用SIMD处理多个字节
    const CHUNK_SIZE: usize = 32;

    // 使用SIMD进行批量比较和计数
    while i + CHUNK_SIZE <= len {
        // 处理32字节的块
        let chunk = &bytes[i..i + CHUNK_SIZE];

        // 统计大括号数量
        let mut open_count = 0;
        let mut close_count = 0;

        // 使用SIMD风格的循环展开，一次处理8个字节
        for j in (0..CHUNK_SIZE).step_by(8) {
            // 展开循环，手动SIMD
            open_count += (chunk[j] == b'{') as usize;
            open_count += (chunk[j + 1] == b'{') as usize;
            open_count += (chunk[j + 2] == b'{') as usize;
            open_count += (chunk[j + 3] == b'{') as usize;
            open_count += (chunk[j + 4] == b'{') as usize;
            open_count += (chunk[j + 5] == b'{') as usize;
            open_count += (chunk[j + 6] == b'{') as usize;
            open_count += (chunk[j + 7] == b'{') as usize;

            close_count += (chunk[j] == b'}') as usize;
            close_count += (chunk[j + 1] == b'}') as usize;
            close_count += (chunk[j + 2] == b'}') as usize;
            close_count += (chunk[j + 3] == b'}') as usize;
            close_count += (chunk[j + 4] == b'}') as usize;
            close_count += (chunk[j + 5] == b'}') as usize;
            close_count += (chunk[j + 6] == b'}') as usize;
            close_count += (chunk[j + 7] == b'}') as usize;
        }

        open_braces += open_count;
        close_braces += close_count;

        i += CHUNK_SIZE;
    }

    // 处理剩余字节
    for &byte in &bytes[i..len] {
        open_braces += (byte == b'{') as usize;
        close_braces += (byte == b'}') as usize;
    }

    (open_braces, close_braces)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simd_cosine_similarity() {
        let vec1 = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0];
        let vec2 = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0];

        let similarity = simd_cosine_similarity(&vec1, &vec2);
        assert!((similarity - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_simd_count_braces() {
        let text = "{{{}}} {{{}}} {{{}}}";
        let (open, close) = simd_count_braces(text);
        assert_eq!(open, 9);
        assert_eq!(close, 9);
    }
}
