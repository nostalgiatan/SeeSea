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

//! DNS 连接池模块
//!
//! 提供 DNS 解析结果的缓存和连接池管理

use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::RwLock;

/// DNS 缓存条目
#[derive(Debug, Clone)]
struct CacheEntry {
    /// IP 地址列表
    ips: Vec<IpAddr>,
    /// 缓存时间
    cached_at: SystemTime,
    /// 生存时间（TTL）
    ttl: Duration,
}

impl CacheEntry {
    /// 检查缓存是否过期
    fn is_expired(&self) -> bool {
        SystemTime::now()
            .duration_since(self.cached_at)
            .map(|d| d > self.ttl)
            .unwrap_or(true)
    }
}

/// DNS 连接池
pub struct DnsPool {
    /// DNS 缓存
    cache: Arc<RwLock<HashMap<String, CacheEntry>>>,
    /// 默认 TTL
    default_ttl: Duration,
}

impl DnsPool {
    /// 创建新的 DNS 连接池
    ///
    /// # 参数
    ///
    /// * `default_ttl` - 默认缓存时间
    pub fn new(default_ttl: Duration) -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            default_ttl,
        }
    }

    /// 获取缓存的 DNS 记录
    ///
    /// # 参数
    ///
    /// * `hostname` - 域名
    ///
    /// # 返回
    ///
    /// 如果缓存存在且未过期，返回 IP 列表
    pub async fn get(&self, hostname: &str) -> Option<Vec<IpAddr>> {
        let cache = self.cache.read().await;
        
        if let Some(entry) = cache.get(hostname) {
            if !entry.is_expired() {
                return Some(entry.ips.clone());
            }
        }
        
        None
    }

    /// 缓存 DNS 记录
    ///
    /// # 参数
    ///
    /// * `hostname` - 域名
    /// * `ips` - IP 地址列表
    /// * `ttl` - 生存时间（可选，使用默认值）
    pub async fn set(&self, hostname: String, ips: Vec<IpAddr>, ttl: Option<Duration>) {
        let entry = CacheEntry {
            ips,
            cached_at: SystemTime::now(),
            ttl: ttl.unwrap_or(self.default_ttl),
        };

        let mut cache = self.cache.write().await;
        cache.insert(hostname, entry);
    }

    /// 清除缓存
    pub async fn clear(&self) {
        let mut cache = self.cache.write().await;
        cache.clear();
    }

    /// 清除过期的缓存条目
    pub async fn cleanup_expired(&self) {
        let mut cache = self.cache.write().await;
        cache.retain(|_, entry| !entry.is_expired());
    }

    /// 获取缓存统计信息
    pub async fn stats(&self) -> DnsPoolStats {
        let cache = self.cache.read().await;
        let total = cache.len();
        let expired = cache.values().filter(|e| e.is_expired()).count();

        DnsPoolStats {
            total_entries: total,
            expired_entries: expired,
            active_entries: total - expired,
        }
    }
}

impl Default for DnsPool {
    fn default() -> Self {
        Self::new(Duration::from_secs(300)) // 默认 5 分钟 TTL
    }
}

/// DNS 连接池统计信息
#[derive(Debug, Clone)]
pub struct DnsPoolStats {
    /// 总缓存条目数
    pub total_entries: usize,
    /// 过期条目数
    pub expired_entries: usize,
    /// 活跃条目数
    pub active_entries: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv4Addr;

    #[tokio::test]
    async fn test_dns_pool_new() {
        let pool = DnsPool::new(Duration::from_secs(60));
        let stats = pool.stats().await;
        assert_eq!(stats.total_entries, 0);
    }

    #[tokio::test]
    async fn test_dns_pool_set_and_get() {
        let pool = DnsPool::default();
        let ips = vec![IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1))];
        
        pool.set("example.com".to_string(), ips.clone(), None).await;
        
        let cached = pool.get("example.com").await;
        assert!(cached.is_some());
        assert_eq!(cached.unwrap(), ips);
    }

    #[tokio::test]
    async fn test_dns_pool_expiration() {
        let pool = DnsPool::new(Duration::from_millis(100));
        let ips = vec![IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1))];
        
        pool.set("example.com".to_string(), ips, None).await;
        
        // 等待过期
        tokio::time::sleep(Duration::from_millis(150)).await;
        
        let cached = pool.get("example.com").await;
        assert!(cached.is_none());
    }

    #[tokio::test]
    async fn test_dns_pool_clear() {
        let pool = DnsPool::default();
        let ips = vec![IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1))];
        
        pool.set("example.com".to_string(), ips, None).await;
        pool.clear().await;
        
        let stats = pool.stats().await;
        assert_eq!(stats.total_entries, 0);
    }

    #[tokio::test]
    async fn test_dns_pool_cleanup_expired() {
        let pool = DnsPool::new(Duration::from_millis(100));
        let ips = vec![IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1))];
        
        pool.set("example.com".to_string(), ips.clone(), None).await;
        pool.set("test.com".to_string(), ips, Some(Duration::from_secs(300))).await;
        
        // 等待第一个条目过期
        tokio::time::sleep(Duration::from_millis(150)).await;
        
        pool.cleanup_expired().await;
        
        let stats = pool.stats().await;
        assert_eq!(stats.total_entries, 1); // 只剩下未过期的
    }

    #[tokio::test]
    async fn test_dns_pool_stats() {
        let pool = DnsPool::default();
        let ips = vec![IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1))];
        
        pool.set("example.com".to_string(), ips.clone(), None).await;
        pool.set("test.com".to_string(), ips, None).await;
        
        let stats = pool.stats().await;
        assert_eq!(stats.total_entries, 2);
        assert_eq!(stats.active_entries, 2);
    }
}
