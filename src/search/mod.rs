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

//! # 搜索模块
//!
//! 提供统一的多引擎搜索服务，支持动态引擎管理和配置化搜索。
//!
//! 新架构特性：
//! - 工厂模式的引擎创建，消除重复代码
//! - 统一的搜索引擎接口，支持配置化创建
//! - 生命周期管理，避免重复创建资源
//! - 清晰的职责划分，每个组件只负责一个功能

pub mod aggregator;
pub mod engines;
pub mod query;
pub mod types;
pub mod scoring;
pub mod standardization;
pub mod engine_manager;

// 核心组件
pub mod engine_config;
pub mod on;

// 统一导出 - 明确导出以避免歧义
pub use aggregator::{SearchAggregator, AggregationStrategy, SortBy};
pub use query::{QueryParser, ParsedQuery};
pub use types::{SearchRequest, SearchResponse, SearchConfig};
pub use scoring::{BM25Params, ScoringWeights, get_engine_authority, score_results, score_and_sort_results};
pub use standardization::{clean_text, standardize_item, deduplicate_by_url, standardize_results};

// 引擎配置导出
pub use engine_config::{EngineListConfig, EngineMode};

// 引擎管理器导出（避免全局导出避免冲突）
pub use engine_manager::{EngineManager, EngineState};

// 主要接口导出
pub use on::{SearchInterface, SearchStats, SearchStatsResult};
