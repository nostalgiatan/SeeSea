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

//! 搜索配置类型定义

use crate::config::common::{ConfigValidationResult, SafeSearchLevel};
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
    /// 默认语言
    pub default_language: String,
    /// 支持的语言列表
    pub supported_languages: Vec<String>,
    /// 时间范围支持
    pub time_range_support: bool,
    /// 默认时间范围
    pub default_time_range: Option<TimeRange>,
    /// 结果聚合配置
    pub aggregation: AggregationConfig,
    /// 查询处理配置
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
    pub enable_deduplication: bool,
    /// 去重算法
    pub deduplication_method: DeduplicationMethod,
    /// 启用结果排序
    pub enable_ranking: bool,
    /// 排序算法
    pub ranking_algorithm: RankingAlgorithm,
    /// 最大聚合结果数
    pub max_results: usize,
    /// 最小引擎权重
    pub min_engine_weight: f32,
    /// 结果分组
    pub enable_grouping: bool,
    /// 分组策略
    pub grouping_strategy: GroupingStrategy,
}

/// 去重算法
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeduplicationMethod {
    /// 基于 URL 去重
    Url,
    /// 基于标题去重
    Title,
    /// 基于 URL 和标题去重
    UrlAndTitle,
    /// 基于内容哈希去重
    ContentHash,
    /// 基于相似度去重
    Similarity,
}

/// 排序算法
#[derive(Debug, Clone, Serialize, Deserialize)]
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
    Hybrid,
}

/// 分组策略
#[derive(Debug, Clone, Serialize, Deserialize)]
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
    Smart,
}

/// 查询处理配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryProcessingConfig {
    /// 启用查询扩展
    pub enable_expansion: bool,
    /// 查询扩展方法
    pub expansion_methods: Vec<ExpansionMethod>,
    /// 启用查询纠正
    pub enable_correction: bool,
    /// 纠正阈值
    pub correction_threshold: f32,
    /// 启用同义词扩展
    pub enable_synonyms: bool,
    /// 启用停用词过滤
    pub enable_stop_words: bool,
    /// 最大查询长度
    pub max_query_length: usize,
    /// 最小查询长度
    pub min_query_length: usize,
}

/// 查询扩展方法
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExpansionMethod {
    /// 同义词扩展
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