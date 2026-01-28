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
//! 提供多搜索引擎支持和查询处理功能。

pub mod search;

// 重新导出常用类型
pub use search::EngineManager;
pub use search::python_engine_bridge::{PythonEngineProxy, PythonEngineRegistry};
pub use search::{ParsedQuery, QueryParser};
pub use search::{SearchRequest, SearchResponse};
pub use seesea_config::{EngineListConfig, EngineMode, SearchConfig as CentralizedSearchConfig};
pub use seesea_derive::{SearchEngine, SearchQuery, SearchResult};
