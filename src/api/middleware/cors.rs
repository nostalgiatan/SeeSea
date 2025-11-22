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

//! CORS 中间件
//!
//! 处理跨域资源共享 (CORS)

use axum::http::{header, Method};
use tower_http::cors::{CorsLayer, Any};

/// 创建 CORS 中间件
///
/// # Returns
///
/// 返回配置好的 CORS 层
pub fn create_cors_layer() -> CorsLayer {
    CorsLayer::new()
        .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
        .allow_origin(Any)
        .allow_headers([header::CONTENT_TYPE, header::AUTHORIZATION])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cors_layer_creation() {
        let _layer = create_cors_layer();
        // CORS layer created successfully
    }
}
