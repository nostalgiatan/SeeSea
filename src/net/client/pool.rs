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

//! 连接池管理模块
//!
//! 提供 HTTP 连接池的管理和优化

use crate::net::types::PoolConfig;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

/// 连接池统计信息
#[derive(Debug, Clone)]
pub struct PoolStats {
    /// 活跃连接数
    pub active_connections: usize,
    /// 空闲连接数
    pub idle_connections: usize,
    /// 总连接数
    pub total_connections: usize,
    /// 连接池命中率
    pub hit_rate: f64,
}

/// 连接池管理器
pub struct PoolManager {
    /// 配置
    config: Arc<PoolConfig>,
    /// 活跃连接计数器
    active_count: Arc<AtomicUsize>,
    /// 总请求数
    total_requests: Arc<AtomicUsize>,
    /// 连接池命中数
    pool_hits: Arc<AtomicUsize>,
}

impl PoolManager {
    /// 创建新的连接池管理器
    ///
    /// # 参数
    ///
    /// * `config` - 连接池配置
    pub fn new(config: PoolConfig) -> Self {
        Self {
            config: Arc::new(config),
            active_count: Arc::new(AtomicUsize::new(0)),
            total_requests: Arc::new(AtomicUsize::new(0)),
            pool_hits: Arc::new(AtomicUsize::new(0)),
        }
    }

    /// 记录新的连接使用
    pub fn record_connection_use(&self, from_pool: bool) {
        self.total_requests.fetch_add(1, Ordering::Relaxed);
        if from_pool {
            self.pool_hits.fetch_add(1, Ordering::Relaxed);
        } else {
            self.active_count.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// 记录连接释放
    pub fn record_connection_release(&self) {
        let current = self.active_count.load(Ordering::Relaxed);
        if current > 0 {
            self.active_count.fetch_sub(1, Ordering::Relaxed);
        }
    }

    /// 获取连接池统计信息
    pub fn stats(&self) -> PoolStats {
        let total = self.total_requests.load(Ordering::Relaxed);
        let hits = self.pool_hits.load(Ordering::Relaxed);
        let active = self.active_count.load(Ordering::Relaxed);

        PoolStats {
            active_connections: active,
            idle_connections: 0, // reqwest 内部管理，无法直接获取
            total_connections: active,
            hit_rate: if total > 0 {
                hits as f64 / total as f64
            } else {
                0.0
            },
        }
    }

    /// 获取配置
    pub fn config(&self) -> &PoolConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pool_manager_creation() {
        let config = PoolConfig::default();
        let manager = PoolManager::new(config);
        let stats = manager.stats();
        assert_eq!(stats.active_connections, 0);
        assert_eq!(stats.hit_rate, 0.0);
    }

    #[test]
    fn test_pool_manager_stats() {
        let config = PoolConfig::default();
        let manager = PoolManager::new(config);
        
        manager.record_connection_use(false);
        manager.record_connection_use(true);
        manager.record_connection_use(true);
        
        let stats = manager.stats();
        assert_eq!(stats.total_connections, 1);
        assert_eq!(stats.hit_rate, 2.0 / 3.0);
    }

    #[test]
    fn test_pool_manager_release() {
        let config = PoolConfig::default();
        let manager = PoolManager::new(config);
        
        manager.record_connection_use(false);
        manager.record_connection_release();
        
        let stats = manager.stats();
        assert_eq!(stats.active_connections, 0);
    }
}
