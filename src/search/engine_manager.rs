//! 搜索引擎管理器
//!
//! 负责管理搜索引擎的生命周期、状态和并发执行

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

use crate::derive::{SearchEngine, SearchQuery, SearchResult};
use crate::search::engines::*;

/// 引擎运行模式
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EngineMode {
    /// 配置模式：只使用配置的引擎
    Configured,
    /// 全局模式：使用所有实现的引擎
    Global,
}

/// 引擎状态
#[derive(Debug, Clone)]
pub struct EngineState {
    /// 引擎名称
    pub name: String,
    /// 是否启用
    pub enabled: bool,
    /// 是否临时禁用（网络问题）
    pub temporarily_disabled: bool,
    /// 禁用到期时间
    pub disabled_until: Option<Instant>,
    /// 连续失败次数
    pub consecutive_failures: u32,
    /// 总请求数
    pub total_requests: u64,
    /// 成功请求数
    pub successful_requests: u64,
    /// 失败请求数
    pub failed_requests: u64,
    /// 平均响应时间（毫秒）
    pub avg_response_time_ms: u64,
}

impl EngineState {
    /// 创建新的引擎状态
    pub fn new(name: String) -> Self {
        Self {
            name,
            enabled: true,
            temporarily_disabled: false,
            disabled_until: None,
            consecutive_failures: 0,
            total_requests: 0,
            successful_requests: 0,
            failed_requests: 0,
            avg_response_time_ms: 0,
        }
    }

    /// 检查引擎是否可用
    pub fn is_available(&self) -> bool {
        if !self.enabled {
            return false;
        }
        
        if self.temporarily_disabled {
            if let Some(until) = self.disabled_until {
                if Instant::now() < until {
                    return false;
                }
            }
        }
        
        true
    }

    /// 临时禁用引擎
    pub fn disable_temporarily(&mut self, duration: Duration) {
        self.temporarily_disabled = true;
        self.disabled_until = Some(Instant::now() + duration);
    }

    /// 重新启用引擎
    pub fn re_enable(&mut self) {
        self.temporarily_disabled = false;
        self.disabled_until = None;
        self.consecutive_failures = 0;
    }

    /// 记录成功请求
    pub fn record_success(&mut self, response_time_ms: u64) {
        self.total_requests += 1;
        self.successful_requests += 1;
        self.consecutive_failures = 0;
        
        // 更新平均响应时间
        if self.total_requests == 1 {
            self.avg_response_time_ms = response_time_ms;
        } else {
            self.avg_response_time_ms = 
                (self.avg_response_time_ms * (self.total_requests - 1) + response_time_ms) 
                / self.total_requests;
        }
    }

    /// 记录失败请求
    pub fn record_failure(&mut self) {
        self.total_requests += 1;
        self.failed_requests += 1;
        self.consecutive_failures += 1;
    }
}

/// 搜索引擎管理器
pub struct EngineManager {
    /// 运行模式
    mode: EngineMode,
    /// 配置的引擎列表
    configured_engines: Vec<String>,
    /// 引擎实例映射（使用 Arc 以支持并发）
    engines: HashMap<String, Arc<Box<dyn SearchEngine + Send + Sync>>>,
    /// 引擎状态
    states: Arc<RwLock<HashMap<String, EngineState>>>,
    /// 临时禁用时长（秒）
    temporary_disable_duration: u64,
    /// 连续失败阈值
    failure_threshold: u32,
    /// 共享的 HTTP 客户端（用于优化性能）
    shared_client: Option<Arc<crate::net::client::HttpClient>>,
}

impl EngineManager {
    /// 创建新的引擎管理器（自动使用共享HTTP客户端优化性能）
    ///
    /// # 参数
    ///
    /// * `mode` - 运行模式
    /// * `configured_engines` - 配置的引擎列表（配置模式下使用）
    ///
    /// # 返回
    ///
    /// 引擎管理器实例
    pub fn new(mode: EngineMode, configured_engines: Vec<String>) -> Self {
        // 创建优化的网络配置
        let mut network_config = crate::net::types::NetworkConfig::default();
        // 提高连接池大小以支持并发搜索
        network_config.pool.max_idle_connections = 200;  // 增加到200
        network_config.pool.max_connections_per_host = 20;  // 每个主机20个连接
        
        // 创建共享客户端
        let shared_client = Arc::new(
            crate::net::client::HttpClient::new(network_config)
                .expect("Failed to create shared HTTP client")
        );
        
        Self::with_shared_client(mode, configured_engines, shared_client)
    }

    /// 使用共享 HTTP 客户端创建新的引擎管理器（性能优化）
    ///
    /// 共享 HTTP 客户端允许所有引擎使用同一个连接池，显著提高性能：
    /// - 避免连接池碎片化
    /// - 减少内存使用
    /// - 提高连接复用率
    /// - 加快首次请求速度
    ///
    /// # 参数
    ///
    /// * `mode` - 运行模式
    /// * `configured_engines` - 配置的引擎列表
    /// * `shared_client` - 共享的 HTTP 客户端
    ///
    /// # 返回
    ///
    /// 引擎管理器实例
    pub fn with_shared_client(
        mode: EngineMode,
        configured_engines: Vec<String>,
        shared_client: Arc<crate::net::client::HttpClient>,
    ) -> Self {
        let mut manager = Self {
            mode,
            configured_engines,
            engines: HashMap::new(),
            states: Arc::new(RwLock::new(HashMap::new())),
            temporary_disable_duration: 300,
            failure_threshold: 3,
            shared_client: Some(shared_client),
        };
        
        manager.initialize_engines();
        manager
    }

    /// 初始化所有引擎
    fn initialize_engines(&mut self) {
        // 总是使用共享客户端创建引擎（性能最优）
        let client = Arc::clone(self.shared_client.as_ref()
            .expect("Shared client must be initialized"));
        
        self.register_engine("google", Box::new(GoogleEngine::with_client(Arc::clone(&client))));
        self.register_engine("bing", Box::new(BingEngine::with_client(Arc::clone(&client))));
        self.register_engine("duckduckgo", Box::new(DuckDuckGoEngine::with_client(Arc::clone(&client))));
        self.register_engine("yahoo", Box::new(YahooEngine::with_client(Arc::clone(&client))));
        self.register_engine("baidu", Box::new(BaiduEngine::with_client(Arc::clone(&client))));
        self.register_engine("yandex", Box::new(YandexEngine::with_client(Arc::clone(&client))));
        self.register_engine("brave", Box::new(BraveEngine::with_client(Arc::clone(&client))));
        self.register_engine("qwant", Box::new(QwantEngine::with_client(Arc::clone(&client))));
        self.register_engine("startpage", Box::new(StartpageEngine::with_client(Arc::clone(&client))));
        self.register_engine("mojeek", Box::new(MojeekEngine::with_client(Arc::clone(&client))));
        self.register_engine("search360", Box::new(Search360Engine::with_client(Arc::clone(&client))));
        self.register_engine("wikipedia", Box::new(WikipediaEngine::with_client(Arc::clone(&client))));
        self.register_engine("wikidata", Box::new(WikidataEngine::with_client(Arc::clone(&client))));
        self.register_engine("github", Box::new(GitHubEngine::with_client(Arc::clone(&client))));
        self.register_engine("stackoverflow", Box::new(StackOverflowEngine::with_client(Arc::clone(&client))));
        self.register_engine("unsplash", Box::new(UnsplashEngine::with_client(client)));
    }

    /// 注册引擎
    fn register_engine(&mut self, name: &str, engine: Box<dyn SearchEngine + Send + Sync>) {
        self.engines.insert(name.to_string(), Arc::new(engine));
    }

    /// 获取活跃的引擎列表
    ///
    /// # 返回
    ///
    /// 活跃的引擎名称列表
    pub async fn get_active_engines(&self) -> Vec<String> {
        let states = self.states.read().await;
        
        match self.mode {
            EngineMode::Configured => {
                // 配置模式：只返回配置的且可用的引擎
                self.configured_engines
                    .iter()
                    .filter(|name| {
                        if let Some(state) = states.get(*name) {
                            state.is_available()
                        } else {
                            true // 如果没有状态，默认可用
                        }
                    })
                    .cloned()
                    .collect()
            }
            EngineMode::Global => {
                // 全局模式：返回所有可用的引擎
                self.engines
                    .keys()
                    .filter(|name| {
                        if let Some(state) = states.get(*name) {
                            state.is_available()
                        } else {
                            true
                        }
                    })
                    .cloned()
                    .collect()
            }
        }
    }

    /// 并发搜索
    ///
    /// # 参数
    ///
    /// * `query` - 搜索查询
    ///
    /// # 返回
    ///
    /// 搜索结果映射（引擎名称 -> 结果）
    pub async fn search_concurrent(
        &self,
        query: &SearchQuery,
    ) -> HashMap<String, Result<SearchResult, String>> {
        let active_engines = self.get_active_engines().await;
        let mut tasks = Vec::new();
        
        for engine_name in active_engines {
            if let Some(engine) = self.engines.get(&engine_name) {
                let engine_clone = Arc::clone(engine);
                let engine_name_clone = engine_name.clone();
                let query_clone = query.clone();
                let states = Arc::clone(&self.states);
                let temp_disable_duration = self.temporary_disable_duration;
                let failure_threshold = self.failure_threshold;
                
                // 创建异步任务
                let task = tokio::spawn(async move {
                    let start_time = Instant::now();
                    let result = engine_clone.search(&query_clone).await;
                    let response_time_ms = start_time.elapsed().as_millis() as u64;
                    
                    // 更新引擎状态
                    let mut states_lock = states.write().await;
                    let state = states_lock
                        .entry(engine_name_clone.clone())
                        .or_insert_with(|| EngineState::new(engine_name_clone.clone()));
                    
                    match &result {
                        Ok(_) => {
                            state.record_success(response_time_ms);
                        }
                        Err(e) => {
                            let error_msg = e.to_string();
                            state.record_failure();
                            
                            // 检查是否为网络错误（非200响应）
                            if error_msg.contains("HTTP 错误") || 
                               error_msg.contains("status") ||
                               error_msg.contains("连接") ||
                               error_msg.contains("超时") {
                                // 网络错误：临时禁用引擎
                                if state.consecutive_failures >= failure_threshold {
                                    state.disable_temporarily(
                                        Duration::from_secs(temp_disable_duration)
                                    );
                                }
                            }
                        }
                    }
                    
                    (engine_name_clone, result.map_err(|e| e.to_string()))
                });
                
                tasks.push(task);
            }
        }
        
        // 等待所有任务完成
        let mut results = HashMap::new();
        for task in tasks {
            if let Ok((name, result)) = task.await {
                results.insert(name, result);
            }
        }
        
        results
    }

    /// 获取引擎统计信息
    ///
    /// # 返回
    ///
    /// 引擎状态映射
    pub async fn get_engine_stats(&self) -> HashMap<String, EngineState> {
        self.states.read().await.clone()
    }

    /// 手动启用引擎
    ///
    /// # 参数
    ///
    /// * `engine_name` - 引擎名称
    pub async fn enable_engine(&self, engine_name: &str) {
        let mut states = self.states.write().await;
        if let Some(state) = states.get_mut(engine_name) {
            state.enabled = true;
            state.re_enable();
        }
    }

    /// 手动禁用引擎
    ///
    /// # 参数
    ///
    /// * `engine_name` - 引擎名称
    pub async fn disable_engine(&self, engine_name: &str) {
        let mut states = self.states.write().await;
        if let Some(state) = states.get_mut(engine_name) {
            state.enabled = false;
        }
    }

    /// 获取运行模式
    pub fn get_mode(&self) -> EngineMode {
        self.mode
    }

    /// 设置运行模式
    pub fn set_mode(&mut self, mode: EngineMode) {
        self.mode = mode;
    }

    /// 获取配置的引擎列表
    pub fn get_configured_engines(&self) -> &[String] {
        &self.configured_engines
    }

    /// 设置配置的引擎列表
    pub fn set_configured_engines(&mut self, engines: Vec<String>) {
        self.configured_engines = engines;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_engine_state_creation() {
        let state = EngineState::new("test".to_string());
        assert_eq!(state.name, "test");
        assert!(state.enabled);
        assert!(!state.temporarily_disabled);
    }

    #[test]
    fn test_engine_state_availability() {
        let mut state = EngineState::new("test".to_string());
        assert!(state.is_available());
        
        state.enabled = false;
        assert!(!state.is_available());
        
        state.enabled = true;
        state.disable_temporarily(Duration::from_secs(60));
        assert!(!state.is_available());
    }

    #[test]
    fn test_engine_state_success_recording() {
        let mut state = EngineState::new("test".to_string());
        state.record_success(100);
        
        assert_eq!(state.total_requests, 1);
        assert_eq!(state.successful_requests, 1);
        assert_eq!(state.consecutive_failures, 0);
    }

    #[test]
    fn test_engine_state_failure_recording() {
        let mut state = EngineState::new("test".to_string());
        state.record_failure();
        
        assert_eq!(state.total_requests, 1);
        assert_eq!(state.failed_requests, 1);
        assert_eq!(state.consecutive_failures, 1);
    }

    #[tokio::test]
    async fn test_engine_manager_creation() {
        let manager = EngineManager::new(
            EngineMode::Global,
            vec![],
        );
        
        assert_eq!(manager.get_mode(), EngineMode::Global);
        assert_eq!(manager.engines.len(), 16); // 所有16个引擎都应该注册
    }

    #[tokio::test]
    async fn test_engine_manager_configured_mode() {
        let configured = vec!["google".to_string(), "bing".to_string()];
        let manager = EngineManager::new(
            EngineMode::Configured,
            configured.clone(),
        );
        
        let active = manager.get_active_engines().await;
        assert_eq!(active.len(), 2);
        assert!(active.contains(&"google".to_string()));
        assert!(active.contains(&"bing".to_string()));
    }

    #[tokio::test]
    async fn test_engine_manager_global_mode() {
        let manager = EngineManager::new(
            EngineMode::Global,
            vec![],
        );
        
        let active = manager.get_active_engines().await;
        assert_eq!(active.len(), 16); // 所有引擎都应该可用
    }
}
