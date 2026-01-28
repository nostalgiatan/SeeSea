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

//! Python bindings for SeeSea Raming系统
//!
//! 提供函数式API来访问Raming系统的共享内存和事件通知功能。
//! 不提供类实例化，所有功能通过全局函数访问。
//! 数据通过共享内存交换，回调仅用于事件通知。

use pyo3::prelude::*;
use std::collections::HashMap;

use seesea_raming::{
    BindingType, RamingEventData, RamingEventListener, RamingEventType, RamingManager, RamingResult,
};
use std::sync::Arc;

/// Python事件监听器包装器
struct PythonEventListener {
    name: String,
    event_type: RamingEventType,
    callback: Py<PyAny>,
}

impl PythonEventListener {
    fn new(name: String, event_type: RamingEventType, callback: Py<PyAny>) -> Self {
        Self {
            name,
            event_type,
            callback,
        }
    }
}

#[async_trait::async_trait]
impl RamingEventListener for PythonEventListener {
    async fn handle_raming_event(&self, _event: RamingEventData) -> RamingResult<()> {
        // 调用Python回调函数（无参数，仅通知）
        Python::attach(|py| {
            if let Err(e) = self.callback.call0(py) {
                eprintln!("Python event notification callback error: {}", e);
            }
        });
        Ok(())
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn supported_binding_types(&self) -> Vec<BindingType> {
        vec![BindingType::EventListener]
    }

    fn supports_event_type(&self, event_type: &RamingEventType) -> bool {
        *event_type == self.event_type
    }
}

/// 写入数据到共享内存区域
///
/// # 参数
///
/// * `region_name` - 内存区域名称
/// * `data` - 要写入的数据（字节串）
///
/// # 返回值
///
/// 成功返回True，失败返回False
#[pyfunction]
pub fn raming_write_memory(region_name: String, data: Vec<u8>) -> PyResult<bool> {
    let raming = RamingManager::global().expect("RamingManager not initialized");

    match raming.shared_memory().get_segment(&region_name) {
        Some(segment) => match segment.write(0, &data) {
            Ok(_) => Ok(true),
            Err(e) => {
                eprintln!(
                    "Failed to write to shared memory region {}: {}",
                    region_name, e
                );
                Ok(false)
            }
        },
        None => {
            eprintln!("Shared memory region {} not found", region_name);
            Ok(false)
        }
    }
}

/// 从共享内存区域读取数据
///
/// # 参数
///
/// * `region_name` - 内存区域名称
///
/// # 返回值
///
/// 成功返回字节串数据，失败返回None
#[pyfunction]
pub fn raming_read_memory(region_name: String) -> PyResult<Option<Vec<u8>>> {
    let raming = RamingManager::global().expect("RamingManager not initialized");

    match raming.shared_memory().get_segment(&region_name) {
        Some(segment) => {
            let size = segment.size();
            match segment.read(0, size) {
                Ok(data) => Ok(Some(data)),
                Err(e) => {
                    eprintln!(
                        "Failed to read from shared memory region {}: {}",
                        region_name, e
                    );
                    Ok(None)
                }
            }
        }
        None => {
            eprintln!("Shared memory region {} not found", region_name);
            Ok(None)
        }
    }
}

/// 创建共享内存区域
///
/// # 参数
///
/// * `region_name` - 内存区域名称
/// * `size` - 区域大小（字节）
///
/// # 返回值
///
/// 成功返回True，失败返回False
#[pyfunction]
pub fn raming_create_memory_region(region_name: String, size: usize) -> PyResult<bool> {
    let raming = RamingManager::global().expect("RamingManager not initialized");

    match raming.shared_memory().create_segment(region_name, size) {
        Ok(_) => Ok(true),
        Err(e) => {
            eprintln!("Failed to create shared memory region: {}", e);
            Ok(false)
        }
    }
}

/// 删除共享内存区域
///
/// # 参数
///
/// * `region_name` - 内存区域名称
///
/// # 返回值
///
/// 成功返回True，失败返回False
#[pyfunction]
pub fn raming_delete_memory_region(region_name: String) -> PyResult<bool> {
    let raming = RamingManager::global().expect("RamingManager not initialized");

    match raming.shared_memory().delete_segment(&region_name) {
        Ok(_) => Ok(true),
        Err(e) => {
            eprintln!(
                "Failed to delete shared memory region {}: {}",
                region_name, e
            );
            Ok(false)
        }
    }
}

/// 发布事件通知（不传输数据）
///
/// # 参数
///
/// * `event_type` - 事件类型字符串
/// * `memory_region` - 相关的内存区域名称（可选）
///
/// # 返回值
///
/// 成功返回True，失败返回False
#[pyfunction]
#[pyo3(signature = (event_type, memory_region=None))]
pub fn raming_publish_event(event_type: String, memory_region: Option<String>) -> PyResult<bool> {
    // 解析事件类型
    let raming_event_type = match event_type.as_str() {
        "search_request" => RamingEventType::SearchRequest,
        "search_response" => RamingEventType::SearchResponse,
        "engine_register" => RamingEventType::EngineRegister,
        "memory_share" => RamingEventType::MemoryShare,
        "memory_update" => RamingEventType::MemoryUpdate,
        "memory_delete" => RamingEventType::MemoryDelete,
        "binding_created" => RamingEventType::BindingCreated,
        "binding_deleted" => RamingEventType::BindingDeleted,
        "system" => RamingEventType::System,
        _ => {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "Unknown event type: {}",
                event_type
            )));
        }
    };

    let raming = RamingManager::global().expect("RamingManager not initialized");

    // 创建事件数据
    let payload = match memory_region {
        Some(region) => serde_json::json!({"memory_region": region}),
        None => serde_json::json!({}),
    };

    let event_data =
        RamingEventData::new(raming_event_type, "python_bindings".to_string(), payload);

    // 使用tokio运行时发布事件
    let runtime = tokio::runtime::Handle::current();
    match runtime.block_on(raming.event_bus().publish(event_data)) {
        Ok(_) => Ok(true),
        Err(e) => {
            eprintln!("Failed to publish raming event: {}", e);
            Ok(false)
        }
    }
}

/// 订阅事件通知（回调函数无参数）
///
/// # 参数
///
/// * `event_type` - 事件类型字符串
/// * `subscriber_id` - 订阅者ID
/// * `callback` - Python回调函数（无参数）
///
/// # 返回值
///
/// 成功返回True，失败返回False
#[pyfunction]
pub fn raming_subscribe_event(
    event_type: String,
    subscriber_id: String,
    callback: Py<PyAny>,
) -> PyResult<bool> {
    // 解析事件类型
    let raming_event_type = match event_type.as_str() {
        "search_request" => RamingEventType::SearchRequest,
        "search_response" => RamingEventType::SearchResponse,
        "engine_register" => RamingEventType::EngineRegister,
        "memory_share" => RamingEventType::MemoryShare,
        "memory_update" => RamingEventType::MemoryUpdate,
        "memory_delete" => RamingEventType::MemoryDelete,
        "binding_created" => RamingEventType::BindingCreated,
        "binding_deleted" => RamingEventType::BindingDeleted,
        "system" => RamingEventType::System,
        _ => {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "Unknown event type: {}",
                event_type
            )));
        }
    };

    let raming = RamingManager::global().expect("RamingManager not initialized");

    // 创建Python事件监听器包装器
    let listener_name = format!("python_listener_{}", subscriber_id);
    let listener = Arc::new(PythonEventListener::new(
        listener_name,
        raming_event_type.clone(),
        callback,
    ));

    // 使用tokio运行时订阅事件
    let runtime = tokio::runtime::Handle::current();
    match runtime.block_on(raming.event_bus().subscribe(raming_event_type, listener)) {
        Ok(_) => Ok(true),
        Err(e) => {
            eprintln!("Failed to subscribe to raming event: {}", e);
            Ok(false)
        }
    }
}

/// 取消订阅事件通知
///
/// # 参数
///
/// * `event_type` - 事件类型字符串
/// * `subscriber_id` - 订阅者ID
///
/// # 返回值
///
/// 成功返回True，失败返回False
#[pyfunction]
pub fn raming_unsubscribe_event(event_type: String, subscriber_id: String) -> PyResult<bool> {
    // 解析事件类型
    let raming_event_type = match event_type.as_str() {
        "search_request" => RamingEventType::SearchRequest,
        "search_response" => RamingEventType::SearchResponse,
        "engine_register" => RamingEventType::EngineRegister,
        "memory_share" => RamingEventType::MemoryShare,
        "memory_update" => RamingEventType::MemoryUpdate,
        "memory_delete" => RamingEventType::MemoryDelete,
        "binding_created" => RamingEventType::BindingCreated,
        "binding_deleted" => RamingEventType::BindingDeleted,
        "system" => RamingEventType::System,
        _ => {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "Unknown event type: {}",
                event_type
            )));
        }
    };

    let raming = RamingManager::global().expect("RamingManager not initialized");

    // 构造监听器名称（与订阅时使用的相同）
    let listener_name = format!("python_listener_{}", subscriber_id);

    // 取消订阅
    match raming
        .event_bus()
        .unsubscribe(&raming_event_type, &listener_name)
    {
        Ok(_) => Ok(true),
        Err(e) => {
            eprintln!("Failed to unsubscribe from raming event: {}", e);
            Ok(false)
        }
    }
}

/// 获取Raming系统状态信息
///
/// # 返回值
///
/// 返回包含系统状态的Python字典
#[pyfunction]
pub fn raming_get_system_status() -> PyResult<HashMap<String, String>> {
    let raming = RamingManager::global().expect("RamingManager not initialized");

    let mut status = HashMap::new();

    // 获取内存使用情况
    let memory_stats = raming.get_memory_stats();
    status.insert(
        "total_memory".to_string(),
        memory_stats.total_memory.to_string(),
    );
    status.insert(
        "active_segments".to_string(),
        memory_stats.active_segments.to_string(),
    );
    status.insert(
        "total_segments".to_string(),
        memory_stats.total_segments.to_string(),
    );
    status.insert(
        "cache_hit_rate".to_string(),
        memory_stats.cache_hit_rate.to_string(),
    );
    status.insert(
        "avg_access_time_ms".to_string(),
        memory_stats.avg_access_time_ms.to_string(),
    );

    // 获取事件总线状态
    let event_stats = raming.get_event_stats();
    status.insert(
        "published_events".to_string(),
        event_stats.published_events.to_string(),
    );
    status.insert(
        "active_listeners".to_string(),
        event_stats.active_listeners.to_string(),
    );

    Ok(status)
}

/// 获取可用的Raming事件类型列表
///
/// # 返回值
///
/// 返回事件类型字符串列表
#[pyfunction]
pub fn raming_get_event_types() -> PyResult<Vec<String>> {
    Ok(vec![
        "search_request".to_string(),
        "search_response".to_string(),
        "engine_register".to_string(),
        "memory_share".to_string(),
        "memory_update".to_string(),
        "memory_delete".to_string(),
        "binding_created".to_string(),
        "binding_deleted".to_string(),
        "system".to_string(),
    ])
}

/// 获取共享内存区域列表
///
/// # 返回值
///
/// 返回内存区域名称列表
#[pyfunction]
pub fn raming_list_memory_regions() -> PyResult<Vec<String>> {
    let raming = RamingManager::global().expect("RamingManager not initialized");

    let segment_infos = raming.shared_memory().list_segments();
    let segment_names: Vec<String> = segment_infos.into_iter().map(|info| info.name).collect();
    Ok(segment_names)
}

/// 获取共享内存区域信息
///
/// # 参数
///
/// * `region_name` - 内存区域名称
///
/// # 返回值
///
/// 返回区域信息字典（大小、状态等）
#[pyfunction]
pub fn raming_get_memory_region_info(
    region_name: String,
) -> PyResult<Option<HashMap<String, String>>> {
    let raming = RamingManager::global().expect("RamingManager not initialized");

    if let Some(segment) = raming.shared_memory().get_segment(&region_name) {
        let info = segment.info();
        let mut result = HashMap::new();
        result.insert("name".to_string(), info.name);
        result.insert("size".to_string(), info.size.to_string());
        result.insert("access_count".to_string(), info.access_count.to_string());
        result.insert("ref_count".to_string(), info.ref_count.to_string());
        result.insert("created_at".to_string(), info.created_at.to_string());
        Ok(Some(result))
    } else {
        eprintln!("Memory region {} not found", region_name);
        Ok(None)
    }
}
