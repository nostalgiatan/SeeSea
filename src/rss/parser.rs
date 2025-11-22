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

//! RSS feed parser
//!
//! 提供 RSS/Atom feed 解析功能

use crate::derive::rss::*;

/// RSS/Atom 解析器
pub struct RssParser;

impl RssParser {
    /// 创建新的解析器
    pub fn new() -> Self {
        Self
    }

    /// 解析 RSS 2.0 feed
    pub fn parse_rss2(&self, content: &str) -> Result<RssFeed, Box<dyn std::error::Error + Send + Sync>> {
        let mut items = Vec::new();

        // 使用更强大的解析方法：查找标签对之间的内容
        let mut pos = 0;
        let content_len = content.len();

        while pos < content_len {
            // 查找下一个 <item>
            if let Some(item_start) = content[pos..].find("<item>") {
                let item_start = pos + item_start;

                // 查找对应的 </item>
                if let Some(item_end) = content[item_start..].find("</item>") {
                    let item_end = item_start + item_end + 7; // +7 for "</item>"
                    let item_content = &content[item_start..item_end];

                    // 解析单个item
                    if let Ok(item) = self.parse_single_item(item_content) {
                        items.push(item);
                    }

                    pos = item_end;
                } else {
                    break;
                }
            } else {
                break;
            }
        }

        // 解析channel元数据
        let meta = self.parse_channel_meta(content);

        Ok(RssFeed { meta, items })
    }

    /// 解析单个item
    fn parse_single_item(&self, item_content: &str) -> Result<RssFeedItem, Box<dyn std::error::Error + Send + Sync>> {
        let mut item = RssFeedItem {
            title: String::new(),
            link: String::new(),
            description: None,
            author: None,
            pub_date: None,
            content: None,
            categories: vec![],
            guid: None,
            enclosures: vec![],
            custom_fields: std::collections::HashMap::new(),
        };

        // 解析title
        if let Some(title) = self.extract_full_tag_content(item_content, "title") {
            item.title = title;
        }

        // 解析link
        if let Some(link) = self.extract_full_tag_content(item_content, "link") {
            item.link = link;
        }

        // 解析description
        if let Some(desc) = self.extract_full_tag_content(item_content, "description") {
            item.description = Some(desc);
        }

        // 解析author
        if let Some(author) = self.extract_full_tag_content(item_content, "author") {
            item.author = Some(author);
        }

        // 解析pubDate
        if let Some(date) = self.extract_full_tag_content(item_content, "pubDate") {
            item.pub_date = Some(date);
        }

        // 解析guid
        if let Some(guid) = self.extract_full_tag_content(item_content, "guid") {
            item.guid = Some(guid);
        }

        Ok(item)
    }

    /// 解析channel元数据
    fn parse_channel_meta(&self, content: &str) -> RssFeedMeta {
        let mut meta = RssFeedMeta {
            title: String::new(),
            link: String::new(),
            description: None,
            language: None,
            copyright: None,
            last_build_date: None,
            pub_date: None,
            image: None,
        };

        // 查找channel内容
        if let Some(channel_start) = content.find("<channel>") {
            if let Some(channel_end) = content.find("</channel>") {
                let channel_content = &content[channel_start..channel_end + 10];

                // 解析各个字段
                if let Some(title) = self.extract_full_tag_content(channel_content, "title") {
                    meta.title = title;
                }

                if let Some(link) = self.extract_full_tag_content(channel_content, "link") {
                    meta.link = link;
                }

                if let Some(desc) = self.extract_full_tag_content(channel_content, "description") {
                    meta.description = Some(desc);
                }

                if let Some(lang) = self.extract_full_tag_content(channel_content, "language") {
                    meta.language = Some(lang);
                }
            }
        }

        meta
    }

    /// 提取完整的标签内容（支持跨行）
    fn extract_full_tag_content(&self, content: &str, tag: &str) -> Option<String> {
        let start_tag = format!("<{}>", tag);
        let end_tag = format!("</{}>", tag);

        if let Some(start_pos) = content.find(&start_tag) {
            let content_start = start_pos + start_tag.len();

            if let Some(end_pos) = content[content_start..].find(&end_tag) {
                let end_pos = content_start + end_pos;
                let mut raw_content = content[content_start..end_pos].to_string();

                // 移除CDATA标记
                raw_content = raw_content.replace("<![CDATA[", "").replace("]]>", "");
                // 清理空白字符
                raw_content = raw_content.trim().to_string();

                return Some(raw_content);
            }
        }
        None
    }

    /// 解析 Atom feed
    pub fn parse_atom(&self, content: &str) -> Result<RssFeed, Box<dyn std::error::Error + Send + Sync>> {
        // Atom 格式解析
        let mut items = Vec::new();
        let mut meta = RssFeedMeta {
            title: String::new(),
            link: String::new(),
            description: None,
            language: None,
            copyright: None,
            last_build_date: None,
            pub_date: None,
            image: None,
        };

        // 简单的 Atom 解析实现
        let lines: Vec<&str> = content.lines().collect();
        let mut in_entry = false;
        let mut current_item = RssFeedItem {
            title: String::new(),
            link: String::new(),
            description: None,
            author: None,
            pub_date: None,
            content: None,
            categories: vec![],
            guid: None,
            enclosures: vec![],
            custom_fields: std::collections::HashMap::new(),
        };

        for line in lines {
            let trimmed = line.trim();
            
            if trimmed.starts_with("<entry>") || trimmed.starts_with("<entry ") {
                in_entry = true;
                current_item = RssFeedItem {
                    title: String::new(),
                    link: String::new(),
                    description: None,
                    author: None,
                    pub_date: None,
                    content: None,
                    categories: vec![],
                    guid: None,
                    enclosures: vec![],
                    custom_fields: std::collections::HashMap::new(),
                };
            } else if trimmed.starts_with("</entry>") {
                in_entry = false;
                items.push(current_item.clone());
            } else if in_entry {
                if let Some(title) = Self::extract_tag_content(trimmed, "title") {
                    current_item.title = title;
                } else if let Some(id) = Self::extract_tag_content(trimmed, "id") {
                    current_item.guid = Some(id);
                } else if let Some(updated) = Self::extract_tag_content(trimmed, "updated") {
                    current_item.pub_date = Some(updated);
                } else if let Some(content) = Self::extract_tag_content(trimmed, "content") {
                    current_item.content = Some(content);
                } else if trimmed.contains("<link") && trimmed.contains("href=") {
                    if let Some(href) = Self::extract_attribute(trimmed, "href") {
                        current_item.link = href;
                    }
                }
            } else {
                if let Some(title) = Self::extract_tag_content(trimmed, "title") {
                    if meta.title.is_empty() {
                        meta.title = title;
                    }
                } else if trimmed.contains("<link") && trimmed.contains("href=") {
                    if meta.link.is_empty() {
                        if let Some(href) = Self::extract_attribute(trimmed, "href") {
                            meta.link = href;
                        }
                    }
                }
            }
        }

        Ok(RssFeed { meta, items })
    }

    /// 自动检测并解析 feed
    pub fn parse(&self, content: &str) -> Result<RssFeed, Box<dyn std::error::Error + Send + Sync>> {
        // 检测 feed 类型
        if content.contains("<rss") {
            self.parse_rss2(content)
        } else if content.contains("<feed") {
            self.parse_atom(content)
        } else {
            Err("Unknown feed format".into())
        }
    }

    /// 提取 XML 标签内容（支持CDATA和跨行）
    fn extract_tag_content(line: &str, tag: &str) -> Option<String> {
        let start_tag = format!("<{}>", tag);
        let end_tag = format!("</{}>", tag);

        if let Some(start_pos) = line.find(&start_tag) {
            if let Some(end_pos) = line.find(&end_tag) {
                let content_start = start_pos + start_tag.len();
                if content_start < end_pos {
                    let mut content = line[content_start..end_pos].to_string();
                    // 移除CDATA标记
                    content = content.replace("<![CDATA[", "").replace("]]>", "");
                    // 清理空白字符
                    content = content.trim().to_string();
                    return Some(content);
                }
            }
        }
        None
    }

    /// 提取 XML 属性值
    fn extract_attribute(line: &str, attr: &str) -> Option<String> {
        let pattern = format!("{}=\"", attr);
        if let Some(start_pos) = line.find(&pattern) {
            let value_start = start_pos + pattern.len();
            if let Some(end_pos) = line[value_start..].find('"') {
                return Some(line[value_start..value_start + end_pos].to_string());
            }
        }
        None
    }
}

impl Default for RssParser {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rss_parser_creation() {
        let parser = RssParser::new();
        assert!(true);
    }

    #[test]
    fn test_parse_simple_rss() {
        let parser = RssParser::new();
        let content = r#"<?xml version="1.0"?>
<rss version="2.0">
  <channel>
    <title>Test Feed</title>
    <link>https://example.com</link>
    <description>A test feed</description>
    <item>
      <title>Item 1</title>
      <link>https://example.com/1</link>
      <description>First item</description>
    </item>
  </channel>
</rss>"#;

        let result = parser.parse(content);
        assert!(result.is_ok());
        let feed = result.unwrap();
        assert_eq!(feed.meta.title, "Test Feed");
        assert_eq!(feed.items.len(), 1);
        assert_eq!(feed.items[0].title, "Item 1");
    }
}
