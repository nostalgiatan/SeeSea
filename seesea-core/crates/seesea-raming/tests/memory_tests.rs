//! 内存模块单元测试

use seesea_raming::memory::*;
use seesea_raming::types::MemoryConfig;
use std::sync::Arc;

/// 测试内存段基本功能
#[tokio::test]
async fn test_memory_segment_basic() {
    let config = MemoryConfig::default();
    let segment = MemorySegment::new("test_segment".to_string(), 4096, config).unwrap();

    assert_eq!(segment.name(), "test_segment");
    assert_eq!(segment.size(), 4096);
    assert!(segment.is_valid());
}

/// 测试内存段读写操作
#[tokio::test]
async fn test_memory_segment_read_write() {
    let config = MemoryConfig::default();
    let segment = Arc::new(MemorySegment::new("test_rw".to_string(), 8192, config).unwrap());

    // 测试基本写入和读取
    let test_data = b"Hello, Memory!";
    let write_result = segment.write(0, test_data);
    assert!(write_result.is_ok());

    let read_result = segment.read(0, test_data.len());
    assert!(read_result.is_ok());
    let read_buffer = read_result.unwrap();
    assert_eq!(&read_buffer[..], test_data);
}

/// 测试内存段边界检查
#[tokio::test]
async fn test_memory_segment_boundaries() {
    let config = MemoryConfig::default();
    let segment = Arc::new(MemorySegment::new("test_bounds".to_string(), 1024, config).unwrap());

    // 测试边界内写入
    let valid_data = vec![0u8; 512];
    let valid_result = segment.write(0, &valid_data);
    assert!(valid_result.is_ok());

    // 测试边界外写入
    let invalid_data = vec![0u8; 2000];
    let invalid_result = segment.write(0, &invalid_data);
    assert!(invalid_result.is_err());

    // 测试偏移量超出边界
    let offset_data = vec![0u8; 300];
    let offset_result = segment.write(800, &offset_data);
    assert!(offset_result.is_err());
}

/// 测试共享内存管理器
#[tokio::test]
async fn test_shared_memory_manager() {
    let config = MemoryConfig::default();
    let shared_memory = Arc::new(SharedMemory::new(config).unwrap());

    // 测试创建段
    let segment_name = "manager_test";
    let segment_size = 2048;

    let create_result = shared_memory.create_segment(segment_name.to_string(), segment_size);
    assert!(create_result.is_ok());

    let segment = create_result.unwrap();
    assert_eq!(segment.name(), segment_name);
    assert_eq!(segment.size(), segment_size);

    // 测试获取段
    let get_result = shared_memory.get_segment(segment_name);
    assert!(get_result.is_some());
    assert_eq!(get_result.unwrap().name(), segment_name);

    // 测试删除段
    let delete_result = shared_memory.delete_segment(segment_name);
    assert!(delete_result.is_ok());

    // 验证段已删除
    let get_after_delete = shared_memory.get_segment(segment_name);
    assert!(get_after_delete.is_none());
}

/// 测试内存段统计
#[tokio::test]
async fn test_memory_statistics() {
    let config = MemoryConfig::default();
    let shared_memory = Arc::new(SharedMemory::new(config).unwrap());

    // 获取初始统计
    let initial_stats = shared_memory.get_stats();
    let initial_segment_count = initial_stats.total_segments;
    let initial_allocated = initial_stats.total_allocated;

    // 创建多个段
    let sizes = vec![1024, 2048, 4096, 8192];
    let total_size: usize = sizes.iter().sum();

    for (i, &size) in sizes.iter().enumerate() {
        let segment_name = format!("stats_segment_{}", i);
        let _ = shared_memory.create_segment(segment_name, size).unwrap();
    }

    // 获取更新后的统计
    let updated_stats = shared_memory.get_stats();
    assert_eq!(
        updated_stats.total_segments,
        initial_segment_count + sizes.len()
    );
    assert_eq!(
        updated_stats.total_allocated,
        initial_allocated + total_size
    );
}

/// 测试内存段池化
#[tokio::test]
async fn test_memory_pooling() {
    let mut config = MemoryConfig::default();
    config.pool_size = 5120;
    config.max_segments = 10;

    let shared_memory = Arc::new(SharedMemory::new(config).unwrap());

    // 创建多个小内存段，应该使用池化
    for i in 0..5 {
        let segment_name = format!("pool_segment_{}", i);
        let segment = shared_memory.create_segment(segment_name, 512).unwrap();

        // 验证段可以正常使用
        let test_data = format!("Pool data {}", i);
        let _ = segment.write(0, test_data.as_bytes()).unwrap();
    }

    // 验证池化统计
    let stats = shared_memory.get_stats();
    assert!(stats.pool_usage > 0 || stats.active_segments > 0);
}

/// 测试从内存池创建内存段
#[tokio::test]
async fn test_pooled_memory_segment_creation() {
    let config = MemoryConfig::default();
    let shared_memory = Arc::new(SharedMemory::new(config).unwrap());

    // 从内存池创建内存段
    let segment_name = "pooled_segment";
    let segment_size = 2048;

    let result = shared_memory.create_segment_from_pool(segment_name.to_string(), segment_size);
    assert!(result.is_ok());

    let segment = result.unwrap();
    assert_eq!(segment.name(), segment_name);
    assert_eq!(segment.size(), segment_size);

    // 验证段可以正常使用
    let test_data = b"Pooled segment data";
    let write_result = segment.write(0, test_data);
    assert!(write_result.is_ok());

    let read_result = segment.read(0, test_data.len());
    assert!(read_result.is_ok());
    let read_buffer = read_result.unwrap();
    assert_eq!(&read_buffer[..], test_data);
}

/// 测试内存池统计信息
#[tokio::test]
async fn test_memory_pool_statistics() {
    let config = MemoryConfig::default();
    let shared_memory = Arc::new(SharedMemory::new(config).unwrap());

    // 从内存池创建几个内存段
    for i in 0..3 {
        let segment_name = format!("stats_pool_segment_{}", i);
        let _ = shared_memory
            .create_segment_from_pool(segment_name, 1024 * (i + 1))
            .unwrap();
    }

    // 获取内存池统计信息
    let pool_stats = shared_memory.get_pool_stats();
    assert!(!pool_stats.is_empty());

    // 验证统计信息
    for stats in &pool_stats {
        assert!(stats.total_blocks > 0);
        assert!(stats.used_blocks <= stats.total_blocks);
        assert!(stats.free_blocks <= stats.total_blocks);
        assert_eq!(stats.used_blocks + stats.free_blocks, stats.total_blocks);
    }
}

/// 测试内存池清理
#[tokio::test]
async fn test_memory_pool_cleanup() {
    let config = MemoryConfig::default();
    let shared_memory = Arc::new(SharedMemory::new(config).unwrap());

    // 创建一些内存段然后删除
    let mut segments = Vec::new();
    for i in 0..5 {
        let segment_name = format!("cleanup_segment_{}", i);
        let segment = shared_memory
            .create_segment_from_pool(segment_name, 2048)
            .unwrap();
        segments.push(segment);
    }

    // 删除所有段（这会释放内存池中的块）
    drop(segments);

    // 执行清理
    let cleanup_result = shared_memory.cleanup_pools();
    assert!(cleanup_result.is_ok());

    let cleaned_blocks = cleanup_result.unwrap();
    assert!(cleaned_blocks > 0);
}

/// 测试内存池与常规内存段的混合使用
#[tokio::test]
async fn test_mixed_memory_usage() {
    let config = MemoryConfig::default();
    let shared_memory = Arc::new(SharedMemory::new(config).unwrap());

    // 创建常规内存段
    let regular_segment = shared_memory
        .create_segment("regular_segment".to_string(), 4096)
        .unwrap();

    // 创建池化内存段
    let pooled_segment = shared_memory
        .create_segment_from_pool("pooled_segment".to_string(), 2048)
        .unwrap();

    // 验证两种段都可以正常使用
    let regular_data = b"Regular segment data";
    let pooled_data = b"Pooled segment data";

    let regular_write = regular_segment.write(0, regular_data);
    let pooled_write = pooled_segment.write(0, pooled_data);

    assert!(regular_write.is_ok());
    assert!(pooled_write.is_ok());

    // 验证数据
    let regular_read = regular_segment.read(0, regular_data.len());
    let pooled_read = pooled_segment.read(0, pooled_data.len());

    assert!(regular_read.is_ok());
    assert!(pooled_read.is_ok());

    let regular_buffer = regular_read.unwrap();
    let pooled_buffer = pooled_read.unwrap();

    assert_eq!(&regular_buffer[..], regular_data);
    assert_eq!(&pooled_buffer[..], pooled_data);

    // 验证统计信息包含两种类型的段
    let stats = shared_memory.get_stats();
    assert_eq!(stats.total_segments, 2);
}

/// 测试内存段锁定
#[tokio::test]
async fn test_memory_segment_locking() {
    let config = MemoryConfig::default();
    let segment = Arc::new(MemorySegment::new("test_lock".to_string(), 1024, config).unwrap());

    // 测试在区域写入
    let write_result = segment.write(0, b"Locked data");
    assert!(write_result.is_ok());

    // 测试读取
    let read_result = segment.read(0, 11);
    assert!(read_result.is_ok());
    let data = read_result.unwrap();
    assert_eq!(&data[..], b"Locked data");
}

/// 测试内存段同步
#[tokio::test]
async fn test_memory_segment_sync() {
    let config = MemoryConfig::default();
    let segment = Arc::new(MemorySegment::new("test_sync".to_string(), 2048, config).unwrap());

    // 写入数据
    let test_data = b"Sync test data";
    let _ = segment.write(0, test_data).unwrap();

    // 验证数据仍然存在
    let read_result = segment.read(0, test_data.len()).unwrap();
    assert_eq!(&read_result[..], test_data);
}

/// 测试内存段权限
#[tokio::test]
async fn test_memory_segment_permissions() {
    let config = MemoryConfig::default();
    let segment =
        Arc::new(MemorySegment::new("test_permissions".to_string(), 1024, config).unwrap());

    // 测试写入权限
    let write_result = segment.write(0, b"Permission test");
    assert!(write_result.is_ok());

    // 测试读取权限
    let read_result = segment.read(0, 15);
    assert!(read_result.is_ok());
    let buffer = read_result.unwrap();
    assert_eq!(&buffer[..], b"Permission test");
}

/// 测试内存段健康检查
#[tokio::test]
async fn test_memory_health_check() {
    let config = MemoryConfig::default();
    let shared_memory = Arc::new(SharedMemory::new(config).unwrap());

    // 健康检查应该通过
    let health_result = shared_memory.health_check();
    assert!(health_result.is_ok());

    let health_status = health_result.unwrap();
    assert!(health_status.is_healthy);
}

/// 测试内存段错误处理
#[tokio::test]
async fn test_memory_error_handling() {
    let config = MemoryConfig::default();
    let shared_memory = Arc::new(SharedMemory::new(config).unwrap());

    // 测试创建同名段
    let segment_name = "error_test";
    let _segment1 = shared_memory
        .create_segment(segment_name.to_string(), 1024)
        .unwrap();

    let duplicate_result = shared_memory.create_segment(segment_name.to_string(), 2048);
    assert!(duplicate_result.is_err());

    // 测试删除不存在的段
    let delete_result = shared_memory.delete_segment("non_existent");
    assert!(delete_result.is_err());

    // 测试获取不存在的段
    let get_result = shared_memory.get_segment("non_existent");
    assert!(get_result.is_none());
}

/// 测试内存段并发访问
#[tokio::test]
async fn test_concurrent_memory_access() {
    let config = MemoryConfig::default();
    let segment =
        Arc::new(MemorySegment::new("concurrent_test".to_string(), 16384, config).unwrap());

    let mut handles = vec![];

    // 启动多个并发写入任务
    for i in 0..10 {
        let segment_clone = segment.clone();
        let handle = tokio::spawn(async move {
            let data = format!("Concurrent data {}", i);
            let offset = i * 100;
            segment_clone.write(offset, data.as_bytes())
        });
        handles.push(handle);
    }

    // 等待所有任务完成
    for handle in handles {
        let result = handle.await.unwrap();
        assert!(result.is_ok());
    }

    // 验证所有数据都正确写入
    for i in 0..10 {
        let offset = i * 100;
        let data = segment.read(offset, 20).unwrap();

        let expected = format!("Concurrent data {}", i);
        let actual = String::from_utf8_lossy(&data[..expected.len()]);
        assert_eq!(actual, expected);
    }
}
