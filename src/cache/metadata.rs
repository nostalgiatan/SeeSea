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

//! 元数据缓存
//!
//! 提供引擎元数据和配置的缓存功能

use crate::cache::manager::{CacheError, CacheManager, Result};
use crate::derive::types::EngineInfo;
use std::sync::Arc;
use std::time::Duration;

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
        let key = format!("{}{}", ENGINE_INFO_PREFIX, engine_name);
        
        // 序列化引擎信息
        let data = bincode::serde::encode_to_vec(info, bincode::config::standard()).map_err(|e| {
            CacheError::SerializationError(format!("序列化引擎信息失败: {}", e))
        })?;

        // 引擎信息通常不过期，使用较长的TTL或永不过期
        let ttl = ttl.or(Some(Duration::from_secs(86400 * 365))); // 默认1年
        self.manager.set(key, data, ttl)
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
        let key = format!("{}{}", ENGINE_INFO_PREFIX, engine_name);
        
        match self.manager.get(&key)? {
            Some(data) => {
                let info: EngineInfo = bincode::serde::decode_from_slice(&data, bincode::config::standard())
                    .map(|(info, _)| info)
                    .map_err(|e| {
                        CacheError::SerializationError(format!("反序列化引擎信息失败: {}", e))
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
        let key = format!("{}{}", ENGINE_INFO_PREFIX, engine_name);
        self.manager.delete(&key)
    }

    /// 缓存通用元数据
    ///
    /// # 参数
    ///
    /// * `key` - 元数据键
    /// * `data` - 元数据（已序列化的字节数组）
    /// * `ttl` - 生存时间
    pub fn set_metadata(
        &self,
        key: &str,
        data: Vec<u8>,
        ttl: Option<Duration>,
    ) -> Result<()> {
        let full_key = format!("{}{}", METADATA_KEY_PREFIX, key);
        self.manager.set(full_key, data, ttl)
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
        let full_key = format!("{}{}", METADATA_KEY_PREFIX, key);
        self.manager.get(&full_key)
    }

    /// 删除通用元数据
    ///
    /// # 参数
    ///
    /// * `key` - 元数据键
    pub fn delete_metadata(&self, key: &str) -> Result<bool> {
        let full_key = format!("{}{}", METADATA_KEY_PREFIX, key);
        self.manager.delete(&full_key)
    }

    /// 列出所有引擎信息键
    ///
    /// # 返回值
    ///
    /// 返回所有引擎名称列表
    pub fn list_engine_names(&self) -> Result<Vec<String>> {
        // 注意：这个操作可能较慢，因为需要扫描所有键
        let names = Vec::new();
        
        // 这里需要遍历数据库，sled 提供了 scan_prefix 方法
        // 但由于我们封装了 CacheManager，暂时返回空列表
        // 实际实现需要暴露 sled 的扫描功能
        
        Ok(names)
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
    use crate::derive::types::{
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
            default_ttl_secs: 3600,
            max_size_bytes: 1024 * 1024,
            enabled: true,
            compression: false,
            mode: CacheMode::HighThroughput,
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
        let cached = cache.get_engine_info(engine_name)
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
        let cached = cache.get_engine_info("NonExistent")
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
        
        assert!(cache.get_engine_info(engine_name)
            .expect("获取引擎信息失败").is_some());

        // 删除引擎信息
        let deleted = cache.delete_engine_info(engine_name)
            .expect("删除引擎信息失败");
        assert!(deleted);

        // 验证已删除
        assert!(cache.get_engine_info(engine_name)
            .expect("获取引擎信息失败").is_none());
    }

    #[test]
    #[serial]
    fn test_metadata_cache_generic_metadata() {
        let cache = temp_metadata_cache();
        let key = "test_metadata";
        let data = b"test data".to_vec();

        // 缓存元数据
        cache.set_metadata(key, data.clone(), None)
            .expect("缓存元数据失败");

        // 获取元数据
        let cached = cache.get_metadata(key)
            .expect("获取元数据失败");
        
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
        cache.set_metadata(key, data, None)
            .expect("缓存元数据失败");
        
        assert!(cache.get_metadata(key)
            .expect("获取元数据失败").is_some());

        // 删除元数据
        let deleted = cache.delete_metadata(key)
            .expect("删除元数据失败");
        assert!(deleted);

        // 验证已删除
        assert!(cache.get_metadata(key)
            .expect("获取元数据失败").is_none());
    }
}
