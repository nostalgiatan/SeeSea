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

//! 并发控制器
//! 实现微并行技术，用于调控中心动态并发

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

/// 并发控制器
pub struct ConcurrentController {
    /// 最大并发数
    max_concurrency: Arc<AtomicUsize>,
    /// DatePage对象池
    date_page_pool: Arc<crate::object_pool::DatePageObjectPool>,
}

impl ConcurrentController {
    /// 创建新的并发控制器
    pub fn new() -> Self {
        let num_cores = num_cpus::get();

        // 默认对象池配置：最小10，最大100
        let min_pool_size = 10;
        let max_pool_size = 100;

        Self {
            max_concurrency: Arc::new(AtomicUsize::new(num_cores)),
            date_page_pool: Arc::new(crate::object_pool::DatePageObjectPool::new(
                min_pool_size,
                max_pool_size,
            )),
        }
    }

    /// 获取DatePage对象池
    pub fn date_page_pool(&self) -> &Arc<crate::object_pool::DatePageObjectPool> {
        &self.date_page_pool
    }

    /// 设置DatePage对象池大小
    pub fn set_date_page_pool_size(&self, min_size: usize, max_size: usize) {
        self.date_page_pool.set_min_size(min_size);
        self.date_page_pool.set_max_size(max_size);
    }

    /// 动态调整DatePage对象池大小
    pub fn resize_date_page_pool(&self, desired_size: usize) {
        self.date_page_pool.resize(desired_size);
    }

    /// 获取DatePage对象池统计信息
    pub fn date_page_pool_stats(&self) -> crate::object_pool::DatePageObjectPoolStats {
        self.date_page_pool.stats()
    }

    /// 设置最大并发数
    pub fn set_max_concurrency(&self, max: usize) {
        self.max_concurrency.store(max, Ordering::SeqCst);
    }

    /// 获取当前最大并发数
    pub fn get_max_concurrency(&self) -> usize {
        self.max_concurrency.load(Ordering::SeqCst)
    }

    /// 并行处理数据块
    pub async fn parallel_process<T, F>(&self, items: Vec<T>, f: F) -> Vec<T>
    where
        T: Send + Sync + Clone + 'static,
        F: Fn(&T) -> T + Send + Sync + 'static + Clone,
    {
        let max_concurrency = self.get_max_concurrency();

        // 如果数据量很小，直接串行处理
        if items.len() <= max_concurrency {
            return items.into_iter().map(|item| f(&item)).collect();
        }

        // 并行处理
        let items = Arc::new(items);
        let results = Arc::new(tokio::sync::Mutex::new(Vec::with_capacity(items.len())));
        let len = items.len();

        // 使用tokio的任务池并行处理
        let mut tasks = Vec::with_capacity(max_concurrency);

        for i in 0..max_concurrency {
            let items = items.clone();
            let results = results.clone();
            let f = f.clone();

            let task = tokio::spawn(async move {
                let chunk_size = len.div_ceil(max_concurrency);
                let start = i * chunk_size;
                let end = (start + chunk_size).min(len);

                let actual_size = end.saturating_sub(start);
                let mut local_results = Vec::with_capacity(actual_size);

                for j in start..end {
                    let processed = f(&items[j]);
                    local_results.push(processed);
                }

                let mut results_guard = results.lock().await;
                results_guard.extend(local_results);
            });

            tasks.push(task);
        }

        // 等待所有任务完成
        for task in tasks {
            task.await.unwrap();
        }

        Arc::try_unwrap(results)
            .ok()
            .unwrap_or_default()
            .into_inner()
    }
}

impl Default for ConcurrentController {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_parallel_process() {
        let controller = ConcurrentController::new();
        let items = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10];

        let results = controller.parallel_process(items, |&x| x * 2).await;

        assert_eq!(results, vec![2, 4, 6, 8, 10, 12, 14, 16, 18, 20]);
    }
}
