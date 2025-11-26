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

//! 搜索引擎模块
//!
//! 包含所有搜索引擎实现和工厂

// Utility functions for optimizing engine performance
pub mod utils;

// 引入保留的引擎实现
pub mod bing;
pub mod baidu;
pub mod bing_images;
pub mod bing_news;
pub mod bing_videos;
pub mod yandex;
pub mod unsplash;
pub mod sogou;
pub mod sogou_images;
pub mod sogou_videos;
pub mod sogou_wechat;
pub mod bilibili;
pub mod so;

// 统一导出引擎类型
pub use bing::BingEngine;
pub use baidu::BaiduEngine;
pub use bing_images::BingImagesEngine;
pub use bing_news::BingNewsEngine;
pub use bing_videos::BingVideosEngine;
pub use yandex::YandexEngine;
pub use unsplash::UnsplashEngine;
pub use sogou::SogouEngine;
pub use sogou_images::SogouImagesEngine;
pub use sogou_videos::SogouVideosEngine;
pub use sogou_wechat::SogouWeChatEngine;
pub use bilibili::BilibiliEngine;
pub use so::SoEngine;

