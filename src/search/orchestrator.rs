//! 搜索编排器模块
//!
//! 负责协调多个搜索引擎的并发执行、超时控制、错误隔离和缓存集成

use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::time::timeout;

use crate::cache::CacheInterface;
use crate::derive::{SearchEngine, SearchQuery, SearchResult};
use crate::error::search_error;
use crate::net::NetworkInterface;

use super::types::{SearchRequest, SearchResponse, SearchConfig};

/// 搜索编排器
/// 
/// 负责管理多个搜索引擎的并发执行，提供超时控制、错误隔离和缓存支持
pub struct SearchOrchestrator {
    /// 搜索配置
    config: SearchConfig,
    /// 网络接口
    network: Arc<NetworkInterface>,
    /// 缓存接口
    cache: Arc<RwLock<CacheInterface>>,
    /// 已注册的搜索引擎
    engines: Vec<Box<dyn SearchEngine + Send + Sync>>,
    /// 统计信息
    stats: Arc<OrchestratorStats>,
}

/// 编排器统计信息
#[derive(Debug, Default)]
pub struct OrchestratorStats {
    /// 总搜索次数
    pub total_searches: std::sync::atomic::AtomicU64,
    /// 缓存命中次数
    pub cache_hits: std::sync::atomic::AtomicU64,
    /// 缓存未命中次数
    pub cache_misses: std::sync::atomic::AtomicU64,
    /// 引擎失败次数
    pub engine_failures: std::sync::atomic::AtomicU64,
    /// 超时次数
    pub timeouts: std::sync::atomic::AtomicU64,
}

impl SearchOrchestrator {
    /// 创建新的搜索编排器
    ///
    /// # Arguments
    ///
    /// * `config` - 搜索配置
    /// * `network` - 网络接口
    /// * `cache` - 缓存接口
    ///
    /// # Returns
    ///
    /// 返回搜索编排器实例
    pub fn new(
        config: SearchConfig,
        network: Arc<NetworkInterface>,
        cache: Arc<RwLock<CacheInterface>>,
    ) -> Self {
        Self {
            config,
            network,
            cache,
            engines: Vec::new(),
            stats: Arc::new(OrchestratorStats::default()),
        }
    }

    /// 注册搜索引擎
    ///
    /// # Arguments
    ///
    /// * `engine` - 搜索引擎实例
    pub fn register_engine(&mut self, engine: Box<dyn SearchEngine + Send + Sync>) {
        self.engines.push(engine);
    }

    /// 执行搜索
    ///
    /// # Arguments
    ///
    /// * `request` - 搜索请求
    ///
    /// # Returns
    ///
    /// 返回搜索响应或错误
    pub async fn search(
        &self,
        request: &SearchRequest,
    ) -> Result<SearchResponse, Box<dyn std::error::Error + Send + Sync>> {
        use std::sync::atomic::Ordering;

        // 增加搜索计数
        self.stats.total_searches.fetch_add(1, Ordering::Relaxed);

        // 检查缓存
        if self.config.enable_cache {
            if let Some(cached) = self.check_cache(&request.query).await? {
                self.stats.cache_hits.fetch_add(1, Ordering::Relaxed);
                return Ok(cached);
            }
            self.stats.cache_misses.fetch_add(1, Ordering::Relaxed);
        }

        // 并发执行所有引擎
        let results = self.execute_engines(&request.query).await?;

        // 创建响应
        let response = SearchResponse {
            query: request.query.clone(),
            results,
            total_count: 0, // 将在聚合时计算
            engines_used: self.engines.iter().map(|e| e.info().name.clone()).collect(),
            query_time_ms: 0, // 将在聚合时计算
            cached: false,
        };

        // 存入缓存
        if self.config.enable_cache {
            self.store_cache(&request.query, &response).await?;
        }

        Ok(response)
    }

    /// 检查缓存
    async fn check_cache(
        &self,
        query: &SearchQuery,
    ) -> Result<Option<SearchResponse>, Box<dyn std::error::Error + Send + Sync>> {
        let cache = self.cache.read().await;
        let results_cache = cache.results();

        // 尝试从缓存获取每个引擎的结果
        let mut all_results = Vec::new();
        let mut engines_used = Vec::new();

        for engine in &self.engines {
            let engine_name = &engine.info().name;
            if let Some(result) = results_cache
                .get(query, engine_name)
                .map_err(|e| search_error(&format!("缓存读取失败: {}", e)))?
            {
                all_results.push(result);
                engines_used.push(engine_name.clone());
            }
        }

        if !all_results.is_empty() {
            Ok(Some(SearchResponse {
                query: query.clone(),
                results: all_results,
                total_count: all_results.iter().map(|r| r.items.len()).sum(),
                engines_used,
                query_time_ms: 0,
                cached: true,
            }))
        } else {
            Ok(None)
        }
    }

    /// 存储到缓存
    async fn store_cache(
        &self,
        query: &SearchQuery,
        response: &SearchResponse,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let cache = self.cache.write().await;
        let results_cache = cache.results();

        for (result, engine_name) in response.results.iter().zip(&response.engines_used) {
            results_cache
                .set(query, engine_name, result, None)
                .map_err(|e| search_error(&format!("缓存写入失败: {}", e)))?;
        }

        Ok(())
    }

    /// 并发执行所有引擎
    async fn execute_engines(
        &self,
        query: &SearchQuery,
    ) -> Result<Vec<SearchResult>, Box<dyn std::error::Error + Send + Sync>> {
        use std::sync::atomic::Ordering;

        let mut handles = Vec::new();

        for engine in &self.engines {
            let engine_info = engine.info().clone();
            let query_clone = query.clone();
            let timeout_duration = Duration::from_secs(self.config.timeout_secs);
            let stats = Arc::clone(&self.stats);

            // 为每个引擎创建一个异步任务
            let handle = tokio::spawn(async move {
                // 执行搜索并应用超时
                match timeout(timeout_duration, async {
                    // 实际执行搜索的逻辑将在具体引擎中实现
                    // 这里返回空结果作为占位符
                    Ok::<SearchResult, Box<dyn std::error::Error + Send + Sync>>(SearchResult {
                        query: query_clone.clone(),
                        items: Vec::new(),
                        engine: engine_info.name.clone(),
                        total_results: 0,
                        page: 1,
                        has_next_page: false,
                    })
                })
                .await
                {
                    Ok(Ok(result)) => Some(result),
                    Ok(Err(e)) => {
                        eprintln!("引擎 {} 搜索失败: {}", engine_info.name, e);
                        stats.engine_failures.fetch_add(1, Ordering::Relaxed);
                        None
                    }
                    Err(_) => {
                        eprintln!("引擎 {} 超时", engine_info.name);
                        stats.timeouts.fetch_add(1, Ordering::Relaxed);
                        None
                    }
                }
            });

            handles.push(handle);
        }

        // 等待所有任务完成
        let results = futures::future::join_all(handles).await;

        // 收集成功的结果
        let successful_results: Vec<SearchResult> = results
            .into_iter()
            .filter_map(|r| r.ok().flatten())
            .collect();

        if successful_results.is_empty() {
            return Err(search_error("所有引擎都失败了"));
        }

        Ok(successful_results)
    }

    /// 获取统计信息
    pub fn stats(&self) -> &OrchestratorStats {
        &self.stats
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cache::types::CacheImplConfig;
    use crate::net::types::NetworkConfig;

    #[test]
    fn test_orchestrator_creation() {
        let config = SearchConfig::default();
        let network = Arc::new(NetworkInterface::new(NetworkConfig::default()).unwrap());
        let cache = Arc::new(RwLock::new(
            CacheInterface::new(CacheImplConfig::default()).unwrap(),
        ));

        let orchestrator = SearchOrchestrator::new(config, network, cache);
        assert_eq!(orchestrator.engines.len(), 0);
    }

    #[tokio::test]
    async fn test_stats_increment() {
        let config = SearchConfig::default();
        let network = Arc::new(NetworkInterface::new(NetworkConfig::default()).unwrap());
        let cache = Arc::new(RwLock::new(
            CacheInterface::new(CacheImplConfig::default()).unwrap(),
        ));

        let orchestrator = SearchOrchestrator::new(config, network, cache);

        use std::sync::atomic::Ordering;
        let initial = orchestrator.stats.total_searches.load(Ordering::Relaxed);
        orchestrator
            .stats
            .total_searches
            .fetch_add(1, Ordering::Relaxed);
        let after = orchestrator.stats.total_searches.load(Ordering::Relaxed);

        assert_eq!(after, initial + 1);
    }

    #[test]
    fn test_orchestrator_stats_default() {
        let stats = OrchestratorStats::default();
        use std::sync::atomic::Ordering;

        assert_eq!(stats.total_searches.load(Ordering::Relaxed), 0);
        assert_eq!(stats.cache_hits.load(Ordering::Relaxed), 0);
        assert_eq!(stats.cache_misses.load(Ordering::Relaxed), 0);
        assert_eq!(stats.engine_failures.load(Ordering::Relaxed), 0);
        assert_eq!(stats.timeouts.load(Ordering::Relaxed), 0);
    }
}
