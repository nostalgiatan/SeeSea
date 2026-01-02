// Copyright (C) 2025 SeeSea Team
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

//! Python bindings for hot trend functionality
//!
//! 提供热点数据获取功能的Python绑定

use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use std::sync::Arc;

use seesea_hot::client::HotTrendClient;

/// Python绑定的热点数据客户端
#[pyclass(name = "PyHotTrendClient")]
pub struct PyHotTrendClient {
    /// 热点数据客户端实例
    client: Arc<HotTrendClient>,
}

#[pymethods]
impl PyHotTrendClient {
    /// 创建新的热点数据客户端
    ///
    /// Args:
    ///     max_concurrency: 最大并发数，默认为10
    ///
    /// Returns:
    ///     PyHotTrendClient: 热点数据客户端实例
    ///
    /// Raises:
    ///     RuntimeError: 初始化失败时抛出
    #[new]
    pub fn new(max_concurrency: Option<usize>) -> PyResult<Self> {
        let client = HotTrendClient::new(max_concurrency.unwrap_or(10)).map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                "Failed to create hot trend client: {e}"
            ))
        })?;

        Ok(Self {
            client: Arc::new(client),
        })
    }

    /// 获取单个平台的热点数据
    ///
    /// Args:
    ///     platform_id: 平台ID，如zhihu、weibo等
    ///
    /// Returns:
    ///     dict: 包含平台信息和热点数据的字典
    ///
    /// Raises:
    ///     ValueError: 平台ID无效时抛出
    ///     RuntimeError: 获取数据失败时抛出
    pub fn fetch_platform(&self, py: Python<'_>, platform_id: &str) -> PyResult<Py<PyAny>> {
        let result = self.client.fetch_platform(platform_id).map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                "Failed to fetch hot trends for platform {platform_id}: {e}"
            ))
        })?;

        // 创建字典并设置平台信息
        let dict = PyDict::new(py);
        dict.set_item("platform_id", result.platform_id)?;
        dict.set_item("platform_name", result.platform_name)?;
        dict.set_item("status", result.status)?;

        // 创建热点数据项列表
        let items_list = PyList::empty(py);
        for item in result.items {
            let item_dict = PyDict::new(py);
            item_dict.set_item("title", item.title)?;
            item_dict.set_item("url", item.url)?;
            item_dict.set_item("mobile_url", item.mobile_url)?;
            item_dict.set_item("rank", item.rank)?;
            items_list.append(item_dict)?;
        }

        dict.set_item("items", items_list)?;
        Ok(dict.into_any().unbind())
    }

    /// 获取所有平台的热点数据
    ///
    /// Returns:
    ///     list: 包含所有平台热点数据的列表
    ///
    /// Raises:
    ///     RuntimeError: 获取数据失败时抛出
    pub fn fetch_all_platforms(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let results = self.client.fetch_all_platforms();

        // 创建平台数据列表
        let platforms_list = PyList::empty(py);

        for result in results {
            match result {
                Ok(result) => {
                    let dict = PyDict::new(py);
                    dict.set_item("platform_id", result.platform_id)?;
                    dict.set_item("platform_name", result.platform_name)?;
                    dict.set_item("status", result.status)?;

                    // 创建热点数据项列表
                    let items_list = PyList::empty(py);
                    for item in result.items {
                        let item_dict = PyDict::new(py);
                        item_dict.set_item("title", item.title)?;
                        item_dict.set_item("url", item.url)?;
                        item_dict.set_item("mobile_url", item.mobile_url)?;
                        item_dict.set_item("rank", item.rank)?;
                        items_list.append(item_dict)?;
                    }

                    dict.set_item("items", items_list)?;
                    platforms_list.append(dict)?;
                }
                Err(e) => {
                    // 记录错误但继续处理其他平台
                    eprintln!("Error fetching hot trends: {e}");
                }
            }
        }

        Ok(platforms_list.into_any().unbind())
    }

    /// 列出所有支持的平台
    ///
    /// Returns:
    ///     dict: 平台ID到平台名称的映射
    pub fn list_platforms(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let platforms = self.client.list_platforms();

        let dict = PyDict::new(py);

        for (id, name) in platforms {
            dict.set_item(id, name)?;
        }

        Ok(dict.into_any().unbind())
    }
}
