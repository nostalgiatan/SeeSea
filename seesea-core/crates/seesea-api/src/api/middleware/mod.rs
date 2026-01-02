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

//! API 中间件模块
//!
//! 提供各种 HTTP 中间件功能

pub mod auth;
pub mod circuitbreaker;
pub mod cors;
pub mod ipfilter;
pub mod logging;
pub mod magiclink;
pub mod metrics_mw;
pub mod ratelimit;

pub use auth::*;
pub use circuitbreaker::*;
pub use cors::*;
pub use ipfilter::*;
pub use logging::*;
pub use magiclink::*;
pub use metrics_mw::*;
pub use ratelimit::*;
