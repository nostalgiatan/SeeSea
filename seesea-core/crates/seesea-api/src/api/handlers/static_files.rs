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

use pyo3::types::PyAnyMethods;

/// 缓存的包根目录路径
static PACKAGE_ROOT: OnceLock<PathBuf> = OnceLock::new();

/// 获取 Python 模块的 __file__ 路径
///
/// 通过 PyO3 的 Python API 直接获取 seesea_core 模块的路径
#[allow(deprecated)]
fn get_python_module_path() -> Option<PathBuf> {
    pyo3::Python::with_gil(|py| {
        // 导入 seesea_core 模块
        let module = py.import("seesea_core").ok()?;
        // 获取 __file__ 属性
        let file_path = module.getattr(pyo3::intern!(py, "__file__")).ok()?;
        // 转换为字符串
        let path_str = file_path.extract::<String>().ok()?;
        Some(PathBuf::from(path_str))
    })
}

/// 获取 seesea-core 包的根目录
///
/// 对于 maturin 构建的 wheel 包，结构如下：
/// ```
/// site-packages/
/// ├── seesea_core/          # Python 模块 (.pyd/.so 文件所在位置)
/// ├── static/               # 静态文件目录（与 seesea_core 同级）
/// │   └── html/
/// └── rss/                  # RSS 模板目录（与 seesea_core 同级）
///     └── template/
/// ```
///
/// 查找顺序：
/// 1. 环境变量 SEESEA_ROOT（最高优先级，用于自定义部署）
/// 2. Python 模块的 __file__ 路径（通过 PyO3 直接获取）
/// 3. 环境变量 SEESEA_MODULE_PATH（Python 传递的模块路径）
/// 4. 当前可执行文件的目录
/// 5. 当前工作目录（开发时使用）
fn get_package_root() -> &'static PathBuf {
    PACKAGE_ROOT.get_or_init(|| {
        // 1. 检查环境变量
        if let Ok(root_path) = std::env::var("SEESEA_ROOT") {
            let path = PathBuf::from(&root_path);
            if path.exists() {
                tracing::info!("Using SEESEA_ROOT: {}", path.display());
                return path;
            }
        }

        // 2. 通过 PyO3 获取 Python 模块路径
        if let Some(module_path) = get_python_module_path() {
            tracing::info!("Found Python module path: {}", module_path.display());

            // site-packages/seesea_core/__init__.pyd -> site-packages
            if let Some(parent) = module_path.parent() {
                let site_packages = if parent.ends_with("seesea_core") {
                    parent.parent()
                } else {
                    Some(parent)
                };

                if let Some(sp) = site_packages
                    && (sp.join("seesea-static").exists() || sp.join("rss").exists())
                {
                    tracing::info!("Using module path site-packages: {}", sp.display());
                    return sp.to_path_buf();
                }
            }
        }

        // 3. 检查环境变量 SEESEA_MODULE_PATH（兼容旧的传递方式）
        if let Ok(module_path) = std::env::var("SEESEA_MODULE_PATH") {
            let path = PathBuf::from(&module_path);
            if let Some(parent) = path.parent() {
                let site_packages = if parent.ends_with("seesea_core") {
                    parent.parent()
                } else {
                    Some(parent)
                };

                if let Some(sp) = site_packages
                    && (sp.join("seesea-static").exists() || sp.join("rss").exists())
                {
                    tracing::info!("Using module path site-packages: {}", sp.display());
                    return sp.to_path_buf();
                }
            }
        }

        // 4. 尝试通过当前可执行文件路径推断
        if let Ok(exe_path) = std::env::current_exe()
            && let Some(exe_dir) = exe_path.parent()
        {
            // 检查 exe_dir/lib/pythonX.Y/site-packages（Unix Python）
            if let Some(parent) = exe_dir.parent() {
                for python_ver in &[
                    "python3.10",
                    "python3.11",
                    "python3.12",
                    "python3.13",
                    "python3.14",
                ] {
                    let site_packages = parent.join("lib").join(python_ver).join("site-packages");
                    if site_packages.join("seesea_core").exists() {
                        tracing::info!(
                            "Found seesea_core in Python site-packages: {}",
                            site_packages.display()
                        );
                        return site_packages;
                    }
                }

                // 检查 exe_dir/Lib/site-packages（Windows Python）
                let site_packages = exe_dir.join("Lib").join("site-packages");
                if site_packages.join("seesea_core").exists() {
                    tracing::info!(
                        "Found seesea_core in Python site-packages: {}",
                        site_packages.display()
                    );
                    return site_packages;
                }

                // 检查 exe_dir 本身
                if exe_dir.join("seesea-static").exists() {
                    tracing::info!(
                        "Found seesea-static in exe directory: {}",
                        exe_dir.display()
                    );
                    return exe_dir.to_path_buf();
                }

                // 检查 exe_dir 父目录
                if parent.join("seesea-static").exists() {
                    tracing::info!(
                        "Found seesea-static in parent directory: {}",
                        parent.display()
                    );
                    return parent.to_path_buf();
                }
            }
        }

        // 5. 检查虚拟环境
        if let Ok(current_dir) = std::env::current_dir() {
            for venv_name in &[".venv", "venv", ".env", "env"] {
                #[cfg(target_os = "windows")]
                let site_packages = current_dir
                    .join(venv_name)
                    .join("Lib")
                    .join("site-packages");
                #[cfg(not(target_os = "windows"))]
                let site_packages = {
                    let mut found = None;
                    for ver in &[
                        "python3.10",
                        "python3.11",
                        "python3.12",
                        "python3.13",
                        "python3.14",
                    ] {
                        let path = current_dir
                            .join(venv_name)
                            .join("lib")
                            .join(ver)
                            .join("site-packages");
                        if path.exists() {
                            found = Some(path);
                            break;
                        }
                    }
                    found.unwrap_or_else(|| {
                        current_dir
                            .join(venv_name)
                            .join("lib")
                            .join("python3.12")
                            .join("site-packages")
                    })
                };

                if site_packages.join("seesea_core").exists() {
                    tracing::info!(
                        "Found seesea_core in venv site-packages: {}",
                        site_packages.display()
                    );
                    return site_packages;
                }
            }

            // 检查当前目录
            if current_dir.join("seesea-static").exists() {
                tracing::info!(
                    "Found seesea-static in current directory: {}",
                    current_dir.display()
                );
                return current_dir;
            }
        }

        // 6. 回退到当前工作目录
        let fallback = PathBuf::from(".");
        tracing::warn!(
            "Could not find seesea_core package root, using current directory: {}",
            fallback.display()
        );
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
