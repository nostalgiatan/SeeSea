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

//! Python bindings for cleaner functionality

use pyo3::prelude::*;
use pyo3::types::PyDict;

use seesea_cleaner::{Cleaner, DataBlock};

/// Python包装的数据块类
#[pyclass]
pub struct PyDataBlock {
    pub block: DataBlock,
}

#[pymethods]
impl PyDataBlock {
    /// 获取数据块内容
    #[getter]
    pub fn content(&self) -> &str {
        &self.block.content
    }

    /// 获取起始行号
    #[getter]
    pub fn start_line(&self) -> usize {
        self.block.start_line
    }

    /// 获取结束行号
    #[getter]
    pub fn end_line(&self) -> usize {
        self.block.end_line
    }

    /// 获取标题相关性
    #[getter]
    pub fn title_relevance(&self) -> f32 {
        self.block.title_relevance
    }

    /// 获取块内连贯性
    #[getter]
    pub fn coherence(&self) -> f32 {
        self.block.coherence
    }

    /// 获取最终得分
    #[getter]
    pub fn score(&self) -> f32 {
        self.block.score
    }

    /// 获取链接列表
    #[getter]
    pub fn links(&self) -> Vec<&str> {
        self.block.links.iter().map(|link| &**link).collect()
    }

    /// 获取图片列表
    #[getter]
    pub fn images(&self) -> Vec<&str> {
        self.block.images.iter().map(|image| &**image).collect()
    }

    /// 获取提取的键值对
    pub fn get_extracted_kv(&self, py: Python) -> PyResult<Py<PyDict>> {
        let dict = PyDict::new(py);
        for (key, value) in &self.block.extracted_kv {
            dict.set_item(key.as_ref(), value.as_ref())?;
        }
        Ok(dict.into())
    }

    /// 数据块是否有效
    #[getter]
    pub fn is_valid(&self) -> bool {
        self.block.is_valid
    }

    /// 获取标题向量
    #[getter]
    pub fn title_vector(&self) -> Option<Vec<f32>> {
        self.block.title_vector.clone()
    }

    /// 获取内容向量
    #[getter]
    pub fn content_vector(&self) -> Option<Vec<f32>> {
        self.block.content_vector.clone()
    }

    /// 获取关键词相似度
    #[getter]
    pub fn keyword_similarity(&self) -> f32 {
        self.block.keyword_similarity
    }

    /// 转换为Python字典
    pub fn to_dict(&self, py: Python) -> PyResult<Py<PyDict>> {
        let dict = PyDict::new(py);
        dict.set_item("content", self.content())?;
        dict.set_item("start_line", self.start_line())?;
        dict.set_item("end_line", self.end_line())?;
        dict.set_item("title_relevance", self.title_relevance())?;
        dict.set_item("coherence", self.coherence())?;
        dict.set_item("score", self.score())?;
        dict.set_item("links", self.links())?;
        dict.set_item("images", self.images())?;
        dict.set_item("extracted_kv", self.get_extracted_kv(py)?.as_ref())?;
        dict.set_item("is_valid", self.is_valid())?;
        dict.set_item("title_vector", self.title_vector())?;
        dict.set_item("content_vector", self.content_vector())?;
        dict.set_item("keyword_similarity", self.keyword_similarity())?;
        Ok(dict.into())
    }
}

/// Python包装的清洗器类
#[pyclass]
pub struct PyCleaner {
    pub cleaner: Cleaner,
    runtime: Option<tokio::runtime::Runtime>,
}

#[pymethods]
impl PyCleaner {
    /// 创建新的清洗器实例
    #[new]
    pub fn new(max_lines_per_block: Option<usize>) -> PyResult<Self> {
        // 检查当前是否已经存在Tokio运行时
        match tokio::runtime::Handle::try_current() {
            // 如果已经存在运行时，不创建新的运行时
            Ok(_) => {
                let cleaner = Cleaner::new(max_lines_per_block.unwrap_or(50));
                Ok(Self {
                    cleaner,
                    runtime: None,
                })
            }
            // 如果不存在运行时，创建新的运行时
            Err(_) => {
                let runtime = tokio::runtime::Runtime::new().map_err(|e| {
                    PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                        "Failed to create runtime: {e}"
                    ))
                })?;

                let cleaner = Cleaner::new(max_lines_per_block.unwrap_or(50));

                Ok(Self {
                    cleaner,
                    runtime: Some(runtime),
                })
            }
        }
    }

    /// 处理文本，返回清洗后的数据块
    pub fn process(&self, text: &str) -> PyResult<Vec<PyDataBlock>> {
        let (_, blocks) = match self.runtime.as_ref() {
            Some(runtime) => runtime.block_on(async { self.cleaner.process(text, None).await }),
            None => tokio::runtime::Handle::current()
                .block_on(async { self.cleaner.process(text, None).await }),
        };

        Ok(blocks
            .into_iter()
            .map(|block| PyDataBlock { block })
            .collect())
    }

    /// 批量处理文本列表
    pub fn batch_process(&self, texts: Vec<String>) -> PyResult<Vec<Vec<PyDataBlock>>> {
        // 预先克隆self，避免异步闭包捕获self导致的生命周期问题
        let cleaner = &self.cleaner;

        let results = match self.runtime.as_ref() {
            Some(runtime) => {
                runtime.block_on(async {
                    futures::future::join_all(texts.into_iter().map(|text| {
                        // 创建新的异步任务，确保text的生命周期足够长
                        async move { cleaner.process(&text, None).await }
                    }))
                    .await
                })
            }
            None => {
                tokio::runtime::Handle::current().block_on(async {
                    futures::future::join_all(texts.into_iter().map(|text| {
                        // 创建新的异步任务，确保text的生命周期足够长
                        async move { cleaner.process(&text, None).await }
                    }))
                    .await
                })
            }
        };

        Ok(results
            .into_iter()
            .map(|(_, blocks)| {
                blocks
                    .into_iter()
                    .map(|block| PyDataBlock { block })
                    .collect()
            })
            .collect())
    }

    /// 处理文本，返回清洗后的上下文
    pub fn process_with_context(&self, text: &str) -> PyResult<String> {
        let (_, blocks) = match self.runtime.as_ref() {
            Some(runtime) => runtime.block_on(async { self.cleaner.process(text, None).await }),
            None => tokio::runtime::Handle::current()
                .block_on(async { self.cleaner.process(text, None).await }),
        };

        // 过滤有效数据块
        let valid_blocks = blocks
            .into_iter()
            .filter(|block| block.is_valid)
            .collect::<Vec<_>>();

        // 拼接上下文
        let context = valid_blocks
            .iter()
            .map(|block| block.content.as_ref())
            .collect::<Vec<_>>()
            .join("\n\n");

        Ok(context)
    }
}

/// 导出Python模块
#[pymodule]
pub fn cleaner(_py: Python, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyCleaner>()?;
    m.add_class::<PyDataBlock>()?;
    Ok(())
}
