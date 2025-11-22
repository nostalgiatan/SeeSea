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

//! Python 浏览器引擎绑定
//!
//! 提供从Python到Rust的浏览器引擎反向API

use pyo3::prelude::*;
use pyo3::types::{PyDict, PyModule};
use std::collections::HashMap;

/// 浏览器配置（Python端）
#[pyclass]
#[derive(Clone)]
pub struct PyBrowserConfig {
    #[pyo3(get, set)]
    pub headless: bool,
    #[pyo3(get, set)]
    pub stealth: bool,
    #[pyo3(get, set)]
    pub browser_type: String,
    #[pyo3(get, set)]
    pub user_agent: Option<String>,
    #[pyo3(get, set)]
    pub viewport_width: u32,
    #[pyo3(get, set)]
    pub viewport_height: u32,
}

#[pymethods]
impl PyBrowserConfig {
    #[new]
    #[pyo3(signature = (headless=true, stealth=true, browser_type="chromium".to_string(), user_agent=None, viewport_width=1920, viewport_height=1080))]
    pub fn new(
        headless: bool,
        stealth: bool,
        browser_type: String,
        user_agent: Option<String>,
        viewport_width: u32,
        viewport_height: u32,
    ) -> Self {
        Self {
            headless,
            stealth,
            browser_type,
            user_agent,
            viewport_width,
            viewport_height,
        }
    }

    fn __repr__(&self) -> String {
        format!(
            "PyBrowserConfig(headless={}, stealth={}, browser={}, viewport={}x{})",
            self.headless, self.stealth, self.browser_type, 
            self.viewport_width, self.viewport_height
        )
    }
}

/// 浏览器引擎客户端（Python端）
#[pyclass]
pub struct PyBrowserEngineClient {
    /// Python playwright 回调函数
    playwright_callback: Option<Py<PyAny>>,
}

#[pymethods]
impl PyBrowserEngineClient {
    #[new]
    pub fn new() -> Self {
        Self {
            playwright_callback: None,
        }
    }

    /// 注册Playwright回调函数
    /// 
    /// 这个函数将从Python端调用，传入一个async函数来执行浏览器操作
    pub fn register_playwright(&mut self, callback: Py<PyAny>) {
        self.playwright_callback = Some(callback);
    }

    /// 执行浏览器操作
    /// 
    /// # 参数
    /// 
    /// * `url` - 目标URL
    /// * `actions` - 操作列表（字典列表）
    /// * `selectors` - 选择器映射
    /// * `config` - 浏览器配置
    /// 
    /// # 返回值
    /// 
    /// 返回提取的数据字典
    pub fn execute<'py>(
        &self,
        py: Python<'py>,
        url: String,
        actions: Vec<HashMap<String, String>>,
        selectors: HashMap<String, String>,
        config: &PyBrowserConfig,
    ) -> PyResult<Py<PyDict>> {
        if let Some(callback) = &self.playwright_callback {
            // 准备参数
            let args_dict = PyDict::new(py);
            args_dict.set_item("url", url)?;
            args_dict.set_item("actions", actions)?;
            args_dict.set_item("selectors", selectors)?;
            args_dict.set_item("config", config.clone())?;

            // 调用Python端的callback
            let result = callback.call1(py, (args_dict,))?;
            Ok(result.extract(py)?)
        } else {
            Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                "Playwright callback not registered. Call register_playwright() first."
            ))
        }
    }

    /// 测试方法：简单检查是否已注册
    pub fn is_registered(&self) -> bool {
        self.playwright_callback.is_some()
    }
}

/// 注册浏览器引擎Python绑定
pub fn register_browser_engine(_py: Python, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyBrowserConfig>()?;
    m.add_class::<PyBrowserEngineClient>()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_browser_config_creation() {
        let config = PyBrowserConfig::new(
            true,
            true,
            "chromium".to_string(),
            None,
            1920,
            1080,
        );
        assert!(config.headless);
        assert!(config.stealth);
        assert_eq!(config.browser_type, "chromium");
    }

    #[test]
    fn test_browser_client_registration() {
        let client = PyBrowserEngineClient::new();
        assert!(!client.is_registered());
    }
}
