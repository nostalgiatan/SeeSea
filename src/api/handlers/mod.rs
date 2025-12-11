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

//! API 处理器模块
//!
//! 包含各种 API 请求的处理逻辑

pub mod cache;
pub mod config;
pub mod health;
pub mod hot;
pub mod metrics;
pub mod pro;
pub mod rss;
pub mod search;
pub mod static_files;

// Re-export handlers for convenient use
pub use config::handle_magic_link_generate;
pub use health::handle_health;
pub use hot::{
    handle_hot_all, handle_hot_multiple, handle_hot_platform, handle_hot_platforms_list,
};
pub use metrics::{
    handle_engines_list, handle_metrics, handle_realtime_metrics, handle_stats, handle_version,
};
pub use pro::handle_pro_api;
pub use search::{handle_search, handle_search_post};
pub use static_files::{get_static_app_path, get_static_html_path, handle_favicon, handle_index};
