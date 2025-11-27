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

//! 搜索外部接口模块
//!
//! 提供统一的搜索接口供外部使用

use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::time::timeout;
use futures::stream::{FuturesUnordered, StreamExt};

use super::aggregator::{SearchAggregator, AggregationStrategy, SortBy};
use super::query::QueryParser;
use super::types::{SearchConfig, SearchRequest, SearchResponse};
use super::engine_config::{EngineListConfig, EngineMode};
use crate::derive::SearchResult;

/// 搜索接口
///
/// 统一的搜索外部接口，封装所有搜索功能
pub struct SearchInterface {
    /// 搜索配置
    config: SearchConfig,
    /// 结果聚合器
    aggregator: SearchAggregator,
    /// 查询解析器
    parser: QueryParser,
    /// HTTP客户端（复用）
    http_client: Arc<crate::net::client::HttpClient>,
    /// 引擎实例缓存
    engine_cache: Arc<RwLock<std::collections::HashMap<String, Arc<dyn crate::derive::SearchEngine + Send + Sync>>>>,
    /// 引擎状态（用于零结果指数禁用）
    engine_states: Arc<RwLock<std::collections::HashMap<String, super::engine_manager::EngineState>>>,
    /// 统计信息
    stats: Arc<SearchStats>,
}

impl SearchInterface {
    /// 创建新的搜索接口（简化版本，减少耦合）
    ///
    /// # Arguments
    ///
    /// * `config` - 搜索配置
    ///
    /// # Returns
    ///
    /// 返回搜索接口实例或错误
    pub fn new(
        config: SearchConfig,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let aggregator = SearchAggregator::default();
        let parser = QueryParser::default();

        // 创建共享HTTP客户端以提高性能
        let network_config = crate::net::types::NetworkConfig::default();
        let http_client = Arc::new(
            crate::net::client::HttpClient::new(network_config)
                .map_err(|e| format!("Failed to create HTTP client: {}", e))?
        );

        Ok(Self {
            config,
            aggregator,
            parser,
            http_client,
            engine_cache: Arc::new(RwLock::new(std::collections::HashMap::new())),
            engine_states: Arc::new(RwLock::new(std::collections::HashMap::new())),
            stats: Arc::new(SearchStats::default()),
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

        // 确定要使用的引擎列表
        let engines_to_use = if request.engines.is_empty() {
            // 如果没有指定引擎，使用默认全局引擎
            let all_engines = EngineListConfig::get_default_engines();
            all_engines
        } else {
            // 使用请求中指定的引擎列表（验证可用性）
            let config = EngineListConfig::default();
            config.filter_available_engines(&request.engines)
        };

        if engines_to_use.is_empty() {
            return Err("No available engines".into());
        }

        // 执行并发搜索
        let mut response = self.execute_concurrent_search(request, &engines_to_use).await?;

        // 对结果进行聚合、评分和排序（无论有几个结果）
        let aggregated = self.aggregator.aggregate_with_scoring(
            response.results.clone(),
            &request.query
        );
        response.total_count = aggregated.items.len();
        // 用聚合后的结果替换原始结果
        response.results = vec![aggregated];

        Ok(response)
    }

    /// 带模式执行搜索
    ///
    /// # Arguments
    ///
    /// * `request` - 搜索请求
    /// * `mode` - 引擎模式（全局/中国/自定义）
    ///
    /// # Returns
    ///
    /// 返回搜索响应或错误
    pub async fn search_with_mode(
        &self,
        request: &SearchRequest,
        mode: EngineMode,
    ) -> Result<SearchResponse, Box<dyn std::error::Error + Send + Sync>> {
        // 解析查询
        let _parsed = self.parser.parse(&request.query.query);

        // 根据模式获取引擎列表
        let engine_config = EngineListConfig::default();
        let engines_to_use = engine_config.get_engines_for_mode(&mode);

        if engines_to_use.is_empty() {
            return Err("No available engines for this mode".into());
        }

        // 执行并发搜索
        let mut response = self.execute_concurrent_search(request, &engines_to_use).await?;

        // 对结果进行聚合、评分和排序（无论有几个结果）
        let aggregated = self.aggregator.aggregate_with_scoring(
            response.results.clone(),
            &request.query
        );
        response.total_count = aggregated.items.len();
        response.results = vec![aggregated];

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

    /// 流式搜索 - 哪个搜索引擎先完成就先返回哪个的结果
    ///
    /// # Arguments
    ///
    /// * `request` - 搜索请求
    /// * `callback` - 回调函数，每个引擎完成时调用
    ///
    /// # Returns
    ///
    /// 返回最终聚合的搜索响应或错误
    pub async fn search_streaming<F>(
        &self,
        request: &SearchRequest,
        mut callback: F,
    ) -> Result<SearchResponse, Box<dyn std::error::Error + Send + Sync>>
    where
        F: FnMut(SearchResult, String) + Send,
    {
        use std::sync::atomic::Ordering;

        // 增加搜索计数
        self.stats.total_searches.fetch_add(1, Ordering::Relaxed);
        
        let start_time = std::time::Instant::now();

        // 解析查询
        let _parsed = self.parser.parse(&request.query.query);

        // 确定要使用的引擎列表
        let engines_to_use = if request.engines.is_empty() {
            let all_engines = EngineListConfig::get_default_engines();
            all_engines
        } else {
            let config = EngineListConfig::default();
            config.filter_available_engines(&request.engines)
        };

        if engines_to_use.is_empty() {
            return Err("No available engines".into());
        }

        // 预先确保所有引擎都有状态记录
        {
            let mut states = self.engine_states.write().await;
            for engine_name in &engines_to_use {
                states.entry(engine_name.clone())
                    .or_insert_with(|| super::engine_manager::EngineState::new(engine_name.clone()));
            }
        }

        // 创建 FuturesUnordered 用于流式处理
        let mut futures_unordered = FuturesUnordered::new();
        let mut engines_to_execute = Vec::new();

        // 获取所有要执行的引擎实例
        for engine_name in &engines_to_use {
            // 检查引擎是否被临时禁用
            {
                let states = self.engine_states.read().await;
                if let Some(state) = states.get(engine_name) {
                    if !state.is_available() {
                        continue;
                    }
                }
            }
            match self.get_or_create_engine(engine_name).await {
                Ok(engine) => {
                    engines_to_execute.push((engine_name.clone(), engine));
                }
                Err(_e) => {
                    self.stats.engine_failures.fetch_add(1, Ordering::Relaxed);
                }
            }
        }

        // 创建并发任务
        for (engine_name, engine) in engines_to_execute {
            let query = request.query.clone();
            let timeout_duration = Duration::from_secs(self.config.default_timeout.as_secs());
            let stats = Arc::clone(&self.stats);
            
            let future = async move {
                let search_start = std::time::Instant::now();
                match timeout(timeout_duration, engine.search(&query)).await {
                    Ok(Ok(mut result)) => {
                        result.elapsed_ms = search_start.elapsed().as_millis() as u64;
                        Some((Ok(result), engine_name))
                    }
                    Ok(Err(e)) => {
                        stats.engine_failures.fetch_add(1, Ordering::Relaxed);
                        Some((Err(format!("Engine {} error: {}", engine_name, e)), engine_name))
                    }
                    Err(_) => {
                        stats.timeouts.fetch_add(1, Ordering::Relaxed);
                        Some((Err(format!("Engine {} timeout", engine_name)), engine_name))
                    }
                }
            };
            
            futures_unordered.push(future);
        }

        // 流式处理结果
        let mut successful_results = Vec::new();
        let mut engines_used = Vec::new();

        while let Some(result) = futures_unordered.next().await {
            if let Some((search_result, engine_name)) = result {
                match search_result {
                    Ok(result) => {
                        // 检查是否为零结果
                        let is_zero_results = result.items.is_empty();

                        if is_zero_results {
                            // 零结果，更新引擎状态
                            let mut states = self.engine_states.write().await;
                            if let Some(state) = states.get_mut(&engine_name) {
                                state.record_zero_results();
                            }
                        } else {
                            // 有结果，记录成功
                            let mut states = self.engine_states.write().await;
                            if let Some(state) = states.get_mut(&engine_name) {
                                state.record_success(result.elapsed_ms);
                            }
                            
                            // 立即回调返回结果
                            callback(result.clone(), engine_name.clone());
                            
                            successful_results.push(result);
                            engines_used.push(engine_name);
                        }
                    }
                    Err(_e) => {
                        // 错误处理
                        self.stats.engine_failures.fetch_add(1, Ordering::Relaxed);
                    }
                }
            }
        }

        let total_count = successful_results.iter().map(|r| r.items.len()).sum();
        let query_time_ms = start_time.elapsed().as_millis() as u64;

        // 聚合最终结果
        let mut response = SearchResponse {
            query: request.query.clone(),
            results: successful_results,
            total_count,
            engines_used,
            query_time_ms,
            cached: false,
        };

        // 对结果进行聚合、评分和排序
        let aggregated = self.aggregator.aggregate_with_scoring(
            response.results.clone(),
            &request.query
        );
        response.total_count = aggregated.items.len();
        response.results = vec![aggregated];

        Ok(response)
    }

    /// 全文搜索 - 搜索网络和数据库（包括过期缓存和RSS）
    ///
    /// # Arguments
    ///
    /// * `request` - 搜索请求
    ///
    /// # Returns
    ///
    /// 返回网络搜索、数据库缓存和RSS的聚合结果
    pub async fn search_fulltext(
        &self,
        request: &SearchRequest,
    ) -> Result<SearchResponse, Box<dyn std::error::Error + Send + Sync>> {
        use std::sync::atomic::Ordering;
        use crate::cache::on::CacheInterface;
        use crate::cache::types::CacheImplConfig;
        
        let start_time = std::time::Instant::now();
        
        // 1. 执行网络搜索
        let network_response = self.search(request).await?;
        
        // 2. 从数据库获取所有相关结果（包括过期的）
        // 创建缓存接口
        let cache_config = CacheImplConfig::default();
        let cache_interface = CacheInterface::new(cache_config)
            .map_err(|e| format!("Failed to create cache interface: {}", e))?;
        
        // 从查询中提取关键词
        let query_keywords: Vec<String> = request.query.query
            .split_whitespace()
            .map(|s| s.to_string())
            .collect();
        
        // 从结果缓存搜索历史结果
        let result_cache = cache_interface.results();
        let cached_items = match result_cache.search_fulltext(&query_keywords, true, Some(50)) {
            Ok(items) => items,
            Err(e) => {
                // 记录错误但不中断搜索流程
                tracing::warn!("Failed to search result cache: {}", e);
                Vec::new()
            }
        };
        
        // 从 RSS 缓存搜索相关内容
        let rss_cache = cache_interface.rss();
        let rss_items = match rss_cache.search_fulltext(&query_keywords, true, Some(30)) {
            Ok(items) => items,
            Err(e) => {
                // 记录错误但不中断搜索流程
                tracing::warn!("Failed to search RSS cache: {}", e);
                Vec::new()
            }
        };
        
        // 3. 将 RSS items 转换为 SearchResultItem
        let rss_search_items: Vec<crate::derive::types::SearchResultItem> = rss_items.into_iter().map(|(feed_url, item)| {
            use crate::derive::types::{SearchResultItem, ResultType};
            use std::collections::HashMap;
            
            SearchResultItem {
                title: item.title,
                url: item.link,
                content: item.description.unwrap_or_default(),
                display_url: Some(feed_url.clone()),
                site_name: Some(feed_url),
                score: 0.7, // RSS 结果的默认得分
                result_type: ResultType::Web,
                thumbnail: None,
                // TODO: Implement date parsing for RSS pub_date string to DateTime
                published_date: None,
                template: None,
                metadata: HashMap::new(),
            }
        }).collect();
        
        // 4. 合并所有结果
        let mut all_items: Vec<crate::derive::types::SearchResultItem> = Vec::new();
        
        // 添加网络搜索结果（优先级最高）
        for result in &network_response.results {
            all_items.extend(result.items.clone());
        }
        
        // 添加缓存的历史结果
        all_items.extend(cached_items);
        
        // 添加 RSS 结果
        all_items.extend(rss_search_items);
        
        // 5. 去重 - 基于 URL
        let mut seen_urls = std::collections::HashSet::new();
        let mut deduped_items = Vec::new();
        
        for item in all_items {
            let url_normalized = item.url.to_lowercase();
            if !seen_urls.contains(&url_normalized) {
                seen_urls.insert(url_normalized);
                deduped_items.push(item);
            }
        }
        
        // 6. 重新评分和排序
        // 使用关键词匹配度进行评分
        for item in &mut deduped_items {
            let mut score = item.score;
            
            // 根据关键词在标题和内容中的出现次数调整得分
            for keyword in &query_keywords {
                let keyword_lower = keyword.to_lowercase();
                
                // 标题匹配权重更高
                if item.title.to_lowercase().contains(&keyword_lower) {
                    score += 0.3;
                }
                
                // 内容匹配
                if item.content.to_lowercase().contains(&keyword_lower) {
                    score += 0.1;
                }
            }
            
            // 限制最大得分
            item.score = score.min(1.0);
        }
        
        // 按得分降序排序
        deduped_items.sort_by(|a, b| {
            b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal)
        });
        
        // 7. 创建聚合的搜索结果
        let aggregated_result = crate::derive::SearchResult {
            engine_name: "FullTextSearch".to_string(),
            total_results: Some(deduped_items.len()),
            elapsed_ms: start_time.elapsed().as_millis() as u64,
            items: deduped_items,
            pagination: None,
            suggestions: Vec::new(),
            metadata: std::collections::HashMap::new(),
        };
        
        let total_count = aggregated_result.items.len();
        let query_time_ms = start_time.elapsed().as_millis() as u64;
        
        // 8. 构建响应
        let mut engines_used = network_response.engines_used.clone();
        engines_used.push("DatabaseCache".to_string());
        engines_used.push("RSSCache".to_string());
        
        self.stats.total_searches.fetch_add(1, Ordering::Relaxed);
        
        Ok(SearchResponse {
            query: request.query.clone(),
            results: vec![aggregated_result],
            total_count,
            engines_used,
            query_time_ms,
            cached: false, // 混合了网络和缓存结果
        })
    }

    /// 获取或创建引擎实例（带缓存）
    async fn get_or_create_engine(
        &self,
        engine_name: &str,
    ) -> Result<Arc<dyn crate::derive::SearchEngine + Send + Sync>, Box<dyn std::error::Error + Send + Sync>> {
        // 先检查缓存
        {
            let cache = self.engine_cache.read().await;
            if let Some(cached_engine) = cache.get(engine_name) {
                return Ok(Arc::clone(cached_engine));
            }
        }

        // 缓存未命中，创建新实例
        let engine = self.create_engine_instance(engine_name)?;

        // 添加到缓存
        {
            let mut cache = self.engine_cache.write().await;
            cache.insert(engine_name.to_string(), Arc::clone(&engine));
        }

        Ok(engine)
    }

    /// 创建引擎实例（Arc版本，用于缓存）
    fn create_engine_instance(
        &self,
        engine_name: &str,
    ) -> Result<Arc<dyn crate::derive::SearchEngine + Send + Sync>, Box<dyn std::error::Error + Send + Sync>> {
        use crate::search::engines::*;

        let engine: Arc<dyn crate::derive::SearchEngine + Send + Sync> = match engine_name {
            "bing" => Arc::new(BingEngine::with_client(Arc::clone(&self.http_client))),
            "baidu" => Arc::new(BaiduEngine::with_client(Arc::clone(&self.http_client))),
            "yandex" => Arc::new(YandexEngine::with_client(Arc::clone(&self.http_client))),
            "so" => Arc::new(SoEngine::with_client(Arc::clone(&self.http_client))),
            "unsplash" => Arc::new(UnsplashEngine::with_client(Arc::clone(&self.http_client))),
            "bing_images" => Arc::new(BingImagesEngine::with_client(Arc::clone(&self.http_client))),
            "bilibili" => Arc::new(BilibiliEngine::with_client(Arc::clone(&self.http_client))),
            "sogou" => Arc::new(SogouEngine::with_client(Arc::clone(&self.http_client))),
            "sogou_videos" => Arc::new(SogouVideosEngine::with_client(Arc::clone(&self.http_client))),
            _ => {
                // 尝试从Python注册表获取引擎
                #[cfg(feature = "python")]
                {
                    use crate::python_bindings::py_engine_registry::try_get_python_engine_sync;
                    if let Some(py_engine) = try_get_python_engine_sync(engine_name) {
                        return Ok(py_engine as Arc<dyn crate::derive::SearchEngine + Send + Sync>);
                    }
                    return Err(format!("Engine '{}' not found in Rust or Python registries", engine_name).into());
                }
                #[cfg(not(feature = "python"))]
                {
                    return Err(format!("Unknown engine: {}", engine_name).into());
                }
            },
        };

        Ok(engine)
    }

    
    /// 并发执行搜索引擎
    async fn execute_concurrent_search(
        &self,
        request: &SearchRequest,
        engine_names: &[String],
    ) -> Result<SearchResponse, Box<dyn std::error::Error + Send + Sync>> {
        use std::sync::atomic::Ordering;
        
        // 增加搜索计数
        self.stats.total_searches.fetch_add(1, Ordering::Relaxed);
        
        let start_time = std::time::Instant::now();
        let mut futures_list = Vec::new();
        let mut engines_to_execute = Vec::new();

        // 预先确保所有引擎都有状态记录
        {
            let mut states = self.engine_states.write().await;
            for engine_name in engine_names {
                states.entry(engine_name.clone())
                    .or_insert_with(|| super::engine_manager::EngineState::new(engine_name.clone()));
            }
        }

        // 获取所有要执行的引擎实例，并过滤掉被禁用的引擎
        for engine_name in engine_names {
            // 检查引擎是否被临时禁用
            {
                let states = self.engine_states.read().await;
                if let Some(state) = states.get(engine_name) {
                    if !state.is_available() {
                        continue;
                    }
                }
            }
            match self.get_or_create_engine(engine_name).await {
                Ok(engine) => {
                    engines_to_execute.push((engine_name.clone(), engine));
                }
                Err(_e) => {
                    self.stats.engine_failures.fetch_add(1, Ordering::Relaxed);
                }
            }
        }

        // 创建并发任务
        for (engine_name, engine) in engines_to_execute {
            let query = request.query.clone();
            let timeout_duration = Duration::from_secs(self.config.default_timeout.as_secs());
            let stats = Arc::clone(&self.stats);
            
            let future = async move {
                let search_start = std::time::Instant::now();
                match timeout(timeout_duration, engine.search(&query)).await {
                    Ok(Ok(mut result)) => {
                        result.elapsed_ms = search_start.elapsed().as_millis() as u64;
                        Some((Ok(result), engine_name))
                    }
                    Ok(Err(e)) => {
                        stats.engine_failures.fetch_add(1, Ordering::Relaxed);
                        Some((Err(format!("Engine {} error: {}", engine_name, e)), engine_name))
                    }
                    Err(_) => {
                        stats.timeouts.fetch_add(1, Ordering::Relaxed);
                        Some((Err(format!("Engine {} timeout", engine_name)), engine_name))
                    }
                }
            };
            
            futures_list.push(future);
        }
        
        // 并发执行所有搜索
        let results = futures::future::join_all(futures_list).await;

        // 收集成功的结果，并检测零结果情况
        let mut successful_results = Vec::new();
        let mut engines_used = Vec::new();

        for result in results.iter() {
            if let Some((search_result, engine_name)) = result {
                match search_result {
                    Ok(result) => {
                        // 检查是否为零结果
                        let is_zero_results = result.items.is_empty();

                        if is_zero_results {
                            // 零结果，更新引擎状态并应用指数退避
                            let mut states = self.engine_states.write().await;
                            if let Some(state) = states.get_mut(engine_name) {
                                state.record_zero_results();
                            }
                        } else {
                            // 有结果，记录成功
                            let mut states = self.engine_states.write().await;
                            if let Some(state) = states.get_mut(engine_name) {
                                state.record_success(result.elapsed_ms);
                            }
                        }

                        
                        successful_results.push(result.clone());
                        engines_used.push(engine_name.clone());
                    }
                    Err(_) => {
                        // 失败，记录失败
                        let mut states = self.engine_states.write().await;
                        let state = states.entry(engine_name.clone())
                            .or_insert_with(|| super::engine_manager::EngineState::new(engine_name.clone()));
                        state.record_failure();
                    }
                }
            }
        }
        
        let query_time_ms = start_time.elapsed().as_millis() as u64;
        let total_count: usize = successful_results.iter().map(|r| r.items.len()).sum();
        Ok(SearchResponse {
            query: request.query.clone(),
            results: successful_results,
            total_count,
            engines_used,
            query_time_ms,
            cached: false,
        })
    }

    /// 获取统计信息
    pub async fn get_stats(&self) -> SearchStatsResult {
        use std::sync::atomic::Ordering;
        
        SearchStatsResult {
            total_searches: self.stats.total_searches.load(Ordering::Relaxed),
            cache_hits: self.stats.cache_hits.load(Ordering::Relaxed),
            cache_misses: self.stats.cache_misses.load(Ordering::Relaxed),
            engine_failures: self.stats.engine_failures.load(Ordering::Relaxed),
            timeouts: self.stats.timeouts.load(Ordering::Relaxed),
        }
    }

    /// 获取引擎缓存统计
    pub async fn get_engine_cache_stats(&self) -> (usize, Vec<String>) {
        let cache = self.engine_cache.read().await;
        let cached_engines: Vec<String> = cache.keys().cloned().collect();
        (cache.len(), cached_engines)
    }

    /// 清理引擎缓存
    pub async fn clear_engine_cache(&self) {
        let mut cache = self.engine_cache.write().await;
        cache.clear();
    }

    /// 清除缓存
    pub async fn clear_cache(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // 缓存清理逻辑
        Ok(())
    }

    /// 列出可用引擎
    pub fn list_engines(&self) -> Vec<String> {
        EngineListConfig::default().all_available_engines.clone()
    }

    /// 列出全局模式引擎
    pub fn list_global_engines(&self) -> Vec<String> {
        EngineListConfig::default().global_engines.clone()
    }

    /// 健康检查
    pub async fn health_check(&self) -> Result<Vec<(String, bool)>, Box<dyn std::error::Error + Send + Sync>> {
        // 可以实现健康检查逻辑
        // 暂时返回所有引擎为健康状态
        let engines = self.list_engines();
        Ok(engines.into_iter().map(|e| (e, true)).collect())
    }

    /// 获取引擎状态
    pub async fn get_engine_states(&self) -> Vec<(String, (bool, bool, u32))> {
        let states = self.engine_states.read().await;
        states.iter().map(|(name, state)| {
            (
                name.clone(),
                (
                    state.enabled,
                    state.temporarily_disabled,
                    state.consecutive_failures
                )
            )
        }).collect()
    }

    /// 使特定引擎缓存失效
    pub async fn invalidate_engine(&self, engine_name: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut cache = self.engine_cache.write().await;
        cache.remove(engine_name);
        Ok(())
    }

    /// 获取隐私保护统计信息
    pub async fn get_privacy_stats(&self) -> Option<crate::net::privacy::PrivacyStats> {
        // 从 HTTP 客户端获取隐私管理器
        if let Some(privacy_mgr) = self.http_client.privacy_manager() {
            Some(privacy_mgr.get_stats().await)
        } else {
            None
        }
    }
}

/// 搜索统计信息
#[derive(Debug)]
pub struct SearchStats {
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

impl Default for SearchStats {
    fn default() -> Self {
        Self {
            total_searches: std::sync::atomic::AtomicU64::new(0),
            cache_hits: std::sync::atomic::AtomicU64::new(0),
            cache_misses: std::sync::atomic::AtomicU64::new(0),
            engine_failures: std::sync::atomic::AtomicU64::new(0),
            timeouts: std::sync::atomic::AtomicU64::new(0),
        }
    }
}

/// 搜索统计结果（用于外部查询）
#[derive(Debug, Clone)]
pub struct SearchStatsResult {
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

    #[test]
    fn test_interface_creation() {
        let config = SearchConfig::default();
        let interface = SearchInterface::new(config);
        assert!(interface.is_ok());
    }

    #[test]
    fn test_stats_structure() {
        use std::sync::atomic::AtomicU64;
        
        let stats = SearchStats {
            total_searches: AtomicU64::new(100),
            cache_hits: AtomicU64::new(50),
            cache_misses: AtomicU64::new(50),
            engine_failures: AtomicU64::new(5),
            timeouts: AtomicU64::new(2),
        };

        use std::sync::atomic::Ordering;
        assert_eq!(stats.total_searches.load(Ordering::Relaxed), 100);
        assert_eq!(stats.cache_hits.load(Ordering::Relaxed), 50);
    }

    #[test]
    fn test_list_engines() {
        let config = SearchConfig::default();
        let interface = SearchInterface::new(config).unwrap();
        let engines = interface.list_engines();
        assert!(!engines.is_empty()); // 应该有预设的引擎列表
    }
}
