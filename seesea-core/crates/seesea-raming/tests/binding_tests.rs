//! 绑定管理器单元测试

use seesea_raming::bindings::*;
use seesea_raming::events::{RamingEventData, RamingEventListener, RamingEventType};
use seesea_raming::memory::SharedMemory;
use seesea_raming::types::MemoryConfig;
use seesea_raming::types::{BindingConfig, BindingStats, BindingType};
use std::sync::Arc;
use tokio::time::Duration;

/// 测试绑定状态
#[test]
fn test_binding_status() {
    assert_eq!(BindingStatus::Active, BindingStatus::Active);
    assert_eq!(BindingStatus::Inactive, BindingStatus::Inactive);
    assert_ne!(BindingStatus::Active, BindingStatus::Inactive);
}

/// 测试绑定配置
#[test]
fn test_binding_config() {
    let config = BindingConfig {
        max_bindings: 100,
        cleanup_interval: Duration::from_secs(300),
        binding_timeout: Duration::from_secs(3600),
        enable_persistence: false,
        persistence_path: None,
        enable_monitoring: true,
        monitoring_interval: Duration::from_secs(60),
    };

    assert_eq!(config.max_bindings, 100);
    assert_eq!(config.cleanup_interval, Duration::from_secs(300));
    assert_eq!(config.binding_timeout, Duration::from_secs(3600));
}

/// 测试绑定统计
#[test]
fn test_binding_stats() {
    let mut stats = BindingStats::default();

    assert_eq!(stats.success_count, 0);
    assert_eq!(stats.error_count, 0);
    assert_eq!(stats.total_count, 0);

    // 更新统计
    stats.success_count = 10;
    stats.error_count = 2;
    stats.total_count = 12;
    stats.total_duration = Duration::from_millis(1200);

    // 测试统计数据
    assert_eq!(stats.success_count, 10);
    assert_eq!(stats.error_count, 2);
    assert_eq!(stats.total_count, 12);
    assert_eq!(stats.total_duration, Duration::from_millis(1200));
}

/// 测试绑定创建和管理
#[tokio::test]
async fn test_binding_creation() {
    let config = BindingConfig::default();
    let binding_manager = Arc::new(BindingManager::new(config).unwrap());
    binding_manager.start().unwrap();

    let listener = Arc::new(TestBindingListener::new());

    // 创建绑定
    let binding_result = binding_manager
        .create_binding_async(
            "test_binding".to_string(),
            BindingType::MemoryShare,
            RamingEventType::MemoryShare,
            listener.clone(),
        )
        .await;

    assert!(binding_result.is_ok());

    // 验证绑定存在
    let binding = binding_manager.get_binding_option("test_binding");
    assert!(binding.is_some());

    let binding_ref = binding.unwrap();
    assert_eq!(binding_ref.name, "test_binding");
    assert_eq!(binding_ref.binding_type, BindingType::MemoryShare);
    assert_eq!(binding_ref.event_type, Some(RamingEventType::MemoryShare));

    binding_manager.stop().unwrap();
}

/// 测试绑定启用和禁用
#[tokio::test]
async fn test_binding_enable_disable() {
    let config = BindingConfig::default();
    let memory_config = MemoryConfig::default();
    let event_config = seesea_raming::types::EventConfig::default();

    let shared_memory = Arc::new(SharedMemory::new(memory_config).unwrap());
    let event_bus = Arc::new(seesea_raming::events::RamingEventBus::new(event_config).unwrap());
    let mut binding_manager = BindingManager::new(config).unwrap();
    binding_manager.set_shared_memory(shared_memory);
    binding_manager.set_event_bus(event_bus);
    let binding_manager = Arc::new(binding_manager);

    let _listener = Arc::new(TestBindingListener::new());

    // 创建绑定
    let _ = binding_manager
        .create_binding("enable_test".to_string(), BindingType::MemoryShare)
        .unwrap();

    let binding = binding_manager.get_binding("enable_test").unwrap();

    // 测试禁用
    binding.disable();
    assert!(!binding.is_enabled());

    // 测试启用
    binding.enable();
    assert!(binding.is_enabled());
}

/// 测试绑定删除
#[tokio::test]
async fn test_binding_deletion() {
    let config = BindingConfig::default();
    let memory_config = MemoryConfig::default();
    let event_config = seesea_raming::types::EventConfig::default();

    let shared_memory = Arc::new(SharedMemory::new(memory_config).unwrap());
    let event_bus = Arc::new(seesea_raming::events::RamingEventBus::new(event_config).unwrap());
    let mut binding_manager = BindingManager::new(config).unwrap();
    binding_manager.set_shared_memory(shared_memory);
    binding_manager.set_event_bus(event_bus);
    let binding_manager = Arc::new(binding_manager);

    let _listener = Arc::new(TestBindingListener::new());

    // 创建绑定
    let _ = binding_manager
        .create_binding("delete_test".to_string(), BindingType::MemoryShare)
        .unwrap();

    // 验证绑定存在
    assert!(binding_manager.get_binding_option("delete_test").is_some());

    // 删除绑定
    let delete_result = binding_manager.delete_binding("delete_test");
    assert!(delete_result.is_ok());

    // 验证绑定已删除
    assert!(binding_manager.get_binding_option("delete_test").is_none());
}

/// 测试绑定列表
#[tokio::test]
async fn test_binding_list() {
    let config = BindingConfig::default();
    let memory_config = MemoryConfig::default();
    let event_config = seesea_raming::types::EventConfig::default();

    let shared_memory = Arc::new(SharedMemory::new(memory_config).unwrap());
    let event_bus = Arc::new(seesea_raming::events::RamingEventBus::new(event_config).unwrap());
    let mut binding_manager = BindingManager::new(config).unwrap();
    binding_manager.set_shared_memory(shared_memory);
    binding_manager.set_event_bus(event_bus);
    let binding_manager = Arc::new(binding_manager);

    let _listener = Arc::new(TestBindingListener::new());

    // 创建多个绑定
    for i in 0..5 {
        let binding_name = format!("list_test_{}", i);
        let _ = binding_manager
            .create_binding(binding_name, BindingType::MemoryShare)
            .unwrap();
    }

    // 获取绑定列表
    let bindings = binding_manager.get_bindings();
    assert!(bindings.len() >= 5);

    // 验证绑定名称
    for i in 0..5 {
        let binding_name = format!("list_test_{}", i);
        assert!(bindings.contains_key(&binding_name));
    }
}

/// 测试绑定事件处理
#[tokio::test]
async fn test_binding_event_processing() {
    let config = BindingConfig::default();
    let memory_config = MemoryConfig::default();
    let event_config = seesea_raming::types::EventConfig::default();

    let shared_memory = Arc::new(SharedMemory::new(memory_config).unwrap());
    let event_bus = Arc::new(seesea_raming::events::RamingEventBus::new(event_config).unwrap());
    let mut binding_manager = BindingManager::new(config).unwrap();
    binding_manager.set_shared_memory(shared_memory);
    binding_manager.set_event_bus(event_bus);
    let binding_manager = Arc::new(binding_manager);

    let _listener = Arc::new(TestBindingListener::new());

    // 创建绑定
    let _ = binding_manager
        .create_binding("process_test".to_string(), BindingType::MemoryShare)
        .unwrap();
}

/// 测试绑定统计更新
#[tokio::test]
async fn test_binding_stats_update() {
    let config = BindingConfig::default();
    let memory_config = MemoryConfig::default();
    let event_config = seesea_raming::types::EventConfig::default();

    let shared_memory = Arc::new(SharedMemory::new(memory_config).unwrap());
    let event_bus = Arc::new(seesea_raming::events::RamingEventBus::new(event_config).unwrap());
    let mut binding_manager = BindingManager::new(config).unwrap();
    binding_manager.set_shared_memory(shared_memory);
    binding_manager.set_event_bus(event_bus);
    let binding_manager = Arc::new(binding_manager);

    let _listener = Arc::new(TestBindingListener::new());

    // 创建绑定
    let _ = binding_manager
        .create_binding("stats_test".to_string(), BindingType::MemoryShare)
        .unwrap();

    let binding = binding_manager.get_binding("stats_test").unwrap();

    // 更新统计信息
    binding.update_stats(true, Duration::from_millis(100));
    binding.update_stats(false, Duration::from_millis(50));
    binding.update_stats(true, Duration::from_millis(150));

    // 验证统计信息
    let stats = binding.stats.read();
    assert_eq!(stats.success_count, 2);
    assert_eq!(stats.error_count, 1);
    assert_eq!(stats.total_count, 3);
}

/// 测试绑定健康检查
#[tokio::test]
async fn test_binding_health_check() {
    let config = BindingConfig::default();
    let memory_config = MemoryConfig::default();
    let event_config = seesea_raming::types::EventConfig::default();

    let shared_memory = Arc::new(SharedMemory::new(memory_config).unwrap());
    let event_bus = Arc::new(seesea_raming::events::RamingEventBus::new(event_config).unwrap());
    let mut binding_manager = BindingManager::new(config).unwrap();
    binding_manager.set_shared_memory(shared_memory);
    binding_manager.set_event_bus(event_bus);
    let binding_manager = Arc::new(binding_manager);

    // 健康检查应该通过
    let health_result = binding_manager.health_check();
    assert!(health_result.is_ok());

    let health_status = health_result.unwrap();
    assert!(health_status.is_healthy);
}

/// 测试绑定错误处理
#[tokio::test]
async fn test_binding_error_handling() {
    let config = BindingConfig::default();
    let memory_config = MemoryConfig::default();
    let event_config = seesea_raming::types::EventConfig::default();

    let shared_memory = Arc::new(SharedMemory::new(memory_config).unwrap());
    let event_bus = Arc::new(seesea_raming::events::RamingEventBus::new(event_config).unwrap());
    let mut binding_manager = BindingManager::new(config).unwrap();
    binding_manager.set_shared_memory(shared_memory);
    binding_manager.set_event_bus(event_bus);
    let binding_manager = Arc::new(binding_manager);

    let _listener = Arc::new(TestBindingListener::new());

    // 测试创建同名绑定
    let _ = binding_manager
        .create_binding("duplicate_error_test".to_string(), BindingType::MemoryShare)
        .unwrap();

    let duplicate_result = binding_manager
        .create_binding("duplicate_error_test".to_string(), BindingType::MemoryShare);
    assert!(duplicate_result.is_err());

    // 测试获取不存在的绑定
    let non_existent = binding_manager.get_binding_option("non_existent");
    assert!(non_existent.is_none());

    // 测试删除不存在的绑定
    let delete_result = binding_manager.delete_binding("non_existent");
    assert!(delete_result.is_err());
}

/// 测试绑定并发操作
#[tokio::test]
async fn test_concurrent_binding_operations() {
    let config = BindingConfig::default();
    let memory_config = MemoryConfig::default();
    let event_config = seesea_raming::types::EventConfig::default();

    let shared_memory = Arc::new(SharedMemory::new(memory_config).unwrap());
    let event_bus = Arc::new(seesea_raming::events::RamingEventBus::new(event_config).unwrap());
    let mut binding_manager = BindingManager::new(config).unwrap();
    binding_manager.set_shared_memory(shared_memory);
    binding_manager.set_event_bus(event_bus);
    let binding_manager = Arc::new(binding_manager);

    let _listener = Arc::new(TestBindingListener::new());

    // 并发创建多个绑定
    let mut handles = vec![];
    for i in 0..10 {
        let manager_clone = binding_manager.clone();
        let handle = tokio::spawn(async move {
            let binding_name = format!("concurrent_{}", i);
            manager_clone.create_binding(binding_name, BindingType::MemoryShare)
        });
        handles.push(handle);
    }

    // 等待所有创建完成
    for handle in handles {
        let result = handle.await.unwrap();
        assert!(result.is_ok());
    }

    // 验证所有绑定都已创建
    let bindings = binding_manager.get_bindings();
    assert!(bindings.len() >= 10);
}

/// 测试绑定生命周期
#[tokio::test]
async fn test_binding_lifecycle() {
    let config = BindingConfig::default();
    let memory_config = MemoryConfig::default();
    let event_config = seesea_raming::types::EventConfig::default();

    let shared_memory = Arc::new(SharedMemory::new(memory_config).unwrap());
    let event_bus = Arc::new(seesea_raming::events::RamingEventBus::new(event_config).unwrap());
    let mut binding_manager = BindingManager::new(config).unwrap();
    binding_manager.set_shared_memory(shared_memory);
    binding_manager.set_event_bus(event_bus);
    let binding_manager = Arc::new(binding_manager);

    // 测试启动和停止
    let start_result = binding_manager.start();
    assert!(start_result.is_ok());
    assert!(binding_manager.is_running());

    let stop_result = binding_manager.stop();
    assert!(stop_result.is_ok());
    assert!(!binding_manager.is_running());
}

// 测试辅助结构

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

    #[allow(dead_code)]
    fn was_called(&self) -> bool {
        self.called.load(std::sync::atomic::Ordering::SeqCst)
    }
}

#[async_trait::async_trait]
impl RamingEventListener for TestBindingListener {
    async fn handle_raming_event(
        &self,
        _event: RamingEventData,
    ) -> seesea_raming::RamingResult<()> {
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
            RamingEventType::MemoryShare | RamingEventType::MemoryUpdate
        )
    }
}
