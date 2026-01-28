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

//! 元数据缓存
//!
//! 提供引擎元数据和配置的缓存功能

use crate::cache::manager::{CacheError, CacheManager, Result};
use seesea_derive::types::EngineInfo;
use std::sync::Arc;
use std::time::Duration;

/// 元数据缓存作用域
const METADATA_SCOPE: &str = "metadata";

/// 元数据缓存键前缀
const METADATA_KEY_PREFIX: &str = "metadata:";

/// 引擎信息缓存键前缀
const ENGINE_INFO_PREFIX: &str = "engine_info:";

/// 元数据缓存
///
/// 封装 CacheManager，提供元数据专用的缓存接口
pub struct MetadataCache {
    manager: Arc<CacheManager>,
}

impl MetadataCache {
    /// 创建元数据缓存实例
    ///
    /// # 参数
    ///
    /// * `manager` - 缓存管理器（Arc包装）
    pub fn new(manager: Arc<CacheManager>) -> Self {
        Self { manager }
    }

    /// 缓存引擎信息
    ///
    /// # 参数
    ///
    /// * `engine_name` - 引擎名称
    /// * `info` - 引擎信息
    /// * `ttl` - 生存时间，None 表示永不过期（引擎信息通常不变）
    pub fn set_engine_info(
        &self,
        engine_name: &str,
        info: &EngineInfo,
        ttl: Option<Duration>,
    ) -> Result<()> {
        let key = format!("{ENGINE_INFO_PREFIX}{engine_name}");

        // 序列化引擎信息
        let data = bincode::serde::encode_to_vec(info, bincode::config::standard())
            .map_err(|e| CacheError::SerializationError(format!("序列化引擎信息失败: {e}")))?;

        // 引擎信息通常不过期，使用较长的TTL或永不过期
        let ttl = ttl.or(Some(Duration::from_secs(86400 * 365))); // 默认1年
        self.manager.set(METADATA_SCOPE, key, data, ttl)
    }

    /// 异步缓存引擎信息
    ///
    /// # 参数
    ///
    /// * `engine_name` - 引擎名称
    /// * `info` - 引擎信息
    /// * `ttl` - 生存时间，None 表示永不过期（引擎信息通常不变）
    pub async fn set_engine_info_async(
        &self,
        engine_name: &str,
        info: &EngineInfo,
        ttl: Option<Duration>,
    ) -> Result<()> {
        let key = format!("{ENGINE_INFO_PREFIX}{engine_name}");

        // 序列化引擎信息
        let data = bincode::serde::encode_to_vec(info, bincode::config::standard())
            .map_err(|e| CacheError::SerializationError(format!("序列化引擎信息失败: {e}")))?;

        // 引擎信息通常不过期，使用较长的TTL或永不过期
        let ttl = ttl.or(Some(Duration::from_secs(86400 * 365))); // 默认1年
        self.manager.set_async(METADATA_SCOPE, key, data, ttl).await
    }

    /// 获取引擎信息
    ///
    /// # 参数
    ///
    /// * `engine_name` - 引擎名称
    ///
    /// # 返回值
    ///
    /// 返回缓存的引擎信息，如果不存在则返回 None
    pub fn get_engine_info(&self, engine_name: &str) -> Result<Option<EngineInfo>> {
        let key = format!("{ENGINE_INFO_PREFIX}{engine_name}");

        match self.manager.get(METADATA_SCOPE, &key)? {
            Some(data) => {
                let info: EngineInfo =
                    bincode::serde::decode_from_slice(&data, bincode::config::standard())
                        .map(|(info, _)| info)
                        .map_err(|e| {
                            CacheError::SerializationError(format!("反序列化引擎信息失败: {e}"))
                        })?;
                Ok(Some(info))
            }
            None => Ok(None),
        }
    }

    /// 异步获取引擎信息
    ///
    /// # 参数
    ///
    /// * `engine_name` - 引擎名称
    ///
    /// # 返回值
    ///
    /// 返回缓存的引擎信息，如果不存在则返回 None
    pub async fn get_engine_info_async(&self, engine_name: &str) -> Result<Option<EngineInfo>> {
        let key = format!("{ENGINE_INFO_PREFIX}{engine_name}");

        match self.manager.get_async(METADATA_SCOPE, &key).await? {
            Some(data) => {
                let info: EngineInfo =
                    bincode::serde::decode_from_slice(&data, bincode::config::standard())
                        .map(|(info, _)| info)
                        .map_err(|e| {
                            CacheError::SerializationError(format!("反序列化引擎信息失败: {e}"))
                        })?;
                Ok(Some(info))
            }
            None => Ok(None),
        }
    }

    /// 删除引擎信息
    ///
    /// # 参数
    ///
    /// * `engine_name` - 引擎名称
    pub fn delete_engine_info(&self, engine_name: &str) -> Result<bool> {
        let key = format!("{ENGINE_INFO_PREFIX}{engine_name}");
        self.manager.delete(METADATA_SCOPE, &key)
    }

    /// 异步删除引擎信息
    ///
    /// # 参数
    ///
    /// * `engine_name` - 引擎名称
    pub async fn delete_engine_info_async(&self, engine_name: &str) -> Result<bool> {
        let key = format!("{ENGINE_INFO_PREFIX}{engine_name}");
        self.manager.delete_async(METADATA_SCOPE, &key).await
    }

    /// 批量缓存引擎信息
    ///
    /// # 参数
    ///
    /// * `items` - 引擎信息列表，每个元素包含引擎名称和引擎信息
    /// * `ttl` - 生存时间，None 表示永不过期
    pub fn set_engine_info_batch(
        &self,
        items: &[(&str, &EngineInfo)],
        ttl: Option<Duration>,
    ) -> Result<()> {
        let ttl = ttl.or(Some(Duration::from_secs(86400 * 365))); // 默认1年

        for (engine_name, info) in items {
            self.set_engine_info(engine_name, info, ttl)?;
        }

        Ok(())
    }

    /// 异步批量缓存引擎信息
    ///
    /// # 参数
    ///
    /// * `items` - 引擎信息列表，每个元素包含引擎名称和引擎信息
    /// * `ttl` - 生存时间，None 表示永不过期
    pub async fn set_engine_info_batch_async(
        &self,
        items: &[(&str, &EngineInfo)],
        ttl: Option<Duration>,
    ) -> Result<()> {
        let ttl = ttl.or(Some(Duration::from_secs(86400 * 365))); // 默认1年

        for (engine_name, info) in items {
            self.set_engine_info_async(engine_name, info, ttl).await?;
        }

        Ok(())
    }

    /// 缓存通用元数据
    ///
    /// # 参数
    ///
    /// * `key` - 元数据键
    /// * `data` - 元数据（已序列化的字节数组）
    /// * `ttl` - 生存时间
    pub fn set_metadata(&self, key: &str, data: Vec<u8>, ttl: Option<Duration>) -> Result<()> {
        let full_key = format!("{METADATA_KEY_PREFIX}{key}");
        self.manager.set(METADATA_SCOPE, full_key, data, ttl)
    }

    /// 异步缓存通用元数据
    ///
    /// # 参数
    ///
    /// * `key` - 元数据键
    /// * `data` - 元数据（已序列化的字节数组）
    /// * `ttl` - 生存时间
    pub async fn set_metadata_async(
        &self,
        key: &str,
        data: Vec<u8>,
        ttl: Option<Duration>,
    ) -> Result<()> {
        let full_key = format!("{METADATA_KEY_PREFIX}{key}");
        self.manager
            .set_async(METADATA_SCOPE, full_key, data, ttl)
            .await
    }

    /// 获取通用元数据
    ///
    /// # 参数
    ///
    /// * `key` - 元数据键
    ///
    /// # 返回值
    ///
    /// 返回元数据字节数组，如果不存在则返回 None
    pub fn get_metadata(&self, key: &str) -> Result<Option<Vec<u8>>> {
        let full_key = format!("{METADATA_KEY_PREFIX}{key}");
        self.manager.get(METADATA_SCOPE, &full_key)
    }

    /// 异步获取通用元数据
    ///
    /// # 参数
    ///
    /// * `key` - 元数据键
    ///
    /// # 返回值
    ///
    /// 返回元数据字节数组，如果不存在则返回 None
    pub async fn get_metadata_async(&self, key: &str) -> Result<Option<Vec<u8>>> {
        let full_key = format!("{METADATA_KEY_PREFIX}{key}");
        self.manager.get_async(METADATA_SCOPE, &full_key).await
    }

    /// 删除通用元数据
    ///
    /// # 参数
    ///
    /// * `key` - 元数据键
    pub fn delete_metadata(&self, key: &str) -> Result<bool> {
        let full_key = format!("{METADATA_KEY_PREFIX}{key}");
        self.manager.delete(METADATA_SCOPE, &full_key)
    }

    /// 异步删除通用元数据
    ///
    /// # 参数
    ///
    /// * `key` - 元数据键
    pub async fn delete_metadata_async(&self, key: &str) -> Result<bool> {
        let full_key = format!("{METADATA_KEY_PREFIX}{key}");
        self.manager.delete_async(METADATA_SCOPE, &full_key).await
    }

    /// 批量缓存通用元数据
    ///
    /// # 参数
    ///
    /// * `items` - 元数据列表，每个元素包含键和值
    /// * `ttl` - 生存时间
    pub fn set_metadata_batch(
        &self,
        items: &[(&str, Vec<u8>)],
        ttl: Option<Duration>,
    ) -> Result<()> {
        for (key, data) in items {
            self.set_metadata(key, data.clone(), ttl)?;
        }

        Ok(())
    }

    /// 异步批量缓存通用元数据
    ///
    /// # 参数
    ///
    /// * `items` - 元数据列表，每个元素包含键和值
    /// * `ttl` - 生存时间
    pub async fn set_metadata_batch_async(
        &self,
        items: &[(&str, Vec<u8>)],
        ttl: Option<Duration>,
    ) -> Result<()> {
        for (key, data) in items {
            self.set_metadata_async(key, data.clone(), ttl).await?;
        }

        Ok(())
    }

    /// 批量获取通用元数据
    ///
    /// # 参数
    ///
    /// * `keys` - 元数据键列表
    ///
    /// # 返回值
    ///
    /// 返回元数据字节数组列表，与输入键列表顺序一致
    pub fn get_metadata_batch(&self, keys: &[&str]) -> Result<Vec<Option<Vec<u8>>>> {
        let mut results = Vec::with_capacity(keys.len());

        for key in keys {
            let result = self.get_metadata(key)?;
            results.push(result);
        }

        Ok(results)
    }

    /// 异步批量获取通用元数据
    ///
    /// # 参数
    ///
    /// * `keys` - 元数据键列表
    ///
    /// # 返回值
    ///
    /// 返回元数据字节数组列表，与输入键列表顺序一致
    pub async fn get_metadata_batch_async(&self, keys: &[&str]) -> Result<Vec<Option<Vec<u8>>>> {
        let mut results = Vec::with_capacity(keys.len());

        for key in keys {
            let result = self.get_metadata_async(key).await?;
            results.push(result);
        }

        Ok(results)
    }

    /// 批量删除通用元数据
    ///
    /// # 参数
    ///
    /// * `keys` - 元数据键列表
    ///
    /// # 返回值
    ///
    /// 返回成功删除的条目数量
    pub fn delete_metadata_batch(&self, keys: &[&str]) -> Result<usize> {
        let mut deleted = 0;

        for key in keys {
            if self.delete_metadata(key)? {
                deleted += 1;
            }
        }

        Ok(deleted)
    }

    /// 异步批量删除通用元数据
    ///
    /// # 参数
    ///
    /// * `keys` - 元数据键列表
    ///
    /// # 返回值
    ///
    /// 返回成功删除的条目数量
    pub async fn delete_metadata_batch_async(&self, keys: &[&str]) -> Result<usize> {
        let mut deleted = 0;

        for key in keys {
            if self.delete_metadata_async(key).await? {
                deleted += 1;
            }
        }

        Ok(deleted)
    }

    /// 清空所有元数据缓存
    pub fn clear(&self) -> Result<()> {
        self.manager.clear_scope(METADATA_SCOPE)
    }

    /// 异步清空所有元数据缓存
    pub async fn clear_async(&self) -> Result<()> {
        self.manager.clear_scope_async(METADATA_SCOPE).await
    }

    /// 清理过期的元数据
    ///
    /// # 返回值
    ///
    /// 返回清理的条目数量
    pub fn cleanup_expired(&self) -> Result<usize> {
        self.manager.cleanup_expired_by_scope(METADATA_SCOPE)
    }

    /// 异步清理过期的元数据
    ///
    /// # 返回值
    ///
    /// 返回清理的条目数量
    pub async fn cleanup_expired_async(&self) -> Result<usize> {
        self.manager
            .cleanup_expired_by_scope_async(METADATA_SCOPE)
            .await
    }

    /// 获取底层缓存管理器引用
    pub fn manager(&self) -> &CacheManager {
        &self.manager
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cache::types::{CacheImplConfig, CacheMode};
    use seesea_derive::types::{
        AboutInfo, EngineCapabilities, EngineStatus, EngineType, ResultType,
    };
    use serial_test::serial;

    fn temp_metadata_cache() -> MetadataCache {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);

        let temp_dir = std::env::temp_dir();
        let unique_id = COUNTER.fetch_add(1, Ordering::SeqCst);
        let db_path = temp_dir.join(format!(
            "test_metadata_cache_{}_{}",
            std::process::id(),
            unique_id
        ));

        let config = CacheImplConfig {
            db_path: db_path.to_string_lossy().to_string(),
            secondary_path: None,
            is_secondary: false,
            default_ttl_secs: 10,
            max_size_bytes: 1024 * 1024,
            enabled: true,
            compression: false,
            mode: CacheMode::HighThroughput,
            enable_bloom_filter: false,
            bloom_filter_expected_elements: 1000,
            bloom_filter_false_positive_rate: 0.01,
        };

        let manager = CacheManager::instance(config).expect("Failed to create cache manager");
        MetadataCache::new(manager)
    }

    fn sample_engine_info() -> EngineInfo {
        EngineInfo {
            name: "TestEngine".to_string(),
            engine_type: EngineType::General,
            description: "A test search engine".to_string(),
            status: EngineStatus::Active,
            categories: vec!["general".to_string()],
            capabilities: EngineCapabilities {
                result_types: vec![ResultType::Web],
                supported_params: vec!["q".to_string()],
                max_page_size: 50,
                supports_pagination: true,
                supports_time_range: false,
                supports_language_filter: true,
                supports_region_filter: false,
                supports_safe_search: true,
                rate_limit: Some(60),
            },
            about: AboutInfo {
                website: Some("https://test.com".to_string()),
                wikidata_id: Some("Q12345".to_string()),
                official_api_documentation: None,
                use_official_api: false,
                require_api_key: false,
                results: "HTML".to_string(),
            },
            shortcut: Some("te".to_string()),
            timeout: Some(30),
            disabled: false,
            inactive: false,
            version: Some("1.0.0".to_string()),
            last_checked: None,
            using_tor_proxy: false,
            display_error_messages: true,
            tokens: Vec::new(),
            max_page: 0,
        }
    }

    #[test]
    #[serial]
    fn test_metadata_cache_set_and_get_engine_info() {
        let cache = temp_metadata_cache();
        let info = sample_engine_info();
        let engine_name = "TestEngine";

        // 缓存引擎信息
        let _ = cache.set_engine_info(engine_name, &info, None);

        // 获取引擎信息
        let cached = cache
            .get_engine_info(engine_name)
            .expect("获取引擎信息失败");

        assert!(cached.is_some());
        let cached_info = cached.unwrap();
        assert_eq!(cached_info.name, info.name);
        assert_eq!(cached_info.engine_type, info.engine_type);
    }

    #[test]
    #[serial]
    fn test_metadata_cache_miss() {
        let cache = temp_metadata_cache();

        // 获取不存在的引擎信息
        let cached = cache
            .get_engine_info("NonExistent")
            .expect("获取引擎信息失败");
        assert!(cached.is_none());
    }

    #[test]
    #[serial]
    fn test_metadata_cache_delete_engine_info() {
        let cache = temp_metadata_cache();
        let info = sample_engine_info();
        let engine_name = "TestEngine";

        // 缓存引擎信息
        let _ = cache.set_engine_info(engine_name, &info, None);

        assert!(
            cache
                .get_engine_info(engine_name)
                .expect("获取引擎信息失败")
                .is_some()
        );

        // 删除引擎信息
        let deleted = cache
            .delete_engine_info(engine_name)
            .expect("删除引擎信息失败");
        assert!(deleted);

        // 验证已删除
        assert!(
            cache
                .get_engine_info(engine_name)
                .expect("获取引擎信息失败")
                .is_none()
        );
    }

    #[test]
    #[serial]
    fn test_metadata_cache_generic_metadata() {
        let cache = temp_metadata_cache();
        let key = "test_metadata";
        let data = b"test data".to_vec();

        // 缓存元数据
        cache
            .set_metadata(key, data.clone(), None)
            .expect("缓存元数据失败");

        // 获取元数据
        let cached = cache.get_metadata(key).expect("获取元数据失败");

        assert!(cached.is_some());
        assert_eq!(cached.unwrap(), data);
    }

    #[test]
    #[serial]
    fn test_metadata_cache_delete_metadata() {
        let cache = temp_metadata_cache();
        let key = "test_metadata";
        let data = b"test data".to_vec();

        // 缓存元数据
        cache.set_metadata(key, data, None).expect("缓存元数据失败");

        assert!(cache.get_metadata(key).expect("获取元数据失败").is_some());

        // 删除元数据
        let deleted = cache.delete_metadata(key).expect("删除元数据失败");
        assert!(deleted);

        // 验证已删除
        assert!(cache.get_metadata(key).expect("获取元数据失败").is_none());
    }
}
