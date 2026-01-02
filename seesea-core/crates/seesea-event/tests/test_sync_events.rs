//! Synchronous Event Tests
//!
//! Tests for the synchronous event bus functionality, including
//! event publishing, listener registration, and event delivery.

use seesea_event::{EventBus, StringEventOperations};
use std::sync::{Arc, Mutex};

/// Helper function to create a test event bus with running event loop
fn create_test_bus() -> (Arc<EventBus>, std::thread::JoinHandle<()>) {
    let bus = Arc::new(EventBus::new());

    // Start event loop in background
    let bus_clone = Arc::clone(&bus);
    let handle = std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let _ = bus_clone.run().await;
        })
    });

    // Give event loop time to start
    std::thread::sleep(std::time::Duration::from_millis(100));

    (bus, handle)
}

#[test]
fn test_publish_string_event() {
    let bus = Arc::new(EventBus::new());

    // Test basic string event publishing
    let result = Arc::clone(&bus).publish_string("test.event", "test data");
    assert!(
        result.is_ok(),
        "Failed to publish string event: {:?}",
        result
    );
}

#[test]
fn test_publish_string_error_event() {
    let bus = Arc::new(EventBus::new());

    // Test error event publishing
    let result = Arc::clone(&bus).publish_string_error("test.error", "error message");
    assert!(
        result.is_ok(),
        "Failed to publish error event: {:?}",
        result
    );
}

#[test]
fn test_event_listener_registration() {
    // Create a test event bus instance
    let bus = Arc::new(seesea_event::EventBus::new());

    // Create a counter to track event delivery
    let counter = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let counter_clone = counter.clone();

    // Create a test listener
    struct TestListener {
        name: String,
        event_type: String,
        counter: Arc<std::sync::atomic::AtomicUsize>,
    }

    #[async_trait::async_trait]
    impl seesea_event::EventListener for TestListener {
        fn name(&self) -> &str {
            &self.name
        }

        fn interested_events(&self) -> &[seesea_event::EventType] {
            &[seesea_event::EventType::Data]
        }

        async fn on_event(&self, event: &seesea_event::Event) -> seesea_event::RamingResult<()> {
            if let seesea_event::Event::Data(data_event) = event {
                if data_event.event_type.as_ref() == self.event_type {
                    if let Ok(data_str) = String::from_utf8(data_event.payload.to_vec()) {
                        assert_eq!(data_str, "listener test data");
                        self.counter
                            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    }
                }
            }
            Ok(())
        }
    }

    // Create and register the listener
    let listener = Arc::new(TestListener {
        name: "test_listener".to_string(),
        event_type: "test.listener".to_string(),
        counter: counter_clone,
    });

    // Use tokio runtime to handle async operations
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        // Register the listener
        bus.subscribe(listener).await.unwrap();

        // Create and process the event synchronously
        let event = seesea_event::EventBuilder::string_event("test.listener", "listener test data");
        bus.process_event_sync(event).await.unwrap();
    });

    // Verify the listener was called
    assert_eq!(counter.load(std::sync::atomic::Ordering::SeqCst), 1);
}

#[test]
fn test_multiple_listeners_same_event() {
    let (bus, _handle) = create_test_bus();

    // Create multiple counters for different listeners
    let counter1 = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let counter2 = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let counter3 = Arc::new(std::sync::atomic::AtomicUsize::new(0));

    let c1 = counter1.clone();
    let c2 = counter2.clone();
    let c3 = counter3.clone();

    // Use a single runtime to register all listeners to avoid conflicts
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        // Register multiple listeners for the same event using async API
        struct Listener1 {
            counter: Arc<std::sync::atomic::AtomicUsize>,
        }

        #[async_trait::async_trait]
        impl seesea_event::EventListener for Listener1 {
            fn name(&self) -> &str {
                "listener1"
            }
            fn interested_events(&self) -> &[seesea_event::EventType] {
                &[seesea_event::EventType::Data]
            }
            async fn on_event(
                &self,
                event: &seesea_event::Event,
            ) -> seesea_event::RamingResult<()> {
                if let seesea_event::Event::Data(data_event) = event {
                    if data_event.event_type.as_ref() == "multi.listener" {
                        self.counter
                            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    }
                }
                Ok(())
            }
        }

        struct Listener2 {
            counter: Arc<std::sync::atomic::AtomicUsize>,
        }

        #[async_trait::async_trait]
        impl seesea_event::EventListener for Listener2 {
            fn name(&self) -> &str {
                "listener2"
            }
            fn interested_events(&self) -> &[seesea_event::EventType] {
                &[seesea_event::EventType::Data]
            }
            async fn on_event(
                &self,
                event: &seesea_event::Event,
            ) -> seesea_event::RamingResult<()> {
                if let seesea_event::Event::Data(data_event) = event {
                    if data_event.event_type.as_ref() == "multi.listener" {
                        self.counter
                            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    }
                }
                Ok(())
            }
        }

        struct Listener3 {
            counter: Arc<std::sync::atomic::AtomicUsize>,
        }

        #[async_trait::async_trait]
        impl seesea_event::EventListener for Listener3 {
            fn name(&self) -> &str {
                "listener3"
            }
            fn interested_events(&self) -> &[seesea_event::EventType] {
                &[seesea_event::EventType::Data]
            }
            async fn on_event(
                &self,
                event: &seesea_event::Event,
            ) -> seesea_event::RamingResult<()> {
                if let seesea_event::Event::Data(data_event) = event {
                    if data_event.event_type.as_ref() == "multi.listener" {
                        self.counter
                            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    }
                }
                Ok(())
            }
        }

        bus.subscribe(Arc::new(Listener1 { counter: c1 }))
            .await
            .unwrap();
        bus.subscribe(Arc::new(Listener2 { counter: c2 }))
            .await
            .unwrap();
        bus.subscribe(Arc::new(Listener3 { counter: c3 }))
            .await
            .unwrap();
    });

    // Give listener registration time to process
    std::thread::sleep(std::time::Duration::from_millis(100));

    // Publish the event
    Arc::clone(&bus)
        .publish_string("multi.listener", "multi listener data")
        .unwrap();

    // Give event processing time
    std::thread::sleep(std::time::Duration::from_millis(100));

    // Verify all listeners were called
    assert_eq!(counter1.load(std::sync::atomic::Ordering::SeqCst), 1);
    assert_eq!(counter2.load(std::sync::atomic::Ordering::SeqCst), 1);
    assert_eq!(counter3.load(std::sync::atomic::Ordering::SeqCst), 1);
}

#[test]
fn test_event_data_capture() {
    let (bus, _handle) = create_test_bus();

    // Create a shared storage for captured data
    let captured_data = Arc::new(Mutex::new(Vec::new()));
    let data_clone = captured_data.clone();

    // Register a listener that captures event data
    Arc::clone(&bus)
        .on("capture.test", move |event_type, data| {
            let mut storage = data_clone.lock().unwrap();
            storage.push((event_type.to_string(), data.to_string()));
        })
        .unwrap();

    // Give listener registration time to process
    std::thread::sleep(std::time::Duration::from_millis(100));

    // Publish events with different data
    Arc::clone(&bus)
        .publish_string("capture.test", "first data")
        .unwrap();
    Arc::clone(&bus)
        .publish_string("capture.test", "second data")
        .unwrap();
    Arc::clone(&bus)
        .publish_string("capture.test", "third data")
        .unwrap();

    // Give event processing time
    std::thread::sleep(std::time::Duration::from_millis(300));

    // Verify captured data
    let storage = captured_data.lock().unwrap();
    assert_eq!(storage.len(), 3);
    assert_eq!(
        storage[0],
        ("capture.test".to_string(), "first data".to_string())
    );
    assert_eq!(
        storage[1],
        ("capture.test".to_string(), "second data".to_string())
    );
    assert_eq!(
        storage[2],
        ("capture.test".to_string(), "third data".to_string())
    );
}

#[test]
fn test_listener_with_different_event_types() {
    let (bus, _handle) = create_test_bus();

    // Create counters for different event types
    let type_a_counter = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let type_b_counter = Arc::new(std::sync::atomic::AtomicUsize::new(0));

    let a_counter = type_a_counter.clone();
    let b_counter = type_b_counter.clone();

    // Register listeners for different event types
    Arc::clone(&bus)
        .on("type.a", move |_, _| {
            a_counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        })
        .unwrap();

    Arc::clone(&bus)
        .on("type.b", move |_, _| {
            b_counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        })
        .unwrap();

    // Give listener registration time to process
    std::thread::sleep(std::time::Duration::from_millis(100));

    // Publish events to different types
    Arc::clone(&bus)
        .publish_string("type.a", "data for a")
        .unwrap();
    Arc::clone(&bus)
        .publish_string("type.a", "more data for a")
        .unwrap();
    Arc::clone(&bus)
        .publish_string("type.b", "data for b")
        .unwrap();

    // Give event processing time
    std::thread::sleep(std::time::Duration::from_millis(300));

    // Verify correct listeners were called
    assert_eq!(type_a_counter.load(std::sync::atomic::Ordering::SeqCst), 2);
    assert_eq!(type_b_counter.load(std::sync::atomic::Ordering::SeqCst), 1);
}

#[test]
fn test_empty_event_data() {
    let (bus, _handle) = create_test_bus();

    let counter = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let counter_clone = counter.clone();

    // Register listener for empty data test
    Arc::clone(&bus)
        .on("empty.data", move |event_type, data| {
            counter_clone.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            assert_eq!(event_type, "empty.data");
            assert_eq!(data, "");
        })
        .unwrap();

    // Give listener registration time to process
    std::thread::sleep(std::time::Duration::from_millis(100));

    // Publish event with empty data
    Arc::clone(&bus).publish_string("empty.data", "").unwrap();

    // Give event processing time
    std::thread::sleep(std::time::Duration::from_millis(100));

    // Verify listener was called with empty data
    assert_eq!(counter.load(std::sync::atomic::Ordering::SeqCst), 1);
}

#[test]
fn test_large_event_data() {
    let (bus, _handle) = create_test_bus();

    let counter = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let captured_data = Arc::new(Mutex::new(String::new()));

    let counter_clone = counter.clone();
    let data_clone = captured_data.clone();

    // Create large data string (1MB)
    let large_data = "x".repeat(1024 * 1024);
    let large_data_clone = large_data.clone();

    // Register listener for large data test
    Arc::clone(&bus)
        .on("large.data", move |_, data| {
            counter_clone.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            let mut storage = data_clone.lock().unwrap();
            *storage = data.to_string();
        })
        .unwrap();

    // Give listener registration time to process
    std::thread::sleep(std::time::Duration::from_millis(100));

    // Publish event with large data
    Arc::clone(&bus)
        .publish_string("large.data", &large_data_clone)
        .unwrap();

    // Give event processing time (longer for large data)
    std::thread::sleep(std::time::Duration::from_millis(200));

    // Verify listener was called with correct large data
    assert_eq!(counter.load(std::sync::atomic::Ordering::SeqCst), 1);
    let captured = captured_data.lock().unwrap();
    assert_eq!(*captured, large_data);
    assert_eq!(captured.len(), 1024 * 1024);
}
