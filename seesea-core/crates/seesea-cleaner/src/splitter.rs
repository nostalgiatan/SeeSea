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

//! 数据块分割模块
//! 按行分割，遇到大括号不切

use super::DataBlock;
use super::zero_copy::{merge_lines, zero_copy_lines};

/// 数据块分割器
pub struct DataBlockSplitter {
    /// 每个块的最大行数
    max_lines_per_block: usize,
}

impl DataBlockSplitter {
    /// 创建新的分割器
    pub fn new(max_lines_per_block: usize) -> Self {
        Self {
            max_lines_per_block,
        }
    }

    /// 分割文本为数据块，使用零拷贝技术
    pub fn split(&self, text: &str) -> Vec<DataBlock> {
        let mut blocks = Vec::new();
        let lines = zero_copy_lines(text);

        if lines.is_empty() {
            return blocks;
        }

        let mut current_block_start = 0;
        let mut current_block_lines = 0;
        let mut brace_stack = 0;

        for (i, line) in lines.iter().enumerate() {
            // 处理大括号，更新括号栈
            brace_stack += line.chars().filter(|&c| c == '{').count();
            brace_stack -= line.chars().filter(|&c| c == '}').count();

            // 确保括号栈不为负数
            brace_stack = brace_stack.max(0);

            current_block_lines += 1;

            // 块分割条件：
            // 1. 括号栈为空
            // 2. 当前块行数达到最大值
            // 3. 到达文件末尾
            let should_split = (brace_stack == 0
                && current_block_lines >= self.max_lines_per_block)
                || (i == lines.len() - 1);

            if should_split {
                // 使用零拷贝合并行，创建数据块
                let content = merge_lines(&lines[current_block_start..=i]);
                let block = DataBlock::new(content, current_block_start, i + 1);

                blocks.push(block);

                // 重置当前块
                current_block_start = i + 1;
                current_block_lines = 0;
            }
        }

        blocks
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_splitter() {
        let splitter = DataBlockSplitter::new(3);
        let text = "Line 1\nLine 2\n{\n  key: value\n  another_key: another_value\n}\nLine 6\nLine 7\nLine 8\nLine 9";

        let blocks = splitter.split(text);

        // 预期：
        // 块1: Line 1, Line 2, {, key: value, another_key: another_value, }
        // 块2: Line 6, Line 7, Line 8
        // 块3: Line 9
        assert_eq!(blocks.len(), 3);
        assert!(blocks[0].content.contains("Line 1"));
        assert!(blocks[0].content.contains("Line 2"));
        assert!(blocks[0].content.contains("{"));
        assert!(blocks[0].content.contains("key: value"));
        assert!(blocks[0].content.contains("another_key: another_value"));
        assert!(blocks[0].content.contains("}"));

        assert!(blocks[1].content.contains("Line 6"));
        assert!(blocks[1].content.contains("Line 7"));
        assert!(blocks[1].content.contains("Line 8"));

        assert!(blocks[2].content.contains("Line 9"));
    }
}
