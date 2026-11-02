// Copyright (C) 2025 SeeSea Team
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

//! 正文抽取处理器
//!
//! `POST /extract`：抓取一个 URL（`seesea-fetch`）并用 `seesea-extract` 抽取正文，返回 Core 与 sections。

use axum::{
    extract::Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use once_cell::sync::Lazy;
use serde::Deserialize;

use crate::api::types::ApiErrorResponse;
use seesea_extract::{Extractor, Options, Scope};
use seesea_fetch::{FetchError, FetchOptions};

/// 内置模型的解压与加载有成本，进程内只做一次
static EXTRACTOR: Lazy<Extractor> = Lazy::new(Extractor::new);

/// 抽取请求
#[derive(Debug, Clone, Deserialize)]
pub struct ApiExtractRequest {
    /// 要抽取的页面地址（仅 http/https）
    pub url: String,
    /// `core`（默认）只要正文；`extended` 连评论等一并算作正文
    #[serde(default)]
    pub scope: Option<String>,
}

fn error(status: StatusCode, code: &str, message: impl Into<String>) -> Response {
    let body = ApiErrorResponse {
        code: code.to_string(),
        message: message.into(),
        details: None,
    };
    (status, Json(body)).into_response()
}

/// 抓取并抽取一个页面的正文
pub async fn handle_extract(Json(req): Json<ApiExtractRequest>) -> Response {
    let scope = match req.scope.as_deref() {
        None | Some("core") => Scope::Core,
        Some("extended") => Scope::Extended,
        Some(other) => {
            return error(
                StatusCode::BAD_REQUEST,
                "invalid_scope",
                format!("unknown scope '{other}', expected 'core' or 'extended'"),
            );
        }
    };
    let page = match seesea_fetch::fetch(&req.url, &FetchOptions::default()).await {
        Ok(p) => p,
        Err(e) => {
            let (status, code) = match e {
                FetchError::InvalidUrl(_) => (StatusCode::BAD_REQUEST, "invalid_url"),
                FetchError::Blocked(_) => (StatusCode::FORBIDDEN, "target_not_allowed"),
                _ => (StatusCode::BAD_GATEWAY, "fetch_failed"),
            };
            return error(status, code, e.to_string());
        }
    };

    // 解析和打分是 CPU 密集的，放到阻塞线程池里
    let result = tokio::task::spawn_blocking(move || {
        let opts = Options {
            scope,
            ..Options::default()
        };
        EXTRACTOR.extract(&page.html, &page.final_url, &opts)
    })
    .await;

    match result {
        Ok(out) => (StatusCode::OK, Json(out)).into_response(),
        Err(e) => error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "extract_failed",
            e.to_string(),
        ),
    }
}
