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

//! HTML解析器模块，用于判定网页类型（SPA或HTML）

use regex::Regex;
use scraper::{Html, Selector};
use std::collections::HashMap;
use std::str::FromStr;

/// 网页类型枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HtmlPageType {
    /// 传统HTML页面，内容直接在HTML中
    Html,
    /// 单页应用（SPA），内容通过JavaScript动态加载
    Spa,
    /// 无法确定类型
    Unknown,
}

impl FromStr for HtmlPageType {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "html" => Ok(HtmlPageType::Html),
            "spa" => Ok(HtmlPageType::Spa),
            _ => Ok(HtmlPageType::Unknown),
        }
    }
}

use std::fmt;

impl fmt::Display for HtmlPageType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HtmlPageType::Html => write!(f, "html"),
            HtmlPageType::Spa => write!(f, "spa"),
            HtmlPageType::Unknown => write!(f, "unknown"),
        }
    }
}

/// HTML解析器，用于判定网页类型
#[derive(Debug)]
pub struct HtmlParser {
    document: Html,
    html_content: String,
}

impl HtmlParser {
    /// 创建一个新的HTML解析器
    pub fn new(html_content: String) -> Self {
        let document = Html::parse_document(&html_content);
        Self {
            document,
            html_content,
        }
    }

    /// 判定网页类型
    pub fn determine_page_type(&self) -> HtmlPageType {
        // 1. 检查页面是否为空或只有基本结构
        if self.is_empty_page() {
            return HtmlPageType::Spa;
        }

        // 2. 检查是否包含SPA框架的特征
        if self.has_spa_framework_features() {
            return HtmlPageType::Spa;
        }

        // 3. 检查JavaScript与HTML内容的比例
        if self.js_to_html_ratio() > 0.8 {
            return HtmlPageType::Spa;
        }

        // 4. 检查是否包含动态加载的特征
        if self.has_dynamic_loading_features() {
            return HtmlPageType::Spa;
        }

        // 5. 检查页面主体内容是否丰富
        if self.is_content_rich() {
            return HtmlPageType::Html;
        }

        // 6. 兜底条件：如果不包含SPA特征，且不是空页面，默认视为HTML页面
        HtmlPageType::Html
    }

    /// 检查页面是否为空或只有基本结构
    fn is_empty_page(&self) -> bool {
        // 检查body内容是否为空或只有少量文本
        let body_selector = Selector::parse("body").unwrap();
        if let Some(body) = self.document.select(&body_selector).next() {
            let text = body.text().collect::<String>();
            let trimmed_text = text.trim();
            if trimmed_text.len() < 100 {
                // 检查是否有script标签
                let script_selector = Selector::parse("script").unwrap();
                let script_count = self.document.select(&script_selector).count();
                // 如果body内容很少但有script标签，可能是SPA
                script_count > 0
            } else {
                false
            }
        } else {
            false
        }
    }

    /// 检查是否包含SPA框架的特征
    fn has_spa_framework_features(&self) -> bool {
        // 检查特定的框架特征
        let spa_features = [
            // React特征
            "react",
            "react-dom",
            "data-reactroot",
            "__react_content",
            // Vue特征
            "vue",
            "v-app",
            "v-cloak",
            "v-bind",
            // Angular特征
            "angular",
            "ng-app",
            "ng-controller",
            // Svelte特征
            "svelte",
            // Next.js特征
            "__next",
            // Nuxt.js特征
            "__nuxt",
            // Gatsby特征
            "gatsby",
            // 其他SPA特征
            "single-spa",
            "micro-app",
            "qiankun",
        ];

        // 检查HTML内容中是否包含这些特征
        for feature in spa_features.iter() {
            if self.html_content.to_lowercase().contains(feature) {
                return true;
            }
        }

        // 检查是否有特定的元标签或属性
        let meta_selector = Selector::parse("meta[name='generator']").unwrap();
        for meta in self.document.select(&meta_selector) {
            if let Some(content) = meta.value().attr("content") {
                let content_lower = content.to_lowercase();
                if content_lower.contains("react")
                    || content_lower.contains("vue")
                    || content_lower.contains("angular")
                {
                    return true;
                }
            }
        }

        false
    }

    /// 计算JavaScript与HTML内容的比例
    fn js_to_html_ratio(&self) -> f64 {
        // 提取所有script标签的内容
        let script_selector = Selector::parse("script").unwrap();
        let mut js_content_length = 0;

        for script in self.document.select(&script_selector) {
            let text = script.text().collect::<String>();
            js_content_length += text.len();
        }

        // 提取HTML内容（不包括script标签）
        let html_content = self.html_content.clone();
        let script_regex = Regex::new(r#"<script[^>]*>[\s\S]*?</script>"#).unwrap();
        let stripped_html = script_regex.replace_all(&html_content, "");
        let html_content_length = stripped_html.len();

        if html_content_length == 0 {
            return 1.0;
        }

        js_content_length as f64 / (js_content_length + html_content_length) as f64
    }

    /// 检查是否包含动态加载的特征
    fn has_dynamic_loading_features(&self) -> bool {
        // 检查是否包含动态加载的特征
        let dynamic_features = [
            "ajax",
            "fetch(",
            "xmlhttprequest",
            "axios",
            "fetchapi",
            "dynamic import",
            "webpackchunk",
            "lazy loading",
            "hydrate",
        ];

        for feature in dynamic_features.iter() {
            if self.html_content.to_lowercase().contains(feature) {
                return true;
            }
        }

        false
    }

    /// 检查页面主体内容是否丰富
    fn is_content_rich(&self) -> bool {
        // 检查body内容的丰富程度
        let body_selector = Selector::parse("body").unwrap();
        if let Some(body) = self.document.select(&body_selector).next() {
            // 计算文本内容长度
            let text = body.text().collect::<String>();
            let text_length = text.len();

            // 计算HTML标签数量
            let tag_selector = Selector::parse("*").unwrap();
            let tag_count = body.select(&tag_selector).count();

            // 调整条件，使其更合理
            // 如果文本长度大于200或标签数量大于20，认为是丰富内容
            text_length > 200 || tag_count > 20
        } else {
            false
        }
    }

    /// 获取页面的元信息
    pub fn get_meta_info(&self) -> HashMap<String, String> {
        let mut meta_info = HashMap::new();

        // 获取title
        let title_selector = Selector::parse("title").unwrap();
        if let Some(title) = self.document.select(&title_selector).next() {
            let title_text = title.text().collect::<String>();
            meta_info.insert("title".to_string(), title_text);
        }

        // 获取meta标签
        let meta_selector = Selector::parse("meta").unwrap();
        for meta in self.document.select(&meta_selector) {
            if let Some(name) = meta.value().attr("name") {
                if let Some(content) = meta.value().attr("content") {
                    meta_info.insert(name.to_string(), content.to_string());
                }
            } else if let Some(property) = meta.value().attr("property") {
                if let Some(content) = meta.value().attr("content") {
                    meta_info.insert(property.to_string(), content.to_string());
                }
            }
        }

        meta_info
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_html_page() {
        // 测试传统HTML页面
        let html = r#"<!DOCTYPE html>
<html>
<head>
    <title>Test Page</title>
    <meta charset="UTF-8">
</head>
<body>
    <h1>Hello World</h1>
    <p>This is a traditional HTML page with lots of content.</p>
    <div class="content">
        <p>More content here.</p>
        <ul>
            <li>Item 1</li>
            <li>Item 2</li>
            <li>Item 3</li>
        </ul>
    </div>
</body>
</html>"#;

        let parser = HtmlParser::new(html.to_string());
        let page_type = parser.determine_page_type();
        assert_eq!(page_type, HtmlPageType::Html);
    }

    #[test]
    fn test_parse_spa_page() {
        // 测试SPA页面
        let html = r#"<!DOCTYPE html>
<html>
<head>
    <title>SPA Test</title>
    <meta charset="UTF-8">
    <script src="https://unpkg.com/react@18/umd/react.production.min.js"></script>
    <script src="https://unpkg.com/react-dom@18/umd/react-dom.production.min.js"></script>
</head>
<body>
    <div id="root"></div>
    <script>
        const root = ReactDOM.createRoot(document.getElementById('root'));
        root.render(
            React.createElement('h1', null, 'Hello, SPA!')
        );
    </script>
</body>
</html>"#;

        let parser = HtmlParser::new(html.to_string());
        let page_type = parser.determine_page_type();
        assert_eq!(page_type, HtmlPageType::Spa);
    }

    #[test]
    fn test_parse_empty_page() {
        // 测试空页面
        let html = r#"<!DOCTYPE html>
<html>
<head>
    <title>Empty Page</title>
    <meta charset="UTF-8">
</head>
<body>
    <div id="app"></div>
    <script src="app.js"></script>
</body>
</html>"#;

        let parser = HtmlParser::new(html.to_string());
        let page_type = parser.determine_page_type();
        assert_eq!(page_type, HtmlPageType::Spa);
    }

    #[test]
    fn test_get_meta_info() {
        let html = r#"<!DOCTYPE html>
<html>
<head>
    <title>Meta Test</title>
    <meta charset="UTF-8">
    <meta name="description" content="This is a test page">
    <meta property="og:title" content="OG Title">
</head>
<body>
    <h1>Hello World</h1>
</body>
</html>"#;

        let parser = HtmlParser::new(html.to_string());
        let meta_info = parser.get_meta_info();

        assert_eq!(meta_info.get("title"), Some(&"Meta Test".to_string()));
        assert_eq!(
            meta_info.get("description"),
            Some(&"This is a test page".to_string())
        );
        assert_eq!(meta_info.get("og:title"), Some(&"OG Title".to_string()));
    }
}
