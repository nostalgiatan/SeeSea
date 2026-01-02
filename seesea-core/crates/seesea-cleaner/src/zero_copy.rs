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

//! 零拷贝工具函数

use std::fs::File;
use std::sync::Arc;

use memmap2::Mmap;

/// 基于内存映射的零拷贝文件读取
pub struct ZeroCopyFileReader {
    mmap: Mmap,
}

impl ZeroCopyFileReader {
    /// 打开文件并创建内存映射
    pub fn open(path: &std::path::Path) -> std::io::Result<Self> {
        let file = File::open(path)?;
        let mmap = unsafe { Mmap::map(&file)? };

        Ok(Self { mmap })
    }

    /// 获取文件内容的字符串视图，零拷贝
    pub fn get_content(&self) -> &str {
        std::str::from_utf8(&self.mmap).expect("Invalid UTF-8 in file")
    }

    /// 零拷贝分割字符串为行
    pub fn zero_copy_lines(&self) -> Vec<Arc<str>> {
        let text = self.get_content();
        let mut lines = Vec::new();
        let bytes = text.as_bytes();
        let len = bytes.len();
        let mut start = 0;

        for i in 0..len {
            if bytes[i] == b'\n' {
                // 处理换行符
                let line = &text[start..i];
                lines.push(line.into());
                start = i + 1;
            }
        }

        // 处理最后一行
        if start < len {
            let line = &text[start..len];
            lines.push(line.into());
        }

        lines
    }
}

/// 零拷贝分割字符串为行
pub fn zero_copy_lines(text: &str) -> Vec<Arc<str>> {
    let mut lines = Vec::new();
    let bytes = text.as_bytes();
    let len = bytes.len();
    let mut start = 0;

    for i in 0..len {
        if bytes[i] == b'\n' {
            // 处理换行符
            let line = &text[start..i];
            lines.push(line.into());
            start = i + 1;
        }
    }

    // 处理最后一行
    if start < len {
        let line = &text[start..len];
        lines.push(line.into());
    }

    lines
}

/// 零拷贝分割字符串为块
pub fn zero_copy_split_blocks(text: &str, max_lines_per_block: usize) -> Vec<Arc<str>> {
    let lines = zero_copy_lines(text);
    let mut blocks = Vec::new();

    if lines.is_empty() {
        return blocks;
    }

    let mut current_block_start = 0;
    let mut current_block_lines = 0;
    let mut brace_stack = 0;

    for (i, line) in lines.iter().enumerate() {
        // 统计括号数量
        let open_braces = line.chars().filter(|&c| c == '{').count();
        let close_braces = line.chars().filter(|&c| c == '}').count();

        brace_stack += open_braces;
        brace_stack -= close_braces;
        brace_stack = brace_stack.max(0);

        current_block_lines += 1;

        // 检查是否需要分割块
        let should_split = (brace_stack == 0 && current_block_lines >= max_lines_per_block)
            || (i == lines.len() - 1);

        if should_split {
            // 合并行，创建数据块
            let block = merge_lines(&lines[current_block_start..=i]);
            blocks.push(block);

            current_block_start = i + 1;
            current_block_lines = 0;
        }
    }

    blocks
}

/// 合并行，使用零拷贝
pub fn merge_lines(lines: &[Arc<str>]) -> Arc<str> {
    if lines.is_empty() {
        return "".into();
    }

    // 计算总长度
    let total_len = lines.iter().map(|line| line.len() + 1).sum::<usize>() - 1;

    let mut buffer = String::with_capacity(total_len);

    for (i, line) in lines.iter().enumerate() {
        if i > 0 {
            buffer.push('\n');
        }
        buffer.push_str(line);
    }

    buffer.into()
}

/// 零拷贝提取大括号内容
pub fn zero_copy_extract_braces(text: &str) -> Vec<Arc<str>> {
    let mut result = Vec::new();
    let mut brace_stack = 0;
    let mut start = None;

    for (i, c) in text.char_indices() {
        if c == '{' {
            brace_stack += 1;
            if brace_stack == 1 {
                start = Some(i);
            }
        } else if c == '}' {
            brace_stack -= 1;
            if brace_stack == 0
                && let Some(s) = start
            {
                let content = &text[s..=i];
                result.push(content.into());
                start = None;
            }
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zero_copy_lines() {
        let text = "Line 1\nLine 2\nLine 3";
        let lines = zero_copy_lines(text);

        assert_eq!(lines.len(), 3);
        assert_eq!(lines[0].as_ref(), "Line 1");
        assert_eq!(lines[1].as_ref(), "Line 2");
        assert_eq!(lines[2].as_ref(), "Line 3");
    }

    #[test]
    fn test_zero_copy_split_blocks() {
        let text =
            "Line 1\nLine 2\n{\n  key: value\n  another_key: another_value\n}\nLine 6\nLine 7";
        let blocks = zero_copy_split_blocks(text, 3);

        // 实际分割结果：
        // 块1: Line 1, Line 2, {, key: value, another_key: another_value, }
        // 块2: Line 6, Line 7
        assert_eq!(blocks.len(), 2);
        assert!(blocks[0].as_ref().contains("Line 1"));
        assert!(blocks[0].as_ref().contains("key: value"));
        assert!(blocks[0].as_ref().contains("another_key: another_value"));
        assert!(blocks[1].as_ref().contains("Line 6"));
        assert!(blocks[1].as_ref().contains("Line 7"));
    }

    #[test]
    fn test_zero_copy_extract_braces() {
        let text = "before {first} middle {second {nested} content} after";
        let braces = zero_copy_extract_braces(text);

        assert_eq!(braces.len(), 2);
        assert_eq!(braces[0].as_ref(), "{first}");
        assert_eq!(braces[1].as_ref(), "{second {nested} content}");
    }

    #[test]
    fn test_zero_copy_file_reader() {
        use std::fs::File;
        use std::io::Write;

        // 创建临时文件
        let temp_dir = std::env::temp_dir();
        let temp_path = temp_dir.join("test_zero_copy.txt");

        let mut temp_file = File::create(&temp_path).unwrap();
        let content = "Line 1\nLine 2\nLine 3";
        temp_file.write_all(content.as_bytes()).unwrap();

        // 测试零拷贝文件读取
        let reader = ZeroCopyFileReader::open(&temp_path).unwrap();
        let text = reader.get_content();
        assert_eq!(text, content);

        let lines = reader.zero_copy_lines();
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[0].as_ref(), "Line 1");
        assert_eq!(lines[1].as_ref(), "Line 2");
        assert_eq!(lines[2].as_ref(), "Line 3");

        // 清理临时文件
        std::fs::remove_file(&temp_path).unwrap();
    }
}
