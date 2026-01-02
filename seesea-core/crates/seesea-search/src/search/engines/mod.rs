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

//! # 搜索引擎模块
//!
//! 搜索引擎模块包含 SeeSea 支持的所有搜索引擎实现，以及用于简化引擎开发的工具和宏。
//!
//! ## 模块架构
//!
//! 搜索引擎模块采用模块化设计，主要包含以下组件：
//!
//! - **utils**：引擎工具函数和通用引擎实现
//! - **define_engine!**：引擎生成宏，用于简化引擎开发
//! - 各种搜索引擎实现：包括 Bing、Baidu、Yandex 等 12+ 专业搜索引擎
//!
//! ## 引擎生成宏
//!
//! `define_engine!` 宏是本模块的核心，它用于自动生成搜索引擎的基本结构体和 trait 实现，
//! 大大减少了重复代码，提高了开发效率。
//!
//! ## 搜索引擎类型
//!
//! SeeSea 支持多种类型的搜索引擎，包括：
//!
//! - **通用搜索**：Bing、Yandex、百度、搜狗、360 搜索
//! - **图片搜索**：Unsplash、Bing Images
//! - **视频搜索**：Bilibili、Bing Videos、搜狗视频
//! - **新闻搜索**：Bing News
//!
//! ## 引擎开发指南
//!
//! 要添加新的搜索引擎，只需：
//! 1. 创建新的模块文件
//! 2. 使用 `define_engine!` 宏定义引擎结构体
//! 3. 实现具体的搜索逻辑
//! 4. 在本文件中导入并导出新引擎

/// 引擎工具模块，包含通用引擎实现和性能优化工具
pub mod utils;

/// 引擎生成宏，用于自动生成搜索引擎的基本结构体和 trait 实现
///
/// 这个宏是搜索引擎开发的核心工具，它自动生成：
/// - 引擎结构体定义
/// - `new()` 和 `with_client()` 构造方法
/// - `Default` trait 实现
/// - `SearchEngine` trait 基本实现
/// - 引擎信息管理
///
/// # 参数
///
/// * `$engine_name` - 引擎结构体名称，如 `BingEngine`
/// * `$info_builder` - 引擎信息构建表达式，用于初始化引擎的基本信息
///
/// # 生成的功能
///
/// 宏生成的引擎结构体包含：
/// - 通用引擎实例，处理HTTP请求和响应
/// - 引擎信息管理
/// - 基本的搜索方法实现
/// - 可用性检查
///
/// # 示例
///
/// ```ignore
/// define_engine! {
///     MyCustomEngine,
///     EngineInfo {
///         name: "my_engine".to_string(),
///         engine_type: EngineType::General,
///         description: "自定义搜索引擎".to_string(),
///         base_url: "https://api.example.com/search".to_string(),
///         supports_images: true,
///         supports_videos: false,
///         supports_news: false,
///         ..Default::default()
///     }
/// }
///
/// // 实现具体的搜索逻辑
/// #[async_trait]
/// impl RequestResponseEngine for MyCustomEngine {
///     async fn search_impl(&self, query: &SearchQuery) -> Result<Vec<SearchResult>> {
///         // 实现自定义搜索逻辑
///     }
/// }
/// ```
macro_rules! define_engine {
    ($engine_name:ident, $info_builder:expr) => {
        /// 自动生成的搜索引擎结构体
        pub struct $engine_name {
            /// 通用引擎实例，处理HTTP请求和响应
            generic: super::utils::GenericEngine,
        }

        impl $engine_name {
            /// 创建新的引擎实例
            ///
            /// # 返回值
            ///
            /// 返回一个新的引擎实例，使用默认的HTTP客户端
            pub fn new() -> Self {
                let info = $info_builder;
                Self {
                    generic: super::utils::GenericEngine::new(info),
                }
            }

            /// 使用共享的HTTP客户端创建引擎实例
            ///
            /// # 参数
            ///
            /// * `client` - 共享的HTTP客户端实例
            ///
            /// # 返回值
            ///
            /// 返回一个使用共享HTTP客户端的引擎实例
            pub fn with_client(client: Arc<seesea_net::client::HttpClient>) -> Self {
                let info = $info_builder;
                Self {
                    generic: super::utils::GenericEngine::with_client(info, client),
                }
            }
        }

        impl Default for $engine_name {
            /// 默认实现，创建新的引擎实例
            fn default() -> Self {
                Self::new()
            }
        }

        #[async_trait]
        impl seesea_derive::SearchEngine for $engine_name {
            /// 获取引擎信息
            ///
            /// # 返回值
            ///
            /// 返回引擎的基本信息，包括名称、类型、描述等
            fn info(&self) -> &seesea_derive::EngineInfo {
                &self.generic.info
            }

            /// 执行搜索
            ///
            /// # 参数
            ///
            /// * `query` - 搜索查询
            ///
            /// # 返回值
            ///
            /// 返回搜索结果，或错误信息
            async fn search(
                &self,
                query: &seesea_derive::SearchQuery,
            ) -> Result<seesea_derive::SearchResult, Box<dyn std::error::Error + Send + Sync>> {
                <Self as seesea_derive::RequestResponseEngine>::search(self, query).await
            }

            /// 检查引擎是否可用
            ///
            /// # 返回值
            ///
            /// 如果引擎可用返回true，否则返回false
            async fn is_available(&self) -> bool {
                // 默认实现：尝试访问引擎的主页
                if let Some(website) = &self.generic.info.about.website {
                    self.generic.client.get(website, None).await.is_ok()
                } else {
                    // 如果没有网站信息，返回false
                    false
                }
            }
        }
    };
}

// 搜索引擎实现模块

/// Bing 搜索引擎实现
pub mod bing;

/// 百度搜索引擎实现
pub mod baidu;

/// Bing 图片搜索引擎实现
pub mod bing_images;

/// Yandex 搜索引擎实现
pub mod yandex;

/// Unsplash 图片搜索引擎实现
pub mod unsplash;

/// 搜狗搜索引擎实现
pub mod sogou;

/// 搜狗视频搜索引擎实现
pub mod sogou_videos;

/// Bilibili 视频搜索引擎实现
pub mod bilibili;

/// 360 搜索引擎实现
pub mod so;

// 统一导出引擎类型，方便外部使用

/// Bing 搜索引擎
pub use bing::BingEngine;

/// 百度搜索引擎
pub use baidu::BaiduEngine;

/// Bing 图片搜索引擎
pub use bing_images::BingImagesEngine;

/// Yandex 搜索引擎
pub use yandex::YandexEngine;

/// Unsplash 图片搜索引擎
pub use unsplash::UnsplashEngine;

/// 搜狗搜索引擎
pub use sogou::SogouEngine;

/// 搜狗视频搜索引擎
pub use sogou_videos::SogouVideosEngine;

/// Bilibili 视频搜索引擎
pub use bilibili::BilibiliEngine;

/// 360 搜索引擎
pub use so::SoEngine;
