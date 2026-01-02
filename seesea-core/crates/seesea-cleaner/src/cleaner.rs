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

//! 清洗器模块
//! 实现向量化前的预处理操作
//! 采用微并行技术、零拷贝和SIMD优化

use crate::ant_colony::AntColonyOptimization;
use crate::concurrent::ConcurrentController;
use crate::data_block::DataBlock;
use crate::md_parser::MDParser;
use crate::splitter::DataBlockSplitter;

use seesea_sys::controller::SystemController;
use std::sync::Arc;

/// 清洗器主结构
pub struct Cleaner {
    splitter: DataBlockSplitter,
    md_parser: MDParser,
    concurrent_controller: Arc<ConcurrentController>,
    ant_colony: AntColonyOptimization,
    /// 系统控制器引用，用于动态调控
    system_controller: Option<Arc<SystemController>>,
}

impl Cleaner {
    /// 创建新的清洗器实例
    pub fn new(max_lines_per_block: usize) -> Self {
        Self {
            splitter: DataBlockSplitter::new(max_lines_per_block),
            md_parser: MDParser::new(),
            concurrent_controller: Arc::new(ConcurrentController::new()),
            ant_colony: AntColonyOptimization::default(),
            system_controller: None,
        }
    }

    /// 设置系统控制器，用于动态调控
    pub fn set_system_controller(&mut self, system_controller: Arc<SystemController>) {
        self.system_controller = Some(system_controller);
    }

    /// 获取并发控制器，用于外部访问
    pub fn concurrent_controller(&self) -> &Arc<ConcurrentController> {
        &self.concurrent_controller
    }

    /// 处理文本，返回清洗后的数据块
    /// 如果old_hash与当前文本的哈希一致，则返回true表示数据未变化
    pub async fn process(&self, text: &str, old_hash: Option<u64>) -> (bool, Vec<DataBlock>) {
        // 计算当前文本的哈希值
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        text.hash(&mut hasher);
        let current_hash = hasher.finish();

        // 如果old_hash存在且与当前哈希一致，返回true表示数据未变化
        if matches!(old_hash, Some(hash) if hash == current_hash) {
            return (true, Vec::new());
        }

        // 1. 分割数据块
        let blocks = self.splitter.split(text);

        // 克隆md_parser，避免闭包捕获self导致的生命周期问题
        let md_parser = self.md_parser.clone();

        // 2. 并行处理数据块
        let mut processed_blocks = self
            .concurrent_controller
            .parallel_process(blocks, move |block| {
                let mut processed_block = block.clone();
                // 3. 解析Markdown
                md_parser.parse(&mut processed_block);
                processed_block
            })
            .await;

        // 4. 使用蚁群算法优化数据块得分
        self.ant_colony.optimize_scores(&mut processed_blocks).await;

        // 返回false表示数据已变化，以及处理后的数据块
        (false, processed_blocks)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_cleaner() {
        let cleaner = Cleaner::new(50);
        let text = "# 标题1\n这是一个测试文档。\n\n{key: value, another_key: another_value}\n\n[链接](https://example.com)\n![图片](https://example.com/image.jpg)";

        let (_, blocks) = cleaner.process(text, None).await;
        assert!(!blocks.is_empty());
        assert!(blocks[0].content.contains("测试文档"));
    }
}
