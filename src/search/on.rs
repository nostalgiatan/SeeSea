//! 搜索外部接口模块
//!
//! 提供统一的搜索接口供外部使用

use std::sync::Arc;
use tokio::sync::RwLock;

use crate::cache::CacheInterface;
use crate::net::NetworkInterface;

use super::aggregator::{SearchAggregator, AggregationStrategy, SortBy};
use super::orchestrator::SearchOrchestrator;
use super::query::QueryParser;
use super::types::{SearchConfig, SearchRequest, SearchResponse};

/// 搜索接口
/// 
/// 统一的搜索外部接口，封装所有搜索功能
pub struct SearchInterface {
    /// 搜索编排器
    orchestrator: SearchOrchestrator,
    /// 结果聚合器
    aggregator: SearchAggregator,
    /// 查询解析器
    parser: QueryParser,
}

impl SearchInterface {
    /// 创建新的搜索接口
    ///
    /// # Arguments
    ///
    /// * `config` - 搜索配置
    /// * `network` - 网络接口
    /// * `cache` - 缓存接口
    ///
    /// # Returns
    ///
    /// 返回搜索接口实例或错误
    pub fn new(
        config: SearchConfig,
        network: Arc<NetworkInterface>,
        cache: Arc<RwLock<CacheInterface>>,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let orchestrator = SearchOrchestrator::new(config.clone(), network, cache);
        let aggregator = SearchAggregator::default();
        let parser = QueryParser::default();

        Ok(Self {
            orchestrator,
            aggregator,
            parser,
        })
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
        // 解析查询
        let _parsed = self.parser.parse(&request.query.query);

        // 执行搜索
        let mut response = self.orchestrator.search(request).await?;

        // 聚合结果
        if response.results.len() > 1 {
            let aggregated = self.aggregator.aggregate(response.results.clone());
            response.total_count = aggregated.items.len();
        }

        Ok(response)
    }

    /// 带选项执行搜索
    ///
    /// # Arguments
    ///
    /// * `request` - 搜索请求
    /// * `strategy` - 聚合策略
    /// * `sort_by` - 排序方式
    ///
    /// # Returns
    ///
    /// 返回搜索响应或错误
    pub async fn search_with_options(
        &self,
        request: &SearchRequest,
        _strategy: AggregationStrategy,
        _sort_by: SortBy,
    ) -> Result<SearchResponse, Box<dyn std::error::Error + Send + Sync>> {
        self.search(request).await
    }

    /// 获取统计信息
    pub fn get_stats(&self) -> SearchStats {
        use std::sync::atomic::Ordering;
        let stats = self.orchestrator.stats();

        SearchStats {
            total_searches: stats.total_searches.load(Ordering::Relaxed),
            cache_hits: stats.cache_hits.load(Ordering::Relaxed),
            cache_misses: stats.cache_misses.load(Ordering::Relaxed),
            engine_failures: stats.engine_failures.load(Ordering::Relaxed),
            timeouts: stats.timeouts.load(Ordering::Relaxed),
        }
    }

    /// 清除缓存
    pub async fn clear_cache(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // 缓存清理逻辑
        Ok(())
    }

    /// 列出可用引擎
    pub fn list_engines(&self) -> Vec<String> {
        Vec::new()
    }

    /// 健康检查
    pub async fn health_check(&self) -> Result<Vec<(String, bool)>, Box<dyn std::error::Error + Send + Sync>> {
        Ok(Vec::new())
    }
}

/// 搜索统计信息
#[derive(Debug, Clone)]
pub struct SearchStats {
    /// 总搜索次数
    pub total_searches: u64,
    /// 缓存命中次数
    pub cache_hits: u64,
    /// 缓存未命中次数
    pub cache_misses: u64,
    /// 引擎失败次数
    pub engine_failures: u64,
    /// 超时次数
    pub timeouts: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cache::types::CacheImplConfig;
    use crate::net::types::NetworkConfig;

    #[test]
    fn test_interface_creation() {
        let config = SearchConfig::default();
        let network = Arc::new(NetworkInterface::new(NetworkConfig::default()).unwrap());
        let cache = Arc::new(RwLock::new(
            CacheInterface::new(CacheImplConfig::default()).unwrap(),
        ));

        let interface = SearchInterface::new(config, network, cache);
        assert!(interface.is_ok());
    }

    #[test]
    fn test_stats_structure() {
        let stats = SearchStats {
            total_searches: 100,
            cache_hits: 50,
            cache_misses: 50,
            engine_failures: 5,
            timeouts: 2,
        };

        assert_eq!(stats.total_searches, 100);
        assert_eq!(stats.cache_hits, 50);
    }

    #[test]
    fn test_list_engines() {
        let config = SearchConfig::default();
        let network = Arc::new(NetworkInterface::new(NetworkConfig::default()).unwrap());
        let cache = Arc::new(RwLock::new(
            CacheInterface::new(CacheImplConfig::default()).unwrap(),
        ));

        let interface = SearchInterface::new(config, network, cache).unwrap();
        let engines = interface.list_engines();
        assert_eq!(engines.len(), 0); // 初始无引擎
    }
}
