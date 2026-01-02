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

//! 搜索配置类型定义

use crate::{ConfigValidationResult, SafeSearchLevel};
use serde::{Deserialize, Serialize};

/// 搜索配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchConfig {
    /// 安全搜索级别
    pub safe_search: SafeSearchLevel,
    /// 自动完成引擎
    pub autocomplete: String,
    /// 输出格式
    pub formats: Vec<String>,
    /// 默认每页结果数
    pub results_per_page: usize,
    /// 最大结果数
    pub max_results_per_page: usize,
    /// 搜索超时时间（秒）
    pub search_timeout: u64,
    /// 并发引擎数量限制
    pub max_concurrent_engines: usize,
    /// 最小并发数
    #[serde(default)]
    pub min_concurrency: usize,
    /// 最大并发数
    #[serde(default)]
    pub max_concurrency: usize,
    /// 并发调整间隔（秒）
    #[serde(default)]
    pub concurrency_adjust_interval: u64,
    /// 并发基础倍数
    #[serde(default)]
    pub concurrency_base_multiplier: f64,
    /// 预创建引擎列表
    #[serde(default)]
    pub precreate_engines: Vec<String>,
    /// 预创建引擎数量
    #[serde(default)]
    pub precreate_engine_count: usize,
    /// 引擎缓存最大大小
    #[serde(default)]
    pub engine_cache_max_size: usize,
    /// 引擎缓存最小大小
    #[serde(default)]
    pub engine_cache_min_size: usize,
    /// 引擎缓存过期时间（秒）
    #[serde(default)]
    pub engine_cache_ttl: u64,
    /// 引擎实例池大小
    #[serde(default)]
    pub engine_pool_size: usize,
    /// 引擎实例池最大大小
    #[serde(default)]
    pub engine_pool_max_size: usize,
    /// 引擎实例池最小大小
    #[serde(default)]
    pub engine_pool_min_size: usize,
    /// 引擎健康检查间隔（秒）
    #[serde(default)]
    pub engine_health_check_interval: u64,
    /// 默认语言
    pub default_language: String,
    /// 支持的语言列表
    pub supported_languages: Vec<String>,
    /// 时间范围支持
    pub time_range_support: bool,
    /// 默认时间范围
    pub default_time_range: Option<TimeRange>,
    /// 结果聚合配置
    #[serde(default)]
    pub aggregation: AggregationConfig,
    /// 查询处理配置
    #[serde(default)]
    pub query_processing: QueryProcessingConfig,
}

/// 时间范围
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TimeRange {
    /// 任意时间
    Any,
    /// 最近一天
    Day,
    /// 最近一周
    Week,
    /// 最近一月
    Month,
    /// 最近一年
    Year,
}

/// 结果聚合配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregationConfig {
    /// 启用结果去重
    #[serde(default)]
    pub enable_deduplication: bool,
    /// 去重算法
    #[serde(default)]
    pub deduplication_method: DeduplicationMethod,
    /// 启用结果排序
    #[serde(default)]
    pub enable_ranking: bool,
    /// 排序算法
    #[serde(default)]
    pub ranking_algorithm: RankingAlgorithm,
    /// 最大聚合结果数
    #[serde(default)]
    pub max_results: usize,
    /// 最小引擎权重
    #[serde(default)]
    pub min_engine_weight: f32,
    /// 结果分组
    #[serde(default)]
    pub enable_grouping: bool,
    /// 分组策略
    #[serde(default)]
    pub grouping_strategy: GroupingStrategy,
}

/// 去重算法
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum DeduplicationMethod {
    /// 基于 URL 去重
    Url,
    /// 基于标题去重
    Title,
    /// 基于 URL 和标题去重
    #[default]
    UrlAndTitle,
    /// 基于内容哈希去重
    ContentHash,
    /// 基于相似度去重
    Similarity,
}

/// 排序算法
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum RankingAlgorithm {
    /// 简单加权
    Weighted,
    /// 引擎排名加权
    EngineRankWeighted,
    /// 时间衰减
    TimeDecay,
    /// 机器学习排序
    MlRanking,
    /// 混合排序
    #[default]
    Hybrid,
}

/// 分组策略
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum GroupingStrategy {
    /// 不分组
    None,
    /// 按引擎分组
    ByEngine,
    /// 按域名分组
    ByDomain,
    /// 按类型分组
    ByType,
    /// 智能分组
    #[default]
    Smart,
}

/// 查询处理配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryProcessingConfig {
    /// 启用查询扩展
    #[serde(default)]
    pub enable_expansion: bool,
    /// 查询扩展方法
    #[serde(default)]
    pub expansion_methods: Vec<ExpansionMethod>,
    /// 启用查询纠正
    #[serde(default)]
    pub enable_correction: bool,
    /// 纠正阈值
    #[serde(default)]
    pub correction_threshold: f32,
    /// 启用同义词扩展
    #[serde(default)]
    pub enable_synonyms: bool,
    /// 启用停用词过滤
    #[serde(default)]
    pub enable_stop_words: bool,
    /// 最大查询长度
    #[serde(default)]
    pub max_query_length: usize,
    /// 最小查询长度
    #[serde(default)]
    pub min_query_length: usize,
}

/// 查询扩展方法
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ExpansionMethod {
    /// 同义词扩展
    #[default]
    Synonyms,
    /// 相关词扩展
    RelatedTerms,
    /// 拼写纠正
    SpellingCorrection,
    /// 语言翻译
    Translation,
    /// 缩写展开
    AbbreviationExpansion,
}

impl Default for SearchConfig {
    fn default() -> Self {
        Self {
            safe_search: SafeSearchLevel::None,
            autocomplete: "".to_string(),
            formats: vec!["json".to_string(), "html".to_string()],
            results_per_page: 10,
            max_results_per_page: 50,
            search_timeout: 30,
            max_concurrent_engines: 5,
            min_concurrency: 1,
            max_concurrency: 200,
            concurrency_adjust_interval: 1,
            concurrency_base_multiplier: 10.0,
            precreate_engines: vec![
                "bing".to_string(),
                "baidu".to_string(),
                "yandex".to_string(),
            ],
            precreate_engine_count: 3,
            engine_cache_max_size: 20,
            engine_cache_min_size: 5,
            engine_cache_ttl: 3600, // 1小时
            engine_pool_size: 5,
            engine_pool_max_size: 10,
            engine_pool_min_size: 2,
            engine_health_check_interval: 300, // 5分钟
            default_language: "auto".to_string(),
            supported_languages: vec![
                "en".to_string(),
                "zh".to_string(),
                "ja".to_string(),
                "ko".to_string(),
                "es".to_string(),
                "fr".to_string(),
                "de".to_string(),
                "ru".to_string(),
            ],
            time_range_support: true,
            default_time_range: None,
            aggregation: AggregationConfig::default(),
            query_processing: QueryProcessingConfig::default(),
        }
    }
}

impl SearchConfig {
    /// 验证搜索配置
    pub fn validate(&self) -> ConfigValidationResult {
        let mut result = ConfigValidationResult::success();

        // 检查结果数量
        if self.results_per_page == 0 {
            result.add_error("默认结果数必须大于 0".to_string());
        }

        if self.max_results_per_page < self.results_per_page {
            result.add_error("最大结果数不能小于默认结果数".to_string());
        }

        if self.max_results_per_page > 100 {
            result.add_warning("最大结果数过大可能影响性能".to_string());
        }

        // 检查搜索超时
        if self.search_timeout == 0 {
            result.add_error("搜索超时时间必须大于 0".to_string());
        }

        if self.search_timeout > 300 {
            result.add_warning("搜索超时时间过长（>5分钟）".to_string());
        }

        // 检查并发引擎数
        if self.max_concurrent_engines == 0 {
            result.add_error("并发引擎数必须大于 0".to_string());
        }

        if self.max_concurrent_engines > 20 {
            result.add_warning("并发引擎数过多可能影响性能".to_string());
        }

        // 检查支持的格式
        if self.formats.is_empty() {
            result.add_error("必须指定至少一种输出格式".to_string());
        }

        // 检查查询长度
        if let Some(processing) = Some(&self.query_processing) {
            if processing.min_query_length >= processing.max_query_length {
                result.add_error("最小查询长度不能大于等于最大查询长度".to_string());
            }

            if processing.min_query_length == 0 {
                result.add_warning("最小查询长度为 0 可能导致空查询".to_string());
            }

            if processing.correction_threshold < 0.0 || processing.correction_threshold > 1.0 {
                result.add_error("纠正阈值必须在 0.0-1.0 之间".to_string());
            }
        }

        result
    }

    /// 检查语言是否支持
    pub fn is_language_supported(&self, language: &str) -> bool {
        language == "auto" || self.supported_languages.contains(&language.to_string())
    }

    /// 检查格式是否支持
    pub fn is_format_supported(&self, format: &str) -> bool {
        self.formats.contains(&format.to_string())
    }

    /// 获取有效的结果数量
    pub fn get_valid_results_count(&self, requested: usize) -> usize {
        requested.clamp(1, self.max_results_per_page)
    }
}

impl Default for AggregationConfig {
    fn default() -> Self {
        Self {
            enable_deduplication: true,
            deduplication_method: DeduplicationMethod::UrlAndTitle,
            enable_ranking: true,
            ranking_algorithm: RankingAlgorithm::Hybrid,
            max_results: 100,
            min_engine_weight: 0.1,
            enable_grouping: true,
            grouping_strategy: GroupingStrategy::Smart,
        }
    }
}

impl Default for QueryProcessingConfig {
    fn default() -> Self {
        Self {
            enable_expansion: true,
            expansion_methods: vec![
                ExpansionMethod::Synonyms,
                ExpansionMethod::SpellingCorrection,
            ],
            enable_correction: true,
            correction_threshold: 0.8,
            enable_synonyms: true,
            enable_stop_words: true,
            max_query_length: 200,
            min_query_length: 1,
        }
    }
}
