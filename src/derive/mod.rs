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

//! 搜索引擎抽象骨架模块
//!
//! 提供搜索引擎的基础 trait 定义和核心类型，构建了一个完整的搜索引擎开发框架。
//!
//! 本模块定义了搜索引擎开发的抽象框架，包括：
//! - 核心数据类型（SearchQuery, SearchResult, EngineInfo 等）
//! - 搜索引擎 trait 体系（SearchEngine, BaseEngine, ConfigurableEngine 等）
//! - RSS Feed 相关类型和抽象接口
//! - 结果和查询处理的抽象接口
//! - 便利开发宏
//!
//! ## 设计原则
//!
//! - **抽象优先**: 使用关联类型和泛型避免具体实现依赖
//! - **模块分离**: HTTP 客户端在 net/client，缓存在 cache/ 模块
//! - **可扩展**: trait 支持灵活的功能组合
//! - **一致性**: 所有搜索引擎实现相同的核心接口
//! - **安全性**: 内置查询验证和清理机制
//!
//! ## 快速开始
//!
//! ```ignore
//! // 使用 engine_metadata! 宏定义引擎元数据
//! engine_metadata! {
//!     name: "MySearchEngine",
//!     categories: ["general"],
//!     paging: true,
//!     time_range_support: false,
//!     safesearch: true,
//!     about: {
//!         website: "https://example.com",
//!         wikidata_id: "Q12345",
//!         use_official_api: false,
//!         require_api_key: false,
//!         results: "HTML"
//!     }
//! }
//!
//! // 实现 SearchEngine trait
//! struct MyEngine {
//!     http_client: HttpClient,
//!     info: EngineInfo,
//! }
//!
//! #[async_trait]
//! impl SearchEngine for MyEngine {
//!     fn info(&self) -> &EngineInfo {
//!         &self.info
//!     }
//!     
//!     async fn search(&self, query: &SearchQuery) -> Result<SearchResult, Box<dyn Error + Send + Sync>> {
//!         // 实现搜索逻辑
//!         Ok(SearchResult::default())
//!     }
//! }
//! ```
//!
//! ## 模块结构
//!
//! - **types**: 定义核心数据结构和枚举
//! - **engine**: 搜索引擎核心 trait 定义
//! - **result**: 搜索结果处理相关 trait
//! - **query**: 查询处理相关 trait
//! - **rss**: RSS feed 相关类型和 trait
//! - **macros**: 便利开发宏
//!
//! ## 核心概念
//!
//! ### 1. 搜索引擎类型体系
//!
//! - **SearchEngine**: 所有搜索引擎的核心 trait，定义了搜索、健康检查等基本方法
//! - **BaseEngine**: 基于 HTTP 的搜索引擎模板，提供了请求构建和响应解析的抽象
//! - **RequestResponseEngine**: 类似 SearxNG 的 request/response 模式
//! - **CacheableEngine**: 支持缓存的搜索引擎
//! - **RetryableEngine**: 支持重试机制的搜索引擎
//! - **ConfigurableEngine**: 可配置的搜索引擎
//!
//! ### 2. 查询处理流程
//!
//! 1. **QueryBuilder**: 构建查询对象
//! 2. **QueryPreprocessor**: 清理和预处理查询
//! 3. **QueryValidator**: 验证查询参数
//! 4. **QueryOptimizer**: 优化查询参数
//! 5. **QueryTransformer**: 转换查询格式
//!
//! ### 3. 结果处理流程
//!
//! 1. **ResultParser**: 解析原始响应为搜索结果
//! 2. **ResultFilter**: 过滤结果（去重、低质量过滤等）
//! 3. **ResultSorter**: 排序结果
//! 4. **ResultEnhancer**: 增强结果（添加图标、语言检测等）
//! 5. **ResultFormatter**: 格式化结果（JSON、HTML、文本等）

pub mod engine;
pub mod macros;
pub mod query;
pub mod result;
pub mod rss;
pub mod types;

// 重新导出主要类型
pub use engine::*;
pub use query::*;
pub use result::*;
pub use rss::*;
pub use types::*;
