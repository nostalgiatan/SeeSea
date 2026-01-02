// Copyright (C) 2024 SeeSea Authors
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as
// published by the Free Software Foundation, either version 3 of the
// License, or (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

//! DatePage 结构体定义
//! 用于存储URL、时间、描述、源数据、数据块列表、向量列表、哈希、最后更新时间、地图和额外信息

use crate::DataBlock;
use std::sync::Arc;
use std::time::SystemTime;

/// 地图项，存储文本和链接
#[derive(Debug, Clone)]
pub struct MapItem {
    /// 文本
    pub text: Arc<str>,
    /// 链接
    pub url: Arc<str>,
}

/// 额外信息项，存储键和值
#[derive(Debug, Clone)]
pub struct ExtraInfoItem {
    /// 键
    pub key: Arc<str>,
    /// 值
    pub value: Arc<str>,
}

/// DatePage 结构体
#[derive(Debug, Clone)]
pub struct DatePage {
    /// URL
    pub url: Arc<str>,
    /// 时间
    pub time: SystemTime,
    /// 描述
    pub description: Arc<str>,
    /// 源数据
    pub source_data: Arc<str>,
    /// 数据块列表
    pub data_blocks: Vec<DataBlock>,
    /// 向量列表
    pub vectors: Vec<Vec<f32>>,
    /// 哈希
    pub hash: u64,
    /// 最后更新时间
    pub last_update_time: SystemTime,
    /// 地图，存储文本和链接
    pub map: Vec<MapItem>,
    /// 额外信息
    pub extra_info: Vec<ExtraInfoItem>,
}

impl DatePage {
    /// 创建新的 DatePage 实例
    pub fn new(
        url: impl Into<Arc<str>>,
        time: SystemTime,
        description: impl Into<Arc<str>>,
        source_data: impl Into<Arc<str>>,
    ) -> Self {
        // 计算源数据的哈希值
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let source_str = source_data.into();
        let mut hasher = DefaultHasher::new();
        source_str.hash(&mut hasher);
        let hash = hasher.finish();

        Self {
            url: url.into(),
            time,
            description: description.into(),
            source_data: source_str,
            data_blocks: Vec::new(),
            vectors: Vec::new(),
            hash,
            last_update_time: SystemTime::now(),
            map: Vec::new(),
            extra_info: Vec::new(),
        }
    }

    /// 检查源数据是否更新
    pub fn is_updated(&self, new_source_data: &str) -> bool {
        // 计算新源数据的哈希值
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        new_source_data.hash(&mut hasher);
        let new_hash = hasher.finish();

        new_hash != self.hash
    }

    /// 更新源数据
    pub fn update_source_data(&mut self, new_source_data: impl Into<Arc<str>>) {
        let source_str = new_source_data.into();

        // 计算新源数据的哈希值
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        source_str.hash(&mut hasher);
        self.hash = hasher.finish();

        self.source_data = source_str;
        self.last_update_time = SystemTime::now();
    }

    /// 清理数据，自动处理各种属性
    pub async fn cleaning(&mut self, cleaner: &crate::Cleaner) {
        // 处理源数据，传递当前哈希值作为old_hash
        let (unchanged, processed_blocks) =
            cleaner.process(&self.source_data, Some(self.hash)).await;

        if !unchanged {
            self.data_blocks = processed_blocks;
            self.last_update_time = SystemTime::now();
        }
    }

    /// 添加地图项
    pub fn add_map_item(&mut self, text: impl Into<Arc<str>>, url: impl Into<Arc<str>>) {
        self.map.push(MapItem {
            text: text.into(),
            url: url.into(),
        });
    }

    /// 添加额外信息项
    pub fn add_extra_info(&mut self, key: impl Into<Arc<str>>, value: impl Into<Arc<str>>) {
        self.extra_info.push(ExtraInfoItem {
            key: key.into(),
            value: value.into(),
        });
    }
}
