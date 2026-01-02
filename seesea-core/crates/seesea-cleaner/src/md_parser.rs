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

//! Markdown解析器

use super::DataBlock;

/// Markdown解析器
pub struct MDParser {
    /// 链接正则表达式
    link_regex: regex::Regex,
    /// 图片正则表达式
    image_regex: regex::Regex,
}

impl Clone for MDParser {
    fn clone(&self) -> Self {
        Self {
            link_regex: regex::Regex::new(r"\[(.*?)\]\((.*?)\)").unwrap(),
            image_regex: regex::Regex::new(r"!\[(.*?)\]\((.*?)\)").unwrap(),
        }
    }
}

impl Default for MDParser {
    fn default() -> Self {
        Self::new()
    }
}

impl MDParser {
    /// 创建新的解析器
    pub fn new() -> Self {
        Self {
            link_regex: regex::Regex::new(r"\[(.*?)\]\((.*?)\)").unwrap(),
            image_regex: regex::Regex::new(r"!\[(.*?)\]\((.*?)\)").unwrap(),
        }
    }

    /// 解析数据块
    pub fn parse(&self, block: &mut DataBlock) {
        let content = block.content.clone();

        // 先解析图片，避免和链接混淆
        let mut image_links = Vec::new();
        for cap in self.image_regex.captures_iter(&content) {
            if let Some(url) = cap.get(2) {
                image_links.push(url.as_str().into());
            }
        }
        block.images.extend(image_links);

        // 解析链接，注意排除图片语法
        let mut _link_count = 0;
        let mut _image_count = 0;

        for cap in self.link_regex.captures_iter(&content) {
            if let (Some(text), Some(url)) = (cap.get(1), cap.get(2)) {
                let text_str = text.as_str();
                let url_str = url.as_str();

                // 检查是否是图片语法（以!开头）
                let full_match = cap.get(0).unwrap().as_str();
                if full_match.starts_with("!") {
                    // 这是图片，跳过，因为已经处理过了
                    continue;
                }

                if text_str.trim().is_empty() {
                    // 空文本，视为图片，添加到图片列表
                    block.images.push(url_str.into());
                    _image_count += 1;
                } else {
                    // 有文本，视为链接，添加到地图
                    block.map.push(crate::data_block::MapItem {
                        text: text_str.into(),
                        url: url_str.into(),
                    });
                    // 同时添加到链接列表，保持兼容性
                    block.links.push(url_str.into());
                    _link_count += 1;
                }
            }
        }

        // 解析大括号内的数据，存储为额外信息
        // 正则表达式：匹配大括号内的键值对，格式如 {key: value, key2: value2}
        let brace_regex = regex::Regex::new(r"\{([^}]+)\}").unwrap();
        for cap in brace_regex.captures_iter(&content) {
            if let Some(inner) = cap.get(1) {
                let inner_str = inner.as_str();
                // 分割键值对
                let pairs = inner_str.split(',');
                for pair in pairs {
                    let mut parts = pair.split(':');
                    if let (Some(key), Some(value)) = (parts.next(), parts.next()) {
                        let key_str = key.trim();
                        let value_str = value.trim();
                        if !key_str.is_empty() && !value_str.is_empty() {
                            block.extra_info.push(crate::data_block::ExtraInfoItem {
                                key: key_str.into(),
                                value: value_str.into(),
                            });
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_md_parser() {
        let parser = MDParser::new();
        let mut block = DataBlock::new(
            "[链接](https://example.com) ![图片](https://example.com/image.jpg) [空链接]()",
            0,
            1,
        );

        parser.parse(&mut block);

        // 实际逻辑：
        // 1. [链接](https://example.com) -> 添加到links和map
        // 2. ![图片](https://example.com/image.jpg) -> 添加到images
        // 3. [空链接]() -> 空文本链接，添加到images
        assert_eq!(block.links.len(), 1);
        assert_eq!(block.images.len(), 2);
        assert!(block.links[0].contains("example.com"));
        assert!(block.images[0].contains("example.com"));
        assert!(block.images[1].contains("example.com"));
    }
}
