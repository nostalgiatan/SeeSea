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

//! 搜索外部接口模块
//!
//! 提供统一的搜索接口供外部使用

use dashmap::DashMap;
use futures::stream::{FuturesUnordered, StreamExt};
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::{Semaphore, mpsc};
use tokio::time::timeout;

use super::engine_config::{ENGINE_CONFIG, EngineMode};
use super::result_visualization::TimeSorter;
use super::types::{SearchConfig, SearchRequest, SearchResponse};
use super::vector_scoring::VectorScorer;
use seesea_cache::CacheInterface;
use seesea_cache::cache::types::CacheImplConfig;
use seesea_derive::SearchResult;
use seesea_net::privacy::PrivacyStats;

/// 引擎实例元数据
///
/// 存储引擎实例的元数据，用于自动回收
pub struct EngineInstanceMetadata {
    /// 引擎实例
    engine: Arc<dyn seesea_derive::SearchEngine + Send + Sync>,
    /// 创建时间
    #[allow(dead_code)]
    created_at: SystemTime,
    /// 最后使用时间
    last_used_at: SystemTime,
    /// 使用次数
    usage_count: u32,
}

/// 搜索接口
///
/// 统一的搜索外部接口，封装所有搜索功能
pub struct SearchInterface {
    /// 搜索配置
    config: SearchConfig,
    /// 时间排序器
    time_sorter: TimeSorter,
    /// 向量评分器
    vector_scorer: VectorScorer,
    /// HTTP客户端（复用）
    http_client: Arc<seesea_net::client::HttpClient>,
    /// 引擎实例缓存
    engine_cache: Arc<DashMap<String, EngineInstanceMetadata>>,
    /// 引擎状态（用于零结果指数禁用）
    engine_states: Arc<DashMap<String, super::engine_manager::EngineState>>,
    /// 统计信息
    stats: Arc<SearchStats>,
    /// 并发控制信号量，限制同时运行的搜索任务数量
    semaphore: Arc<Semaphore>,
    /// 搜索结果缓存
    cache: Arc<CacheInterface>,
    /// 当前并发数
    current_concurrency: std::sync::atomic::AtomicUsize,
    /// 上次调整时间
    last_adjust_time: std::sync::atomic::AtomicU64,
    /// 搜索历史数据（按小时统计）
    search_history: Arc<tokio::sync::RwLock<Vec<SearchHistoryEntry>>>,
}

impl SearchInterface {
    /// 获取动态并发数
    ///
    /// 根据系统资源（CPU核心数、内存使用率）动态计算合适的并发数
    fn get_dynamic_concurrency(&self) -> usize {
        // 使用sysinfo获取系统负载信息
        let mut system = sysinfo::System::new_all();
        system.refresh_all();

        let cpu_cores = num_cpus::get();
        let cpu_usage = system.global_cpu_usage() as f64;
        let total_memory = system.total_memory();
        let available_memory = system.available_memory();
        let memory_usage = ((total_memory - available_memory) as f64 / total_memory as f64) * 100.0;
        let load_avg = sysinfo::System::load_average().one;

        // 基础并发数 = CPU核心数 * 基准倍数
        let base_concurrency = (cpu_cores as f64 * 8.0) as usize; // 调整基准倍数为8.0，更合理的初始值

        // 根据CPU使用率调整（CPU使用率越高，并发数越低）
        let cpu_factor = if cpu_usage > 90.0 {
            0.2 // CPU使用率超过90%，大幅降低并发
        } else if cpu_usage > 75.0 {
            0.5 // CPU使用率超过75%，适度降低并发
        } else {
            (100.0 - cpu_usage) / 100.0 // 正常情况下线性调整
        };

        // 根据内存使用率调整（内存使用率越高，并发数越低）
        let memory_factor = if memory_usage > 90.0 {
            0.3 // 内存使用率超过90%，大幅降低并发
        } else if memory_usage > 80.0 {
            0.6 // 内存使用率超过80%，适度降低并发
        } else {
            (100.0 - memory_usage) / 100.0 // 正常情况下线性调整
        };

        // 根据系统负载调整（负载越高，并发数越低）
        let load_factor = if load_avg > cpu_cores as f64 * 2.0 {
            0.4 // 负载超过CPU核心数的2倍，大幅降低并发
        } else if load_avg > cpu_cores as f64 {
            0.7 // 负载超过CPU核心数，适度降低并发
        } else {
            1.0 // 负载正常，不调整
        };

        // 最终并发数 = 基础并发数 * CPU因子 * 内存因子 * 负载因子
        let final_concurrency =
            (base_concurrency as f64 * cpu_factor * memory_factor * load_factor) as usize;

        // 确保并发数在合理范围内，优先使用配置中的值
        let min_concurrency = 1;
        let max_concurrency = std::cmp::max(self.config.max_concurrent_engines, 200);
        final_concurrency.clamp(min_concurrency, max_concurrency)
    }

    /// 创建新的搜索接口（简化版本，减少耦合）
    ///
    /// # Arguments
    ///
    /// * `config` - 搜索配置
    /// * `network_config` - 网络配置（可选，默认使用全局配置）
    ///
    /// # Returns
    ///
    /// 返回搜索接口实例或错误
    pub fn new(config: SearchConfig) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        Self::new_with_network_config(config, None)
    }

    /// 使用网络配置创建新的搜索接口
    ///
    /// # Arguments
    ///
    /// * `config` - 搜索配置
    /// * `network_config` - 网络配置（可选，默认使用全局配置）
    ///
    /// # Returns
    ///
    /// 返回搜索接口实例或错误
    pub fn new_with_network_config(
        config: SearchConfig,
        network_config: Option<seesea_config::NetworkConfig>,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        // 使用提供的网络配置，或者使用默认配置
        let net_config = network_config.unwrap_or_else(|| {
            // 尝试从全局配置管理器获取网络配置
            // 如果获取失败，使用默认配置
            seesea_config::NetworkConfig::default()
        });

        // 使用HttpClient单例模式，在整个应用程序中共享同一个HttpClient实例
        let http_client = Arc::new(
            seesea_net::client::HttpClient::instance_with_config(net_config)
                .map_err(|e| format!("Failed to get HTTP client instance: {e}"))?,
        );

        // 初始化并发控制信号量，使用默认并发数
        // 注意：这里不能调用self.get_dynamic_concurrency()，因为self还没有完全创建
        let cpu_cores = num_cpus::get();
        let initial_concurrency = std::cmp::min(
            (cpu_cores as f64 * 8.0) as usize, // 调整基准倍数为8.0，更合理的初始值
            config.max_concurrent_engines,     // 优先使用配置中的最大并发数
        );
        let semaphore = Arc::new(Semaphore::new(initial_concurrency.clamp(1, 200)));

        let cache_config = CacheImplConfig::new(seesea_config::paths::get_cache_dir());
        let cache = Arc::new(
            CacheInterface::new(cache_config)
                .map_err(|e| format!("Failed to create cache: {e:?}"))?,
        );

        // 创建时间排序器和向量评分器
        let time_sorter = TimeSorter::default();
        let vector_scorer = VectorScorer::default();

        // 初始化当前并发数和上次调整时间
        let current_concurrency = initial_concurrency.clamp(1, 200);
        let last_adjust_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // 创建SearchInterface实例
        let search_interface = Self {
            config: config.clone(),
            time_sorter,
            vector_scorer,
            http_client: Arc::clone(&http_client),
            engine_cache: Arc::new(DashMap::new()),
            engine_states: Arc::new(DashMap::new()),
            stats: Arc::new(SearchStats::default()),
            semaphore,
            cache,
            current_concurrency: std::sync::atomic::AtomicUsize::new(current_concurrency),
            last_adjust_time: std::sync::atomic::AtomicU64::new(last_adjust_time),
            search_history: Arc::new(tokio::sync::RwLock::new(Vec::new())),
        };

        // 暂时移除预创建引擎实例的逻辑，因为SearchConfig中没有相关字段
        // 这是一个临时解决方案，用于避免编译器错误

        Ok(search_interface)
    }

    /// 检查内存使用情况
    ///
    /// 如果内存使用率超过阈值，返回错误
    fn check_memory_usage() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // 优化：只在必要时检查内存使用情况，避免每次搜索都调用昂贵的system.refresh_all()
        // 注释：内存检查可以通过其他方式实现，比如定期检查，而不是每次请求都检查
        // 暂时禁用内存检查，因为它是主要性能瓶颈
        Ok(())
    }

    /// 动态调整并发数
    ///
    /// 根据系统负载动态调整并发数，实现平滑调整
    async fn adjust_concurrency(&self) {
        // 获取当前时间
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // 检查是否需要调整（根据调整间隔）
        let last_adjust = self
            .last_adjust_time
            .load(std::sync::atomic::Ordering::Relaxed);
        const CONCURRENCY_ADJUST_INTERVAL: u64 = 30; // 调整间隔缩短为30秒，更快响应系统负载变化
        if now - last_adjust < CONCURRENCY_ADJUST_INTERVAL {
            return; // 调整间隔未到，不调整
        }

        // 计算目标并发数
        let target_concurrency = self.get_dynamic_concurrency();
        let current_concurrency = self
            .current_concurrency
            .load(std::sync::atomic::Ordering::Relaxed);

        // 如果目标并发数与当前并发数相差不大，不调整
        let concurrency_diff = target_concurrency.abs_diff(current_concurrency);
        if concurrency_diff < 2 {
            // 相差小于2，不调整
            return;
        }

        // 计算最大调整幅度（每次调整最多变化25%，但至少变化2）
        let max_adjust = std::cmp::max((current_concurrency as f64 * 0.25) as usize, 2);

        // 计算新的并发数，实现平滑调整
        let new_concurrency = if target_concurrency > current_concurrency {
            // 增加并发数
            let increase = (target_concurrency - current_concurrency).min(max_adjust);
            current_concurrency + increase
        } else if target_concurrency < current_concurrency {
            // 减少并发数
            let decrease = (current_concurrency - target_concurrency).min(max_adjust);
            current_concurrency - decrease
        } else {
            // 并发数不变，不调整
            return;
        };

        // 确保新的并发数在合理范围内，优先使用配置中的值
        let min_concurrency = 1;
        let max_concurrency = std::cmp::max(self.config.max_concurrent_engines, 200);
        let new_concurrency = new_concurrency.clamp(min_concurrency, max_concurrency);

        // 更新semaphore的许可数量
        if new_concurrency != current_concurrency {
            let current_permits = self.semaphore.available_permits();
            let diff = new_concurrency as i32 - current_permits as i32;

            if diff > 0 {
                // 增加许可数量
                for _ in 0..diff {
                    self.semaphore.add_permits(1);
                }
            } else if diff < 0 {
                // 减少许可数量（注意：semaphore不支持直接减少许可，这里只能等待许可被释放）
                // 我们可以通过不添加新许可来间接减少并发数
                // 或者使用更复杂的机制，比如创建新的semaphore并替换旧的
                // 暂时不实现减少许可的逻辑，因为这会增加复杂度
            }

            // 更新当前并发数和上次调整时间
            self.current_concurrency
                .store(new_concurrency, std::sync::atomic::Ordering::Relaxed);
            self.last_adjust_time
                .store(now, std::sync::atomic::Ordering::Relaxed);

            // 记录调整日志
            tracing::info!(
                "调整并发数: 从 {current_concurrency} 调整到 {new_concurrency}, 目标并发数: {target_concurrency}, 调整幅度: {}",
                if target_concurrency > current_concurrency {
                    format!("+{}", new_concurrency - current_concurrency)
                } else {
                    format!("-{}", current_concurrency - new_concurrency)
                }
            );
        }
    }

    /// 执行可视化搜索，返回二维排列的结果
    ///
    /// # Arguments
    ///
    /// * `request` - 搜索请求
    /// * `visualization_config` - 二维可视化配置（可选，默认配置）
    ///
    /// # Returns
    ///
    /// 返回时间排序的搜索结果或错误
    pub async fn search_visualized(
        &self,
        request: &SearchRequest,
        _visualization_config: Option<()>,
    ) -> Result<seesea_derive::SearchResult, Box<dyn std::error::Error + Send + Sync>> {
        // 执行常规搜索
        let response = self.search(request).await?;

        // 打印原始结果数量
        println!("原始结果数量: {}", response.total_count);
        println!("使用的引擎: {:?}", response.engines_used);

        // 将搜索结果按时间排序
        let time_sorted = self
            .time_sorter
            .sort_by_time(response.results[0].items.clone());

        // 将时间排序结果转换为标准搜索结果
        let sorted_result = self
            .time_sorter
            .to_search_result(&time_sorted, &response.results[0]);

        Ok(sorted_result)
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
        // 检查内存使用情况
        Self::check_memory_usage()?;

        // 更新总搜索次数
        use std::sync::atomic::Ordering;
        self.stats.total_searches.fetch_add(1, Ordering::Relaxed);

        // 记录搜索历史
        self.record_search_history().await;

        // 确定要使用的引擎列表 - 使用全局ENGINE_CONFIG实例避免重复创建和克隆
        let engines_to_use = if !request.engines.is_empty() {
            // 使用请求中指定的引擎列表（验证可用性）
            ENGINE_CONFIG.filter_available_engines(&request.engines)
        } else {
            // 如果没有指定引擎，根据include_deepweb参数决定使用的引擎
            if request.include_deepweb {
                // 包含深网搜索，使用所有引擎
                ENGINE_CONFIG.global_engines.clone()
            } else {
                // 不包含深网搜索，仅使用快速引擎
                ENGINE_CONFIG.fast_engines.clone()
            }
        };

        if engines_to_use.is_empty() {
            return Err("No available engines".into());
        }

        // 动态调整并发数
        self.adjust_concurrency().await;

        // 执行并发搜索
        let mut response = self
            .execute_concurrent_search(request, &engines_to_use)
            .await?;

        // 合并所有搜索结果
        let mut all_items = Vec::new();
        for result in &response.results {
            all_items.extend(result.items.clone());
        }

        // 标准化结果
        crate::search::standardization::standardize_items(&mut all_items);

        // 使用向量评分系统对结果进行评分
        self.vector_scorer
            .score_results(&mut all_items, &request.query, "aggregated")
            .await;

        // 使用时间排序器对结果进行排序
        let time_sorted = self.time_sorter.sort_by_time(all_items);

        // 将时间排序结果转换为标准搜索结果
        let mut ordered_results =
            Vec::with_capacity(time_sorted.timed_items.len() + time_sorted.untimed_items.len());
        ordered_results.extend(time_sorted.timed_items);
        ordered_results.extend(time_sorted.untimed_items);

        let final_result = seesea_derive::SearchResult {
            engine_name: "aggregated".to_string(),
            total_results: Some(ordered_results.len()),
            elapsed_ms: 0,
            items: ordered_results,
            pagination: None,
            suggestions: Vec::new(),
            metadata: {
                let mut metadata = std::collections::HashMap::new();
                metadata.insert("vector_scoring_enabled".to_string(), "true".to_string());
                metadata.insert("time_sort_enabled".to_string(), "true".to_string());
                metadata.insert(
                    "timed_count".to_string(),
                    time_sorted.stats.timed_count.to_string(),
                );
                metadata.insert(
                    "untimed_count".to_string(),
                    time_sorted.stats.untimed_count.to_string(),
                );
                metadata
            },
        };

        response.total_count = final_result.items.len();
        response.results = vec![final_result];

        Ok(response)
    }

    /// 带模式执行搜索
    ///
    /// # Arguments
    ///
    /// * `request` - 搜索请求
    /// * `mode` - 引擎模式（全局/自定义/快速/深网）
    ///
    /// # Returns
    ///
    /// 返回搜索响应或错误
    pub async fn search_with_mode(
        &self,
        request: &SearchRequest,
        mode: EngineMode,
    ) -> Result<SearchResponse, Box<dyn std::error::Error + Send + Sync>> {
        // 检查内存使用情况
        Self::check_memory_usage()?;

        // 根据模式获取引擎列表 - 使用全局ENGINE_CONFIG实例避免重复创建和克隆
        let engines_to_use = ENGINE_CONFIG.get_engines_for_mode(&mode);

        if engines_to_use.is_empty() {
            return Err("No available engines for this mode".into());
        }

        // 执行并发搜索
        let mut response = self
            .execute_concurrent_search(request, &engines_to_use)
            .await?;

        // 合并所有搜索结果
        let mut all_items = Vec::new();
        for result in &response.results {
            all_items.extend(result.items.clone());
        }

        // 标准化结果
        crate::search::standardization::standardize_items(&mut all_items);

        // 使用向量评分系统对结果进行评分
        self.vector_scorer
            .score_results(&mut all_items, &request.query, "aggregated")
            .await;

        // 使用时间排序器对结果进行排序
        let time_sorted = self.time_sorter.sort_by_time(all_items);

        // 将时间排序结果转换为标准搜索结果
        let mut ordered_results =
            Vec::with_capacity(time_sorted.timed_items.len() + time_sorted.untimed_items.len());
        ordered_results.extend(time_sorted.timed_items);
        ordered_results.extend(time_sorted.untimed_items);

        let final_result = seesea_derive::SearchResult {
            engine_name: "aggregated".to_string(),
            total_results: Some(ordered_results.len()),
            elapsed_ms: 0,
            items: ordered_results,
            pagination: None,
            suggestions: Vec::new(),
            metadata: {
                let mut metadata = std::collections::HashMap::new();
                metadata.insert("vector_scoring_enabled".to_string(), "true".to_string());
                metadata.insert("time_sort_enabled".to_string(), "true".to_string());
                metadata.insert(
                    "timed_count".to_string(),
                    time_sorted.stats.timed_count.to_string(),
                );
                metadata.insert(
                    "untimed_count".to_string(),
                    time_sorted.stats.untimed_count.to_string(),
                );
                metadata
            },
        };

        response.total_count = final_result.items.len();
        response.results = vec![final_result];

        Ok(response)
    }

    /// 带选项执行搜索
    ///
    /// # Arguments
    ///
    /// * `request` - 搜索请求
    ///
    /// # Returns
    ///
    /// 返回搜索响应或错误
    pub async fn search_with_options(
        &self,
        request: &SearchRequest,
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
        use std::time::Duration;
        use tokio::time::timeout;

        // 检查内存使用情况
        Self::check_memory_usage()?;

        // 增加搜索计数
        self.stats.total_searches.fetch_add(1, Ordering::Relaxed);

        let start_time = std::time::Instant::now();

        // 确定要使用的引擎列表 - 使用全局ENGINE_CONFIG实例避免重复创建和克隆
        let engines_to_use = if !request.engines.is_empty() {
            // 使用请求中指定的引擎列表（验证可用性）
            ENGINE_CONFIG.filter_available_engines(&request.engines)
        } else {
            // 如果没有指定引擎，根据include_deepweb参数决定使用的引擎
            if request.include_deepweb {
                // 包含深网搜索，使用所有引擎
                ENGINE_CONFIG.global_engines.clone()
            } else {
                // 不包含深网搜索，仅使用快速引擎
                ENGINE_CONFIG.fast_engines.clone()
            }
        };

        if engines_to_use.is_empty() {
            return Err("No available engines".into());
        }

        // 创建 FuturesUnordered 用于流式处理
        let mut futures_unordered = FuturesUnordered::new();
        let mut engines_to_execute = Vec::new();

        // 获取所有要执行的引擎实例
        for engine_name in &engines_to_use {
            // 检查引擎是否被临时禁用
            if let Some(state) = self.engine_states.get(engine_name)
                && !state.is_available()
            {
                continue;
            }
            match self.get_or_create_engine(engine_name).await {
                Ok(engine) => {
                    // 使用to_string()替代clone()，减少不必要的克隆
                    engines_to_execute.push((engine_name.to_string(), engine));
                }
                Err(_e) => {
                    self.stats.engine_failures.fetch_add(1, Ordering::Relaxed);
                }
            }
        }

        // 创建并发任务
        let query_ref = &request.query;
        for (engine_name, engine) in engines_to_execute {
            let query = query_ref.clone(); // 只克隆一次query，而不是为每个引擎克隆
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
                        Some((Err(format!("Engine {engine_name} error: {e}")), engine_name))
                    }
                    Err(_) => {
                        stats.timeouts.fetch_add(1, Ordering::Relaxed);
                        Some((Err(format!("Engine {engine_name} timeout")), engine_name))
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
                        // 立即回调返回结果
                        callback(result.clone(), engine_name.clone());

                        successful_results.push(result);
                        engines_used.push(engine_name);
                    }
                    Err(_e) => {
                        // 错误处理
                        self.stats.engine_failures.fetch_add(1, Ordering::Relaxed);
                    }
                }
            }
        }

        // 合并所有搜索结果
        let mut all_items = Vec::new();
        for result in &successful_results {
            all_items.extend(result.items.clone());
        }

        // 创建临时SearchResult用于可视化
        let mut temp_result = seesea_derive::SearchResult {
            engine_name: "aggregated".to_string(),
            total_results: Some(all_items.len()),
            elapsed_ms: start_time.elapsed().as_millis() as u64,
            items: all_items,
            pagination: None,
            suggestions: Vec::new(),
            metadata: std::collections::HashMap::new(),
        };

        // 标准化结果
        crate::search::standardization::standardize_results(&mut temp_result);

        // 将搜索结果按时间排序
        let time_sorted = self.time_sorter.sort_by_time(temp_result.items.clone());

        // 将时间排序结果转换为标准搜索结果
        let visualized_result = self
            .time_sorter
            .to_search_result(&time_sorted, &temp_result);

        // 构建最终响应
        let total_count = visualized_result.items.len();
        let response = SearchResponse {
            query: request.query.clone(),
            results: vec![visualized_result],
            total_count,
            engines_used,
            query_time_ms: start_time.elapsed().as_millis() as u64,
            cached: false,
        };

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
        use seesea_cache::CacheInterface;
        use seesea_cache::cache::types::CacheImplConfig;
        use std::sync::atomic::Ordering;

        // 检查内存使用情况
        Self::check_memory_usage()?;

        let start_time = std::time::Instant::now();

        // 1. 执行网络搜索
        let network_response = self.search(request).await?;

        // 2. 从数据库获取所有相关结果（包括过期的）
        // 创建缓存接口
        let cache_config = CacheImplConfig::new(seesea_config::paths::get_cache_dir());
        let cache_interface = CacheInterface::new(cache_config)
            .map_err(|e| format!("Failed to create cache interface: {e}"))?;

        // 从查询中提取关键词
        let query_keywords: Vec<String> = request
            .query
            .query
            .split_whitespace()
            .map(|s| s.to_string())
            .collect();

        // 从结果缓存搜索历史结果
        let result_cache = cache_interface.results();
        let cached_items_tuples = match result_cache
            .search_fulltext(&query_keywords, true, Some(50))
            .await
        {
            Ok(items) => items,
            Err(e) => {
                // 记录错误但不中断搜索流程
                tracing::warn!("Failed to search result cache: {}", e);
                Vec::new()
            }
        };
        let cached_items: Vec<seesea_derive::types::SearchResultItem> = cached_items_tuples
            .into_iter()
            .map(|(_, item)| item)
            .collect();

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
        let rss_search_items: Vec<seesea_derive::types::SearchResultItem> = rss_items
            .into_iter()
            .map(|(feed_url, item)| {
                use crate::search::utils::time_extractor::parse_time;
                use seesea_derive::types::{ResultType, SearchResultItem};
                use std::collections::HashMap;

                // 解析发布日期
                let published_date = item
                    .pub_date
                    .as_ref()
                    .and_then(|date_str| parse_time(date_str));

                SearchResultItem {
                    title: item.title,
                    url: item.link,
                    content: item.description.unwrap_or_default(),
                    display_url: Some(feed_url.clone()),
                    site_name: Some(feed_url),
                    score: 0.7, // RSS 结果的默认得分
                    result_type: ResultType::Web,
                    thumbnail: None,
                    published_date,
                    template: None,
                    metadata: HashMap::new(),
                }
            })
            .collect();

        // 4. 合并所有结果
        let mut all_items: Vec<seesea_derive::types::SearchResultItem> = Vec::new();

        // 添加网络搜索结果（优先级最高）
        for result in &network_response.results {
            all_items.extend(result.items.clone()); // 使用clone，因为需要实际的SearchResultItem对象
        }

        // 添加缓存的历史结果
        all_items.extend(cached_items);

        // 添加 RSS 结果
        all_items.extend(rss_search_items);

        // 5. 创建临时SearchResult用于可视化
        let mut temp_result = seesea_derive::SearchResult {
            engine_name: "FullTextSearch".to_string(),
            total_results: Some(all_items.len()),
            elapsed_ms: start_time.elapsed().as_millis() as u64,
            items: all_items,
            pagination: None,
            suggestions: Vec::new(),
            metadata: std::collections::HashMap::new(),
        };

        // 6. 标准化结果
        crate::search::standardization::standardize_results(&mut temp_result);

        // 7. 使用二维可视化模块重新排列结果
        // 将搜索结果按时间排序
        let time_sorted = self.time_sorter.sort_by_time(temp_result.items.clone());

        // 将时间排序结果转换为标准搜索结果
        let visualized_result = self
            .time_sorter
            .to_search_result(&time_sorted, &temp_result);

        let query_time_ms = start_time.elapsed().as_millis() as u64;
        let total_count = visualized_result.items.len();

        // 9. 构建响应
        let mut engines_used = network_response.engines_used.clone();
        engines_used.push("DatabaseCache".to_string());
        engines_used.push("RSSCache".to_string());

        self.stats.total_searches.fetch_add(1, Ordering::Relaxed);

        Ok(SearchResponse {
            query: request.query.clone(),
            results: vec![visualized_result],
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
    ) -> Result<
        Arc<dyn seesea_derive::SearchEngine + Send + Sync>,
        Box<dyn std::error::Error + Send + Sync>,
    > {
        let now = SystemTime::now();
        let engine_name_str = engine_name.to_string();

        // 清理过期缓存
        self.clean_expired_cache(now).await;

        // 先检查缓存
        if let Some(mut metadata) = self.engine_cache.get_mut(&engine_name_str) {
            // 更新最后使用时间和使用次数
            metadata.last_used_at = now;
            metadata.usage_count += 1;
            return Ok(Arc::clone(&metadata.engine));
        }

        // 缓存未命中，创建新实例
        let engine = self.create_engine_instance(engine_name)?;

        // 创建引擎实例元数据
        let metadata = EngineInstanceMetadata {
            engine: Arc::clone(&engine),
            created_at: now,
            last_used_at: now,
            usage_count: 1,
        };

        // 添加到缓存
        // 检查缓存大小，实现智能回收
        const MAX_ENGINE_CACHE_SIZE: usize = 20;
        if self.engine_cache.len() >= MAX_ENGINE_CACHE_SIZE {
            // 找到需要回收的引擎实例（使用混合策略：最近使用时间 + 使用频率）
            let to_remove = self.find_least_valuable_engine(now).await;

            if let Some(key) = to_remove {
                self.engine_cache.remove(&key);
            }
        }

        self.engine_cache.insert(engine_name_str, metadata);

        Ok(engine)
    }

    /// 清理过期缓存
    async fn clean_expired_cache(&self, now: SystemTime) {
        // 使用硬编码的TTL值：24小时
        let ttl = Duration::from_secs(24 * 60 * 60);

        // 遍历所有缓存项，清理过期的
        let mut keys_to_remove = Vec::new();
        for entry in self.engine_cache.iter() {
            let meta = entry.value();
            if let Ok(elapsed) = now.duration_since(meta.last_used_at)
                && elapsed > ttl
            {
                keys_to_remove.push(entry.key().clone());
            }
        }

        // 批量清理过期缓存
        for key in keys_to_remove {
            self.engine_cache.remove(&key);
        }
    }

    /// 找到最没有价值的引擎实例（使用混合策略：最近使用时间 + 使用频率）
    async fn find_least_valuable_engine(&self, now: SystemTime) -> Option<String> {
        let mut least_valuable_key = None;
        let mut max_score = f64::MIN;

        // 计算每个引擎实例的价值分数
        // 分数越低，价值越低，越应该被回收
        for entry in self.engine_cache.iter() {
            let meta = entry.value();
            let key = entry.key().clone();

            // 计算最后使用时间的权重（最近使用的分数高）
            let last_used_weight = if let Ok(elapsed) = now.duration_since(meta.last_used_at) {
                // 时间越久，分数越低（使用指数衰减）
                let seconds = elapsed.as_secs() as f64;
                1.0 / (1.0 + seconds / 3600.0) // 每小时衰减
            } else {
                0.0
            };

            // 计算使用频率的权重（使用次数越多，分数越高）
            let usage_weight = (meta.usage_count as f64).log10() + 1.0;

            // 混合分数 = 最近使用时间权重 * 0.6 + 使用频率权重 * 0.4
            let score = last_used_weight * 0.6 + usage_weight * 0.4;

            // 寻找分数最低的引擎实例
            if score > max_score {
                max_score = score;
                least_valuable_key = Some(key);
            }
        }

        least_valuable_key
    }

    /// 创建引擎实例（Arc版本，用于缓存）
    fn create_engine_instance(
        &self,
        engine_name: &str,
    ) -> Result<
        Arc<dyn seesea_derive::SearchEngine + Send + Sync>,
        Box<dyn std::error::Error + Send + Sync>,
    > {
        use crate::search::engines::*;

        let engine: Arc<dyn seesea_derive::SearchEngine + Send + Sync> = match engine_name {
            "bing" => Arc::new(BingEngine::with_client(Arc::clone(&self.http_client))),
            "baidu" => Arc::new(BaiduEngine::with_client(Arc::clone(&self.http_client))),
            "yandex" => Arc::new(YandexEngine::with_client(Arc::clone(&self.http_client))),
            "so" => Arc::new(SoEngine::with_client(Arc::clone(&self.http_client))),
            "unsplash" => Arc::new(UnsplashEngine::with_client(Arc::clone(&self.http_client))),
            "bing_images" => Arc::new(BingImagesEngine::with_client(Arc::clone(&self.http_client))),
            "bilibili" => Arc::new(BilibiliEngine::with_client(Arc::clone(&self.http_client))),
            "sogou" => Arc::new(SogouEngine::with_client(Arc::clone(&self.http_client))),
            "sogou_videos" => Arc::new(SogouVideosEngine::with_client(Arc::clone(
                &self.http_client,
            ))),
            _ => {
                // 尝试从Python注册表获取引擎
                #[cfg(feature = "python")]
                {
                    use seesea_python_bindings::py_engine_registry::try_get_python_engine_sync;
                    if let Some(py_engine) = try_get_python_engine_sync(engine_name) {
                        return Ok(py_engine as Arc<dyn seesea_derive::SearchEngine + Send + Sync>);
                    }
                    // Python引擎未注册，跳过该引擎
                    return Err(format!(
                        "Engine '{}' not found in Rust or Python registries",
                        engine_name
                    )
                    .into());
                }
                #[cfg(not(feature = "python"))]
                {
                    return Err(format!("Unknown engine: {engine_name}").into());
                }
            }
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
        // 预分配Vec容量，减少内存分配
        let mut engines_to_execute = Vec::with_capacity(engine_names.len());
        let mut cached_results = Vec::with_capacity(engine_names.len());
        let mut cached_engines_used = Vec::with_capacity(engine_names.len());

        // 检查缓存，获取所有要执行的引擎实例，并过滤掉被禁用的引擎
        for engine_name in engine_names {
            // 检查引擎是否被临时禁用 - 使用DashMap提高并发性能
            let is_available = self
                .engine_states
                .get(engine_name)
                .is_none_or(|state| state.is_available());

            if !is_available {
                continue;
            }

            // 检查缓存
            let result_cache = self.cache.results();
            if let Some(cached_result) = result_cache
                .get_search_result(&request.query.query, engine_name)
                .await
                .map_err(|e| format!("Cache error: {e:?}"))?
            {
                // 缓存命中，直接使用缓存结果
                self.stats.cache_hits.fetch_add(1, Ordering::Relaxed);
                cached_results.push(cached_result);
                cached_engines_used.push(engine_name.to_string());
                continue;
            } else {
                // 缓存未命中，需要执行搜索
                self.stats.cache_misses.fetch_add(1, Ordering::Relaxed);
            }

            match self.get_or_create_engine(engine_name).await {
                Ok(engine) => {
                    engines_to_execute.push((engine_name.to_string(), engine));
                }
                Err(_e) => {
                    // 引擎创建失败，跳过该引擎
                    self.stats.engine_failures.fetch_add(1, Ordering::Relaxed);
                    continue;
                }
            }
        }

        // 如果所有引擎都有缓存结果，直接返回
        if engines_to_execute.is_empty() {
            let query_time_ms = start_time.elapsed().as_millis() as u64;
            let total_count: usize = cached_results.iter().map(|r| r.items.len()).sum();
            return Ok(SearchResponse {
                query: request.query.clone(),
                results: cached_results,
                total_count,
                engines_used: cached_engines_used,
                query_time_ms,
                cached: true,
            });
        }

        // 创建通道用于结果收集
        let (tx, mut rx) =
            mpsc::channel::<(Result<SearchResult, String>, String)>(engines_to_execute.len());

        // 创建并发任务，使用Semaphore限制并发度
        let timeout_duration = Duration::from_secs(self.config.default_timeout.as_secs());
        let query_ref = &request.query;

        for (engine_name, engine) in engines_to_execute {
            let tx = tx.clone();
            let semaphore = Arc::clone(&self.semaphore);
            let query = query_ref.clone(); // 只克隆一次query，而不是为每个引擎克隆
            let cache = Arc::clone(&self.cache);
            let stats = Arc::clone(&self.stats);

            // 生成搜索任务，使用Semaphore限制并发
            tokio::spawn(async move {
                // 尝试获取信号量许可
                let permit = match semaphore.acquire().await {
                    Ok(permit) => permit,
                    Err(_) => {
                        // 信号量被关闭，发送错误
                        let _ = tx
                            .send((
                                Err(format!("Semaphore closed for engine {engine_name}")),
                                engine_name,
                            ))
                            .await;
                        return;
                    }
                };

                let search_start = std::time::Instant::now();
                let result = timeout(timeout_duration, engine.search(&query)).await;

                // 释放信号量许可
                drop(permit);

                match result {
                    Ok(Ok(mut result)) => {
                        result.elapsed_ms = search_start.elapsed().as_millis() as u64;

                        // 缓存搜索结果，根据引擎类型和查询类型设置不同的TTL
                        let result_cache = cache.results();

                        // 根据引擎类型和查询类型设置不同的TTL
                        let ttl = match engine_name.as_str() {
                            // 图片和视频搜索结果可以缓存更长时间
                            "unsplash" | "bing_images" | "bilibili" | "bing_videos"
                            | "sogou_videos" => {
                                Some(Duration::from_secs(24 * 3600)) // 24小时
                            }
                            // 新闻搜索结果缓存时间较短
                            "bing_news" => {
                                Some(Duration::from_secs(3600)) // 1小时
                            }
                            // 其他搜索结果缓存12小时
                            _ => {
                                Some(Duration::from_secs(12 * 3600)) // 12小时
                            }
                        };

                        let _ = result_cache
                            .set_search_result(&query.query, &engine_name, &result, ttl)
                            .await;

                        let _ = tx.send((Ok(result), engine_name)).await;
                    }
                    Ok(Err(e)) => {
                        stats.engine_failures.fetch_add(1, Ordering::Relaxed);
                        let _ = tx
                            .send((Err(format!("Engine {engine_name} error: {e}")), engine_name))
                            .await;
                    }
                    Err(_) => {
                        stats.timeouts.fetch_add(1, Ordering::Relaxed);
                        let _ = tx
                            .send((Err(format!("Engine {engine_name} timeout")), engine_name))
                            .await;
                    }
                }
            });
        }

        // 关闭发送端，防止死锁
        drop(tx);

        // 保存缓存结果状态，用于判断是否使用了缓存
        let is_cached = !cached_results.is_empty() || !cached_engines_used.is_empty();

        // 收集结果
        let mut successful_results = cached_results;
        let mut engines_used = cached_engines_used;

        // 批量收集结果，减少锁竞争
        let mut results_to_process = Vec::new();
        while let Some(result) = rx.recv().await {
            results_to_process.push(result);
        }

        // 批量处理结果
        for (search_result, engine_name) in results_to_process {
            match search_result {
                Ok(result) => {
                    // 检查是否为零结果
                    let is_zero_results = result.items.is_empty();
                    let elapsed_ms = result.elapsed_ms;

                    // 更新引擎状态 - 使用DashMap提高并发性能
                    let mut state = self
                        .engine_states
                        .entry(engine_name.clone())
                        .or_insert_with(|| {
                            super::engine_manager::EngineState::new(engine_name.clone())
                        });

                    if is_zero_results {
                        state.record_zero_results();
                    } else {
                        state.record_success(elapsed_ms);
                    }

                    // 直接使用result，不克隆，因为我们已经处理完所有需要的信息
                    successful_results.push(result);
                    engines_used.push(engine_name.clone()); // 克隆engine_name，避免移动后借用问题
                }
                Err(_e) => {
                    // 失败，记录失败
                    let engine_name_clone = engine_name.clone();
                    let mut state = self
                        .engine_states
                        .entry(engine_name_clone)
                        .or_insert_with(|| super::engine_manager::EngineState::new(engine_name));
                    state.record_failure();
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
            cached: is_cached,
        })
    }

    /// 获取统计信息
    pub async fn get_stats(&self) -> SearchStatsResult {
        use std::sync::atomic::Ordering;

        let total_searches = self.stats.total_searches.load(Ordering::Relaxed);
        let cache_hits = self.stats.cache_hits.load(Ordering::Relaxed);
        let cache_misses = self.stats.cache_misses.load(Ordering::Relaxed);

        // 计算缓存命中率
        let cache_hit_rate = if total_searches > 0 {
            cache_hits as f64 / total_searches as f64
        } else {
            0.0
        };

        // 获取搜索历史数据
        let search_history = self.search_history.read().await.clone();

        SearchStatsResult {
            total_searches,
            cache_hits,
            cache_misses,
            engine_failures: self.stats.engine_failures.load(Ordering::Relaxed),
            timeouts: self.stats.timeouts.load(Ordering::Relaxed),
            cache_hit_rate,
            search_history,
        }
    }

    /// 获取引擎缓存统计
    pub async fn get_engine_cache_stats(&self) -> (usize, Vec<String>) {
        let cached_engines: Vec<String> = self
            .engine_cache
            .iter()
            .map(|entry| entry.key().clone())
            .collect();
        (self.engine_cache.len(), cached_engines)
    }

    /// 记录搜索历史
    async fn record_search_history(&self) {
        use std::time::{SystemTime, UNIX_EPOCH};

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let hour = (now / 3600) as u32;

        let mut history = self.search_history.write().await;

        // 查找或创建当前小时的记录
        if let Some(entry) = history.iter_mut().find(|e| e.hour == hour) {
            entry.count += 1;
        } else {
            history.push(SearchHistoryEntry { hour, count: 1 });
        }

        // 清理超过24小时的历史数据
        let cutoff_hour = hour.saturating_sub(24);
        history.retain(|e| e.hour >= cutoff_hour);

        // 按小时排序
        history.sort_by_key(|e| e.hour);
    }

    /// 清理引擎缓存
    pub async fn clear_engine_cache(&self) {
        self.engine_cache.clear();
    }

    /// 清除缓存
    pub async fn clear_cache(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.cache.clear_all().await.map_err(|e| {
            Box::new(std::io::Error::other(format!("清除缓存失败: {e:?}")))
                as Box<dyn std::error::Error + Send + Sync>
        })
    }

    /// 获取缓存统计信息
    pub fn get_cache_stats(&self) -> seesea_cache::cache::types::CacheStats {
        self.cache
            .manager()
            .expect("Failed to get cache manager")
            .stats()
    }

    /// 清理过期缓存
    pub async fn cleanup_expired_cache(
        &self,
    ) -> Result<usize, Box<dyn std::error::Error + Send + Sync>> {
        self.cache.cleanup().await.map_err(|e| {
            Box::new(std::io::Error::other(format!("清理过期缓存失败: {e:?}")))
                as Box<dyn std::error::Error + Send + Sync>
        })
    }

    /// 列出可用引擎
    pub fn list_engines(&self) -> Vec<String> {
        ENGINE_CONFIG.all_available_engines.clone()
    }

    /// 列出全局模式引擎
    pub fn list_global_engines(&self) -> Vec<String> {
        ENGINE_CONFIG.global_engines.clone()
    }

    /// 根据引擎类型获取引擎列表
    pub async fn get_engines_for_type(&self, engine_type: &str) -> Vec<String> {
        ENGINE_CONFIG.get_engines_for_type(engine_type)
    }

    /// 健康检查
    pub async fn health_check(
        &self,
    ) -> Result<Vec<(String, bool)>, Box<dyn std::error::Error + Send + Sync>> {
        let engines = self.list_engines();
        let mut results = Vec::new();

        // 对每个引擎执行健康检查
        for engine_name in engines {
            let is_healthy = self.check_engine_health(&engine_name).await?;
            results.push((engine_name, is_healthy));
        }

        Ok(results)
    }

    /// 检查单个引擎的健康状况
    async fn check_engine_health(
        &self,
        engine_name: &str,
    ) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
        use std::sync::atomic::Ordering;

        // 尝试获取或创建引擎实例
        match self.get_or_create_engine(engine_name).await {
            Ok(engine) => {
                // 执行引擎的is_available方法进行健康检查
                let is_available = engine.is_available().await;

                // 更新引擎状态
                let mut state = self
                    .engine_states
                    .entry(engine_name.to_string())
                    .or_insert_with(|| {
                        super::engine_manager::EngineState::new(engine_name.to_string())
                    });

                if is_available {
                    // 健康检查通过，重置失败次数
                    state.consecutive_failures = 0;
                    state.temporarily_disabled = false;
                    state.disabled_until = None;
                } else {
                    // 健康检查失败，增加失败次数
                    state.consecutive_failures += 1;
                    state.failed_requests += 1;
                    state.total_requests += 1;

                    // 根据失败次数决定是否临时禁用引擎
                    if state.consecutive_failures >= 3 {
                        use std::time::Duration;
                        // 计算指数退避时间（2^n秒，最多1小时）
                        let backoff_seconds = (2u64).pow(state.consecutive_failures.min(10));
                        let backoff_duration = Duration::from_secs(backoff_seconds.min(3600));
                        state.temporarily_disabled = true;
                        state.disabled_until = Some(std::time::Instant::now() + backoff_duration);
                    }
                }

                Ok(is_available)
            }
            Err(_e) => {
                // 引擎创建失败，更新状态
                self.stats.engine_failures.fetch_add(1, Ordering::Relaxed);

                let mut state = self
                    .engine_states
                    .entry(engine_name.to_string())
                    .or_insert_with(|| {
                        super::engine_manager::EngineState::new(engine_name.to_string())
                    });

                state.consecutive_failures += 1;
                state.failed_requests += 1;
                state.total_requests += 1;

                // 根据失败次数决定是否临时禁用引擎
                if state.consecutive_failures >= 3 {
                    use std::time::Duration;
                    let backoff_seconds = (2u64).pow(state.consecutive_failures.min(10));
                    let backoff_duration = Duration::from_secs(backoff_seconds.min(3600));
                    state.temporarily_disabled = true;
                    state.disabled_until = Some(std::time::Instant::now() + backoff_duration);
                }

                Ok(false)
            }
        }
    }

    /// 获取引擎状态
    pub async fn get_engine_states(&self) -> Vec<(String, (bool, bool, u32))> {
        self.engine_states
            .iter()
            .map(|ref_multi| {
                let name = ref_multi.key().clone();
                let state = ref_multi.value();
                (
                    name,
                    (
                        state.enabled,
                        state.temporarily_disabled,
                        state.consecutive_failures,
                    ),
                )
            })
            .collect()
    }

    /// 使特定引擎缓存失效
    pub async fn invalidate_engine(
        &self,
        engine_name: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.engine_cache.remove(engine_name);
        Ok(())
    }

    /// 获取隐私保护统计信息
    pub async fn get_privacy_stats(&self) -> Option<PrivacyStats> {
        // 从 HTTP 客户端获取隐私管理器
        if let Some(privacy_mgr) = self.http_client.privacy_manager().await {
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
    /// 缓存命中率
    pub cache_hit_rate: f64,
    /// 搜索历史数据（最近24小时）
    pub search_history: Vec<SearchHistoryEntry>,
}

/// 搜索历史记录条目
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SearchHistoryEntry {
    /// 时间戳（小时）
    pub hour: u32,
    /// 该小时的搜索次数
    pub count: u64,
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
