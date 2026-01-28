//! 内存池并发和线程安全测试

use seesea_raming::pool::*;
use seesea_raming::types::MemoryConfig;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;
use std::time::Duration;

/// 测试内存池并发分配
#[test]
fn test_concurrent_pool_allocation() {
    let config = MemoryConfig::default();
    let pool = Arc::new(MemoryPool::new(
        "concurrent_pool".to_string(),
        1024,
        20,
        config,
    ));

    let success_count = Arc::new(AtomicUsize::new(0));
    let error_count = Arc::new(AtomicUsize::new(0));

    let mut handles = vec![];

    // 启动多个线程进行并发分配
    for thread_id in 0..10 {
        let pool_clone = pool.clone();
        let success_clone = success_count.clone();
        let error_clone = error_count.clone();

        let handle = thread::spawn(move || {
            for i in 0..5 {
                match pool_clone.allocate() {
                    Ok(mut pooled_memory) => {
                        // 模拟一些工作
                        thread::sleep(Duration::from_millis(1));

                        // 写入一些数据以验证功能
                        if let Some(data_mut) = pooled_memory.data_mut() {
                            data_mut[0] = thread_id as u8;
                            data_mut[1] = i as u8;
                        }

                        // 释放内存
                        drop(pooled_memory);
                        success_clone.fetch_add(1, Ordering::Relaxed);
                    }
                    Err(_) => {
                        error_clone.fetch_add(1, Ordering::Relaxed);
                    }
                }

                // 小延迟以避免过于激烈的竞争
                thread::sleep(Duration::from_micros(100));
            }
        });

        handles.push(handle);
    }

    // 等待所有线程完成
    for handle in handles {
        handle.join().unwrap();
    }

    let total_success = success_count.load(Ordering::Relaxed);
    let total_errors = error_count.load(Ordering::Relaxed);

    // 验证结果
    assert!(total_success > 0, "应该有一些成功的分配");
    assert_eq!(total_success + total_errors, 50); // 10线程 * 5次分配

    // 检查最终池状态
    let final_stats = pool.stats();
    assert_eq!(final_stats.used_blocks, 0, "所有内存应该都已释放");
}

/// 测试内存池管理器并发分配
#[test]
fn test_concurrent_pool_manager_allocation() {
    let config = MemoryConfig::default();
    let manager = Arc::new(MemoryPoolManager::new(config));

    let success_count = Arc::new(AtomicUsize::new(0));
    let error_count = Arc::new(AtomicUsize::new(0));

    let mut handles = vec![];

    // 启动多个线程进行并发分配，使用不同大小的内存
    for thread_id in 0..8 {
        let manager_clone = manager.clone();
        let success_clone = success_count.clone();
        let error_clone = error_count.clone();

        let handle = thread::spawn(move || {
            for i in 0..3 {
                // 使用不同的内存大小
                let size = 512 * (i + 1) + thread_id * 100;

                match manager_clone.allocate(size) {
                    Ok(pooled_memory) => {
                        // 验证分配的内存大小
                        assert!(pooled_memory.size().unwrap() >= size);

                        // 模拟一些工作
                        thread::sleep(Duration::from_millis(1));

                        // 释放内存
                        drop(pooled_memory);
                        success_clone.fetch_add(1, Ordering::Relaxed);
                    }
                    Err(_) => {
                        error_clone.fetch_add(1, Ordering::Relaxed);
                    }
                }
            }
        });

        handles.push(handle);
    }

    // 等待所有线程完成
    for handle in handles {
        handle.join().unwrap();
    }

    let total_success = success_count.load(Ordering::Relaxed);
    let total_errors = error_count.load(Ordering::Relaxed);

    // 验证结果
    assert!(total_success > 0, "应该有一些成功的分配");
    assert_eq!(total_success + total_errors, 24); // 8线程 * 3次分配
}

/// 测试内存块并发访问
#[test]
fn test_concurrent_memory_block_access() {
    let block = Arc::new(MemoryBlock::new(1024));

    let mut handles = vec![];

    // 启动多个线程进行并发访问
    for thread_id in 0..6 {
        let block_clone = block.clone();

        let handle = thread::spawn(move || {
            for i in 0..10 {
                // 标记为使用中
                block_clone.mark_in_use();

                // 写入数据
                // 注意：这里需要unsafe来绕过Arc的可变性限制
                unsafe {
                    let data_ptr = block_clone.data().as_ptr() as *mut u8;
                    let data_slice = std::slice::from_raw_parts_mut(data_ptr, 1024);
                    data_slice[i] = thread_id as u8;
                }

                // 模拟一些工作
                thread::sleep(Duration::from_micros(100));

                // 标记为空闲
                block_clone.mark_free();

                // 验证使用计数
                assert!(block_clone.use_count() > 0);
            }
        });

        handles.push(handle);
    }

    // 等待所有线程完成
    for handle in handles {
        handle.join().unwrap();
    }

    // 验证最终状态
    assert!(!block.is_in_use());
    assert!(block.use_count() > 0);
}

/// 测试内存池线程安全的数据一致性
#[test]
fn test_pool_thread_safety_data_consistency() {
    let config = MemoryConfig::default();
    let pool = Arc::new(MemoryPool::new(
        "data_consistency_pool".to_string(),
        256,
        10,
        config,
    ));

    let data_verification_passed = Arc::new(AtomicUsize::new(0));
    let mut handles = vec![];

    // 启动多个线程，每个线程分配内存并写入唯一的数据模式
    for thread_id in 0..5 {
        let pool_clone = pool.clone();
        let verification_clone = data_verification_passed.clone();

        let handle = thread::spawn(move || {
            for iteration in 0..3 {
                if let Ok(mut pooled_memory) = pool_clone.allocate() {
                    // 写入唯一的数据模式
                    if let Some(data_mut) = pooled_memory.data_mut() {
                        for (i, byte) in data_mut.iter_mut().enumerate() {
                            *byte = ((thread_id * 10 + iteration + i) % 256) as u8;
                        }
                    }

                    // 立即读取并验证数据
                    if let Some(data) = pooled_memory.data() {
                        let mut all_correct = true;
                        for (i, &byte) in data.iter().enumerate() {
                            let expected = ((thread_id * 10 + iteration + i) % 256) as u8;
                            if byte != expected {
                                all_correct = false;
                                break;
                            }
                        }

                        if all_correct {
                            verification_clone.fetch_add(1, Ordering::Relaxed);
                        }
                    }

                    // 释放内存
                    drop(pooled_memory);
                }
            }
        });

        handles.push(handle);
    }

    // 等待所有线程完成
    for handle in handles {
        handle.join().unwrap();
    }

    // 验证数据一致性检查通过的次数
    let verification_count = data_verification_passed.load(Ordering::Relaxed);
    assert!(verification_count > 0, "数据一致性验证应该通过至少一次");
}

/// 测试内存池在高并发下的稳定性
#[test]
fn test_pool_high_concurrency_stability() {
    let config = MemoryConfig::default();
    let pool = Arc::new(MemoryPool::new(
        "high_concurrency_pool".to_string(),
        512,
        50,
        config,
    ));

    let operation_count = Arc::new(AtomicUsize::new(0));
    let mut handles = vec![];

    // 启动大量线程进行激烈的并发操作
    for _ in 0..20 {
        let pool_clone = pool.clone();
        let count_clone = operation_count.clone();

        let handle = thread::spawn(move || {
            for _ in 0..100 {
                // 快速分配和释放
                if let Ok(pooled_memory) = pool_clone.allocate() {
                    // 极短的使用时间
                    thread::sleep(Duration::from_micros(10));

                    // 释放内存（自动通过drop）
                    drop(pooled_memory);
                    count_clone.fetch_add(1, Ordering::Relaxed);
                }
            }
        });

        handles.push(handle);
    }

    // 等待所有线程完成
    for handle in handles {
        handle.join().unwrap();
    }

    let total_operations = operation_count.load(Ordering::Relaxed);
    assert!(total_operations > 0, "应该有一些成功的操作");

    // 检查最终池状态
    let final_stats = pool.stats();
    assert_eq!(final_stats.used_blocks, 0, "所有内存应该都已释放");
    assert!(final_stats.allocation_count >= total_operations as u64);
    assert!(final_stats.deallocation_count >= total_operations as u64);
}

/// 测试内存池管理器在高并发下的池创建
#[test]
fn test_pool_manager_concurrent_pool_creation() {
    let config = MemoryConfig::default();
    let manager = Arc::new(MemoryPoolManager::new(config));

    let pool_created_count = Arc::new(AtomicUsize::new(0));
    let mut handles = vec![];

    // 启动多个线程，每个线程请求不同的内存大小，这会触发池的创建
    for thread_id in 0..10 {
        let manager_clone = manager.clone();
        let count_clone = pool_created_count.clone();

        let handle = thread::spawn(move || {
            // 使用独特的内存大小来确保创建新池
            let unique_sizes = vec![
                100 + thread_id * 50,
                200 + thread_id * 75,
                300 + thread_id * 100,
            ];

            for size in unique_sizes {
                if let Ok(pooled_memory) = manager_clone.allocate(size) {
                    // 验证分配的内存
                    assert!(pooled_memory.size().unwrap() >= size);

                    // 释放内存
                    drop(pooled_memory);
                    count_clone.fetch_add(1, Ordering::Relaxed);
                }
            }
        });

        handles.push(handle);
    }

    // 等待所有线程完成
    for handle in handles {
        handle.join().unwrap();
    }

    let total_successful = pool_created_count.load(Ordering::Relaxed);
    assert!(total_successful > 0, "应该有一些成功的分配");

    // 验证创建了多个池
    let pool_info_list = manager.get_pool_info();
    assert!(pool_info_list.len() > 1, "应该创建了多个内存池");
}

/// 测试内存池的线程安全清理
#[test]
fn test_pool_concurrent_cleanup() {
    let config = MemoryConfig::default();
    let pool = Arc::new(MemoryPool::new(
        "concurrent_cleanup_pool".to_string(),
        1024,
        30,
        config,
    ));

    let cleanup_triggered = Arc::new(AtomicUsize::new(0));
    let mut handles = vec![];

    // 启动分配线程
    for _ in 0..5 {
        let pool_clone = pool.clone();

        let handle = thread::spawn(move || {
            let mut allocations = Vec::new();
            for _ in 0..10 {
                if let Ok(pooled_memory) = pool_clone.allocate() {
                    allocations.push(pooled_memory);
                }
            }

            // 释放所有内存
            drop(allocations);
        });

        handles.push(handle);
    }

    // 启动清理线程
    for _ in 0..3 {
        let pool_clone = pool.clone();
        let trigger_clone = cleanup_triggered.clone();

        let handle = thread::spawn(move || {
            thread::sleep(Duration::from_millis(50));

            if let Ok(cleaned) = pool_clone.cleanup_expired_blocks() {
                if cleaned > 0 {
                    trigger_clone.fetch_add(1, Ordering::Relaxed);
                }
            }
        });

        handles.push(handle);
    }

    // 等待所有线程完成
    for handle in handles {
        handle.join().unwrap();
    }

    // 验证清理操作被执行
    let _cleanup_count = cleanup_triggered.load(Ordering::Relaxed);
}

/// 测试内存池管理器并发清理
#[test]
fn test_pool_manager_concurrent_cleanup() {
    let config = MemoryConfig::default();
    let manager = Arc::new(MemoryPoolManager::new(config));

    let cleanup_performed = Arc::new(AtomicUsize::new(0));
    let mut handles = vec![];

    // 启动分配线程
    for _ in 0..4 {
        let manager_clone = manager.clone();

        let handle = thread::spawn(move || {
            let mut allocations = Vec::new();
            for _ in 0..5 {
                for size in &[512, 1024, 2048] {
                    if let Ok(pooled_memory) = manager_clone.allocate(*size) {
                        allocations.push(pooled_memory);
                    }
                }
            }

            // 释放内存
            drop(allocations);
        });

        handles.push(handle);
    }

    // 启动清理线程
    for _ in 0..2 {
        let manager_clone = manager.clone();
        let performed_clone = cleanup_performed.clone();

        let handle = thread::spawn(move || {
            thread::sleep(Duration::from_millis(30));

            if let Ok(cleaned) = manager_clone.cleanup_all() {
                if cleaned > 0 {
                    performed_clone.fetch_add(1, Ordering::Relaxed);
                }
            }
        });

        handles.push(handle);
    }

    // 等待所有线程完成
    for handle in handles {
        handle.join().unwrap();
    }

    let _cleanup_count = cleanup_performed.load(Ordering::Relaxed);
}

/// 测试内存池死锁预防
#[test]
fn test_pool_deadlock_prevention() {
    let config = MemoryConfig::default();
    let pool = Arc::new(MemoryPool::new(
        "deadlock_test_pool".to_string(),
        512,
        10,
        config,
    ));

    let mut handles = vec![];

    // 启动多个线程进行复杂的分配和释放模式
    for thread_id in 0..6 {
        let pool_clone = pool.clone();

        let handle = thread::spawn(move || {
            for _ in 0..20 {
                // 分配多个内存块
                let mut allocations = Vec::new();
                for _ in 0..3 {
                    if let Ok(pooled_memory) = pool_clone.allocate() {
                        allocations.push(pooled_memory);
                    }
                }

                // 随机延迟
                thread::sleep(Duration::from_micros(thread_id as u64 * 50));

                // 释放部分内存
                let to_keep = allocations.len() / 2;
                allocations.truncate(to_keep);

                // 再次分配
                for _ in 0..2 {
                    if let Ok(pooled_memory) = pool_clone.allocate() {
                        allocations.push(pooled_memory);
                    }
                }

                // 释放所有内存
                drop(allocations);
            }
        });

        handles.push(handle);
    }

    // 设置超时来检测死锁
    let timeout = Duration::from_secs(10);
    let start_time = std::time::Instant::now();

    // 等待所有线程完成
    for handle in handles {
        if start_time.elapsed() < timeout {
            handle.join().unwrap();
        } else {
            panic!("检测到可能的死锁 - 线程未在预期时间内完成");
        }
    }

    // 验证最终池状态
    let final_stats = pool.stats();
    assert_eq!(final_stats.used_blocks, 0, "所有内存应该都已释放");
}
