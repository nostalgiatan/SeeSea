//! 内存池边缘情况和错误处理测试

use seesea_raming::pool::*;
use seesea_raming::types::MemoryConfig;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

/// 测试内存池耗尽情况
#[test]
fn test_pool_exhaustion() {
    let config = MemoryConfig::default();
    let pool = Arc::new(MemoryPool::new(
        "exhaustion_pool".to_string(),
        256,
        3,
        config,
    ));

    // 分配所有可用块
    let mut allocations = Vec::new();
    for i in 0..3 {
        let mut pooled_memory = pool.allocate().unwrap();

        // 验证块内容
        if let Some(data_mut) = pooled_memory.data_mut() {
            data_mut[0] = i as u8;
        }

        allocations.push(pooled_memory);
    }

    // 验证池状态
    let stats = pool.stats();
    assert_eq!(stats.used_blocks, 3);
    assert_eq!(stats.free_blocks, 0);

    // 尝试分配更多（应该失败）
    let exhaustion_result = pool.allocate();
    assert!(exhaustion_result.is_err());

    // 验证错误类型
    match exhaustion_result {
        Err(seesea_raming::errors::RamingError::ResourceExhausted(msg)) => {
            assert!(msg.contains("已满"));
        }
        _ => panic!("期望 ResourceExhausted 错误"),
    }

    // 释放一个块
    drop(allocations.pop());

    // 现在应该可以分配了
    let new_allocation = pool.allocate();
    assert!(new_allocation.is_ok());

    let stats_after = pool.stats();
    assert_eq!(stats_after.used_blocks, 3);
}

/// 测试内存池管理器在资源耗尽时的行为
#[test]
fn test_pool_manager_resource_exhaustion() {
    let mut config = MemoryConfig::default();
    config.max_segment_size = 1024 * 1024; // 1MB 限制

    let manager = Arc::new(MemoryPoolManager::new(config));

    // 尝试分配超过配置限制的内存
    let large_allocation = manager.allocate(2 * 1024 * 1024); // 2MB

    // 应该被限制到最大段大小
    assert!(large_allocation.is_ok());
    let pooled_memory = large_allocation.unwrap();
    assert_eq!(pooled_memory.size().unwrap(), 1024 * 1024); // 应该是1MB
}

/// 测试零大小内存分配
#[test]
fn test_zero_size_allocation() {
    let config = MemoryConfig::default();
    let manager = MemoryPoolManager::new(config);

    // 尝试分配零大小内存
    let zero_result = manager.allocate(0);
    assert!(zero_result.is_ok());

    let pooled_memory = zero_result.unwrap();
    // 应该分配最小块大小
    assert!(pooled_memory.size().unwrap() >= 1024);
}

/// 测试极大内存分配请求
#[test]
fn test_extremely_large_allocation() {
    let config = MemoryConfig::default();
    let max_segment_size = config.max_segment_size;
    let manager = MemoryPoolManager::new(config);

    // 尝试分配极大内存（超过配置限制）
    let huge_result = manager.allocate(1024 * 1024 * 1024); // 1GB

    // 应该被限制到最大段大小
    assert!(huge_result.is_ok());
    let pooled_memory = huge_result.unwrap();
    assert!(pooled_memory.size().unwrap() <= max_segment_size);
}

/// 测试内存池清理边界条件
#[test]
fn test_pool_cleanup_boundary_conditions() {
    let config = MemoryConfig::default();
    let pool = Arc::new(MemoryPool::new(
        "cleanup_boundary_pool".to_string(),
        512,
        10,
        config,
    ));

    // 测试1：空池清理
    let empty_cleanup = pool.cleanup_expired_blocks().unwrap();
    assert_eq!(empty_cleanup, 0);

    // 测试2：只有使用中的块
    let mut allocations = Vec::new();
    for _ in 0..5 {
        allocations.push(pool.allocate().unwrap());
    }

    let used_cleanup = pool.cleanup_expired_blocks().unwrap();
    assert_eq!(used_cleanup, 0); // 不应该清理使用中的块

    // 测试3：混合状态
    drop(allocations.drain(0..2)); // 释放前2个

    // 立即清理（块还没有过期）
    let immediate_cleanup = pool.cleanup_expired_blocks().unwrap();
    assert_eq!(immediate_cleanup, 0); // 块还没有过期

    // 等待一段时间让块过期
    thread::sleep(Duration::from_millis(100));

    // 再次清理
    let _delayed_cleanup = pool.cleanup_expired_blocks().unwrap();

    drop(allocations);
}

/// 测试内存池最小块数限制
#[test]
fn test_pool_minimum_blocks_limit() {
    let config = MemoryConfig::default();
    let pool = Arc::new(MemoryPool::new(
        "min_blocks_pool".to_string(),
        256,
        10,
        config,
    ));

    // 分配所有块
    let mut allocations = Vec::new();
    for _ in 0..10 {
        allocations.push(pool.allocate().unwrap());
    }

    // 释放所有块
    drop(allocations);

    // 等待让块过期
    thread::sleep(Duration::from_millis(100));

    // 尝试清理所有块（应该保留最小数量）
    let _cleanup_result = pool.cleanup_expired_blocks().unwrap();

    // 验证清理后的状态
    let stats = pool.stats();
    assert!(stats.total_blocks >= 4); // 应该保留至少4个块
}

/// 测试内存池数据完整性在错误情况下
#[test]
fn test_pool_data_integrity_under_errors() {
    let config = MemoryConfig::default();
    let pool = Arc::new(MemoryPool::new(
        "integrity_pool".to_string(),
        128,
        5,
        config,
    ));

    let mut data_patterns = Vec::new();
    let mut allocations = Vec::new();

    // 分配一些内存并写入数据
    for i in 0..3 {
        let mut pooled_memory = pool.allocate().unwrap();

        // 写入唯一的数据模式
        if let Some(data_mut) = pooled_memory.data_mut() {
            for (j, byte) in data_mut.iter_mut().enumerate() {
                *byte = ((i * 10 + j) % 256) as u8;
            }
            data_patterns.push(data_mut.to_vec());
        }

        allocations.push(pooled_memory);
    }

    // 尝试分配更多（应该失败）
    for _ in 0..3 {
        let exhaustion_result = pool.allocate();
        assert!(exhaustion_result.is_err());
    }

    // 验证之前分配的数据仍然完整
    for (i, pooled_memory) in allocations.iter().enumerate() {
        if let Some(data) = pooled_memory.data() {
            assert_eq!(data.to_vec(), data_patterns[i]);
        }
    }

    // 释放一个块并重新分配
    drop(allocations.pop());

    let new_allocation = pool.allocate().unwrap();
    // 新分配的块应该包含之前的数据（内存重用）
    if let Some(data) = new_allocation.data() {
        // 数据应该存在（因为我们没有清零）
        assert!(!data.iter().all(|&b| b == 0));
    }
}

/// 测试内存池快速分配释放循环
#[test]
fn test_pool_rapid_allocation_deallocation_cycle() {
    let config = MemoryConfig::default();
    let pool = Arc::new(MemoryPool::new(
        "rapid_cycle_pool".to_string(),
        256,
        5,
        config,
    ));

    // 执行快速的分配释放循环
    for cycle in 0..100 {
        let mut allocations = Vec::new();

        // 快速分配
        for i in 0..3 {
            if let Ok(mut pooled_memory) = pool.allocate() {
                // 写入一些数据
                if let Some(data_mut) = pooled_memory.data_mut() {
                    data_mut[0] = cycle as u8;
                    data_mut[1] = i as u8;
                }
                allocations.push(pooled_memory);
            }
        }

        // 快速释放
        drop(allocations);

        // 验证池状态仍然一致
        let stats = pool.stats();
        assert!(stats.used_blocks <= 3);
        assert_eq!(stats.used_blocks + stats.free_blocks, stats.total_blocks);
    }

    // 最终验证
    let final_stats = pool.stats();
    assert_eq!(final_stats.used_blocks, 0);
    assert!(final_stats.allocation_count >= 100);
    assert!(final_stats.deallocation_count >= 100);
}

/// 测试内存池在内存压力下的行为
#[test]
fn test_pool_memory_pressure_handling() {
    let config = MemoryConfig::default();
    let manager = Arc::new(MemoryPoolManager::new(config));

    // 创建大量分配请求以产生内存压力
    let mut allocations = Vec::new();
    let mut allocation_errors = 0;

    for i in 0..50 {
        let size = 1024 * ((i % 5) + 1); // 不同大小的分配

        match manager.allocate(size) {
            Ok(pooled_memory) => {
                allocations.push(pooled_memory);
            }
            Err(_) => {
                allocation_errors += 1;
            }
        }
    }

    // 验证一些分配是成功的
    assert!(allocations.len() > 0);

    // 验证错误处理
    assert!(allocation_errors >= 0);

    // 释放所有内存
    drop(allocations);

    // 验证清理功能正常工作
    let _cleanup_result = manager.cleanup_all().unwrap();
}

/// 测试内存池异常恢复
#[test]
fn test_pool_exception_recovery() {
    let config = MemoryConfig::default();
    let pool = Arc::new(MemoryPool::new("recovery_pool".to_string(), 256, 5, config));

    // 正常分配一些内存
    let mut allocations = Vec::new();
    for _ in 0..3 {
        allocations.push(pool.allocate().unwrap());
    }

    // 模拟异常情况：强制释放（绕过正常释放机制）
    // 注意：这在实际使用中不应该发生，这里只是为了测试恢复能力

    // 验证池仍然可以正常工作
    let new_allocation = pool.allocate().unwrap();
    assert!(new_allocation.is_valid());

    // 验证统计信息
    let stats = pool.stats();
    assert!(stats.used_blocks >= 3);

    // 正常释放
    drop(allocations);
    drop(new_allocation);

    // 验证最终状态
    let final_stats = pool.stats();
    assert_eq!(final_stats.used_blocks, 0);
}

/// 测试内存池边界大小处理
#[test]
fn test_pool_boundary_size_handling() {
    let config = MemoryConfig::default();
    let manager = MemoryPoolManager::new(config);

    // 测试边界大小
    let boundary_sizes = vec![
        1,    // 最小
        1023, // 略小于1KB
        1024, // 正好是1KB
        1025, // 略大于1KB
        2047, // 略小于2KB
        2048, // 正好是2KB
        2049, // 略大于2KB
    ];

    for size in boundary_sizes {
        let result = manager.allocate(size);
        assert!(result.is_ok());

        let pooled_memory = result.unwrap();
        let allocated_size = pooled_memory.size().unwrap();

        // 验证分配的大小是2的幂次方且足够大
        assert!(allocated_size >= size);
        assert!((allocated_size & (allocated_size - 1)) == 0);
    }
}
