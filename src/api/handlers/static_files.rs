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

//! 静态文件处理器
//!
//! 提供首页和静态资源服务

use axum::{
    http::StatusCode,
    response::{Html, IntoResponse},
};

/// 嵌入的 HTML 首页内容
const INDEX_HTML: &str = include_str!("../../../static/html/index.html");

/// 处理首页请求
pub async fn handle_index() -> impl IntoResponse {
    Html(INDEX_HTML)
}

/// 处理 favicon 请求（返回空图标避免 404）
pub async fn handle_favicon() -> impl IntoResponse {
    // 返回一个简单的海浪 emoji 作为 SVG favicon
    let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100"><text y=".9em" font-size="90">🌊</text></svg>"#;
    (
        StatusCode::OK,
        [("content-type", "image/svg+xml")],
        svg.to_string(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_index_html_not_empty() {
        assert!(!INDEX_HTML.is_empty());
        assert!(INDEX_HTML.contains("SeeSea"));
    }
}
