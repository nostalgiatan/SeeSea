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

use axum::{extract::State, http::StatusCode, response::IntoResponse};
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;

use crate::api::on::ApiState;

/// 获取静态文件根目录
///
/// 优先使用二进制文件所在目录的 static 目录，如果不存在则使用当前工作目录的 static 目录
fn get_static_root() -> PathBuf {
    // 尝试获取二进制文件所在目录
    if let Ok(exe_path) = std::env::current_exe()
        && let Some(exe_dir) = exe_path.parent()
    {
        // 尝试二进制文件同级目录
        let static_path = exe_dir.join("static");
        if static_path.exists() {
            return static_path;
        }
        // 尝试父目录
        if let Some(parent_dir) = exe_dir.parent() {
            let parent_static = parent_dir.join("static");
            if parent_static.exists() {
                return parent_static;
            }
        }
    }

    // 回退到当前工作目录
    PathBuf::from("static")
}

/// 处理首页请求
pub async fn handle_index(State(state): State<ApiState>) -> impl IntoResponse {
    // 获取 index.html 文件路径
    let static_root = get_static_root();
    let index_path = static_root.join("html/index.html");
    let mut content = match File::open(index_path) {
        Ok(mut file) => {
            let mut content = String::new();
            if file.read_to_string(&mut content).is_err() {
                // 如果读取失败，使用内嵌的默认内容
                include_str!("../../../static/html/index.html").to_string()
            } else {
                content
            }
        }
        Err(_) => {
            // 如果文件不存在，使用内嵌的默认内容
            include_str!("../../../static/html/index.html").to_string()
        }
    };

    // 从配置中获取前端 API 地址
    // 如果为空，表示使用同源（前端会自动使用 window.location.origin）
    // 如果有值，则注入该 URL（用于 nginx 反向代理等场景）
    let api_base_url = &state.frontend_api_url;

    // 在 HTML 中注入配置脚本，使前端可以访问 API 地址
    let config_script = format!(
        r#"<script>
window.__SEESEA_CONFIG__ = {{
  API_BASE_URL: '{}'
}};
</script>"#,
        api_base_url
    );

    // 在 </head> 之前插入配置脚本
    content = content.replace("</head>", &format!("{}\n</head>", config_script));

    // 返回 Html 响应
    axum::response::Html(content)
}

/// 处理 favicon 请求
pub async fn handle_favicon() -> impl IntoResponse {
    // 获取静态文件根目录
    let static_root = get_static_root();

    // 尝试读取 ICO 文件，按优先级顺序
    let favicon_paths = [
        static_root.join("image/favicon.ico"),
        static_root.join("html/favicon.ico"),
        PathBuf::from("server/static/favicon.ico"),
    ];

    for path in &favicon_paths {
        if let Ok(content) = std::fs::read(path) {
            return (
                StatusCode::OK,
                [
                    ("content-type", "image/x-icon"),
                    ("cache-control", "public, max-age=86400"),
                ],
                content,
            )
                .into_response();
        }
    }

    // 如果找不到 ICO 文件，返回一个简单的海浪 emoji SVG
    let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100"><text y=".9em" font-size="90">🌊</text></svg>"#;
    (
        StatusCode::OK,
        [
            ("content-type", "image/svg+xml"),
            ("cache-control", "public, max-age=86400"),
        ],
        svg.as_bytes().to_vec(),
    )
        .into_response()
}

/// 获取静态文件目录路径（供外部使用）
pub fn get_static_html_path() -> PathBuf {
    get_static_root().join("html")
}

/// 获取静态文件 _app 目录路径（供外部使用）
pub fn get_static_app_path() -> PathBuf {
    get_static_root().join("html/_app")
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_index_html_contains_seesea() {
        assert!(include_str!("../../../static/html/index.html").contains("SeeSea"));
    }
}
