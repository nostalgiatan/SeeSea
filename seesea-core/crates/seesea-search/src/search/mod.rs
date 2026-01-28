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

//! # 搜索模块
//!
//! 搜索模块是 SeeSea 的核心组件，负责处理搜索请求的整个生命周期，包括查询解析、引擎调度、结果聚合和排序。
//!
//! ## 模块架构
//!
//! 搜索模块采用分层设计，主要包含以下核心组件：
//!
//! - **query**：查询解析组件，负责将原始查询转换为结构化的搜索请求
//! - **engines**：搜索引擎集合，包含各种搜索引擎的实现
//! - **engine_manager**：引擎管理器，负责引擎的生命周期管理和调度
//! - **engine_config**：引擎配置组件，负责管理引擎的配置信息
//! - **aggregator**：结果聚合组件，负责将多个引擎的结果合并和排序
//! - **scoring**：结果评分组件，负责对搜索结果进行评分和排序
//! - **standardization**：结果标准化组件，负责结果的清洗、去重和标准化
//! - **types**：核心数据类型定义，包括搜索请求、响应和配置
//! - **on**：主要接口定义，提供统一的搜索服务接口
//!
//! ## 工作流程
//!
//! 1. 接收搜索请求，通过 QueryParser 解析为结构化查询
//! 2. EngineManager 根据配置选择合适的搜索引擎
//! 3. 并行调用多个搜索引擎执行搜索
//! 4. 收集所有搜索引擎的结果
//! 5. 通过 standardization 组件对结果进行清洗和标准化
//! 6. 通过 scoring 组件对结果进行评分
//! 7. 通过 aggregator 组件对结果进行聚合和排序
//! 8. 返回最终的搜索结果
//!
//! ## 核心特性
//!
//! - **多引擎并行搜索**：支持同时调用多个搜索引擎，提高搜索效率
//! - **智能结果聚合**：支持多种聚合策略，包括按相关性、时间、引擎等排序
//! - **动态引擎管理**：支持运行时添加、移除和配置搜索引擎
//! - **结果去重**：基于URL、标题等多种方式进行结果去重
//! - **语义评分**：支持 BM25 等多种评分算法
//! - **可扩展架构**：支持轻松添加新的搜索引擎和聚合策略
//!
//! ## 使用示例
//!
//! ```rust
//! use seesea::seesea_seesea_search::{SearchInterface, SearchRequest, EngineManager};
//!
//! // 创建引擎管理器
//! let engine_manager = EngineManager::new();
//!
//! // 创建搜索请求
//! let request = SearchRequest {
//!     query: "Rust 编程".to_string(),
//!     engines: vec!["bing", "baidu"].into_iter().collect(),
//!     page: 1,
//!     ..Default::default()
//! };
//!
//! // 执行搜索
//! let response = engine_manager.search(&request).await;
//! ```

/// 搜索引擎集合，包含各种搜索引擎的实现
pub mod engines;

/// 查询解析组件，负责将原始查询转换为结构化的搜索请求
pub mod query;

/// 核心数据类型定义，包括搜索请求、响应和配置
pub mod types;

/// 结果评分组件，负责对搜索结果进行评分和排序
pub mod scoring;

/// 结果标准化组件，负责结果的清洗、去重和标准化
pub mod standardization;

/// 引擎管理器，负责引擎的生命周期管理和调度
pub mod engine_manager;

/// 引擎配置组件，负责管理引擎的配置信息
pub mod engine_config;

/// 搜索工具模块，提供搜索相关的工具函数和组件
pub mod utils;

/// 搜索结果可视化模块，负责将搜索结果以二维方式排列
pub mod result_visualization;

/// 向量化相关性评分模块，使用嵌入向量进行语义相关性计算
pub mod vector_scoring;

/// Python引擎桥接模块，通过raming系统与Python引擎通信
pub mod python_engine_bridge;

/// 主要接口定义，提供统一的搜索服务接口
pub mod on;

// 统一导出核心组件，方便外部使用

/// 查询解析器，负责将原始查询转换为结构化查询
pub use query::{ParsedQuery, QueryParser};

/// 搜索请求、响应和配置的数据类型
pub use types::{SearchConfig, SearchRequest, SearchResponse};

/// 结果评分相关辅助函数，包括引擎权威度和精确匹配
pub use scoring::{exact_match_bonus, exact_match_bonus_optimized, get_engine_authority};

/// 结果标准化相关函数，包括文本清洗、结果标准化和去重
pub use standardization::{
    clean_text, deduplicate_by_url, standardize_item, standardize_items, standardize_results,
};

/// 引擎配置相关类型，包括引擎列表配置和引擎模式
pub use engine_config::{EngineListConfig, EngineMode};

/// 引擎管理器，负责引擎的生命周期管理和调度
pub use engine_manager::{EngineManager, EngineState};

/// 主要搜索接口，提供统一的搜索服务
pub use on::{SearchInterface, SearchStats, SearchStatsResult};

/// 搜索结果时间排序相关组件
pub use result_visualization::{TimeSortStats, TimeSortedResult, TimeSorter};

/// 向量化评分相关组件，使用 SIMD 加速的语义相关性计算
pub use vector_scoring::{
    VectorScorer, VectorScoringWeights, get_vector_cache, is_vector_scoring_available,
};

/// Python引擎桥接相关组件，用于与Python引擎通信
pub use python_engine_bridge::{PythonEngineInfo, PythonEngineProxy, PythonEngineRegistry};
