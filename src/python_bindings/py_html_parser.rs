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

//! Python bindings for HTML parser functionality

use pyo3::prelude::*;
use pyo3::types::PyDict;

use crate::html_parser::HtmlParser;

/// 直接导出的函数：判定网页类型
///
/// 参数:
/// - html_content: HTML内容字符串
///
/// 返回:
/// - 字符串，表示网页类型（"html"、"spa"或"unknown"）
#[pyfunction]
pub fn determine_page_type(html_content: String) -> String {
    let parser = HtmlParser::new(html_content);
    parser.determine_page_type().to_string()
}

/// 直接导出的函数：获取页面元信息
///
/// 参数:
/// - html_content: HTML内容字符串
///
/// 返回:
/// - 字典，包含页面的元信息
#[pyfunction]
pub fn get_html_meta_info(html_content: String) -> PyResult<Py<PyAny>> {
    Python::attach(|py| {
        let parser = HtmlParser::new(html_content);
        let meta_info = parser.get_meta_info();
        let result = PyDict::new(py);

        for (key, value) in meta_info {
            result.set_item(key, value)?;
        }

        Ok(result.into_any().unbind())
    })
}
