//! 内存池统计和监控测试

use seesea_raming::pool::*;
use seesea_raming::types::MemoryConfig;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

/// 测试内存池统计信息准确性
#[test]
fn test_pool_statistics_accuracy() {
    let config = MemoryConfig::default();
    let pool = Arc::new(MemoryPool::new("stats_pool".to_string(), 1024, 10, config));

    // 初始统计
    let initial_stats = pool.stats();
    assert!(initial_stats.total_blocks > 0);
    assert_eq!(initial_stats.used_blocks, 0);
    assert_eq!(initial_stats.free_blocks, initial_stats.total_blocks);
    assert_eq!(initial_stats.allocation_count, 0);
    assert_eq!(initial_stats.deallocation_count, 0);

    // 分配内存
    let mut allocations = Vec::new();
    for _ in 0..3 {
        let pooled_memory = pool.allocate().unwrap();
        allocations.push(pooled_memory);
    }

    // 检查分配后的统计
    let after_allocation_stats = pool.stats();
    assert_eq!(after_allocation_stats.used_blocks, 3);
    assert_eq!(
        after_allocation_stats.free_blocks,
        initial_stats.total_blocks - 3
    );
    assert_eq!(after_allocation_stats.allocation_count, 3);
    assert_eq!(after_allocation_stats.deallocation_count, 0);

    // 释放内存
    drop(allocations);

    // 检查释放后的统计
    let after_release_stats = pool.stats();
    assert_eq!(after_release_stats.used_blocks, 0);
    assert_eq!(after_release_stats.free_blocks, initial_stats.total_blocks);
    assert_eq!(after_release_stats.allocation_count, 3);
    assert_eq!(after_release_stats.deallocation_count, 3);
}

/// 测试内存池缓存命中率计算
#[test]
fn test_pool_cache_hit_rate() {
    let config = MemoryConfig::default();
    let pool = Arc::new(MemoryPool::new("hit_rate_pool".to_string(), 512, 5, config));

    // 分配和释放多次以产生缓存命中
    for _ in 0..10 {
        let pooled_memory = pool.allocate().unwrap();
        drop(pooled_memory);
    }

    let stats = pool.stats();
    assert!(stats.cache_hit_rate >= 0.0);
    assert!(stats.cache_hit_rate <= 1.0);

    // 由于我们重复分配和释放相同的块，应该有较高的缓存命中率
    assert!(stats.cache_hit_rate > 0.5);
}

/// 测试内存池平均分配时间统计
#[test]
fn test_pool_average_allocation_time() {
    let config = MemoryConfig::default();
    let pool = Arc::new(MemoryPool::new("timing_pool".to_string(), 256, 10, config));

    // 进行多次分配
    for _ in 0..5 {
        let _pooled_memory = pool.allocate().unwrap();
    }

    let stats = pool.stats();
    assert!(stats.avg_allocation_time_ms >= 0.0);

    // 平均分配时间应该是合理的（小于100毫秒）
    assert!(stats.avg_allocation_time_ms < 100.0);
}

/// 测试内存池管理器统计信息聚合
#[test]
fn test_pool_manager_statistics_aggregation() {
    let config = MemoryConfig::default();
    let manager = MemoryPoolManager::new(config);

    // 分配不同大小的内存以使用多个池
    let sizes = vec![500, 1500, 3000, 7000];
    let mut allocations = Vec::new();

    for size in sizes {
        let pooled_memory = manager.allocate(size).unwrap();
        allocations.push(pooled_memory);
    }

    // 获取所有池的统计信息
    let all_stats = manager.get_all_stats();
    assert!(!all_stats.is_empty());

    // 验证统计信息的一致性
    for stats in &all_stats {
        assert!(stats.total_blocks > 0);
        assert_eq!(stats.used_blocks + stats.free_blocks, stats.total_blocks);
        assert!(stats.allocation_count > 0);
    }

    drop(allocations);
}

/// 测试内存池信息获取
#[test]
fn test_pool_info_retrieval() {
    let config = MemoryConfig::default();
    let pool = Arc::new(MemoryPool::new("info_pool".to_string(), 2048, 15, config));

    // 获取池信息
    let pool_info = pool.info();
    assert_eq!(pool_info.name, "info_pool");
    assert_eq!(pool_info.block_size, 2048);
    assert_eq!(pool_info.max_blocks, 15);
    assert!(pool_info.total_blocks > 0);
    assert!(pool_info.free_blocks <= pool_info.total_blocks);
    assert!(pool_info.used_blocks <= pool_info.total_blocks);
    assert!(pool_info.cache_hit_rate >= 0.0);
    assert!(pool_info.cache_hit_rate <= 1.0);
}

/// 测试内存池管理器池信息获取
#[test]
fn test_pool_manager_info_retrieval() {
    let config = MemoryConfig::default();
    let manager = MemoryPoolManager::new(config);

    // 分配一些内存以激活池
    let _pooled_memory = manager.allocate(1024).unwrap();

    // 获取所有池的信息
    let pool_info_list = manager.get_pool_info();
    assert!(!pool_info_list.is_empty());

    // 验证每个池的信息
    for pool_info in pool_info_list {
        assert!(!pool_info.name.is_empty());
        assert!(pool_info.block_size > 0);
        assert!(pool_info.max_blocks > 0);
        assert!(pool_info.total_blocks > 0);
        assert!(pool_info.cache_hit_rate >= 0.0);
    }
}

/// 测试内存池性能监控
#[test]
fn test_pool_performance_monitoring() {
    let config = MemoryConfig::default();
    let pool = Arc::new(MemoryPool::new("perf_pool".to_string(), 1024, 20, config));

    // 测量分配性能
    let start_time = std::time::Instant::now();
    let mut allocations = Vec::new();

    for _ in 0..100 {
        let pooled_memory = pool.allocate().unwrap();
        allocations.push(pooled_memory);
    }

    let allocation_time = start_time.elapsed();

    // 验证分配性能（100次分配应该在合理时间内完成）
    assert!(allocation_time.as_millis() < 1000); // 小于1秒

    // 检查统计信息
    let stats = pool.stats();
    assert_eq!(stats.allocation_count, 100);
    assert!(stats.avg_allocation_time_ms < 10.0); // 平均每次分配小于10毫秒

    drop(allocations);
}

/// 测试内存池清理统计更新
#[test]
fn test_pool_cleanup_statistics() {
    let config = MemoryConfig::default();
    let pool = Arc::new(MemoryPool::new(
        "cleanup_stats_pool".to_string(),
        512,
        10,
        config,
    ));

    // 分配和释放所有块
    let mut allocations = Vec::new();
    for _ in 0..10 {
        let pooled_memory = pool.allocate().unwrap();
        allocations.push(pooled_memory);
    }
    drop(allocations);

    // 记录清理前的统计
    let before_cleanup_stats = pool.stats();

    // 等待一段时间让块过期
    thread::sleep(Duration::from_millis(100));

    // 执行清理
    let _cleaned_blocks = pool.cleanup_expired_blocks().unwrap();

    // 检查清理后的统计
    let after_cleanup_stats = pool.stats();
    assert!(after_cleanup_stats.total_blocks <= before_cleanup_stats.total_blocks);
}

/// 测试内存池并发统计一致性
#[test]
fn test_pool_concurrent_statistics_consistency() {
    let config = MemoryConfig::default();
    let pool = Arc::new(MemoryPool::new(
        "concurrent_stats_pool".to_string(),
        256,
        50,
        config,
    ));

    let mut handles = vec![];

    // 启动多个线程进行分配和释放
    for _ in 0..5 {
        let pool_clone = pool.clone();
        let handle = thread::spawn(move || {
            for _ in 0..20 {
                let pooled_memory = pool_clone.allocate().unwrap();
                thread::sleep(Duration::from_millis(1));
                drop(pooled_memory);
            }
        });
        handles.push(handle);
    }

    // 启动统计监控线程
    let pool_clone = pool.clone();
    let stats_handle = thread::spawn(move || {
        let mut stats_checks = 0;
        for _ in 0..10 {
            let stats = pool_clone.stats();
            // 验证统计一致性
            assert_eq!(stats.used_blocks + stats.free_blocks, stats.total_blocks);
            assert!(stats.allocation_count >= stats.deallocation_count);
            stats_checks += 1;
            thread::sleep(Duration::from_millis(10));
        }
        stats_checks
    });

    // 等待所有线程完成
    for handle in handles {
        handle.join().unwrap();
    }

    let stats_checks = stats_handle.join().unwrap();
    assert_eq!(stats_checks, 10);

    // 最终统计验证
    let final_stats = pool.stats();
    assert_eq!(
        final_stats.used_blocks + final_stats.free_blocks,
        final_stats.total_blocks
    );
}

/// 测试内存池错误统计
#[test]
fn test_pool_error_statistics() {
    let config = MemoryConfig::default();
    let pool = Arc::new(MemoryPool::new(
        "error_stats_pool".to_string(),
        128,
        2,
        config,
    ));

    // 分配所有可用块
    let mut allocations = Vec::new();
    for _ in 0..2 {
        let pooled_memory = pool.allocate().unwrap();
        allocations.push(pooled_memory);
    }

    // 尝试分配更多（应该失败）
    let error_result = pool.allocate();
    assert!(error_result.is_err());

    // 验证统计信息不受影响
    let stats = pool.stats();
    assert_eq!(stats.used_blocks, 2);
    assert_eq!(stats.allocation_count, 2);

    drop(allocations);
}

/// 测试内存池长期运行统计
#[test]
fn test_pool_long_running_statistics() {
    let config = MemoryConfig::default();
    let pool = Arc::new(MemoryPool::new(
        "long_run_pool".to_string(),
        512,
        10,
        config,
    ));

    // 模拟长期运行的分配模式
    let mut total_allocations = 0;
    let mut total_deallocations = 0;

    for _cycle in 0..5 {
        // 分配阶段
        let mut allocations = Vec::new();
        for _ in 0..3 {
            if let Ok(pooled_memory) = pool.allocate() {
                allocations.push(pooled_memory);
                total_allocations += 1;
            }
        }

        // 随机释放一些
        let to_release = allocations.len() / 2;
        allocations.truncate(allocations.len() - to_release);
        total_deallocations += to_release;

        // 记录周期统计
        let stats = pool.stats();
        assert!(stats.allocation_count >= total_allocations);
        assert!(stats.deallocation_count >= total_deallocations as u64);

        // 清理剩余的分配
        drop(allocations);

        thread::sleep(Duration::from_millis(10));
    }

    // 最终验证
    let final_stats = pool.stats();
    assert!(final_stats.allocation_count >= total_allocations);
    assert!(final_stats.deallocation_count >= total_deallocations as u64);

    // 验证缓存命中率
    assert!(final_stats.cache_hit_rate >= 0.0);
    assert!(final_stats.cache_hit_rate <= 1.0);
}
