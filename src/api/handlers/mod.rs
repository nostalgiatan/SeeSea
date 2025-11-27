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

//! API 处理器模块
//!
//! 包含各种 API 请求的处理逻辑

pub mod search;
pub mod health;
pub mod config;
pub mod metrics;
pub mod rss;
pub mod cache;
pub mod static_files;

// Re-export handlers for convenient use
pub use search::{handle_search, handle_search_post};
pub use health::handle_health;
pub use config::handle_magic_link_generate;
pub use metrics::{
    handle_stats, handle_engines_list, handle_version,
    handle_metrics, handle_realtime_metrics
};
pub use static_files::{handle_index, handle_favicon};
