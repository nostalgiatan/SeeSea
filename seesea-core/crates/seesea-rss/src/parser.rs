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
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

//! RSS feed parser
//!
//! 提供 RSS/Atom feed 解析功能

use crate::types::RssResult;
use seesea_derive::rss::*;
use seesea_errors::parse as parse_errors;

/// RSS/Atom 解析器
pub struct RssParser;

impl Default for RssParser {
    fn default() -> Self {
        Self::new()
    }
}

impl RssParser {
    /// 创建新的解析器
    pub fn new() -> Self {
        Self
    }

    /// 解析 RSS 2.0 feed
    pub fn parse_rss2(&self, content: &str) -> RssResult<RssFeed> {
        // 验证输入内容
        if content.trim().is_empty() {
            return Err(parse_errors::xml_parse_error("RSS内容为空").into());
        }

        let mut items = Vec::new();

        // 使用更强大的解析方法：查找标签对之间的内容
        let mut pos = 0;
        let content_len = content.len();

        while pos < content_len {
            // 查找下一个 <item>
            let item_start_search = &content[pos..];
            if let Some(item_start_offset) = item_start_search.find("<item>") {
                let item_start = pos + item_start_offset;

                // 查找对应的 </item>
                let item_content_search = &content[item_start..];
                if let Some(item_end_offset) = item_content_search.find("</item>") {
                    let item_end = item_start + item_end_offset + 7; // +7 for "</item>"
                    let item_content = &content[item_start..item_end];

                    // 解析单个item
                    match self.parse_single_item(item_content) {
                        Ok(item) => items.push(item),
                        Err(e) => {
                            // 记录解析错误但继续处理其他项目
                            eprintln!("解析RSS项目失败: {}", e);
                        }
                    }

                    pos = item_end;
                } else {
                    return Err(parse_errors::xml_parse_error("未找到对应的</item>标签").into());
                }
            } else {
                break;
            }
        }

        // 检查是否解析到任何项目
        if items.is_empty() && content.contains("<item>") {
            return Err(parse_errors::xml_parse_error("未能解析任何RSS项目").into());
        }

        // 解析channel元数据
        let meta = self.parse_channel_meta(content);

        Ok(RssFeed { meta, items })
    }

    /// 解析单个item
    fn parse_single_item(&self, item_content: &str) -> RssResult<RssFeedItem> {
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
        } else {
            return Err(parse_errors::missing_field("title", "RSS").into());
        }

        // 解析link
        if let Some(link) = self.extract_full_tag_content(item_content, "link") {
            item.link = link;
        } else {
            return Err(parse_errors::missing_field("link", "RSS").into());
        }

        // 解析description
        item.description = self.extract_full_tag_content(item_content, "description");

        // 解析author
        item.author = self.extract_full_tag_content(item_content, "author");

        // 解析pubDate
        item.pub_date = self.extract_full_tag_content(item_content, "pubDate");

        // 解析content
        item.content = self.extract_full_tag_content(item_content, "content:encoded");

        // 解析categories
        item.categories = self.extract_all_tag_contents(item_content, "category");

        // 解析guid
        item.guid = self.extract_full_tag_content(item_content, "guid");

        // 验证必需字段
        if item.title.is_empty() {
            return Err(parse_errors::invalid_field_value("title", "", "标题不能为空").into());
        }

        Ok(item)
    }

    /// 提取完整标签内容
    fn extract_full_tag_content(&self, content: &str, tag_name: &str) -> Option<String> {
        let start_tag = format!("<{}>", tag_name);
        let end_tag = format!("</{}>", tag_name);

        if let Some(start_pos) = content.find(&start_tag) {
            let content_after_start = &content[start_pos + start_tag.len()..];
            if let Some(end_pos) = content_after_start.find(&end_tag) {
                let extracted = &content_after_start[..end_pos];
                return Some(extracted.trim().to_string());
            }
        }

        None
    }

    /// 提取所有匹配标签的内容
    fn extract_all_tag_contents(&self, content: &str, tag_name: &str) -> Vec<String> {
        let mut results = Vec::new();
        let start_tag = format!("<{}>", tag_name);
        let end_tag = format!("</{}>", tag_name);

        let mut pos = 0;
        while let Some(start_pos) = content[pos..].find(&start_tag) {
            let actual_start = pos + start_pos;
            let content_after_start = &content[actual_start + start_tag.len()..];

            if let Some(end_pos) = content_after_start.find(&end_tag) {
                let extracted = &content_after_start[..end_pos];
                results.push(extracted.trim().to_string());
                pos = actual_start + start_tag.len() + end_pos + end_tag.len();
            } else {
                break;
            }
        }

        results
    }

    /// 解析channel元数据
    fn parse_channel_meta(&self, content: &str) -> RssFeedMeta {
        RssFeedMeta {
            title: self
                .extract_full_tag_content(content, "title")
                .unwrap_or_default(),
            link: self
                .extract_full_tag_content(content, "link")
                .unwrap_or_default(),
            description: self.extract_full_tag_content(content, "description"),
            language: self.extract_full_tag_content(content, "language"),
            pub_date: self.extract_full_tag_content(content, "pubDate"),
            last_build_date: self.extract_full_tag_content(content, "lastBuildDate"),
            copyright: None, // RSS 2.0 没有直接的 copyright 字段
            image: None,     // 稍后实现图片解析
        }
    }

    /// 解析 Atom feed
    pub fn parse_atom(&self, content: &str) -> RssResult<RssFeed> {
        // 验证输入内容
        if content.trim().is_empty() {
            return Err(parse_errors::xml_parse_error("Atom内容为空").into());
        }

        // 检查是否为有效的 Atom feed
        if !content.contains("<feed") || !content.contains("xmlns=\"http://www.w3.org/2005/Atom\"")
        {
            return Err(parse_errors::xml_parse_error("不是有效的Atom格式").into());
        }

        let mut items = Vec::new();

        // 解析 entry 元素（Atom 中的项目）
        let mut pos = 0;
        while let Some(entry_start) = content[pos..].find("<entry") {
            let actual_start = pos + entry_start;
            let content_after_entry = &content[actual_start..];

            if let Some(entry_end) = content_after_entry.find("</entry>") {
                let entry_content = &content_after_entry[..entry_end + 8]; // +8 for "</entry>"

                match self.parse_single_atom_entry(entry_content) {
                    Ok(item) => items.push(item),
                    Err(e) => {
                        // 记录解析错误但继续处理其他项目
                        eprintln!("解析Atom项目失败: {}", e);
                    }
                }

                pos = actual_start + entry_end + 8;
            } else {
                break;
            }
        }

        // 解析 feed 元数据
        let meta = self.parse_atom_feed_meta(content);

        Ok(RssFeed { meta, items })
    }

    /// 解析单个 Atom entry
    fn parse_single_atom_entry(&self, entry_content: &str) -> RssResult<RssFeedItem> {
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

        // 解析 title
        if let Some(title) = self.extract_full_tag_content(entry_content, "title") {
            item.title = title;
        } else {
            return Err(parse_errors::missing_field("title", "Atom").into());
        }

        // 解析 link (Atom 使用 href 属性)
        if let Some(link) = self.extract_atom_link(entry_content) {
            item.link = link;
        } else {
            return Err(parse_errors::missing_field("link", "Atom").into());
        }

        // 解析 content
        item.content = self.extract_full_tag_content(entry_content, "content");

        // 解析 summary (Atom 的 description 等效)
        item.description = self.extract_full_tag_content(entry_content, "summary");

        // 解析 author
        item.author = self.extract_full_tag_content(entry_content, "author");

        // 解析 updated (Atom 的 pub_date 等效)
        item.pub_date = self.extract_full_tag_content(entry_content, "updated");

        // 解析 id (Atom 的 guid 等效)
        item.guid = self.extract_full_tag_content(entry_content, "id");

        // 解析 category
        item.categories = self.extract_all_tag_contents(entry_content, "category");

        // 验证必需字段
        if item.title.is_empty() {
            return Err(parse_errors::invalid_field_value("title", "", "标题不能为空").into());
        }

        Ok(item)
    }

    /// 提取 Atom link (考虑 href 属性)
    fn extract_atom_link(&self, content: &str) -> Option<String> {
        if let Some(link_start) = content.find("<link") {
            let content_after_link = &content[link_start..];
            if let Some(link_end) = content_after_link.find(">") {
                let link_tag = &content_after_link[..link_end + 1];

                // 查找 href 属性
                if let Some(href_start) = link_tag.find("href=\"") {
                    let href_content = &link_tag[href_start + 7..];
                    if let Some(href_end) = href_content.find("\"") {
                        return Some(href_content[..href_end].to_string());
                    }
                }
            }
        }
        None
    }

    /// 解析 Atom feed 元数据
    fn parse_atom_feed_meta(&self, content: &str) -> RssFeedMeta {
        RssFeedMeta {
            title: self
                .extract_full_tag_content(content, "title")
                .unwrap_or_default(),
            link: self.extract_atom_link(content).unwrap_or_default(),
            description: self.extract_full_tag_content(content, "subtitle"),
            language: None,  // Atom 没有直接的 language 字段
            copyright: None, // Atom 没有直接的 copyright 字段
            pub_date: self.extract_full_tag_content(content, "updated"),
            last_build_date: self.extract_full_tag_content(content, "updated"),
            image: None, // Atom 没有直接的 image 字段
        }
    }

    /// 自动检测并解析 RSS/Atom feed
    pub fn parse(&self, content: &str) -> RssResult<RssFeed> {
        // 验证输入内容
        if content.trim().is_empty() {
            return Err(parse_errors::xml_parse_error("RSS内容为空").into());
        }

        // 检测 feed 类型
        if content.contains("<rss") {
            // RSS 2.0
            self.parse_rss2(content)
        } else if content.contains("<feed")
            && content.contains("xmlns=\"http://www.w3.org/2005/Atom\"")
        {
            // Atom 1.0
            self.parse_atom(content)
        } else {
            // 尝试按 RSS 解析，如果不成功则尝试 Atom
            match self.parse_rss2(content) {
                Ok(feed) => Ok(feed),
                Err(_) => self.parse_atom(content),
            }
        }
    }
}
