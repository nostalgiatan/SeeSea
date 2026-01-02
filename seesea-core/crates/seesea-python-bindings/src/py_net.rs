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

//! Python bindings for net client functionality

use pyo3::prelude::*;
use pyo3::types::PyDict;

use seesea_config::RequestOptions;
use seesea_net::client::HttpClient;

#[pyclass]
pub struct PyNetClient {
    runtime: Option<tokio::runtime::Runtime>,
}

#[pymethods]
impl PyNetClient {
    /// 创建网络客户端（内部使用，Python中不直接实例化）
    #[new]
    pub fn new() -> PyResult<Self> {
        // 检查当前是否已经存在Tokio运行时
        match tokio::runtime::Handle::try_current() {
            // 如果已经存在运行时，不创建新的运行时
            Ok(_) => Ok(Self { runtime: None }),
            // 如果不存在运行时，创建新的运行时
            Err(_) => {
                let runtime = tokio::runtime::Runtime::new().map_err(|e| {
                    PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                        "Failed to create runtime: {e}"
                    ))
                })?;

                Ok(Self {
                    runtime: Some(runtime),
                })
            }
        }
    }

    /// 发送GET请求
    ///
    /// 参数:
    /// - url: 请求URL
    /// - headers: 请求头（可选，字典类型）
    ///
    /// 返回:
    /// - 包含状态码、响应头和响应内容的字典
    pub fn get(&self, url: String, headers: Option<Py<PyAny>>) -> PyResult<Py<PyAny>> {
        Python::attach(|py| {
            // 处理请求头
            let request_options = self.process_headers(py, headers)?;

            // 复用全局HttpClient实例
            let http_client = match self.runtime.as_ref() {
                Some(runtime) => runtime.block_on(async { HttpClient::instance().await }),
                None => tokio::runtime::Handle::current()
                    .block_on(async { HttpClient::instance().await }),
            }
            .map_err(|e| {
                PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                    "Failed to get HTTP client instance: {e}"
                ))
            })?;

            // 发送GET请求
            let response = match self.runtime.as_ref() {
                Some(runtime) => {
                    runtime.block_on(async { http_client.get(&url, Some(request_options)).await })
                }
                None => tokio::runtime::Handle::current()
                    .block_on(async { http_client.get(&url, Some(request_options)).await }),
            }
            .map_err(|e| {
                PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                    "GET request failed: {e}"
                ))
            })?;

            // 处理响应
            self.process_response(py, response)
        })
    }

    /// 发送POST请求
    ///
    /// 参数:
    /// - url: 请求URL
    /// - data: 请求体（字节数组或字符串）
    /// - headers: 请求头（可选，字典类型）
    ///
    /// 返回:
    /// - 包含状态码、响应头和响应内容的字典
    pub fn post(
        &self,
        url: String,
        data: Py<PyAny>,
        headers: Option<Py<PyAny>>,
    ) -> PyResult<Py<PyAny>> {
        Python::attach(|py| {
            // 处理请求体
            let body = self.process_body(py, data)?;

            // 处理请求头
            let request_options = self.process_headers(py, headers)?;

            // 复用全局HttpClient实例
            let http_client = match self.runtime.as_ref() {
                Some(runtime) => runtime.block_on(async { HttpClient::instance().await }),
                None => tokio::runtime::Handle::current()
                    .block_on(async { HttpClient::instance().await }),
            }
            .map_err(|e| {
                PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                    "Failed to get HTTP client instance: {e}"
                ))
            })?;

            // 发送POST请求
            let response = match self.runtime.as_ref() {
                Some(runtime) => runtime
                    .block_on(async { http_client.post(&url, body, Some(request_options)).await }),
                None => tokio::runtime::Handle::current()
                    .block_on(async { http_client.post(&url, body, Some(request_options)).await }),
            }
            .map_err(|e| {
                PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                    "POST request failed: {e}"
                ))
            })?;

            // 处理响应
            self.process_response(py, response)
        })
    }
}

impl PyNetClient {
    /// 处理请求体
    fn process_body(&self, py: Python, data: Py<PyAny>) -> PyResult<Vec<u8>> {
        let data = data.bind(py);
        if let Ok(bytes) = data.cast::<pyo3::types::PyBytes>() {
            // 处理字节数组
            Ok(bytes.as_bytes().to_vec())
        } else if let Ok(str_obj) = data.cast::<pyo3::types::PyString>() {
            // 处理字符串
            Ok(str_obj.str()?.to_string().as_bytes().to_vec())
        } else {
            Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
                "data must be bytes or string",
            ))
        }
    }

    /// 处理请求头
    fn process_headers(&self, py: Python, headers: Option<Py<PyAny>>) -> PyResult<RequestOptions> {
        let mut options = RequestOptions::default();

        if let Some(headers) = headers {
            let headers = headers.bind(py);
            if let Ok(headers_dict) = headers.cast::<PyDict>() {
                let mut header_map = std::collections::HashMap::new();
                for (key, value) in headers_dict.iter() {
                    let key_str = key.str()?.to_string();
                    let value_str = value.str()?.to_string();
                    header_map.insert(key_str, value_str);
                }
                options.headers = Some(header_map);
            } else {
                return Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
                    "headers must be a dictionary",
                ));
            }
        }

        Ok(options)
    }

    /// 处理响应
    fn process_response(&self, py: Python, response: reqwest::Response) -> PyResult<Py<PyAny>> {
        // 获取状态码
        let status = response.status().as_u16();

        // 获取响应头
        let headers_dict = PyDict::new(py);
        for (key, value) in response.headers() {
            let key_str = key.as_str().to_string();
            if let Ok(value_str) = value.to_str() {
                headers_dict.set_item(key_str, value_str)?;
            }
        }

        // 获取响应内容
        let body = match self.runtime.as_ref() {
            Some(runtime) => runtime.block_on(async { response.bytes().await }),
            None => tokio::runtime::Handle::current().block_on(async { response.bytes().await }),
        }
        .map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                "Failed to read response body: {e}"
            ))
        })?;

        // 创建响应字典
        let response_dict = PyDict::new(py);
        response_dict.set_item("status", status)?;
        response_dict.set_item("headers", headers_dict)?;
        response_dict.set_item("content", pyo3::types::PyBytes::new(py, &body))?;

        Ok(response_dict.into_any().unbind())
    }
}

/// 直接导出的GET请求函数
///
/// 参数:
/// - url: 请求URL
/// - headers: 请求头（可选，字典类型）
///
/// 返回:
/// - 包含状态码、响应头和响应内容的字典
#[pyfunction]
pub fn get(url: String, headers: Option<Py<PyAny>>) -> PyResult<Py<PyAny>> {
    Python::attach(|_py| {
        // 创建临时客户端实例处理请求
        let client = PyNetClient::new()?;
        client.get(url, headers)
    })
}

/// 直接导出的POST请求函数
///
/// 参数:
/// - url: 请求URL
/// - data: 请求体（字节数组或字符串）
/// - headers: 请求头（可选，字典类型）
///
/// 返回:
/// - 包含状态码、响应头和响应内容的字典
#[pyfunction]
pub fn post(url: String, data: Py<PyAny>, headers: Option<Py<PyAny>>) -> PyResult<Py<PyAny>> {
    Python::attach(|_py| {
        // 创建临时客户端实例处理请求
        let client = PyNetClient::new()?;
        client.post(url, data, headers)
    })
}

/// 直接导出的大文件下载函数（支持零拷贝）
///
/// 参数:
/// - url: 请求URL
/// - file_path: 本地文件路径，用于保存下载的文件
/// - headers: 请求头（可选，字典类型）
///
/// 返回:
/// - 包含状态码、响应头和文件保存信息的字典
#[pyfunction]
pub fn get_file(url: String, file_path: String, headers: Option<Py<PyAny>>) -> PyResult<Py<PyAny>> {
    Python::attach(|py| {
        // 创建临时客户端实例处理请求
        let client = PyNetClient::new()?;

        // 处理请求头
        let mut request_options = client.process_headers(py, headers)?;

        // 为大文件下载设置更长的超时时间（5分钟）
        request_options.timeout_secs = Some(300);

        // 使用已有的HttpClient，确保使用stream_client进行流式请求
        let http_client = match client.runtime.as_ref() {
            Some(runtime) => runtime.block_on(async { HttpClient::instance().await }),
            None => {
                tokio::runtime::Handle::current().block_on(async { HttpClient::instance().await })
            }
        }
        .map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                "Failed to get HTTP client instance: {e}"
            ))
        })?;

        // 发送流式GET请求获取原始字节流
        let (status, headers, mut reader) = match client.runtime.as_ref() {
            Some(runtime) => runtime
                .block_on(async { http_client.get_stream(&url, Some(request_options)).await }),
            None => tokio::runtime::Handle::current()
                .block_on(async { http_client.get_stream(&url, Some(request_options)).await }),
        }
        .map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                "GET stream request failed: {e}"
            ))
        })?;

        // 创建异步文件
        let mut file = match client.runtime.as_ref() {
            Some(runtime) => runtime.block_on(async { tokio::fs::File::create(&file_path).await }),
            None => tokio::runtime::Handle::current()
                .block_on(async { tokio::fs::File::create(&file_path).await }),
        }
        .map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyIOError, _>(format!("Failed to create file: {e}"))
        })?;

        // 使用零拷贝技术将流写入文件
        let bytes_written = match client.runtime.as_ref() {
            Some(runtime) => {
                runtime.block_on(async { tokio::io::copy(&mut reader, &mut file).await })
            }
            None => tokio::runtime::Handle::current()
                .block_on(async { tokio::io::copy(&mut reader, &mut file).await }),
        }
        .map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("Failed to write file: {e}"))
        })?;

        // 创建响应字典
        let response_dict = PyDict::new(py);

        // 设置状态码
        response_dict.set_item("status", status)?;

        // 设置响应头
        let headers_dict = PyDict::new(py);
        for (key, value) in headers {
            // 使用更安全的方式处理HeaderName，避免panic
            if let Some(key) = key.as_ref() {
                let key_str = key.as_str().to_string();
                if let Ok(value_str) = value.to_str() {
                    headers_dict.set_item(key_str, value_str)?;
                }
            }
        }
        response_dict.set_item("headers", headers_dict)?;

        // 设置文件信息
        let file_info_dict = PyDict::new(py);
        file_info_dict.set_item("path", file_path)?;
        file_info_dict.set_item("size", bytes_written)?;
        response_dict.set_item("file", file_info_dict)?;

        Ok(response_dict.into_any().unbind())
    })
}

/// 直接导出的大文件上传函数（支持零拷贝）
///
/// 参数:
/// - url: 请求URL
/// - file_path: 本地文件路径，用于读取要上传的文件
/// - content_type: 内容类型（可选）
/// - headers: 请求头（可选，字典类型）
///
/// 返回:
/// - 包含状态码、响应头和响应内容的字典
#[pyfunction]
pub fn post_file(
    url: String,
    file_path: String,
    content_type: Option<String>,
    headers: Option<Py<PyAny>>,
) -> PyResult<Py<PyAny>> {
    Python::attach(|py| {
        // 创建临时客户端实例处理请求
        let client = PyNetClient::new()?;

        // 处理请求头
        let request_options = client.process_headers(py, headers)?;

        // 获取文件元数据
        let file_metadata = match client.runtime.as_ref() {
            Some(runtime) => runtime.block_on(async { tokio::fs::metadata(&file_path).await }),
            None => tokio::runtime::Handle::current()
                .block_on(async { tokio::fs::metadata(&file_path).await }),
        }
        .map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyIOError, _>(format!(
                "Failed to get file metadata: {e}"
            ))
        })?;

        let content_length = file_metadata.len();

        // 打开文件
        let file = match client.runtime.as_ref() {
            Some(runtime) => runtime.block_on(async { tokio::fs::File::open(&file_path).await }),
            None => tokio::runtime::Handle::current()
                .block_on(async { tokio::fs::File::open(&file_path).await }),
        }
        .map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyIOError, _>(format!("Failed to open file: {e}"))
        })?;

        // 获取HTTP客户端实例
        let http_client = match client.runtime.as_ref() {
            Some(runtime) => runtime.block_on(async { HttpClient::instance().await }),
            None => {
                tokio::runtime::Handle::current().block_on(async { HttpClient::instance().await })
            }
        }
        .map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                "Failed to get HTTP client instance: {e}"
            ))
        })?;

        // 发送流式POST请求
        let response = match client.runtime.as_ref() {
            Some(runtime) => runtime.block_on(async {
                http_client
                    .post_stream(
                        &url,
                        file,
                        Some(content_length),
                        content_type.as_deref(),
                        Some(request_options),
                    )
                    .await
            }),
            None => tokio::runtime::Handle::current().block_on(async {
                http_client
                    .post_stream(
                        &url,
                        file,
                        Some(content_length),
                        content_type.as_deref(),
                        Some(request_options),
                    )
                    .await
            }),
        }
        .map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                "POST stream request failed: {e}"
            ))
        })?;

        // 保存状态码和响应头（在消耗response之前）
        let status = response.status().as_u16();
        let headers = response.headers().clone();

        // 读取响应内容
        let body = match client.runtime.as_ref() {
            Some(runtime) => runtime.block_on(async { response.bytes().await }),
            None => tokio::runtime::Handle::current().block_on(async { response.bytes().await }),
        }
        .map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                "Failed to read response body: {e}"
            ))
        })?;

        // 创建响应字典
        let response_dict = PyDict::new(py);
        response_dict.set_item("status", status)?;

        // 设置响应头
        let headers_dict = PyDict::new(py);
        for (key, value) in headers {
            // 使用更安全的方式处理HeaderName，避免panic
            if let Some(key) = key.as_ref() {
                let key_str = key.as_str().to_string();
                if let Ok(value_str) = value.to_str() {
                    headers_dict.set_item(key_str, value_str)?;
                }
            }
        }
        response_dict.set_item("headers", headers_dict)?;

        // 设置响应内容
        response_dict.set_item("content", pyo3::types::PyBytes::new(py, &body))?;

        Ok(response_dict.into_any().unbind())
    })
}
