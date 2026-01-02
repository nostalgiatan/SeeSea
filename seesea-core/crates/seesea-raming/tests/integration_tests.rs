//! seesea-raming集成测试
//!
//! 测试共享内存、事件系统和绑定管理的完整功能

use seesea_raming::*;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;

/// 测试内存段创建和管理
#[tokio::test]
async fn test_memory_segment_creation() {
    // 初始化管理器
    let config = ManagerConfig::default();
    let manager = RamingManager::new(config).unwrap();
    manager.start().await.unwrap();

    // 测试创建内存段
    let segment_name = "test_segment";
    let segment_size = 1024 * 1024; // 1MB

    let result = manager
        .create_memory_segment(segment_name.to_string(), segment_size, false)
        .await;
    assert!(result.is_ok(), "创建内存段失败: {:?}", result);

    // 获取内存段
    let segment = manager.shared_memory().get_segment(segment_name).unwrap();
    assert_eq!(segment.name(), segment_name);
    assert_eq!(segment.size(), segment_size);

    // 测试重复创建
    let duplicate_result = manager
        .create_memory_segment(segment_name.to_string(), segment_size, false)
        .await;
    assert!(duplicate_result.is_err(), "重复创建内存段应该失败");

    // 测试获取现有段
    let get_result = manager.shared_memory().get_segment(segment_name);
    assert!(get_result.is_some(), "获取内存段失败");
    assert_eq!(get_result.unwrap().name(), segment_name);

    // 清理
    let _ = manager.delete_memory_segment(segment_name).await;
}

/// 测试内存段读写操作
#[tokio::test]
async fn test_memory_segment_operations() {
    // 初始化管理器
    let config = ManagerConfig::default();
    let manager = RamingManager::new(config).unwrap();
    manager.start().await.unwrap();
    let segment_name = "test_ops_segment";
    let segment_size = 4096; // 4KB

    // 创建内存段
    manager
        .create_memory_segment(segment_name.to_string(), segment_size, false)
        .await
        .unwrap();
    let segment = manager.shared_memory().get_segment(segment_name).unwrap();

    // 测试写入数据
    let test_data = b"Hello, Shared Memory!";
    let write_result = segment.write(0, test_data);
    assert!(write_result.is_ok(), "写入数据失败: {:?}", write_result);

    // 测试读取数据
    let read_result = segment.read(0, test_data.len());
    assert!(read_result.is_ok(), "读取数据失败: {:?}", read_result);
    let read_buffer = read_result.unwrap();
    assert_eq!(&read_buffer[..], test_data, "读取的数据不匹配");

    // 测试边界检查
    let large_write = vec![0u8; segment_size + 1];
    let boundary_result = segment.write(0, &large_write);
    assert!(boundary_result.is_err(), "超出边界的写入应该失败");

    // 清理
    let _ = manager.delete_memory_segment(segment_name).await;
}

/// 测试事件发布和订阅
#[tokio::test]
async fn test_event_publish_subscribe() {
    // 初始化管理器
    let config = ManagerConfig::default();
    let manager = RamingManager::new(config).unwrap();
    manager.start().await.unwrap();

    // 创建测试监听器
    let test_listener = Arc::new(TestEventListener::new());

    // 订阅内存共享事件
    let event_bus = manager.event_bus();
    let subscribe_result = event_bus
        .subscribe(RamingEventType::MemoryShare, test_listener.clone())
        .await;
    assert!(
        subscribe_result.is_ok(),
        "订阅事件失败: {:?}",
        subscribe_result
    );

    // 发布事件
    let event_data = RamingEventData::new(
        RamingEventType::MemoryShare,
        "test_source".to_string(),
        serde_json::json!({
            "segment_name": "test_segment",
            "operation": "create"
        }),
    );

    let publish_result = event_bus.publish(event_data).await;
    assert!(publish_result.is_ok(), "发布事件失败: {:?}", publish_result);

    // 等待事件处理
    sleep(Duration::from_millis(100)).await;

    // 验证事件被接收
    assert!(test_listener.was_called(), "事件监听器未被调用");
    assert_eq!(
        test_listener.get_call_count(),
        1,
        "事件监听器调用次数不正确"
    );
}

/// 测试内存和事件绑定
#[tokio::test]
async fn test_memory_event_binding() {
    // 初始化管理器
    let config = ManagerConfig::default();
    let manager = RamingManager::new(config).unwrap();
    manager.start().await.unwrap();

    // 创建内存段
    let segment_name = "test_binding_segment";
    let segment_size = 8192; // 创建内存段
    manager
        .create_memory_segment(segment_name.to_string(), segment_size, false)
        .await
        .unwrap();
    let _segment = manager.shared_memory().get_segment(segment_name).unwrap();

    // 创建事件监听器
    let binding_listener = Arc::new(TestBindingListener::new());

    // 创建绑定
    let binding_result = manager
        .create_event_binding(
            "test_binding".to_string(),
            BindingType::MemoryShare,
            RamingEventType::MemoryUpdate,
            binding_listener.clone(),
        )
        .await;
    assert!(binding_result.is_ok(), "创建绑定失败: {:?}", binding_result);

    // 触发内存更新事件
    let update_event = RamingEventData::new(
        RamingEventType::MemoryUpdate,
        "test_source".to_string(),
        serde_json::json!({
            "segment_name": segment_name,
            "offset": 0,
            "size": 1024
        }),
    );

    let event_bus = manager.event_bus();
    let _ = event_bus.publish(update_event).await;
    sleep(Duration::from_millis(100)).await;

    // 验证绑定监听器被调用
    assert!(binding_listener.was_called(), "绑定监听器未被调用");

    // 清理
    let _ = manager.delete_event_binding("test_binding").await;
    let _ = manager.delete_memory_segment(segment_name).await;
}

/// 测试健康检查
#[tokio::test]
async fn test_health_check() {
    // 初始化管理器
    let config = ManagerConfig::default();
    let manager = RamingManager::new(config).unwrap();
    manager.start().await.unwrap();

    // 检查系统健康状态
    let health_result = manager.health_check();
    assert!(health_result.is_ok(), "健康检查失败: {:?}", health_result);

    let health_status = health_result.unwrap();
    assert!(health_status.is_healthy, "系统应该处于健康状态");

    // 创建一些组件后再次检查
    manager
        .create_memory_segment("health_test_segment".to_string(), 4096, false)
        .await
        .unwrap();

    let health_result2 = manager.health_check();
    assert!(health_result2.is_ok(), "健康检查失败: {:?}", health_result2);
    assert!(health_result2.unwrap().is_healthy, "系统应该仍然健康");

    // 清理
    let _ = manager.delete_memory_segment("health_test_segment").await;
}

/// 测试统计信息
#[tokio::test]
async fn test_statistics() {
    // 初始化管理器
    let config = ManagerConfig::default();
    let manager = RamingManager::new(config).unwrap();
    manager.start().await.unwrap();

    // 获取初始统计
    let initial_stats = manager.get_memory_stats();
    let _initial_event_stats = manager.get_event_stats();

    // 创建内存段
    manager
        .create_memory_segment("stats_segment1".to_string(), 1024, false)
        .await
        .unwrap();
    manager
        .create_memory_segment("stats_segment2".to_string(), 2048, false)
        .await
        .unwrap();

    // 获取更新后的统计
    let updated_stats = manager.get_memory_stats();
    assert!(
        updated_stats.total_segments >= initial_stats.total_segments + 2,
        "内存段统计未更新"
    );
    assert!(
        updated_stats.total_memory >= initial_stats.total_memory + 3072,
        "内存分配统计未更新"
    );

    // 清理
    let _ = manager.delete_memory_segment("stats_segment1").await;
    let _ = manager.delete_memory_segment("stats_segment2").await;
}

/// 测试并发操作
#[tokio::test]
async fn test_concurrent_operations() {
    // 初始化管理器
    let config = ManagerConfig::default();
    let manager = RamingManager::new(config).unwrap();
    manager.start().await.unwrap();
    let segment_name = "concurrent_segment";
    let segment_size = 16384; // 16KB

    // 创建内存段
    manager
        .create_memory_segment(segment_name.to_string(), segment_size, false)
        .await
        .unwrap();
    let segment = manager.shared_memory().get_segment(segment_name).unwrap();

    // 并发写入测试
    let mut handles = vec![];

    for i in 0..10 {
        let segment_clone = segment.clone();
        let handle = tokio::spawn(async move {
            let data = format!("Thread {} data", i);
            let offset = i * 100;
            segment_clone.write(offset, data.as_bytes())
        });
        handles.push(handle);
    }

    // 等待所有写入完成
    for handle in handles {
        let result = handle.await.unwrap();
        assert!(result.is_ok(), "并发写入失败: {:?}", result);
    }

    // 验证数据完整性
    for i in 0..10 {
        let offset = i * 100;
        let buffer = segment.read(offset, 20).unwrap();

        let expected = format!("Thread {} data", i);
        let actual = String::from_utf8_lossy(&buffer[..expected.len()]);
        assert_eq!(actual, expected, "并发写入的数据不匹配");
    }

    // 清理
    let _ = manager.delete_memory_segment(segment_name).await;
}

/// 测试错误处理
#[tokio::test]
async fn test_error_handling() {
    // 初始化管理器
    let config = ManagerConfig::default();
    let manager = RamingManager::new(config).unwrap();
    manager.start().await.unwrap();

    // 测试不存在的内存段访问
    let non_existent_result = manager.shared_memory().get_segment("non_existent");
    assert!(non_existent_result.is_none(), "访问不存在的内存段应该失败");

    // 测试删除不存在的内存段
    let delete_result = manager.delete_memory_segment("non_existent").await;
    assert!(delete_result.is_err(), "删除不存在的内存段应该失败");

    // 测试创建同名内存段
    let segment_name = "error_test_segment";
    manager
        .create_memory_segment(segment_name.to_string(), 1024, false)
        .await
        .unwrap();

    let duplicate_result = manager
        .create_memory_segment(segment_name.to_string(), 2048, false)
        .await;
    assert!(duplicate_result.is_err(), "创建同名内存段应该失败");

    // 清理
    let _ = manager.delete_memory_segment(segment_name).await;
}

/// 测试系统启动和关闭
#[tokio::test]
async fn test_system_lifecycle() {
    // 创建管理器配置
    let config = ManagerConfig::default();

    // 创建管理器实例
    let manager = RamingManager::new(config).unwrap();

    // 测试系统启动
    let start_result = manager.start().await;
    assert!(start_result.is_ok(), "系统启动失败: {:?}", start_result);

    // 验证系统运行状态
    assert!(manager.is_running(), "系统应该处于运行状态");

    // 测试系统关闭
    let stop_result = manager.stop().await;
    assert!(stop_result.is_ok(), "系统关闭失败: {:?}", stop_result);

    // 验证系统停止状态
    assert!(!manager.is_running(), "系统应该处于停止状态");
}

/// 测试事件类型转换
#[tokio::test]
async fn test_event_type_conversion() {
    // 测试事件类型名称转换
    let share_event = RamingEventType::MemoryShare;
    let update_event = RamingEventType::MemoryUpdate;
    let custom_event = RamingEventType::Custom("test_event".to_string());

    assert_eq!(share_event.name(), "memory_share");
    assert_eq!(update_event.name(), "memory_update");
    assert_eq!(custom_event.name(), "custom_test_event");

    // 测试基础事件类型转换
    assert_eq!(share_event.to_base_type(), seesea_event::EventType::Data);
    assert_eq!(update_event.to_base_type(), seesea_event::EventType::Data);
    assert_eq!(custom_event.to_base_type(), seesea_event::EventType::Data);
}

/// 测试事件数据序列化
#[tokio::test]
async fn test_event_data_serialization() {
    let event_data = RamingEventData::new(
        RamingEventType::MemoryShare,
        "test".to_string(),
        serde_json::json!({
            "segment_name": "test_segment",
            "data": "test_data",
            "timestamp": chrono::Utc::now().to_rfc3339()
        }),
    );

    // 测试JSON序列化
    let json_result = serde_json::to_string(&event_data);
    assert!(json_result.is_ok(), "JSON序列化失败: {:?}", json_result);

    let json_string = json_result.unwrap();

    // 测试JSON反序列化
    let deserialize_result = serde_json::from_str::<RamingEventData>(&json_string);
    assert!(
        deserialize_result.is_ok(),
        "JSON反序列化失败: {:?}",
        deserialize_result
    );

    let deserialized = deserialize_result.unwrap();
    assert_eq!(deserialized.event_type, event_data.event_type);
    assert_eq!(deserialized.payload, event_data.payload);
}

/// 测试绑定管理
#[tokio::test]
async fn test_binding_management() {
    // 初始化管理器
    let config = ManagerConfig::default();
    let manager = RamingManager::new(config).unwrap();
    manager.start().await.unwrap();

    // 创建测试监听器
    let listener = Arc::new(TestEventListener::new());

    // 创建多个绑定
    for i in 0..5 {
        let binding_name = format!("test_binding_{}", i);
        let result = manager
            .create_event_binding(
                binding_name.clone(),
                BindingType::MemoryShare,
                RamingEventType::MemoryShare,
                listener.clone() as Arc<dyn seesea_event::AsyncEventHandler>,
            )
            .await;
        assert!(result.is_ok(), "创建绑定 {} 失败: {:?}", i, result);
    }

    // 验证绑定列表
    let bindings = manager.binding_manager().get_all_bindings();
    assert!(bindings.len() >= 5, "绑定数量不正确");

    // 删除绑定
    for i in 0..5 {
        let binding_name = format!("test_binding_{}", i);
        let delete_result = manager.delete_event_binding(&binding_name).await;
        assert!(
            delete_result.is_ok(),
            "删除绑定 {} 失败: {:?}",
            i,
            delete_result
        );
    }

    // 验证绑定已删除
    let remaining_bindings = manager.binding_manager().get_all_bindings();
    for i in 0..5 {
        let binding_name = format!("test_binding_{}", i);
        assert!(
            !remaining_bindings.iter().any(|b| {
                let def = b.definition.read();
                def.name == binding_name
            }),
            "绑定 {} 应该已被删除",
            i
        );
    }
}

// 测试辅助结构

/// 测试事件监听器
struct TestEventListener {
    called: std::sync::Arc<std::sync::atomic::AtomicBool>,
    call_count: std::sync::Arc<std::sync::atomic::AtomicUsize>,
}

impl TestEventListener {
    fn new() -> Self {
        Self {
            called: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
            call_count: std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0)),
        }
    }

    fn was_called(&self) -> bool {
        self.called.load(std::sync::atomic::Ordering::SeqCst)
    }

    fn get_call_count(&self) -> usize {
        self.call_count.load(std::sync::atomic::Ordering::SeqCst)
    }
}

#[async_trait::async_trait]
impl RamingEventListener for TestEventListener {
    async fn handle_raming_event(&self, _event: RamingEventData) -> RamingResult<()> {
        self.called.store(true, std::sync::atomic::Ordering::SeqCst);
        self.call_count
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        Ok(())
    }

    fn name(&self) -> &str {
        "TestEventListener"
    }

    fn supported_binding_types(&self) -> Vec<BindingType> {
        vec![BindingType::MemoryShare]
    }

    fn supports_event_type(&self, event_type: &RamingEventType) -> bool {
        matches!(event_type, RamingEventType::MemoryShare)
    }
}

#[async_trait::async_trait]
impl seesea_event::AsyncEventHandler for TestEventListener {
    async fn handle_event_with_response(
        &self,
        _event: &seesea_event::AsyncEvent,
    ) -> seesea_event::RamingResult<seesea_event::EventPayload> {
        Ok(seesea_event::EventPayload::Empty)
    }

    fn name(&self) -> &str {
        "TestEventListener"
    }

    fn can_handle(&self, _event_type: &seesea_event::async_events::EventType) -> bool {
        true
    }
}

/// 测试绑定监听器
struct TestBindingListener {
    called: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

impl TestBindingListener {
    fn new() -> Self {
        Self {
            called: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }

    fn was_called(&self) -> bool {
        self.called.load(std::sync::atomic::Ordering::SeqCst)
    }
}

#[async_trait::async_trait]
impl RamingEventListener for TestBindingListener {
    async fn handle_raming_event(&self, _event: RamingEventData) -> RamingResult<()> {
        self.called.store(true, std::sync::atomic::Ordering::SeqCst);
        Ok(())
    }

    fn name(&self) -> &str {
        "TestBindingListener"
    }

    fn supported_binding_types(&self) -> Vec<BindingType> {
        vec![BindingType::MemoryShare, BindingType::EventListener]
    }

    fn supports_event_type(&self, event_type: &RamingEventType) -> bool {
        matches!(
            event_type,
            RamingEventType::MemoryUpdate | RamingEventType::MemoryShare
        )
    }
}

#[async_trait::async_trait]
impl seesea_event::AsyncEventHandler for TestBindingListener {
    async fn handle_event_with_response(
        &self,
        _event: &seesea_event::AsyncEvent,
    ) -> seesea_event::RamingResult<seesea_event::EventPayload> {
        self.called.store(true, std::sync::atomic::Ordering::SeqCst);
        Ok(seesea_event::EventPayload::Empty)
    }

    fn name(&self) -> &str {
        "TestBindingListener"
    }

    fn can_handle(&self, _event_type: &seesea_event::async_events::EventType) -> bool {
        true
    }
}
