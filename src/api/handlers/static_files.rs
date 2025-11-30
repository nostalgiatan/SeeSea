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
