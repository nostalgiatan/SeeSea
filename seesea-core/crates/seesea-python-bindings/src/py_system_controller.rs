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

//! Python bindings for SystemController functionality
//!
//! 提供系统调控中心的Python函数接口，用于动态调控组件并发数量

use pyo3::prelude::*;
use pyo3::types::PyDict;
use std::sync::Arc;

use seesea_sys::controller::{
    SystemController, get_global_system_controller, get_or_create_runtime,
};
use seesea_sys::types::{
    AdjustmentRequest, AdjustmentType, ComponentConfig, ComponentId, ComponentType,
};

/// 在适当的运行时上下文中执行异步操作
///
/// 如果当前已有 Tokio 运行时，则使用当前运行时
/// 否则使用全局运行时
fn block_on_async<F, T>(future: F) -> T
where
    F: std::future::Future<Output = T>,
{
    match tokio::runtime::Handle::try_current() {
        Ok(handle) => handle.block_on(future),
        Err(_) => get_or_create_runtime().block_on(future),
    }
}

/// 初始化系统控制器
///
/// 只允许初始化一次，确保系统中只有一个系统控制器实例
#[pyfunction]
pub fn init_system_controller(_py: Python) -> PyResult<()> {
    // 使用全局系统控制器，自动初始化
    let _controller = get_global_system_controller();
    Ok(())
}

/// 获取系统控制器实例
///
/// 内部使用，确保只返回已初始化的实例
fn get_system_controller() -> PyResult<Arc<SystemController>> {
    // 使用全局系统控制器，自动初始化
    Ok(get_global_system_controller())
}

/// 注册组件到系统控制器
///
/// Args:
///     component_type: 组件类型 ("vector_store", "search_engine", "rss_fetcher", "pro_processor", "crawl4ai")
///     component_name: 组件名称
///     priority: 优先级 (0-100)
///     max_resource_usage: 最大资源使用率 (0.0-1.0)
///     min_resource_allocation: 最小资源分配 (0.0-1.0)
#[pyfunction]
pub fn register_component(
    py: Python,
    component_type: &str,
    component_name: &str,
    priority: u8,
    max_resource_usage: f64,
    min_resource_allocation: f64,
) -> PyResult<()> {
    let controller = get_system_controller()?;

    // 转换组件类型
    let comp_type = match component_type.to_lowercase().as_str() {
        "vector_store" => ComponentType::VectorStore,
        "pro_processor" => ComponentType::ProProcessor,
        "crawl4ai" => ComponentType::Crawl4Ai,
        _ => {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "Invalid component type: {component_type}"
            )));
        }
    };

    // 创建组件ID
    let component_id = ComponentId::new(comp_type, component_name);

    // 创建组件配置
    let config = ComponentConfig {
        id: component_id,
        priority,
        max_resource_usage,
        min_resource_allocation,
        enable_dynamic_adjustment: true,
        adjustment_params: serde_json::Value::Null,
    };

    // 发送调整请求（使用安全的运行时上下文）
    let result =
        py.detach(|| block_on_async(async move { controller.register_component(config).await }));

    match result {
        Ok(_) => Ok(()),
        Err(e) => Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
            "Failed to register component: {e}"
        ))),
    }
}

/// 调整组件的并发数量
///
/// Args:
///     component_type: 组件类型
///     component_name: 组件名称
///     concurrency: 新的并发数量
#[pyfunction]
pub fn adjust_component_concurrency(
    py: Python,
    component_type: &str,
    component_name: &str,
    concurrency: usize,
) -> PyResult<()> {
    let controller = get_system_controller()?;

    // 转换组件类型
    let comp_type = match component_type.to_lowercase().as_str() {
        "vector_store" => ComponentType::VectorStore,
        "pro_processor" => ComponentType::ProProcessor,
        "crawl4ai" => ComponentType::Crawl4Ai,
        _ => {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "Invalid component type: {component_type}"
            )));
        }
    };

    // 创建组件ID
    let component_id = ComponentId::new(comp_type, component_name);

    // 创建调整请求
    let request = AdjustmentRequest {
        component_id,
        adjustment_type: AdjustmentType::AdjustConcurrency,
        params: serde_json::json!({
            "concurrency": concurrency
        }),
    };

    // 发送调整请求（使用安全的运行时上下文）
    let result = py.detach(|| {
        block_on_async(async move {
            let response = controller.handle_adjustment_request(request).await;
            if response.success {
                Ok(())
            } else {
                Err(format!("Adjustment failed: {}", response.reason))
            }
        })
    });

    match result {
        Ok(_) => Ok(()),
        Err(e) => Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
            "Failed to adjust concurrency: {e}"
        ))),
    }
}

/// 调整组件的优先级
///
/// Args:
///     component_type: 组件类型
///     component_name: 组件名称
///     priority: 新的优先级 (0-100)
#[pyfunction]
pub fn adjust_component_priority(
    py: Python,
    component_type: &str,
    component_name: &str,
    priority: u8,
) -> PyResult<()> {
    let controller = get_system_controller()?;

    // 转换组件类型
    let comp_type = match component_type.to_lowercase().as_str() {
        "vector_store" => ComponentType::VectorStore,
        "pro_processor" => ComponentType::ProProcessor,
        "crawl4ai" => ComponentType::Crawl4Ai,
        _ => {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "Invalid component type: {component_type}"
            )));
        }
    };

    // 创建组件ID
    let component_id = ComponentId::new(comp_type, component_name);

    // 创建调整请求
    let request = AdjustmentRequest {
        component_id,
        adjustment_type: AdjustmentType::AdjustPriority,
        params: serde_json::json!({
            "priority": priority
        }),
    };

    // 发送调整请求（使用安全的运行时上下文）
    let result = py.detach(|| {
        block_on_async(async move {
            let response = controller.handle_adjustment_request(request).await;
            if response.success {
                Ok(())
            } else {
                Err(format!("Adjustment failed: {}", response.reason))
            }
        })
    });

    match result {
        Ok(_) => Ok(()),
        Err(e) => Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
            "Failed to adjust priority: {e}"
        ))),
    }
}

/// 检查系统是否应该终止
///
/// Returns:
///     bool: 如果系统应该终止则返回True，否则返回False
#[pyfunction]
pub fn should_terminate(py: Python) -> PyResult<bool> {
    let controller = get_system_controller()?;

    // 检查终止标志（使用安全的运行时上下文）
    let terminate =
        py.detach(|| block_on_async(async move { controller.should_terminate().await }));

    Ok(terminate)
}

/// 获取系统状态
///
/// Returns:
///     Dict: 系统状态信息
#[pyfunction]
pub fn get_system_status(py: Python) -> PyResult<Py<PyDict>> {
    // 获取系统状态（使用安全的运行时上下文）
    let status = py.detach(|| {
        // 直接获取控制器，不需要处理错误（内部已处理）
        let controller = get_global_system_controller();
        block_on_async(async move { controller.get_system_status().await })
    });

    // 检查终止标志（使用安全的运行时上下文）
    let terminate = py.detach(|| {
        // 直接获取控制器，不需要处理错误（内部已处理）
        let controller = get_global_system_controller();
        block_on_async(async move { controller.should_terminate().await })
    });

    // 转换为Python字典
    let dict = PyDict::new(py);

    // 资源状态
    let resource_dict = PyDict::new(py);
    resource_dict.set_item("cpu_usage", status.resource_status.cpu_usage)?;
    resource_dict.set_item("memory_usage", status.resource_status.memory_usage)?;
    resource_dict.set_item("disk_io_usage", status.resource_status.disk_io_usage)?;
    resource_dict.set_item("network_io_usage", status.resource_status.network_io_usage)?;
    resource_dict.set_item("available_memory", status.resource_status.available_memory)?;
    resource_dict.set_item("available_disk", status.resource_status.available_disk)?;
    resource_dict.set_item("total_disk", status.resource_status.total_disk)?;
    resource_dict.set_item(
        "disk_usage_percent",
        status.resource_status.disk_usage_percent,
    )?;
    resource_dict.set_item("load_avg_1", status.resource_status.load_avg_1)?;
    resource_dict.set_item("load_avg_5", status.resource_status.load_avg_5)?;
    resource_dict.set_item("load_avg_15", status.resource_status.load_avg_15)?;

    dict.set_item("resource_status", resource_dict)?;
    dict.set_item("running", status.controller_running)?;
    dict.set_item("should_terminate", terminate)?;

    Ok(dict.into())
}

/// 设置系统资源阈值
///
/// Args:
///     threshold: 资源阈值 (0.0-1.0)
#[pyfunction]
pub fn set_resource_threshold(_py: Python, _threshold: f64) -> PyResult<()> {
    // 注意：当前SystemController不支持动态修改配置，这里返回成功但不做实际操作
    // 后续可以扩展SystemController支持动态配置
    Ok(())
}

/// 调整Crawl4AI的并发数量
///
/// 方便的快捷函数，专门用于调整Crawl4AI的并发数量
///
/// Args:
///     concurrency: 新的并发数量
#[pyfunction]
pub fn adjust_crawl4ai_concurrency(py: Python, concurrency: usize) -> PyResult<()> {
    adjust_component_concurrency(py, "crawl4ai", "default", concurrency)
}

/// 调整Pro处理器的并发数量
///
/// 方便的快捷函数，专门用于调整Pro处理器的并发数量
///
/// Args:
///     concurrency: 新的并发数量
#[pyfunction]
pub fn adjust_pro_processor_concurrency(py: Python, concurrency: usize) -> PyResult<()> {
    adjust_component_concurrency(py, "pro_processor", "default", concurrency)
}

/// 启动系统控制器守护进程
///
/// 在后台启动系统控制器，持续监控资源使用和动态调整组件并发。
/// 确保 SeeSea 高性能稳定运行。
#[pyfunction]
pub fn start_system_controller_daemon(_py: Python) -> PyResult<()> {
    use tracing::info;

    // 获取全局系统控制器（会自动启动，使用全局运行时）
    // 此调用是安全的，即使在没有 Tokio 运行时的上下文中
    let controller = get_global_system_controller();

    // 启动守护进程监控循环
    let controller_clone = controller.clone();
    std::thread::spawn(move || {
        // 使用全局运行时而不是创建新的
        let runtime = get_or_create_runtime();

        runtime.block_on(async move {
            // 启动系统控制器（如果尚未运行）
            if !controller_clone.is_running().await {
                controller_clone.start().await;
            }

            info!("系统调控中心守护进程已启动");

            // 持续运行，监控系统状态
            loop {
                tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;

                // 检查是否需要终止
                if controller_clone.should_terminate().await {
                    info!("系统调控中心守护进程收到终止信号");
                    break;
                }
            }
        });
    });

    Ok(())
}

/// 停止系统控制器守护进程
#[pyfunction]
pub fn stop_system_controller_daemon(py: Python) -> PyResult<()> {
    let controller = get_system_controller()?;

    // 使用安全的运行时上下文
    py.detach(|| {
        block_on_async(async move {
            controller.set_should_terminate(true).await;
            controller.stop().await;
        })
    });

    Ok(())
}

/// 导出Python模块
#[pymodule]
pub fn sys_controller(_py: Python, m: &Bound<'_, PyModule>) -> PyResult<()> {
    // 初始化函数
    m.add_function(wrap_pyfunction!(init_system_controller, m)?)?;

    // 组件管理函数
    m.add_function(wrap_pyfunction!(register_component, m)?)?;

    // 调整函数
    m.add_function(wrap_pyfunction!(adjust_component_concurrency, m)?)?;
    m.add_function(wrap_pyfunction!(adjust_component_priority, m)?)?;

    // 状态查询函数
    m.add_function(wrap_pyfunction!(get_system_status, m)?)?;
    m.add_function(wrap_pyfunction!(should_terminate, m)?)?;

    // 资源配置函数
    m.add_function(wrap_pyfunction!(set_resource_threshold, m)?)?;

    // 快捷函数
    m.add_function(wrap_pyfunction!(adjust_crawl4ai_concurrency, m)?)?;
    m.add_function(wrap_pyfunction!(adjust_pro_processor_concurrency, m)?)?;

    // 守护进程函数
    m.add_function(wrap_pyfunction!(start_system_controller_daemon, m)?)?;
    m.add_function(wrap_pyfunction!(stop_system_controller_daemon, m)?)?;

    Ok(())
}
