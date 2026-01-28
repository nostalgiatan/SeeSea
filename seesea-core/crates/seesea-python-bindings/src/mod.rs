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

//! Python bindings module

#[cfg(feature = "python")]
pub mod py_api;
#[cfg(feature = "python")]
pub mod py_browser;
#[cfg(feature = "python")]
pub mod py_cache;
#[cfg(feature = "python")]
pub mod py_cleaner;
#[cfg(feature = "python")]
pub mod py_config;
#[cfg(feature = "python")]
pub mod py_date_page;
#[cfg(feature = "python")]
pub mod py_embedding_callback;
#[cfg(feature = "python")]
pub mod py_engine_registry;
#[cfg(feature = "python")]
pub mod py_event;
#[cfg(feature = "python")]
pub mod py_hot;

#[cfg(feature = "python")]
pub mod py_net;
#[cfg(feature = "python")]
pub mod py_object_pool;
#[cfg(feature = "python")]
pub mod py_raming;
#[cfg(feature = "python")]
pub mod py_rss;
#[cfg(feature = "python")]
pub mod py_search;
#[cfg(feature = "python")]
pub mod py_stock;
#[cfg(feature = "python")]
pub mod py_system_controller;
#[cfg(feature = "python")]
pub mod py_vector_store;
