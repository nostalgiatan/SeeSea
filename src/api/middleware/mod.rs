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

//! API 中间件模块
//!
//! 提供各种 HTTP 中间件功能

pub mod cors;
pub mod ratelimit;
pub mod logging;
pub mod auth;
pub mod circuitbreaker;
pub mod ipfilter;
pub mod magiclink;

pub use cors::*;
pub use ratelimit::*;
pub use logging::*;
pub use auth::*;
pub use circuitbreaker::*;
pub use ipfilter::*;
pub use magiclink::*;
