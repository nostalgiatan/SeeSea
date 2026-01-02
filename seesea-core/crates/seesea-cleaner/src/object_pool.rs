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

//! 对象池模块
//! 实现DatePage对象的对象池管理，由调控中心动态调整大小

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

use crate::date_page::DatePage;

/// DatePage对象池
pub struct DatePageObjectPool {
    /// 存储空闲DatePage对象的池
    pool: Mutex<Vec<DatePage>>,
    /// 当前池大小
    current_size: Arc<AtomicUsize>,
    /// 最大池大小
    max_size: Arc<AtomicUsize>,
    /// 最小池大小
    min_size: Arc<AtomicUsize>,
    /// 池命中率
    hits: Arc<AtomicUsize>,
    /// 池未命中率
    misses: Arc<AtomicUsize>,
    /// 对象创建计数
    created: Arc<AtomicUsize>,
    /// 对象回收计数
    recycled: Arc<AtomicUsize>,
}

impl DatePageObjectPool {
    /// 创建新的DatePage对象池
    pub fn new(min_size: usize, max_size: usize) -> Self {
        let pool = Self {
            pool: Mutex::new(Vec::with_capacity(max_size)),
            current_size: Arc::new(AtomicUsize::new(0)),
            max_size: Arc::new(AtomicUsize::new(max_size)),
            min_size: Arc::new(AtomicUsize::new(min_size)),
            hits: Arc::new(AtomicUsize::new(0)),
            misses: Arc::new(AtomicUsize::new(0)),
            created: Arc::new(AtomicUsize::new(0)),
            recycled: Arc::new(AtomicUsize::new(0)),
        };

        // 预填充最小数量的对象
        pool.prefill();

        pool
    }

    /// 预填充对象池
    fn prefill(&self) {
        let min_size = self.min_size.load(Ordering::SeqCst);
        let mut pool = self.pool.lock().unwrap();

        while pool.len() < min_size {
            let date_page = self.create_date_page();
            pool.push(date_page);
            self.current_size.fetch_add(1, Ordering::SeqCst);
        }
    }

    /// 创建新的DatePage对象
    fn create_date_page(&self) -> DatePage {
        self.created.fetch_add(1, Ordering::SeqCst);

        // 创建一个空的DatePage对象
        DatePage {
            url: "".into(),
            time: SystemTime::UNIX_EPOCH,
            description: "".into(),
            source_data: "".into(),
            data_blocks: Vec::new(),
            vectors: Vec::new(),
            hash: 0,
            last_update_time: SystemTime::UNIX_EPOCH,
            map: Vec::new(),
            extra_info: Vec::new(),
        }
    }

    /// 从对象池获取一个DatePage对象
    pub fn get(&self) -> DatePage {
        let mut pool = self.pool.lock().unwrap();

        if let Some(mut date_page) = pool.pop() {
            // 重置对象状态
            self.reset_date_page(&mut date_page);
            self.hits.fetch_add(1, Ordering::SeqCst);
            self.current_size.fetch_sub(1, Ordering::SeqCst);
            date_page
        } else {
            // 池为空，创建新对象
            self.misses.fetch_add(1, Ordering::SeqCst);
            self.create_date_page()
        }
    }

    /// 将DatePage对象归还到对象池
    pub fn put(&self, mut date_page: DatePage) {
        let max_size = self.max_size.load(Ordering::SeqCst);
        let mut pool = self.pool.lock().unwrap();

        if pool.len() < max_size {
            // 重置对象状态
            self.reset_date_page(&mut date_page);
            pool.push(date_page);
            self.current_size.fetch_add(1, Ordering::SeqCst);
            self.recycled.fetch_add(1, Ordering::SeqCst);
        }
        // 否则让对象自然销毁
    }

    /// 重置DatePage对象状态，使其可以被重用
    fn reset_date_page(&self, date_page: &mut DatePage) {
        // 重置所有字段
        date_page.url = "".into();
        date_page.time = SystemTime::UNIX_EPOCH;
        date_page.description = "".into();
        date_page.source_data = "".into();
        date_page.data_blocks.clear();
        date_page.vectors.clear();
        date_page.hash = 0;
        date_page.last_update_time = SystemTime::UNIX_EPOCH;
        date_page.map.clear();
        date_page.extra_info.clear();
    }

    /// 设置对象池最大大小
    pub fn set_max_size(&self, max_size: usize) {
        let old_max = self.max_size.swap(max_size, Ordering::SeqCst);

        // 如果新的最大值小于旧值，需要收缩池
        if max_size < old_max {
            let mut pool = self.pool.lock().unwrap();
            if pool.len() > max_size {
                let excess = pool.len() - max_size;
                pool.truncate(max_size);
                self.current_size.fetch_sub(excess, Ordering::SeqCst);
            }
        }
    }

    /// 设置对象池最小大小
    pub fn set_min_size(&self, min_size: usize) {
        self.min_size.store(min_size, Ordering::SeqCst);
        // 如果新的最小值大于当前池大小，需要预填充
        self.prefill();
    }

    /// 获取当前池大小
    pub fn current_size(&self) -> usize {
        self.current_size.load(Ordering::SeqCst)
    }

    /// 获取最大池大小
    pub fn max_size(&self) -> usize {
        self.max_size.load(Ordering::SeqCst)
    }

    /// 获取最小池大小
    pub fn min_size(&self) -> usize {
        self.min_size.load(Ordering::SeqCst)
    }

    /// 获取池命中率
    pub fn hit_rate(&self) -> f64 {
        let hits = self.hits.load(Ordering::SeqCst);
        let misses = self.misses.load(Ordering::SeqCst);
        let total = hits + misses;

        if total == 0 {
            0.0
        } else {
            hits as f64 / total as f64
        }
    }

    /// 获取池统计信息
    pub fn stats(&self) -> DatePageObjectPoolStats {
        DatePageObjectPoolStats {
            current_size: self.current_size.load(Ordering::SeqCst),
            max_size: self.max_size.load(Ordering::SeqCst),
            min_size: self.min_size.load(Ordering::SeqCst),
            hits: self.hits.load(Ordering::SeqCst),
            misses: self.misses.load(Ordering::SeqCst),
            hit_rate: self.hit_rate(),
            created: self.created.load(Ordering::SeqCst),
            recycled: self.recycled.load(Ordering::SeqCst),
        }
    }

    /// 清理对象池，移除所有对象
    pub fn clear(&self) {
        let mut pool = self.pool.lock().unwrap();
        pool.clear();
        self.current_size.store(0, Ordering::SeqCst);
    }

    /// 动态调整池大小
    pub fn resize(&self, desired_size: usize) {
        // 调整最大大小
        self.set_max_size(desired_size);
        // 调整最小大小为最大大小的10%
        self.set_min_size(desired_size / 10);
    }
}

/// DatePage对象池统计信息
#[derive(Debug, Clone, Copy)]
pub struct DatePageObjectPoolStats {
    /// 当前池大小
    pub current_size: usize,
    /// 最大池大小
    pub max_size: usize,
    /// 最小池大小
    pub min_size: usize,
    /// 池命中率
    pub hits: usize,
    /// 池未命中率
    pub misses: usize,
    /// 池命中率百分比
    pub hit_rate: f64,
    /// 对象创建计数
    pub created: usize,
    /// 对象回收计数
    pub recycled: usize,
}

impl Default for DatePageObjectPool {
    fn default() -> Self {
        // 默认配置：最小10，最大100
        Self::new(10, 100)
    }
}

// 注意：ConcurrentController已经直接包含了DatePageObjectPool字段
// 因此不需要额外的trait或扩展方法

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_date_page_object_pool_new() {
        let pool = DatePageObjectPool::new(5, 20);
        assert_eq!(pool.min_size(), 5);
        assert_eq!(pool.max_size(), 20);
        assert_eq!(pool.current_size(), 5);
    }

    #[test]
    fn test_date_page_object_pool_get_put() {
        let pool = DatePageObjectPool::new(1, 5);

        // 从池获取对象
        let mut date_page = pool.get();
        assert_eq!(pool.current_size(), 0);

        // 设置一些数据
        date_page.url = "https://example.com".into();
        date_page.description = "Test description".into();

        // 归还对象到池
        pool.put(date_page);
        assert_eq!(pool.current_size(), 1);

        // 再次获取对象，应该是同一个对象（已重置）
        let date_page2 = pool.get();
        assert_eq!(pool.current_size(), 0);
        assert_eq!(date_page2.url.as_ref(), "");
        assert_eq!(date_page2.description.as_ref(), "");
    }

    #[test]
    fn test_date_page_object_pool_stats() {
        let pool = DatePageObjectPool::new(1, 5);

        // 获取对象
        let date_page = pool.get();
        pool.put(date_page);

        let stats = pool.stats();
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.misses, 0);
        assert_eq!(stats.hit_rate, 1.0);
        assert_eq!(stats.created, 1);
        assert_eq!(stats.recycled, 1);
    }

    #[test]
    fn test_date_page_object_pool_resize() {
        let pool = DatePageObjectPool::new(2, 10);

        // 调整大小
        pool.set_max_size(5);
        assert_eq!(pool.max_size(), 5);

        // 调整大小
        pool.set_min_size(3);
        assert_eq!(pool.min_size(), 3);
        assert_eq!(pool.current_size(), 3);
    }

    #[test]
    fn test_date_page_object_pool_clear() {
        let pool = DatePageObjectPool::new(3, 10);
        assert_eq!(pool.current_size(), 3);

        pool.clear();
        assert_eq!(pool.current_size(), 0);
    }
}
