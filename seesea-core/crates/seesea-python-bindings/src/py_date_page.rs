// Copyright (C) 2024 SeeSea Authors
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as
// published by the Free Software Foundation, either version 3 of the
// License, or (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

//! Python bindings for DatePage functionality

use pyo3::prelude::*;
use pyo3::types::PyDict;
use std::time::SystemTime;

use crate::py_cleaner::PyDataBlock;
use seesea_cleaner::{DatePage, ExtraInfoItem, MapItem};

/// Python包装的MapItem类
#[pyclass]
pub struct PyMapItem {
    map_item: MapItem,
}

#[pymethods]
impl PyMapItem {
    /// 获取文本
    #[getter]
    pub fn text(&self) -> &str {
        &self.map_item.text
    }

    /// 获取URL
    #[getter]
    pub fn url(&self) -> &str {
        &self.map_item.url
    }

    /// 转换为Python字典
    pub fn to_dict(&self, py: Python) -> PyResult<Py<PyDict>> {
        let dict = PyDict::new(py);
        dict.set_item("text", self.text())?;
        dict.set_item("url", self.url())?;
        Ok(dict.into())
    }
}

/// Python包装的ExtraInfoItem类
#[pyclass]
pub struct PyExtraInfoItem {
    extra_info_item: ExtraInfoItem,
}

#[pymethods]
impl PyExtraInfoItem {
    /// 获取键
    #[getter]
    pub fn key(&self) -> &str {
        &self.extra_info_item.key
    }

    /// 获取值
    #[getter]
    pub fn value(&self) -> &str {
        &self.extra_info_item.value
    }

    /// 转换为Python字典
    pub fn to_dict(&self, py: Python) -> PyResult<Py<PyDict>> {
        let dict = PyDict::new(py);
        dict.set_item("key", self.key())?;
        dict.set_item("value", self.value())?;
        Ok(dict.into())
    }
}

/// Python包装的DatePage类
#[pyclass]
pub struct PyDatePage {
    pub date_page: DatePage,
    pub runtime: Option<tokio::runtime::Runtime>,
}

#[pymethods]
impl PyDatePage {
    /// 创建新的DatePage实例
    #[new]
    pub fn new(
        url: &str,
        time: f64, // Unix timestamp in seconds
        description: &str,
        source_data: &str,
    ) -> PyResult<Self> {
        // 转换Unix时间戳为SystemTime
        let time = SystemTime::UNIX_EPOCH + std::time::Duration::from_secs_f64(time);

        // 检查当前是否已经存在Tokio运行时
        let runtime = match tokio::runtime::Handle::try_current() {
            // 如果已经存在运行时，不创建新的运行时
            Ok(_) => None,
            // 如果不存在运行时，创建新的运行时
            Err(_) => Some(tokio::runtime::Runtime::new().map_err(|e| {
                PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                    "Failed to create runtime: {e}"
                ))
            })?),
        };

        let date_page = DatePage::new(url, time, description, source_data);

        Ok(Self { date_page, runtime })
    }

    /// URL
    #[getter]
    pub fn url(&self) -> &str {
        &self.date_page.url
    }

    /// 时间（Unix时间戳）
    #[getter]
    pub fn time(&self) -> f64 {
        self.date_page
            .time
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs_f64()
    }

    /// 描述
    #[getter]
    pub fn description(&self) -> &str {
        &self.date_page.description
    }

    /// 源数据
    #[getter]
    pub fn source_data(&self) -> &str {
        &self.date_page.source_data
    }

    /// 数据块列表
    #[getter]
    pub fn data_blocks(&self) -> Vec<PyDataBlock> {
        self.date_page
            .data_blocks
            .iter()
            .map(|block| PyDataBlock {
                block: block.clone(),
            })
            .collect()
    }

    /// 向量列表
    #[getter]
    pub fn vectors(&self) -> Vec<Vec<f32>> {
        self.date_page.vectors.clone()
    }

    /// 哈希
    #[getter]
    pub fn hash(&self) -> u64 {
        self.date_page.hash
    }

    /// 最后更新时间（Unix时间戳）
    #[getter]
    pub fn last_update_time(&self) -> f64 {
        self.date_page
            .last_update_time
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs_f64()
    }

    /// 地图
    #[getter]
    pub fn map(&self) -> Vec<PyMapItem> {
        self.date_page
            .map
            .iter()
            .map(|item| PyMapItem {
                map_item: item.clone(),
            })
            .collect()
    }

    /// 额外信息
    #[getter]
    pub fn extra_info(&self) -> Vec<PyExtraInfoItem> {
        self.date_page
            .extra_info
            .iter()
            .map(|item| PyExtraInfoItem {
                extra_info_item: item.clone(),
            })
            .collect()
    }

    /// 检查源数据是否更新
    pub fn is_updated(&self, new_source_data: &str) -> bool {
        self.date_page.is_updated(new_source_data)
    }

    /// 更新源数据
    pub fn update_source_data(&mut self, new_source_data: &str) {
        self.date_page.update_source_data(new_source_data);
    }

    /// 清理数据
    pub fn cleaning(&mut self, cleaner: &crate::py_cleaner::PyCleaner) -> PyResult<()> {
        let cleaner = &cleaner.cleaner;

        // 根据runtime是否存在来执行异步操作
        match self.runtime.as_ref() {
            Some(runtime) => {
                runtime.block_on(async {
                    self.date_page.cleaning(cleaner).await;
                });
            }
            None => {
                tokio::runtime::Handle::current().block_on(async {
                    self.date_page.cleaning(cleaner).await;
                });
            }
        }

        Ok(())
    }

    /// 添加地图项
    pub fn add_map_item(&mut self, text: &str, url: &str) {
        self.date_page.add_map_item(text, url);
    }

    /// 添加额外信息项
    pub fn add_extra_info(&mut self, key: &str, value: &str) {
        self.date_page.add_extra_info(key, value);
    }

    /// 转换为Python字典
    pub fn to_dict(&self, py: Python) -> PyResult<Py<PyDict>> {
        let dict = PyDict::new(py);
        dict.set_item("url", self.url())?;
        dict.set_item("time", self.time())?;
        dict.set_item("description", self.description())?;
        dict.set_item("source_data", self.source_data())?;
        dict.set_item("data_blocks", self.data_blocks())?;
        dict.set_item("vectors", self.vectors())?;
        dict.set_item("hash", self.hash())?;
        dict.set_item("last_update_time", self.last_update_time())?;
        dict.set_item("map", self.map())?;
        dict.set_item("extra_info", self.extra_info())?;
        Ok(dict.into())
    }
}

/// 导出Python模块
#[pymodule]
pub fn date_page(_py: Python, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyMapItem>()?;
    m.add_class::<PyExtraInfoItem>()?;
    m.add_class::<PyDatePage>()?;
    Ok(())
}
