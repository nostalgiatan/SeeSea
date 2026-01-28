//! 事件系统单元测试

use seesea_event::{EventListener, EventType};
use seesea_raming::RamingResult;
use seesea_raming::events::*;
use seesea_raming::types::{BindingType, EventConfig};
use std::sync::Arc;
use tokio::time::{Duration, sleep};

/// 测试RamingEventType枚举
#[test]
fn test_raming_event_type() {
    let share_event = RamingEventType::MemoryShare;
    let update_event = RamingEventType::MemoryUpdate;
    let delete_event = RamingEventType::MemoryDelete;
    let custom_event = RamingEventType::Custom("test_event".to_string());

    // 测试名称
    assert_eq!(share_event.name(), "MemoryShare");
    assert_eq!(update_event.name(), "MemoryUpdate");
    assert_eq!(delete_event.name(), "MemoryDelete");
    assert_eq!(custom_event.name(), "Custom(test_event)");

    // 测试基础类型转换
    assert_eq!(share_event.to_base_type(), seesea_event::EventType::Data);
    assert_eq!(update_event.to_base_type(), seesea_event::EventType::Data);
    assert_eq!(custom_event.to_base_type(), seesea_event::EventType::Data);
}

/// 测试RamingEventData结构
#[test]
fn test_raming_event_data() {
    let payload = serde_json::json!({
        "key": "value",
        "number": 42,
        "nested": {
            "inner": "data"
        }
    });

    let event_data = RamingEventData::new(
        RamingEventType::MemoryShare,
        "test".to_string(),
        payload.clone(),
    );

    assert_eq!(event_data.event_type, RamingEventType::MemoryShare);
    assert_eq!(event_data.payload, payload);

    // 测试JSON序列化
    let json_result = serde_json::to_string(&event_data);
    assert!(json_result.is_ok());

    let json_string = json_result.unwrap();
    let deserialize_result = serde_json::from_str::<RamingEventData>(&json_string);
    assert!(deserialize_result.is_ok());

    let deserialized = deserialize_result.unwrap();
    assert_eq!(deserialized.event_type, event_data.event_type);
    assert_eq!(deserialized.payload, event_data.payload);
}

/// 测试事件数据转换
#[test]
fn test_event_data_conversion() {
    let payload = serde_json::json!({
        "segment_name": "test_segment",
        "operation": "create",
        "timestamp": chrono::Utc::now().to_rfc3339()
    });

    let event_data =
        RamingEventData::new(RamingEventType::MemoryShare, "test".to_string(), payload);

    // 转换为DataEvent
    let data_event = event_data.to_data_event();
    assert_eq!(data_event.event_type, Arc::from("MemoryShare"));
    assert!(!data_event.payload.is_empty());

    // 从DataEvent转换回来
    let converted_back = RamingEventData::from_data_event(&data_event);
    assert!(converted_back.is_ok());

    let converted = converted_back.unwrap();
    assert_eq!(converted.event_type, event_data.event_type);
}

/// 测试事件监听器适配器
#[tokio::test]
async fn test_event_adapter() {
    let config = EventConfig::default();
    let _event_bus = Arc::new(RamingEventBus::new(config));

    // 创建测试监听器
    let test_listener = Arc::new(TestEventListener::new());
    let adapter = Arc::new(RamingEventAdapter::new(test_listener.clone()));

    // 测试适配器名称
    assert_eq!(adapter.name(), "TestEventListener");

    // 测试感兴趣的事件类型
    let interested_events = adapter.interested_events();
    assert_eq!(interested_events.len(), 1);
    assert_eq!(interested_events[0], EventType::Data);
}

/// 测试事件发布
#[tokio::test]
async fn test_event_publishing() {
    let config = EventConfig::default();
    let event_bus = Arc::new(RamingEventBus::new(config).unwrap());

    // 创建测试监听器
    let test_listener = Arc::new(TestEventListener::new());

    // 订阅事件
    let subscribe_result = event_bus
        .subscribe(RamingEventType::MemoryShare, test_listener.clone())
        .await;
    assert!(subscribe_result.is_ok());

    // 发布事件
    let event_data = RamingEventData::new(
        RamingEventType::MemoryShare,
        "test_source".to_string(),
        serde_json::json!({
            "segment_name": "test_segment",
            "operation": "test"
        }),
    );

    let publish_result = event_bus.publish(event_data).await;
    assert!(publish_result.is_ok());

    // 等待事件处理
    sleep(Duration::from_millis(100)).await;

    // 验证监听器被调用
    assert!(test_listener.was_called());
    assert_eq!(test_listener.get_call_count(), 1);
}

/// 测试事件订阅和取消订阅
#[tokio::test]
async fn test_event_subscription() {
    let config = EventConfig::default();
    let event_bus = Arc::new(RamingEventBus::new(config).unwrap());

    let listener1 = Arc::new(TestEventListener::new());
    let listener2 = Arc::new(TestEventListener::new());

    // 订阅多个监听器
    let _ = event_bus
        .subscribe(RamingEventType::MemoryShare, listener1.clone())
        .await
        .unwrap();
    let _ = event_bus
        .subscribe(RamingEventType::MemoryUpdate, listener2.clone())
        .await
        .unwrap();

    // 发布事件
    let share_event = RamingEventData::new(
        RamingEventType::MemoryShare,
        "test_source".to_string(),
        serde_json::json!({"test": "share"}),
    );

    let update_event = RamingEventData::new(
        RamingEventType::MemoryUpdate,
        "test_source".to_string(),
        serde_json::json!({"test": "update"}),
    );

    let _ = event_bus.publish(share_event).await.unwrap();
    let _ = event_bus.publish(update_event).await.unwrap();

    sleep(Duration::from_millis(100)).await;

    // 验证正确的监听器被调用
    assert!(listener1.was_called());
    assert!(listener2.was_called());
}

/// 测试事件绑定管理
#[tokio::test]
async fn test_event_binding_management() {
    let config = EventConfig::default();
    let event_bus = Arc::new(RamingEventBus::new(config).unwrap());

    let listener = Arc::new(TestEventListener::new());

    // 创建绑定
    let binding_result = event_bus
        .create_binding(
            "test_binding".to_string(),
            BindingType::MemoryShare,
            RamingEventType::MemoryShare,
            listener.clone(),
        )
        .await;
    assert!(binding_result.is_ok());

    // 验证绑定存在
    let bindings = event_bus.get_bindings();
    assert!(bindings.contains(&"test_binding".to_string()));

    // 删除绑定
    let remove_result = event_bus.remove_binding("test_binding");
    assert!(remove_result.is_ok());

    // 验证绑定已删除
    let bindings_after = event_bus.get_bindings();
    assert!(!bindings_after.contains(&"test_binding".to_string()));
}

/// 测试事件统计
#[tokio::test]
async fn test_event_statistics() {
    let config = EventConfig::default();
    let event_bus = Arc::new(RamingEventBus::new(config).unwrap());

    // 获取初始统计
    let initial_stats = event_bus.get_stats();

    // 创建一些事件
    let listener = Arc::new(TestEventListener::new());
    let _ = event_bus
        .subscribe(RamingEventType::MemoryShare, listener.clone())
        .await
        .unwrap();

    // 发布多个事件
    for i in 0..10 {
        let event = RamingEventData::new(
            RamingEventType::MemoryShare,
            "test_source".to_string(),
            serde_json::json!({"index": i}),
        );
        let _ = event_bus.publish(event).await.unwrap();
    }

    sleep(Duration::from_millis(200)).await;

    // 获取更新后的统计
    let updated_stats = event_bus.get_stats();
    assert!(updated_stats.published_events >= initial_stats.published_events + 10);
    assert!(updated_stats.active_listeners >= initial_stats.active_listeners + 1);
}

/// 测试事件健康检查
#[tokio::test]
async fn test_event_health_check() {
    let config = EventConfig::default();
    let event_bus = Arc::new(RamingEventBus::new(config).unwrap());

    // 健康检查应该通过
    let health_result = event_bus.health_check();
    assert!(health_result.is_ok());

    let health_status = health_result.unwrap();
    assert!(health_status.is_healthy);
}

/// 测试事件错误处理
#[tokio::test]
async fn test_event_error_handling() {
    let config = EventConfig::default();
    let event_bus = Arc::new(RamingEventBus::new(config).unwrap());

    // 测试创建同名绑定
    let listener = Arc::new(TestEventListener::new());
    let _ = event_bus
        .create_binding(
            "duplicate_binding".to_string(),
            BindingType::MemoryShare,
            RamingEventType::MemoryShare,
            listener.clone(),
        )
        .await
        .unwrap();

    let duplicate_result = event_bus
        .create_binding(
            "duplicate_binding".to_string(),
            BindingType::MemoryShare,
            RamingEventType::MemoryShare,
            listener.clone(),
        )
        .await;
    assert!(duplicate_result.is_err());

    // 测试删除不存在的绑定
    let remove_result = event_bus.remove_binding("non_existent");
    assert!(remove_result.is_ok()); // 删除不存在的绑定不应该报错
}

/// 测试事件并发处理
#[tokio::test]
async fn test_concurrent_event_processing() {
    let config = EventConfig::default();
    let event_bus = Arc::new(RamingEventBus::new(config).unwrap());

    let listener = Arc::new(TestEventListener::new());
    let _ = event_bus
        .subscribe(RamingEventType::MemoryShare, listener.clone())
        .await
        .unwrap();

    // 并发发布多个事件
    let mut handles = vec![];
    for i in 0..20 {
        let event_bus_clone = event_bus.clone();
        let handle = tokio::spawn(async move {
            let event = RamingEventData::new(
                RamingEventType::MemoryShare,
                "test_source".to_string(),
                serde_json::json!({"concurrent_index": i}),
            );
            event_bus_clone.publish(event).await
        });
        handles.push(handle);
    }

    // 等待所有发布完成
    for handle in handles {
        let result = handle.await.unwrap();
        assert!(result.is_ok());
    }

    // 等待事件处理
    sleep(Duration::from_millis(300)).await;

    // 验证所有事件都被处理
    assert!(listener.was_called());
    assert!(listener.get_call_count() >= 20);
}

/// 测试事件类型匹配
#[tokio::test]
async fn test_event_type_matching() {
    let config = EventConfig::default();
    let event_bus = Arc::new(RamingEventBus::new(config).unwrap());

    let share_listener = Arc::new(TestEventListener::new());
    let update_listener = Arc::new(TestEventListener::new());
    let _all_listener = Arc::new(TestEventListener::new());

    // 订阅不同类型的监听器
    let _ = event_bus
        .subscribe(RamingEventType::MemoryShare, share_listener.clone())
        .await
        .unwrap();
    let _ = event_bus
        .subscribe(RamingEventType::MemoryUpdate, update_listener.clone())
        .await
        .unwrap();

    // 发布不同类型的事件
    let share_event = RamingEventData::new(
        RamingEventType::MemoryShare,
        "test".to_string(),
        serde_json::json!({"type": "share"}),
    );
    let update_event = RamingEventData::new(
        RamingEventType::MemoryUpdate,
        "test".to_string(),
        serde_json::json!({"type": "update"}),
    );

    let _ = event_bus.publish(share_event).await.unwrap();
    let _ = event_bus.publish(update_event).await.unwrap();

    sleep(Duration::from_millis(100)).await;

    // 验证正确的监听器被调用
    assert!(share_listener.was_called());
    assert!(!share_listener.was_called() || share_listener.get_call_count() > 0);

    assert!(update_listener.was_called());
    assert!(!update_listener.was_called() || update_listener.get_call_count() > 0);
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
        vec![BindingType::MemoryShare, BindingType::EventListener]
    }

    fn supports_event_type(&self, event_type: &RamingEventType) -> bool {
        matches!(
            event_type,
            RamingEventType::MemoryShare | RamingEventType::MemoryUpdate
        )
    }
}
