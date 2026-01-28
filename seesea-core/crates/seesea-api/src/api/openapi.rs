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

//! OpenAPI 文档模块
//!
//! 使用 utoipa 自动生成 OpenAPI 3.0 规范文档

use utoipa::OpenApi;
use utoipa::openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme};

/// API 文档结构
#[derive(OpenApi)]
#[openapi(
    paths(
        crate::api::handlers::search::handle_search,
        crate::api::handlers::search::handle_search_post,
        crate::api::handlers::health::handle_health,
        crate::api::handlers::hot::handle_hot_platform,
        crate::api::handlers::hot::handle_hot_all,
        crate::api::handlers::hot::handle_hot_multiple,
        crate::api::handlers::hot::handle_hot_platforms_list,
        crate::api::handlers::rss::handle_rss_feeds_list,
        crate::api::handlers::rss::handle_rss_fetch,
        crate::api::handlers::rss::handle_rss_templates_list,
        crate::api::handlers::rss::handle_rss_template_add,
        crate::api::handlers::cache::handle_cache_stats,
        crate::api::handlers::cache::handle_cache_clear,
        crate::api::handlers::cache::handle_cache_cleanup,
        crate::api::handlers::metrics::handle_stats,
        crate::api::handlers::metrics::handle_engines_list,
        crate::api::handlers::metrics::handle_version,
        crate::api::handlers::metrics::handle_metrics,
        crate::api::handlers::metrics::handle_realtime_metrics,
        crate::api::handlers::config::handle_magic_link_generate,
        crate::api::handlers::internal::handle_system_resources,
        crate::api::handlers::internal::handle_engines_status,
        crate::api::handlers::internal::handle_system_status,
        crate::api::handlers::internal::handle_cache_keys,
        crate::api::handlers::internal_extended::handle_engines_list_full,
        crate::api::handlers::internal_extended::handle_engine_toggle,
        crate::api::handlers::internal_extended::handle_engines_batch,
        crate::api::handlers::internal_extended::handle_cache_clear_pattern,
        crate::api::handlers::internal_extended::handle_cache_stats_detail,
        crate::api::handlers::internal_extended::handle_controller_action,
        crate::api::handlers::internal_extended::handle_config_get,
        crate::api::handlers::internal_extended::handle_config_update,
        crate::api::handlers::internal_extended::handle_version_info,
        crate::api::handlers::internal_extended::handle_logs_directory,
        crate::api::handlers::internal_extended::handle_logs_files,
        crate::api::handlers::internal_extended::handle_logs_read,
        crate::api::handlers::internal_extended::handle_logs_tail,
        crate::api::handlers::internal_extended::handle_logs_errors,
        crate::api::handlers::internal_extended::handle_connections_stats,
        crate::api::handlers::internal_extended::handle_health_detail,
    ),
    components(schemas(
        crate::api::types::ApiSearchRequest,
        crate::api::types::ApiSearchResponse,
        crate::api::types::ApiSearchResultItem,
        crate::api::types::ApiErrorResponse,
        crate::api::types::ApiHealthResponse,
        crate::api::handlers::hot::ApiHotSearchRequest,
        seesea_hot::types::HotTrendResult,
        crate::api::handlers::rss::RssFetchRequest,
        crate::api::handlers::rss::RssFeedResponse,
        crate::api::handlers::rss::RssFeedMeta,
        crate::api::handlers::rss::RssFeedItemResponse,
        crate::api::handlers::rss::TemplateAddRequest,
        crate::api::handlers::rss::TemplateAddResponse,
        crate::api::handlers::rss::StoredFeed,
        crate::api::handlers::cache::CacheStatsResponse,
        crate::api::handlers::cache::CacheClearResponse,
        crate::api::internal_types::InternalResourceStatusResponse,
        crate::api::internal_types::InternalSystemStatusResponse,
        crate::api::internal_types::InternalCacheKeysResponse,
        crate::api::internal_types::InternalEngineStatus,
        crate::api::handlers::internal_extended::EngineToggleRequest,
        crate::api::handlers::internal_extended::BatchEngineRequest,
        crate::api::handlers::internal_extended::CacheClearRequest,
        crate::api::handlers::internal_extended::ControllerActionRequest,
        crate::api::handlers::internal_extended::ActionResponse,
        crate::api::handlers::internal_extended::ConfigValueRequest,
        crate::api::handlers::config::MagicLinkResponse,
        crate::api::handlers::internal_extended::LogFileMetadata,
    )),
    tags(
        (name = "search", description = "搜索相关接口"),
        (name = "health", description = "健康检查接口"),
        (name = "hot", description = "热门搜索接口"),
        (name = "rss", description = "RSS 订阅接口"),
        (name = "cache", description = "缓存管理接口"),
        (name = "metrics", description = "指标和统计接口"),
        (name = "config", description = "配置管理接口"),
        (name = "internal", description = "内部管理接口（仅限本地访问）"),
    ),
    modifiers(&SecurityAddon),
    info(
        title = "SeeSea API",
        version = "2.1.0",
        description = "SeeSea 搜索引擎 API 文档",
        contact(
            name = "SeeSea Team",
            email = "support@seesea.com"
        ),
        license(
            name = "AGPL-3.0",
            url = "https://www.gnu.org/licenses/agpl-3.0.html"
        )
    )
)]
pub struct ApiDoc;

/// 安全认证修饰符
struct SecurityAddon;

impl utoipa::Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        if let Some(components) = &mut openapi.components {
            components.add_security_scheme(
                "api_key",
                SecurityScheme::Http(
                    HttpBuilder::new()
                        .scheme(HttpAuthScheme::Bearer)
                        .bearer_format("JWT")
                        .build(),
                ),
            )
        }
    }
}
