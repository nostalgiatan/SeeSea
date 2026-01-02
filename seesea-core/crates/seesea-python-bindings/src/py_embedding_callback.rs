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

//! Python 嵌入回调绑定模块
//!
//! 提供 Python 嵌入模型回调的注册和调用功能，
//! 支持标准模式和 Pro 模式的嵌入向量化。

use pyo3::Bound;
use pyo3::prelude::*;
use pyo3::types::PyList;
use std::sync::{Arc, RwLock};
use tokio::sync::Semaphore;
use tracing::info;

/// 全局嵌入回调存储
static EMBEDDING_CALLBACK: RwLock<Option<Arc<EmbeddingCallback>>> = RwLock::new(None);

/// 嵌入模式
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EmbeddingMode {
    /// 标准模式 - 使用轻量级模型
    #[default]
    Standard,
    /// Pro模式 - 使用高质量模型
    Pro,
}

/// 嵌入回调封装
pub struct EmbeddingCallback {
    /// Python 回调函数
    callback: Py<PyAny>,
    /// 向量维度
    dimension: usize,
    /// 嵌入模式
    mode: EmbeddingMode,
    /// 并发控制信号量
    semaphore: Arc<Semaphore>,
}

impl EmbeddingCallback {
    /// 创建新的嵌入回调
    pub fn new(
        callback: Py<PyAny>,
        dimension: usize,
        mode: EmbeddingMode,
        max_concurrency: usize,
    ) -> Self {
        Self {
            callback,
            dimension,
            mode,
            semaphore: Arc::new(Semaphore::new(max_concurrency)),
        }
    }

    /// 获取向量维度
    pub fn dimension(&self) -> usize {
        self.dimension
    }

    /// 获取嵌入模式
    pub fn mode(&self) -> EmbeddingMode {
        self.mode
    }

    /// 调用 Python 回调进行嵌入
    pub fn embed(&self, text: &str) -> Result<Vec<f32>, String> {
        // 获取并发许可
        let _permit = self
            .semaphore
            .clone()
            .try_acquire_owned()
            .map_err(|_| "嵌入并发限制已达上限".to_string())?;

        Python::attach(|py| {
            let result = self
                .callback
                .call1(py, (text,))
                .map_err(|e| format!("Python 嵌入回调失败: {e}"))?;

            // 将 Python 列表转换为 Vec<f32>
            let bound_result = result.bind(py);
            let list: &Bound<'_, PyList> = bound_result
                .cast()
                .map_err(|e| format!("返回值不是列表: {e}"))?;

            let mut vec = Vec::with_capacity(list.len());
            for item in list.iter() {
                let value: f32 = item.extract().map_err(|e| format!("无法提取浮点值: {e}"))?;
                vec.push(value);
            }

            Ok(vec)
        })
    }

    /// 批量嵌入（带并发控制）
    pub async fn embed_batch(&self, texts: &[String]) -> Vec<Result<Vec<f32>, String>> {
        let mut results = Vec::with_capacity(texts.len());

        for text in texts {
            // 获取并发许可
            let permit = self.semaphore.clone().acquire_owned().await;

            let result = self.embed(text);
            results.push(result);

            drop(permit);
        }

        results
    }
}

/// 注册嵌入回调
#[pyfunction]
pub fn register_embedding_callback(
    py: Python<'_>,
    callback: Py<PyAny>,
    dimension: usize,
    mode: &str,
    max_concurrency: Option<usize>,
) -> PyResult<()> {
    let embedding_mode = match mode.to_lowercase().as_str() {
        "pro" => EmbeddingMode::Pro,
        _ => EmbeddingMode::Standard,
    };

    let max_concurrency = max_concurrency.unwrap_or(4);

    let embedding_callback = EmbeddingCallback::new(
        callback.clone_ref(py),
        dimension,
        embedding_mode,
        max_concurrency,
    );

    // 存储回调
    let mut guard = EMBEDDING_CALLBACK
        .write()
        .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("锁定失败: {e}")))?;
    *guard = Some(Arc::new(embedding_callback));

    info!(
        "嵌入回调已注册: 维度={}, 模式={:?}, 最大并发={}",
        dimension, embedding_mode, max_concurrency
    );

    Ok(())
}

/// 取消注册嵌入回调
#[pyfunction]
pub fn unregister_embedding_callback() -> PyResult<()> {
    let mut guard = EMBEDDING_CALLBACK
        .write()
        .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("锁定失败: {e}")))?;
    *guard = None;

    info!("嵌入回调已取消注册");
    Ok(())
}

/// 检查嵌入回调是否已注册
#[pyfunction]
pub fn is_embedding_callback_registered() -> PyResult<bool> {
    let guard = EMBEDDING_CALLBACK
        .read()
        .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("锁定失败: {e}")))?;
    Ok(guard.is_some())
}

/// 获取当前嵌入模式
#[pyfunction]
pub fn get_embedding_mode() -> PyResult<String> {
    let guard = EMBEDDING_CALLBACK
        .read()
        .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("锁定失败: {e}")))?;

    match &*guard {
        Some(callback) => Ok(match callback.mode() {
            EmbeddingMode::Pro => "pro".to_string(),
            EmbeddingMode::Standard => "standard".to_string(),
        }),
        None => Ok("none".to_string()),
    }
}

/// 获取当前嵌入维度
#[pyfunction]
pub fn get_embedding_dimension() -> PyResult<usize> {
    let guard = EMBEDDING_CALLBACK
        .read()
        .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("锁定失败: {e}")))?;

    match &*guard {
        Some(callback) => Ok(callback.dimension()),
        None => Err(pyo3::exceptions::PyRuntimeError::new_err("嵌入回调未注册")),
    }
}

/// 获取全局嵌入回调（供 Rust 内部使用）
pub fn get_embedding_callback() -> Option<Arc<EmbeddingCallback>> {
    EMBEDDING_CALLBACK
        .read()
        .ok()
        .and_then(|guard| guard.clone())
}

/// 嵌入单个文本（供 Rust 内部使用）
pub fn embed_text(text: &str) -> Result<Vec<f32>, String> {
    let callback = get_embedding_callback().ok_or_else(|| "嵌入回调未注册".to_string())?;
    callback.embed(text)
}

/// 批量嵌入文本（供 Rust 内部使用）
pub async fn embed_texts(texts: &[String]) -> Vec<Result<Vec<f32>, String>> {
    match get_embedding_callback() {
        Some(callback) => callback.embed_batch(texts).await,
        None => texts
            .iter()
            .map(|_| Err("嵌入回调未注册".to_string()))
            .collect(),
    }
}

/// Python 模块导出
#[pymodule]
pub fn embedding_callback(_py: Python, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(register_embedding_callback, m)?)?;
    m.add_function(wrap_pyfunction!(unregister_embedding_callback, m)?)?;
    m.add_function(wrap_pyfunction!(is_embedding_callback_registered, m)?)?;
    m.add_function(wrap_pyfunction!(get_embedding_mode, m)?)?;
    m.add_function(wrap_pyfunction!(get_embedding_dimension, m)?)?;
    Ok(())
}
