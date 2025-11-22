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

//! 搜索引擎抽象骨架模块
//!
//! 提供搜索引擎的基础 trait 定义和核心类型。
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

pub mod types;
pub mod engine;
pub mod result;
pub mod query;
pub mod macros;
pub mod rss;

// 重新导出主要类型
pub use types::*;
pub use engine::*;
pub use result::*;
pub use query::*;
pub use rss::*;