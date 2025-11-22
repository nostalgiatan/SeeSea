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

//! Python引擎注册模块
//!
//! 提供从Python端动态注册搜索引擎的功能

use pyo3::prelude::*;
use pyo3::types::PyDict;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;
use async_trait::async_trait;

use crate::derive::{SearchEngine, SearchQuery, SearchResult, EngineInfo, EngineType, EngineStatus, EngineCapabilities, ResultType};

/// 统一的tokio运行时辅助函数
///
/// 检测当前是否在tokio运行时上下文中，如果没有则创建一个临时的运行时来执行异步代码
fn execute_with_runtime<F, R>(future: F) -> PyResult<R>
where
    F: std::future::Future<Output = R> + Send + 'static,
    R: Send + 'static,
{
    match tokio::runtime::Handle::try_current() {
        Ok(handle) => {
            // 在运行时上下文中，直接执行
            Ok(handle.block_on(future))
        }
        Err(_) => {
            // 不在运行时上下文中，创建一个新的运行时来执行
            let rt = tokio::runtime::Runtime::new()
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                    format!("Failed to create tokio runtime: {}", e)
                ))?;
            Ok(rt.block_on(future))
        }
    }
}

/// Python引擎包装器
///
/// 实现SearchEngine trait，通过Python回调执行实际搜索
pub struct PythonEngineWrapper {
    info: EngineInfo,
    callback: Arc<RwLock<Option<Py<PyAny>>>>,
}

impl PythonEngineWrapper {
    /// 创建新的Python引擎包装器
    pub fn new(
        name: String,
        engine_type: EngineType,
        description: String,
        categories: Vec<String>,
    ) -> Self {
        Self {
            info: EngineInfo {
                name,
                engine_type,
                description,
                status: EngineStatus::Active,
                categories,
                capabilities: EngineCapabilities {
                    result_types: vec![ResultType::Web, ResultType::News, ResultType::Image, ResultType::Video],
                    supported_params: vec![],
                    max_page_size: 50,
                    supports_pagination: true,
                    supports_time_range: false,
                    supports_language_filter: false,
                    supports_region_filter: false,
                    supports_safe_search: false,
                    rate_limit: Some(30),
                },
                about: crate::derive::types::AboutInfo {
                    wikidata_id: None,
                    official_api_documentation: None,
                    use_official_api: false,
                    require_api_key: false,
                    results: String::new(),
                    website: None,
                },
                shortcut: None,
                timeout: Some(30),
                disabled: false,
                inactive: false,
                version: Some("1.0.0".to_string()),
                last_checked: None,
                using_tor_proxy: false,
                display_error_messages: true,
                max_page: 50,
                tokens: vec![],
            },
            callback: Arc::new(RwLock::new(None)),
        }
    }

    /// 设置Python回调函数
    pub async fn set_callback(&self, callback: Py<PyAny>) {
        let mut cb = self.callback.write().await;
        *cb = Some(callback);
    }
}

#[async_trait]
impl SearchEngine for PythonEngineWrapper {
    fn info(&self) -> &EngineInfo {
        &self.info
    }

    async fn search(&self, query: &SearchQuery) -> Result<SearchResult, Box<dyn std::error::Error + Send + Sync>> {
        let callback_guard = self.callback.read().await;
        
        if let Some(ref callback) = *callback_guard {
            // 调用Python回调函数
            let result = Python::attach(|py| -> PyResult<SearchResult> {
                let dict = PyDict::new(py);
                dict.set_item("query", &query.query)?;
                dict.set_item("page", query.page)?;
                dict.set_item("page_size", query.page_size)?;
                
                if let Some(ref lang) = query.language {
                    dict.set_item("language", lang)?;
                }
                if let Some(ref region) = query.region {
                    dict.set_item("region", region)?;
                }
                
                // 调用Python函数
                let py_result = callback.call1(py, (dict,))?;

                // 检查结果是否是协程，如果是则运行它
                let final_result = if py_result.getattr(py, "__await__").is_ok() {
                    // 这是一个协程，需要运行它
                    Python::attach(|py_inner| -> PyResult<Py<PyAny>> {
                        // 运行协程到完成
                        let coro = py_result.clone_ref(py_inner);

                        // 使用 asyncio.run() 来运行协程
                        let asyncio = py_inner.import("asyncio")?;
                        let run_result = asyncio.call_method1("run", (coro,))?;
                        Ok(run_result.into())
                    }).map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                        format!("Failed to run async callback: {}", e)
                    ))?
                } else {
                    py_result.into()
                };
                
                // 解析结果
                let result_dict = final_result.bind(py).cast::<PyDict>()?;
                
                // 提取结果列表
                let items = if let Ok(Some(results_list)) = result_dict.get_item("results") {
                    results_list.extract::<Vec<HashMap<String, String>>>()?
                        .into_iter()
                        .map(|item| {
                            crate::derive::SearchResultItem {
                                title: item.get("title").cloned().unwrap_or_default(),
                                url: item.get("url").cloned().unwrap_or_default(),
                                content: item.get("snippet").or(item.get("content")).cloned().unwrap_or_default(),
                                display_url: item.get("display_url").cloned(),
                                site_name: item.get("site_name").cloned(),
                                result_type: ResultType::Web,
                                thumbnail: item.get("thumbnail").cloned(),
                                metadata: HashMap::new(),
                                published_date: None,
                                score: 1.0,
                                template: None,
                            }
                        })
                        .collect()
                } else {
                    vec![]
                };
                
                Ok(SearchResult {
                    engine_name: self.info.name.clone(),
                    total_results: Some(items.len()),
                    elapsed_ms: 0,
                    pagination: None,
                    suggestions: vec![],
                    metadata: HashMap::new(),
                    items,
                })
            });
            
            result.map_err(|e| format!("Python callback error: {}", e).into())
        } else {
            Err("No callback registered for this Python engine".into())
        }
    }

    async fn is_available(&self) -> bool {
        let callback_guard = self.callback.read().await;
        callback_guard.is_some()
    }

    async fn health_check(&self) -> Result<crate::derive::engine::EngineHealth, Box<dyn std::error::Error + Send + Sync>> {
        Ok(crate::derive::engine::EngineHealth {
            status: if self.is_available().await {
                EngineStatus::Active
            } else {
                EngineStatus::Disabled
            },
            response_time_ms: 0,
            error_message: None,
        })
    }

    fn validate_query(&self, query: &SearchQuery) -> Result<(), crate::derive::types::ValidationError> {
        if query.query.is_empty() {
            return Err(crate::derive::types::ValidationError::EmptyQuery);
        }
        Ok(())
    }
}

/// Python引擎注册器 (内部使用，不暴露给Python)
///
/// 全局注册表只在Rust侧创建，Python侧只能通过函数访问
pub struct PyEngineRegistry {
    engines: Arc<RwLock<HashMap<String, Arc<PythonEngineWrapper>>>>,
}

impl PyEngineRegistry {
    /// 创建新的注册表（内部使用）
    pub fn new() -> Self {
        Self {
            engines: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 注册一个新的Python引擎（内部使用）
    pub async fn register_engine_internal(
        &self,
        name: String,
        engine_type: EngineType,
        description: String,
        categories: Vec<String>,
        callback: Py<PyAny>,
    ) -> Result<(), String> {
        let wrapper = Arc::new(PythonEngineWrapper::new(
            name.clone(),
            engine_type,
            description,
            categories,
        ));

        wrapper.set_callback(callback).await;

        let mut engines = self.engines.write().await;
        engines.insert(name, wrapper);

        Ok(())
    }

    /// 获取已注册的引擎列表（内部使用）
    pub async fn list_engines_internal(&self) -> Vec<String> {
        let engines = self.engines.read().await;
        engines.keys().cloned().collect()
    }

    /// 注销一个引擎（内部使用）
    pub async fn unregister_engine_internal(&self, name: &str) -> bool {
        let mut engines = self.engines.write().await;
        engines.remove(name).is_some()
    }

    /// 检查引擎是否已注册（内部使用）
    pub async fn has_engine_internal(&self, name: &str) -> bool {
        let engines = self.engines.read().await;
        engines.contains_key(name)
    }
}

impl PyEngineRegistry {
    /// 获取引擎实例（内部使用）
    pub async fn get_engine(&self, name: &str) -> Option<Arc<PythonEngineWrapper>> {
        let engines = self.engines.read().await;
        engines.get(name).cloned()
    }
    
    /// 获取所有已注册引擎的列表（同步版本，用于Rust调用）
    pub fn get_all_engines_sync(&self) -> Vec<(String, Arc<PythonEngineWrapper>)> {
        // 使用tokio的block_on来同步获取
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                let engines = self.engines.read().await;
                engines.iter().map(|(k, v)| (k.clone(), Arc::clone(v))).collect()
            })
        })
    }
}

/// 全局引擎注册表实例（只在Rust侧创建）
static GLOBAL_REGISTRY: once_cell::sync::Lazy<Arc<PyEngineRegistry>> =
    once_cell::sync::Lazy::new(|| Arc::new(PyEngineRegistry::new()));

/// 获取全局引擎注册表
pub fn get_global_registry() -> Arc<PyEngineRegistry> {
    Arc::clone(&GLOBAL_REGISTRY)
}

/// Python函数：注册一个新的Python引擎
///
/// # 参数
///
/// * `name` - 引擎名称
/// * `engine_type` - 引擎类型 ("web", "news", "images", "videos" 等)
/// * `description` - 引擎描述
/// * `categories` - 分类列表
/// * `callback` - Python搜索函数，接受查询参数dict，返回结果dict
///
/// # 返回
///
/// 成功返回 True，失败抛出异常
#[pyfunction]
pub fn register_engine(
    name: String,
    engine_type: String,
    description: String,
    categories: Vec<String>,
    callback: Py<PyAny>,
) -> PyResult<bool> {
    let registry = get_global_registry();
    
    let engine_type_enum = match engine_type.to_lowercase().as_str() {
        "general" | "web" => EngineType::General,
        "news" => EngineType::News,
        "images" | "image" => EngineType::Image,
        "videos" | "video" => EngineType::Video,
        "academic" => EngineType::Academic,
        "code" => EngineType::Code,
        "shopping" => EngineType::Shopping,
        "music" => EngineType::Music,
        "custom" => EngineType::Custom,
        _ => EngineType::General,
    };
    
    execute_with_runtime(async move {
        registry.register_engine_internal(
            name,
            engine_type_enum,
            description,
            categories,
            callback,
        ).await.map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e))?;
        Ok(true)
    })?
}

/// Python函数：获取已注册的引擎列表
#[pyfunction]
pub fn list_engines() -> PyResult<Vec<String>> {
    let registry = get_global_registry();
    execute_with_runtime(async move {
        Ok(registry.list_engines_internal().await)
    })?
}

/// Python函数：注销一个引擎
#[pyfunction]
pub fn unregister_engine(name: String) -> PyResult<bool> {
    let registry = get_global_registry();
    execute_with_runtime(async move {
        Ok(registry.unregister_engine_internal(&name).await)
    })?
}

/// Python函数：检查引擎是否已注册
#[pyfunction]
pub fn has_engine(name: String) -> PyResult<bool> {
    let registry = get_global_registry();
    execute_with_runtime(async move {
        Ok(registry.has_engine_internal(&name).await)
    })?
}

/// 从Python全局注册表获取引擎（同步版本，用于SearchInterface）
///
/// 这个函数尝试从Python全局注册表获取引擎实例
pub fn try_get_python_engine_sync(name: &str) -> Option<Arc<PythonEngineWrapper>> {
    let registry = get_global_registry();
    
    // 尝试在当前运行时中执行，如果没有运行时则创建临时运行时
    match tokio::runtime::Handle::try_current() {
        Ok(_) => {
            // 有运行时，使用 block_in_place
            tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current().block_on(async {
                    registry.get_engine(name).await
                })
            })
        }
        Err(_) => {
            // 没有运行时，创建一个新的
            if let Ok(rt) = tokio::runtime::Runtime::new() {
                rt.block_on(async {
                    registry.get_engine(name).await
                })
            } else {
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_python_engine_creation() {
        let wrapper = PythonEngineWrapper::new(
            "test_engine".to_string(),
            EngineType::Web,
            "Test engine".to_string(),
            vec!["test".to_string()],
        );
        
        assert_eq!(wrapper.info().name, "test_engine");
        assert_eq!(wrapper.info().description, "Test engine");
    }

    #[tokio::test]
    async fn test_registry_operations() {
        let registry = PyEngineRegistry::new();
        
        // 测试初始状态
        let engines = registry.list_engines().unwrap();
        assert_eq!(engines.len(), 0);
        
        // 注意：实际的注册测试需要Python环境
    }
}
