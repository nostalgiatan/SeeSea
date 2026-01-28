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

//! Python引擎桥接模块
//!
//! 通过raming系统和事件机制与Python引擎进行通信。
//! Python创建引擎信息 -> Rust读取注册 -> Rust删除临时内存区域。
//! 搜索时通过事件系统和共享内存进行通信。

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use uuid::Uuid;

use seesea_derive::{
    EngineInfo, EngineStatus, EngineType, SearchEngine, SearchQuery, SearchResult,
};
use seesea_raming::{
    RamingEventType, RamingManager,
    events::{RamingEventData, RamingEventListener},
};

/// Python引擎信息结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PythonEngineInfo {
    pub name: String,
    pub r#type: String,
    pub description: String,
    pub version: String,
    pub author: String,
    pub python_class: String,
    pub py_engine: bool,
    pub capabilities: PythonEngineCapabilities,
    pub status: String,
    pub registered_at: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PythonEngineCapabilities {
    pub supports_pagination: bool,
    pub supports_language_filter: bool,
    pub supports_region_filter: bool,
    pub supports_time_range: bool,
    pub max_page_size: u32,
    pub default_page_size: u32,
}

/// Python引擎代理
///
/// 实现SearchEngine trait，通过raming系统与Python引擎通信
pub struct PythonEngineProxy {
    info: EngineInfo,
    python_info: PythonEngineInfo,
    raming: Arc<RamingManager>,
}

impl PythonEngineProxy {
    /// 创建新的Python引擎代理
    pub fn new(python_info: PythonEngineInfo, raming: Arc<RamingManager>) -> Self {
        let engine_type = match python_info.r#type.as_str() {
            "web" => EngineType::General,
            "news" => EngineType::News,
            "images" | "image" => EngineType::Image,
            "videos" | "video" => EngineType::Video,
            "academic" => EngineType::Academic,
            "code" => EngineType::Code,
            "shopping" => EngineType::Shopping,
            "music" => EngineType::Music,
            _ => EngineType::Custom,
        };

        let info = EngineInfo {
            name: python_info.name.clone(),
            engine_type,
            description: python_info.description.clone(),
            status: EngineStatus::Active,
            categories: vec!["python".to_string()],
            capabilities: seesea_derive::EngineCapabilities {
                result_types: vec![seesea_derive::ResultType::Web],
                supported_params: vec![],
                max_page_size: python_info.capabilities.max_page_size as usize,
                supports_pagination: python_info.capabilities.supports_pagination,
                supports_time_range: python_info.capabilities.supports_time_range,
                supports_language_filter: python_info.capabilities.supports_language_filter,
                supports_region_filter: python_info.capabilities.supports_region_filter,
                supports_safe_search: false,
                rate_limit: Some(30),
            },
            about: seesea_derive::types::AboutInfo {
                wikidata_id: None,
                official_api_documentation: None,
                use_official_api: false,
                require_api_key: false,
                results: format!("Python引擎: {}", python_info.python_class),
                website: None,
            },
            shortcut: None,
            timeout: Some(30),
            disabled: false,
            inactive: false,
            version: Some(python_info.version.clone()),
            last_checked: None,
            using_tor_proxy: false,
            display_error_messages: true,
            max_page: 50,
            tokens: vec![],
        };

        Self {
            info,
            python_info,
            raming,
        }
    }

    /// 获取 Python 引擎信息
    pub fn python_engine_info(&self) -> &PythonEngineInfo {
        &self.python_info
    }

    /// 通过raming系统发送搜索请求
    async fn send_search_request(
        &self,
        query: &SearchQuery,
    ) -> Result<SearchResult, Box<dyn std::error::Error + Send + Sync>> {
        let request_id = Uuid::new_v4().to_string();
        let request_region = format!("search_request_{}", request_id);
        let response_region = format!("search_response_{}", request_id);

        // 创建请求和响应内存区域
        self.raming
            .shared_memory()
            .create_segment(request_region.clone(), 1024 * 1024)?;
        self.raming
            .shared_memory()
            .create_segment(response_region.clone(), 1024 * 1024)?;

        // 构建搜索请求数据
        let search_request = serde_json::json!({
            "request_id": request_id,
            "engine_name": self.info.name,
            "query": query.query,
            "page": query.page,
            "page_size": query.page_size,
            "language": query.language,
            "region": query.region,
            "timestamp": std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs_f64(),
        });

        // 写入请求数据到共享内存
        let request_json = serde_json::to_string(&search_request)?;
        let request_segment = self
            .raming
            .shared_memory()
            .get_segment(&request_region)
            .ok_or("Request segment not found")?;
        request_segment.write(0, request_json.as_bytes())?;

        // 发布搜索请求事件，传递请求内存区域名称
        let event_payload = serde_json::json!({
            "request_region": request_region.clone(),
            "response_region": response_region.clone(),
            "engine_name": self.info.name.clone(),
        });

        let event = RamingEventData::new(
            RamingEventType::SearchRequest,
            "PythonEngineProxy".to_string(),
            event_payload,
        );

        self.raming.event_bus().publish(event).await?;

        // 等待响应（最多30秒）
        let response_data = self
            .wait_for_response(&response_region, Duration::from_secs(30))
            .await?;

        // 清理内存区域
        let _ = self.raming.shared_memory().delete_segment(&request_region);
        let _ = self.raming.shared_memory().delete_segment(&response_region);

        Ok(response_data)
    }

    /// 等待Python引擎的响应
    async fn wait_for_response(
        &self,
        response_region: &str,
        timeout: Duration,
    ) -> Result<SearchResult, Box<dyn std::error::Error + Send + Sync>> {
        let start_time = tokio::time::Instant::now();

        loop {
            if start_time.elapsed() > timeout {
                return Err("Search request timeout".into());
            }

            // 尝试从响应内存区域读取数据
            match self.raming.shared_memory().get_segment(response_region) {
                Some(segment) => {
                    match segment.read(0, segment.size()) {
                        Ok(data) => {
                            if !data.is_empty() {
                                // 解析响应数据
                                let response_str = String::from_utf8(data)?;
                                let response_json: serde_json::Value =
                                    serde_json::from_str(&response_str)?;

                                // 转换为SearchResult
                                let search_result = self.parse_search_response(response_json)?;
                                return Ok(search_result);
                            }
                        }
                        Err(_) => {
                            // 读取失败，继续等待
                        }
                    }
                }
                None => {
                    // 区域可能还没有数据，继续等待
                }
            }

            // 短暂等待后重试
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }

    /// 解析Python引擎的搜索响应
    fn parse_search_response(
        &self,
        response: serde_json::Value,
    ) -> Result<SearchResult, Box<dyn std::error::Error + Send + Sync>> {
        if !response
            .get("success")
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
        {
            let error_msg = response
                .get("error")
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown error from Python engine");
            return Err(error_msg.into());
        }

        let results = response
            .get("results")
            .and_then(|v| v.as_array())
            .unwrap_or(&vec![])
            .iter()
            .filter_map(|item| {
                let title = item.get("title")?.as_str()?.to_string();
                let url = item.get("url")?.as_str()?.to_string();
                let content = item
                    .get("content")
                    .or(item.get("snippet"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();

                Some(seesea_derive::SearchResultItem {
                    title,
                    url,
                    content,
                    display_url: item
                        .get("display_url")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    site_name: item
                        .get("site_name")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    result_type: seesea_derive::ResultType::Web,
                    thumbnail: item
                        .get("thumbnail")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    metadata: HashMap::new(),
                    published_date: None,
                    score: 1.0,
                    template: None,
                })
            })
            .collect();

        Ok(SearchResult {
            engine_name: self.info.name.clone(),
            total_results: response
                .get("total_results")
                .and_then(|v| v.as_u64())
                .map(|n| n as usize),
            elapsed_ms: response
                .get("elapsed_ms")
                .and_then(|v| v.as_u64())
                .unwrap_or(0),
            pagination: None,
            suggestions: response
                .get("suggestions")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|item| item.as_str().map(|s| s.to_string()))
                        .collect()
                })
                .unwrap_or_default(),
            metadata: HashMap::new(),
            items: results,
        })
    }
}

#[async_trait::async_trait]
impl SearchEngine for PythonEngineProxy {
    fn info(&self) -> &EngineInfo {
        &self.info
    }

    async fn search(
        &self,
        query: &SearchQuery,
    ) -> Result<SearchResult, Box<dyn std::error::Error + Send + Sync>> {
        self.send_search_request(query).await
    }

    async fn is_available(&self) -> bool {
        // 简单检查：Python引擎如果注册了就认为可用
        true
    }

    async fn health_check(
        &self,
    ) -> Result<seesea_derive::engine::EngineHealth, Box<dyn std::error::Error + Send + Sync>> {
        Ok(seesea_derive::engine::EngineHealth {
            status: EngineStatus::Active,
            response_time_ms: 0,
            error_message: None,
        })
    }

    fn validate_query(&self, query: &SearchQuery) -> Result<(), seesea_derive::ValidationError> {
        if query.query.is_empty() {
            return Err(seesea_derive::ValidationError {
                code: "EMPTY_QUERY".to_string(),
                message: "Query cannot be empty".to_string(),
                field: Some("query".to_string()),
            });
        }
        Ok(())
    }
}

/// Python引擎注册监听器
pub struct PythonEngineRegistry {
    engines: Arc<RwLock<HashMap<String, Arc<PythonEngineProxy>>>>,
    raming: Arc<RamingManager>,
}

impl PythonEngineRegistry {
    /// 创建新的Python引擎注册器
    pub fn new() -> Self {
        let raming = RamingManager::global().expect("RamingManager not initialized");
        Self {
            engines: Arc::new(RwLock::new(HashMap::new())),
            raming: Arc::new(raming),
        }
    }

    /// 启动引擎注册监听器
    pub async fn start_listening(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // 首先扫描现有的引擎信息区域（处理启动前已注册的引擎）
        println!("🔍 扫描现有的Python引擎...");
        if let Err(e) =
            Self::handle_engine_registration(Arc::clone(&self.engines), Arc::clone(&self.raming))
                .await
        {
            eprintln!("Warning: Failed to process existing engines: {}", e);
        }

        let engines = Arc::clone(&self.engines);
        let raming = Arc::clone(&self.raming);

        // 创建事件监听器并订阅引擎注册事件
        let listener = Arc::new(EngineRegistrationListener::new(engines, raming));

        self.raming
            .event_bus()
            .subscribe(RamingEventType::EngineRegister, listener)
            .await?;

        println!("✅ Python引擎注册监听器已启动，监听新的注册事件");
        Ok(())
    }

    /// 处理引擎注册事件
    async fn handle_engine_registration(
        engines: Arc<RwLock<HashMap<String, Arc<PythonEngineProxy>>>>,
        raming: Arc<RamingManager>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // 获取所有引擎信息内存区域
        let segments = raming.shared_memory().list_segments();
        let regions: Vec<String> = segments.iter().map(|s| s.name.clone()).collect();

        for region_name in regions {
            if region_name.starts_with("engine_info_") {
                match Self::process_engine_registration(&region_name, &engines, &raming).await {
                    Ok(engine_name) => {
                        // 注册成功，删除临时内存区域
                        let _ = raming.shared_memory().delete_segment(&region_name);
                        println!("✅ 成功注册Python引擎: {}", engine_name);
                    }
                    Err(e) => {
                        eprintln!("❌ 引擎注册失败 {}: {}", region_name, e);
                        // 注册失败也删除内存区域，避免重复处理
                        let _ = raming.shared_memory().delete_segment(&region_name);
                    }
                }
            }
        }

        Ok(())
    }

    /// 处理单个引擎注册
    async fn process_engine_registration(
        region_name: &str,
        engines: &Arc<RwLock<HashMap<String, Arc<PythonEngineProxy>>>>,
        raming: &Arc<RamingManager>,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        // 读取引擎信息
        let segment = raming
            .shared_memory()
            .get_segment(region_name)
            .ok_or("Engine info segment not found")?;
        let data = segment.read(0, segment.size())?;
        let info_str = String::from_utf8(data)?;
        let python_info: PythonEngineInfo = serde_json::from_str(&info_str)?;

        // 验证是Python引擎
        if !python_info.py_engine {
            return Err("Not a Python engine".into());
        }

        // 创建引擎代理
        let proxy = Arc::new(PythonEngineProxy::new(
            python_info.clone(),
            Arc::clone(raming),
        ));

        // 添加到引擎注册表
        let mut engines_map = engines.write().await;
        engines_map.insert(python_info.name.clone(), proxy);

        Ok(python_info.name)
    }

    /// 获取已注册的Python引擎
    pub async fn get_engine(&self, name: &str) -> Option<Arc<PythonEngineProxy>> {
        let engines = self.engines.read().await;
        engines.get(name).cloned()
    }

    /// 获取所有已注册的Python引擎
    pub async fn list_engines(&self) -> Vec<String> {
        let engines = self.engines.read().await;
        engines.keys().cloned().collect()
    }

    /// 移除引擎
    pub async fn remove_engine(&self, name: &str) -> bool {
        let mut engines = self.engines.write().await;
        engines.remove(name).is_some()
    }
}

impl Default for PythonEngineRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Python引擎注册事件监听器
pub struct EngineRegistrationListener {
    engines: Arc<RwLock<HashMap<String, Arc<PythonEngineProxy>>>>,
    raming: Arc<RamingManager>,
}

impl EngineRegistrationListener {
    pub fn new(
        engines: Arc<RwLock<HashMap<String, Arc<PythonEngineProxy>>>>,
        raming: Arc<RamingManager>,
    ) -> Self {
        Self { engines, raming }
    }
}

#[async_trait::async_trait]
impl RamingEventListener for EngineRegistrationListener {
    async fn handle_raming_event(
        &self,
        _event: RamingEventData,
    ) -> seesea_raming::errors::RamingResult<()> {
        // 当接收到引擎注册事件时，扫描并注册新引擎
        if let Err(e) = PythonEngineRegistry::handle_engine_registration(
            Arc::clone(&self.engines),
            Arc::clone(&self.raming),
        )
        .await
        {
            eprintln!("Error handling engine registration: {}", e);
        }
        Ok(())
    }

    fn name(&self) -> &str {
        "EngineRegistrationListener"
    }

    fn supported_binding_types(&self) -> Vec<seesea_raming::types::BindingType> {
        vec![seesea_raming::types::BindingType::EventListener]
    }

    fn supports_event_type(&self, event_type: &RamingEventType) -> bool {
        matches!(event_type, RamingEventType::EngineRegister)
    }
}
