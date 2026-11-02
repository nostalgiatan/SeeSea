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
use std::sync::OnceLock;

use crate::api::on::ApiState;

/// 缓存的包根目录路径
static PACKAGE_ROOT: OnceLock<PathBuf> = OnceLock::new();

/// 获取包根目录（包含 `seesea-static`、`rss` 的目录）
///
/// 查找顺序：
/// 1. 环境变量 `SEESEA_ROOT`
/// 2. 可执行文件所在目录及其上两级
/// 3. 当前工作目录
/// 4. 回退到 `.`
fn get_package_root() -> &'static PathBuf {
    PACKAGE_ROOT.get_or_init(|| {
        if let Ok(root_path) = std::env::var("SEESEA_ROOT") {
            let path = PathBuf::from(&root_path);
            if path.exists() {
                tracing::info!("Using SEESEA_ROOT: {}", path.display());
                return path;
            }
        }

        let has_assets = |dir: &std::path::Path| dir.join("seesea-static").exists() || dir.join("rss").exists();

        if let Ok(exe_path) = std::env::current_exe() {
            for dir in exe_path.ancestors().skip(1).take(3) {
                if has_assets(dir) {
                    tracing::info!("Found assets next to the executable: {}", dir.display());
                    return dir.to_path_buf();
                }
            }
        }

        if let Ok(current_dir) = std::env::current_dir()
            && has_assets(&current_dir)
        {
            tracing::info!("Found assets in the current directory: {}", current_dir.display());
            return current_dir;
        }

        let fallback = PathBuf::from(".");
        tracing::warn!("Could not find the asset root, using the current directory");
        fallback
    })
}

/// 获取静态文件根目录
pub fn get_static_root() -> PathBuf {
    let root = get_package_root();
    root.join("seesea-static")
}

/// 获取 RSS 模板目录
pub fn get_rss_template_dir() -> PathBuf {
    let root = get_package_root();
    root.join("rss").join("template")
}

/// 处理首页请求
pub async fn handle_index(State(state): State<ApiState>) -> impl IntoResponse {
    // 获取 index.html 文件路径
    let static_root = get_static_root();
    let index_path = static_root.join("html/index.html");

    tracing::debug!("Looking for index.html at: {}", index_path.display());

    let mut content = match File::open(&index_path) {
        Ok(mut file) => {
            let mut content = String::new();
            if file.read_to_string(&mut content).is_err() {
                tracing::warn!("Failed to read index.html, using embedded fallback");
                include_str!("../../../../../seesea-static/html/index.html").to_string()
            } else {
                tracing::info!("Loaded index.html from: {}", index_path.display());
                content
            }
        }
        Err(e) => {
            tracing::warn!(
                "index.html not found at {}: {}, using embedded fallback",
                index_path.display(),
                e
            );
            include_str!("../../../../../seesea-static/html/index.html").to_string()
        }
    };

    // 从配置中获取前端 API 地址
    let api_base_url = &state.frontend_api_url;

    // 在 HTML 中注入配置脚本
    let config_script = format!(
        r#"<script>
window.__SEESEA_CONFIG__ = {{
  API_BASE_URL: '{}'
}};
</script>"#,
        api_base_url
    );

    content = content.replace("</head>", &format!("{}\n</head>", config_script));

    axum::response::Html(content)
}

/// 处理 favicon 请求
pub async fn handle_favicon() -> impl IntoResponse {
    let static_root = get_static_root();

    let favicon_paths = [
        static_root.join("image/favicon.ico"),
        static_root.join("html/favicon.ico"),
        PathBuf::from("server/seesea-static/favicon.ico"),
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

/// 获取静态文件 assets 目录路径（供外部使用）
pub fn get_static_assets_path() -> PathBuf {
    get_static_root().join("html/assets")
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_index_html_contains_seesea() {
        assert!(include_str!("../../../../../seesea-static/html/index.html").contains("SeeSea"));
    }
}
