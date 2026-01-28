//! 内存池模块单元测试

use seesea_config::raming::MemoryConfig;
use seesea_raming::pool::*;
use std::sync::Arc;
use std::time::Duration;

/// 测试内存块基本功能
#[test]
fn test_memory_block_creation() {
    let block_size = 1024;
    let block = MemoryBlock::new(block_size);

    assert_eq!(block.size(), block_size);
    assert_eq!(block.data().len(), block_size);
    assert!(!block.is_in_use());
    assert_eq!(block.use_count(), 0);
}

/// 测试内存块使用状态管理
#[test]
fn test_memory_block_usage_management() {
    let block = MemoryBlock::new(512);

    // 初始状态
    assert!(!block.is_in_use());
    assert_eq!(block.use_count(), 0);

    // 标记为使用中
    block.mark_in_use();
    assert!(block.is_in_use());
    assert_eq!(block.use_count(), 1);

    // 再次标记为使用中
    block.mark_in_use();
    assert!(block.is_in_use());
    assert_eq!(block.use_count(), 2);

    // 标记为空闲
    block.mark_free();
    assert!(!block.is_in_use());
    assert_eq!(block.use_count(), 2); // 使用计数不减少
}

/// 测试内存块数据访问
#[test]
fn test_memory_block_data_access() {
    let mut block = MemoryBlock::new(256);

    // 测试可变访问
    let data_mut = block.data_mut();
    assert_eq!(data_mut.len(), 256);

    // 写入数据
    for (i, byte) in data_mut.iter_mut().enumerate() {
        *byte = (i % 256) as u8;
    }

    // 测试不可变访问
    let data = block.data();
    for (i, byte) in data.iter().enumerate() {
        assert_eq!(*byte, (i % 256) as u8);
    }
}

/// 测试内存块重置功能
#[test]
fn test_memory_block_reset() {
    let mut block = MemoryBlock::new(128);

    // 修改数据
    let data_mut = block.data_mut();
    for byte in data_mut.iter_mut() {
        *byte = 0xFF;
    }

    // 标记为使用中
    block.mark_in_use();

    // 重置
    block.reset();

    // 验证重置结果
    assert!(!block.is_in_use());
    assert_eq!(block.use_count(), 0);

    let data = block.data();
    for byte in data.iter() {
        assert_eq!(*byte, 0);
    }
}

/// 测试内存池创建
#[test]
fn test_memory_pool_creation() {
    let config = MemoryConfig::default();
    let pool = MemoryPool::new("test_pool".to_string(), 1024, 10, config);

    let info = pool.info();
    assert_eq!(info.name, "test_pool");
    assert_eq!(info.block_size, 1024);
    assert_eq!(info.max_blocks, 10);
    assert!(info.total_blocks >= 2); // 至少预分配一些块
}

/// 测试内存池分配和释放
#[test]
fn test_memory_pool_allocation() {
    let config = MemoryConfig::default();
    let pool = Arc::new(MemoryPool::new("test_pool".to_string(), 512, 5, config));

    // 分配内存
    let pooled_memory = pool.allocate_from_arc().unwrap();
    assert!(pooled_memory.is_valid());
    assert_eq!(pooled_memory.size(), Some(512));

    // 检查池状态
    let stats = pool.stats();
    assert_eq!(stats.used_blocks, 1);
    assert_eq!(stats.free_blocks, stats.total_blocks - 1);
    assert_eq!(stats.allocation_count, 1);
}

/// 测试内存池多次分配
#[test]
fn test_memory_pool_multiple_allocations() {
    let config = MemoryConfig::default();
    let pool = Arc::new(MemoryPool::new("test_pool".to_string(), 256, 3, config));

    // 分配多个内存块
    let mut allocations = Vec::new();
    for _ in 0..3 {
        let memory = pool.allocate_from_arc().unwrap();
        allocations.push(memory);
    }

    // 检查池状态
    let stats = pool.stats();
    assert_eq!(stats.used_blocks, 3);
    assert_eq!(stats.allocation_count, 3);

    // 尝试分配更多（应该失败）
    let result = pool.allocate_from_arc();
    assert!(result.is_err());
}

/// 测试池化内存自动释放
#[test]
fn test_pooled_memory_auto_release() {
    let config = MemoryConfig::default();
    let pool = Arc::new(MemoryPool::new("test_pool".to_string(), 128, 2, config));

    // 分配内存
    {
        let pooled_memory = pool.allocate_from_arc().unwrap();
        println!("Got pooled_memory: {:p}", &pooled_memory);
        let stats = pool.stats();
        println!(
            "After allocation - used_blocks: {}, free_blocks: {}, total_blocks: {}",
            stats.used_blocks, stats.free_blocks, stats.total_blocks
        );
        assert_eq!(stats.used_blocks, 1);

        // 强制立即drop以测试
        drop(pooled_memory);
        println!("Explicitly dropped pooled_memory");
    }

    // 检查释放后的状态
    let stats = pool.stats();
    println!(
        "After drop - used_blocks: {}, free_blocks: {}, total_blocks: {}",
        stats.used_blocks, stats.free_blocks, stats.total_blocks
    );
    assert_eq!(stats.used_blocks, 0);
    assert_eq!(stats.free_blocks, stats.total_blocks);
}

/// 测试内存池数据访问
#[test]
fn test_pooled_memory_data_access() {
    let config = MemoryConfig::default();
    let pool = Arc::new(MemoryPool::new("test_pool".to_string(), 64, 1, config));

    let mut pooled_memory = pool.allocate_from_arc().unwrap();

    // 测试数据访问
    if let Some(data_mut) = pooled_memory.data_mut() {
        assert_eq!(data_mut.len(), 64);
        for (i, byte) in data_mut.iter_mut().enumerate() {
            *byte = (i % 256) as u8;
        }
    }

    if let Some(data) = pooled_memory.data() {
        for (i, byte) in data.iter().enumerate() {
            assert_eq!(*byte, (i % 256) as u8);
        }
    }
}

/// 测试内存池清理功能
#[test]
fn test_memory_pool_cleanup() {
    let config = MemoryConfig::default();
    let pool = Arc::new(MemoryPool::new("test_pool".to_string(), 1024, 20, config));

    // 分配一些内存
    let mut allocations = Vec::new();
    for _ in 0..10 {
        let memory = pool.allocate_from_arc().unwrap();
        allocations.push(memory);
    }

    // 释放所有内存
    drop(allocations);

    // 等待一段时间（模拟块过期）
    std::thread::sleep(Duration::from_millis(100));

    // 执行清理
    let _cleaned = pool.cleanup_expired_blocks().unwrap();
}

/// 测试内存池管理器创建
#[test]
fn test_memory_pool_manager_creation() {
    let config = MemoryConfig::default();
    let manager = MemoryPoolManager::new(config);

    // 检查默认池是否已创建
    let pool_info = manager.get_pool_info();
    assert!(!pool_info.is_empty());

    // 检查默认池大小
    let expected_sizes = vec![1024, 4096, 16384, 65536, 262144, 1048576];
    for size in expected_sizes {
        assert!(pool_info.iter().any(|info| info.block_size == size));
    }
}

/// 测试内存池管理器分配
#[test]
fn test_memory_pool_manager_allocation() {
    let config = MemoryConfig::default();
    let manager = MemoryPoolManager::new(config);

    // 分配不同大小的内存
    let sizes = vec![500, 2000, 8000, 32000];
    for size in sizes {
        let pooled_memory = manager.allocate(size).unwrap();
        assert!(pooled_memory.is_valid());

        let allocated_size = pooled_memory.size().unwrap();
        assert!(allocated_size >= size);

        // 验证分配的大小是2的幂次方
        assert!((allocated_size & (allocated_size - 1)) == 0);
    }
}

/// 测试内存池管理器池大小选择
#[test]
fn test_memory_pool_manager_size_selection() {
    let config = MemoryConfig::default();
    let manager = MemoryPoolManager::new(config);

    // 测试不同请求大小的池选择
    struct TestCase {
        requested: usize,
        expected: usize,
    }

    let test_cases = vec![
        TestCase {
            requested: 100,
            expected: 1024,
        },
        TestCase {
            requested: 1000,
            expected: 1024,
        },
        TestCase {
            requested: 1500,
            expected: 2048,
        },
        TestCase {
            requested: 5000,
            expected: 8192,
        },
    ];

    for case in test_cases {
        let pooled_memory = manager.allocate(case.requested).unwrap();
        let allocated_size = pooled_memory.size().unwrap();
        assert_eq!(allocated_size, case.expected);
    }
}

/// 测试内存池统计信息
#[test]
fn test_memory_pool_statistics() {
    let config = MemoryConfig::default();
    let pool = Arc::new(MemoryPool::new("test_pool".to_string(), 256, 5, config));

    // 初始统计
    let initial_stats = pool.stats();
    assert!(initial_stats.total_blocks > 0);
    assert_eq!(initial_stats.used_blocks, 0);

    // 分配内存
    let pooled_memory = pool.allocate_from_arc().unwrap();
    let after_allocation_stats = pool.stats();

    assert_eq!(after_allocation_stats.used_blocks, 1);
    assert_eq!(
        after_allocation_stats.allocation_count,
        initial_stats.allocation_count + 1
    );

    // 释放内存
    drop(pooled_memory);
    let after_release_stats = pool.stats();

    assert_eq!(after_release_stats.used_blocks, 0);
    assert_eq!(
        after_release_stats.free_blocks,
        after_release_stats.total_blocks
    );
}

/// 测试内存池管理器统计信息
#[test]
fn test_memory_pool_manager_statistics() {
    let config = MemoryConfig::default();
    let manager = MemoryPoolManager::new(config);

    // 分配一些内存
    let mut allocations = Vec::new();
    for i in 0..5 {
        let size = 1024 * (i + 1);
        let pooled_memory = manager.allocate(size).unwrap();
        allocations.push(pooled_memory);
    }

    // 获取所有池的统计信息
    let all_stats = manager.get_all_stats();
    assert!(!all_stats.is_empty());

    // 验证统计信息
    for stats in &all_stats {
        assert!(stats.total_blocks > 0);
        if stats.used_blocks > 0 {
            assert!(stats.allocation_count > 0);
        }
    }
}

/// 测试内存池管理器清理功能
#[test]
fn test_memory_pool_manager_cleanup() {
    let config = MemoryConfig::default();
    let manager = MemoryPoolManager::new(config);

    // 分配一些内存然后释放
    let mut allocations = Vec::new();
    for _ in 0..10 {
        let pooled_memory = manager.allocate(1024).unwrap();
        allocations.push(pooled_memory);
    }
    drop(allocations);

    // 执行清理
    let _cleaned = manager.cleanup_all().unwrap();
}

/// 测试池化内存生命周期管理
#[test]
fn test_pooled_memory_lifecycle() {
    let config = MemoryConfig::default();
    let pool = Arc::new(MemoryPool::new("test_pool".to_string(), 128, 2, config));

    // 分配内存
    let mut pooled_memory = pool.allocate_from_arc().unwrap();

    // 验证数据访问
    assert!(pooled_memory.data().is_some());
    assert!(pooled_memory.data_mut().is_some());
    assert_eq!(pooled_memory.size(), Some(128));

    // 释放内存（通过drop）
    drop(pooled_memory);

    // 再次分配，应该重用之前的块
    let new_pooled_memory = pool.allocate_from_arc().unwrap();

    // 由于池的工作方式，指针可能相同也可能不同，但功能应该正常
    assert!(new_pooled_memory.is_valid());
    assert_eq!(new_pooled_memory.size(), Some(128));
}
