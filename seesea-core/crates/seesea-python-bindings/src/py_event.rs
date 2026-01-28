//! Python事件绑定模块
//! 提供基于字符串的事件注册和监听功能

use pyo3::prelude::*;
use pyo3_async_runtimes::tokio::future_into_py;
use seesea_event::{
    EventPayload, StringAsyncEventOperations, StringEventOperations, get_global_async_event_bus,
    get_global_event_bus,
};
use seesea_sys::get_or_create_runtime;
use std::sync::Arc;
use tracing::info;

/// Python事件包装器
#[pyclass]
#[derive(Clone)]
pub struct PyEvent {
    #[pyo3(get)]
    pub event_type: String,
    #[pyo3(get)]
    pub data: Option<Vec<u8>>,
    #[pyo3(get)]
    pub error_type: Option<String>,
    #[pyo3(get)]
    pub error_message: Option<String>,
}

#[pymethods]
impl PyEvent {
    #[new]
    fn new(
        event_type: String,
        data: Option<Vec<u8>>,
        error_type: Option<String>,
        error_message: Option<String>,
    ) -> Self {
        Self {
            event_type,
            data,
            error_type,
            error_message,
        }
    }

    fn is_data_event(&self) -> bool {
        self.data.is_some()
    }

    fn is_error_event(&self) -> bool {
        self.error_type.is_some()
    }

    fn get_data_string(&self) -> PyResult<String> {
        match &self.data {
            Some(data) => String::from_utf8(data.clone()).map_err(|e| {
                PyErr::new::<pyo3::exceptions::PyUnicodeDecodeError, _>(e.to_string())
            }),
            None => Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "No data available",
            )),
        }
    }
}

/// 发布字符串事件到全局事件总线
#[pyfunction]
pub fn publish_string_event(event_type: String, data: String) -> PyResult<()> {
    let bus = get_global_event_bus();
    bus.publish_string(&event_type, &data)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))
}

/// 发布字符串错误事件到全局事件总线
#[pyfunction]
pub fn publish_string_error_event(error_type: String, message: String) -> PyResult<()> {
    let bus = get_global_event_bus();
    bus.publish_string_error(&error_type, &message)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))
}

/// 异步发送字符串请求事件
#[pyfunction]
pub fn send_string_request_event(
    py: Python,
    event_type: String,
    data: String,
) -> PyResult<Py<PyAny>> {
    let bus = get_global_async_event_bus();

    let future = async move {
        match bus.send_string_request(&event_type, &data).await {
            Ok(payload) => match payload {
                EventPayload::Data(data) => {
                    let event = PyEvent {
                        event_type: "data".to_string(),
                        data: Some(data),
                        error_type: None,
                        error_message: None,
                    };
                    Ok(event)
                }
                EventPayload::Error(msg) => {
                    let event = PyEvent {
                        event_type: "error".to_string(),
                        data: None,
                        error_type: Some("async_error".to_string()),
                        error_message: Some(msg),
                    };
                    Ok(event)
                }
                EventPayload::Empty => {
                    let event = PyEvent {
                        event_type: "empty".to_string(),
                        data: None,
                        error_type: None,
                        error_message: None,
                    };
                    Ok(event)
                }
            },
            Err(e) => Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                e.to_string(),
            )),
        }
    };

    let result = future_into_py(py, future)?;
    Ok(result.into())
}

/// 注册字符串事件处理器
#[pyfunction]
pub fn on_string_event(py: Python, event_type: String, handler: Py<PyAny>) -> PyResult<Py<PyAny>> {
    let bus = get_global_async_event_bus();

    // 使用Arc包装handler以便在异步上下文中共享
    let handler = Arc::new(handler);

    let future = async move {
        bus.on(&event_type, move |event_type: &str, data: &str| {
            let handler = Arc::clone(&handler);
            let event_type = event_type.to_string();
            let data = data.to_string();
            Box::pin(async move {
                Python::attach(|py| {
                    // 调用Python处理器
                    let result = handler.call1(py, (event_type, data));

                    match result {
                        Ok(result) => {
                            // 如果处理器返回数据，转换为EventPayload
                            if let Ok(data_str) = result.extract::<String>(py) {
                                EventPayload::Data(data_str.into_bytes())
                            } else {
                                EventPayload::Empty
                            }
                        }
                        Err(e) => {
                            // 处理器出错，返回错误
                            EventPayload::Error(e.to_string())
                        }
                    }
                })
            })
        })
        .await
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;

        Ok::<(), PyErr>(())
    };

    let result = future_into_py(py, future)?;
    Ok(result.into())
}

/// 异步发送字符串通知事件
#[pyfunction]
pub fn send_string_notification_event(
    py: Python,
    event_type: String,
    data: String,
) -> PyResult<Py<PyAny>> {
    let bus = get_global_async_event_bus();

    let future = async move {
        bus.send_string_notification(&event_type, &data)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;

        Ok::<(), PyErr>(())
    };

    let result = future_into_py(py, future)?;
    Ok(result.into())
}

/// 异步发送字符串错误事件
#[pyfunction]
pub fn send_string_error_event(py: Python, error_message: String) -> PyResult<Py<PyAny>> {
    let bus = get_global_async_event_bus();

    let future = async move {
        bus.send_string_error(&error_message)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;

        Ok::<(), PyErr>(())
    };

    let result = future_into_py(py, future)?;
    Ok(result.into())
}

/// 确保全局 Tokio 运行时已初始化
///
/// 在使用任何异步事件功能之前调用此函数，确保运行时存在。
/// 可以安全地多次调用，只会初始化一次。
#[pyfunction]
pub fn ensure_event_runtime() -> PyResult<()> {
    let _ = get_or_create_runtime();
    info!("✅ 事件运行时已确保存在");
    Ok(())
}

/// 同步注册字符串事件处理器
///
/// 这是 on_string_event 的同步版本，在没有运行中的 event loop 时使用。
/// 它会确保 Tokio 运行时存在，然后在该运行时上注册事件处理器。
#[pyfunction]
pub fn on_string_event_sync(event_type: String, handler: Py<PyAny>) -> PyResult<()> {
    // 确保运行时存在
    let runtime = get_or_create_runtime();
    let bus = get_global_async_event_bus();

    // 使用Arc包装handler以便在异步上下文中共享
    let handler = Arc::new(handler);
    let event_type_clone = event_type.clone();

    // 在全局运行时上注册事件处理器
    runtime.block_on(async {
        bus.on(&event_type_clone, move |event_type: &str, data: &str| {
            let handler = Arc::clone(&handler);
            let event_type = event_type.to_string();
            let data = data.to_string();
            Box::pin(async move {
                Python::attach(|py| {
                    // 调用Python处理器
                    let result = handler.call1(py, (event_type, data));

                    match result {
                        Ok(result) => {
                            // 如果处理器返回数据，转换为EventPayload
                            if let Ok(data_str) = result.extract::<String>(py) {
                                EventPayload::Data(data_str.into_bytes())
                            } else {
                                EventPayload::Empty
                            }
                        }
                        Err(e) => {
                            // 处理器出错，返回错误
                            EventPayload::Error(e.to_string())
                        }
                    }
                })
            })
        })
        .await
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;

        Ok::<(), PyErr>(())
    })?;

    info!("✅ 事件处理器已注册: {}", event_type);
    Ok(())
}
