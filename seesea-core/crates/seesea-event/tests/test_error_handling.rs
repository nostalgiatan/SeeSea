//! Error Handling Tests
//!
//! Tests for error scenarios, edge cases, and error propagation
//! in both synchronous and asynchronous event systems.

use seesea_event::{EventBus, EventPayload, StringAsyncEventOperations, StringEventOperations};
use std::panic;
use std::sync::Arc;

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
fn test_sync_error_propagation() {
    let (bus, _handle) = create_test_bus();

    // Register a listener that panics
    Arc::clone(&bus)
        .on("panic.test", |_event_type, _data| {
            panic!("Intentional panic in listener");
        })
        .unwrap();

    // Give listener registration time to process
    std::thread::sleep(std::time::Duration::from_millis(100));

    // The publish should not panic, but the listener will fail internally
    let result = panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        Arc::clone(&bus)
            .publish_string("panic.test", "test data")
            .unwrap();
    }));

    // The publish operation itself should succeed
    assert!(result.is_ok(), "Publish operation should not panic");
}

#[test]
fn test_empty_event_type_handling() {
    let (bus, _handle) = create_test_bus();

    // Test with empty event type
    let counter = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let counter_clone = counter.clone();

    Arc::clone(&bus)
        .on("", move |_, _| {
            counter_clone.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        })
        .unwrap();

    // Give listener registration time to process
    std::thread::sleep(std::time::Duration::from_millis(100));

    Arc::clone(&bus)
        .publish_string("", "empty type data")
        .unwrap();

    // Give event processing time
    std::thread::sleep(std::time::Duration::from_millis(100));

    assert_eq!(counter.load(std::sync::atomic::Ordering::SeqCst), 1);
}

#[test]
fn test_special_characters_in_event_type() {
    let (bus, _handle) = create_test_bus();

    let counter = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let counter_clone = counter.clone();

    // Test with special characters in event type
    Arc::clone(&bus)
        .on("user:create/update", move |_, _| {
            counter_clone.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        })
        .unwrap();

    // Give listener registration time to process
    std::thread::sleep(std::time::Duration::from_millis(100));

    Arc::clone(&bus)
        .publish_string("user:create/update", "special chars data")
        .unwrap();

    // Give event processing time
    std::thread::sleep(std::time::Duration::from_millis(100));

    assert_eq!(counter.load(std::sync::atomic::Ordering::SeqCst), 1);
}

#[test]
fn test_unicode_event_data() {
    let (bus, _handle) = create_test_bus();

    let captured_data = Arc::new(std::sync::Mutex::new(String::new()));
    let data_clone = captured_data.clone();

    // Test with unicode characters
    Arc::clone(&bus)
        .on("unicode.test", move |_, data| {
            let mut storage = data_clone.lock().unwrap();
            *storage = data.to_string();
        })
        .unwrap();

    // Give listener registration time to process
    std::thread::sleep(std::time::Duration::from_millis(100));

    let unicode_data = "Hello 世界 🌍 こんにちは";
    Arc::clone(&bus)
        .publish_string("unicode.test", unicode_data)
        .unwrap();

    // Give event processing time
    std::thread::sleep(std::time::Duration::from_millis(100));

    let stored_data = captured_data.lock().unwrap();
    assert_eq!(*stored_data, unicode_data);
}

#[test]
fn test_very_long_event_type() {
    let (bus, _handle) = create_test_bus();

    let counter = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let counter_clone = counter.clone();

    // Create a very long event type
    let long_type = "a".repeat(1000);

    Arc::clone(&bus)
        .on(&long_type, move |_, _| {
            counter_clone.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        })
        .unwrap();

    // Give listener registration time to process
    std::thread::sleep(std::time::Duration::from_millis(100));

    Arc::clone(&bus)
        .publish_string(&long_type, "long type data")
        .unwrap();

    // Give event processing time
    std::thread::sleep(std::time::Duration::from_millis(100));

    assert_eq!(counter.load(std::sync::atomic::Ordering::SeqCst), 1);
}

#[tokio::test]
async fn test_async_error_propagation() {
    // Create a dedicated test bus to avoid interference
    let bus = Arc::new(seesea_event::AsyncEventBus::new());

    // Start the event loop in background
    let bus_clone = Arc::clone(&bus);
    tokio::spawn(async move {
        let _ = bus_clone.run_event_loop().await;
    });

    // Give the event loop time to start
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Register a listener that returns an error for specific input
    Arc::clone(&bus)
        .on("async.error.test", |_, data| {
            let data = data.to_string();
            Box::pin(async move {
                if data == "error_input" {
                    EventPayload::Error("async processing error".to_string())
                } else {
                    EventPayload::Data(b"success".to_vec())
                }
            })
        })
        .await
        .unwrap();

    // Give handler time to register
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Test successful case
    let bus_clone = Arc::clone(&bus);
    let success_result = bus_clone
        .send_string_request("async.error.test", "valid_input")
        .await;
    assert!(success_result.is_ok());

    // Test error case
    let bus_clone = Arc::clone(&bus);
    let error_result = bus_clone
        .send_string_request("async.error.test", "error_input")
        .await;
    assert!(error_result.is_ok()); // The request itself succeeds, but returns error payload

    match error_result {
        Ok(EventPayload::Error(msg)) => {
            assert_eq!(msg, "async processing error");
        }
        _ => panic!("Expected error payload"),
    }
}

#[tokio::test]
async fn test_async_listener_panic() {
    // Create a dedicated test bus to avoid interference
    let bus = Arc::new(seesea_event::AsyncEventBus::new());

    // Start the event loop in background
    let bus_clone = Arc::clone(&bus);
    tokio::spawn(async move {
        let _ = bus_clone.run_event_loop().await;
    });

    // Give the event loop time to start
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Register a listener that panics in async context
    Arc::clone(&bus)
        .on("async.panic.test", |_, _| {
            Box::pin(async move {
                panic!("Intentional panic in async listener");
            })
        })
        .await
        .unwrap();

    // Give handler time to register
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // The request should handle the panic gracefully
    let bus_clone = Arc::clone(&bus);
    let result = bus_clone
        .send_string_request("async.panic.test", "test data")
        .await;

    // The request might timeout or return an error payload
    // This test verifies the system doesn't crash on listener panics
    println!("Async panic test result: {:?}", result);
}

#[test]
fn test_malformed_utf8_handling() {
    let (bus, _handle) = create_test_bus();

    let captured_data = Arc::new(std::sync::Mutex::new(Vec::new()));
    let data_clone = captured_data.clone();

    // Test with binary data that might not be valid UTF-8
    Arc::clone(&bus)
        .on("binary.test", move |_, data| {
            let mut storage = data_clone.lock().unwrap();
            // Store as bytes since we can't guarantee UTF-8 validity
            storage.extend_from_slice(data.as_bytes());
        })
        .unwrap();

    // Give listener registration time to process
    std::thread::sleep(std::time::Duration::from_millis(100));

    // Send some potentially problematic data
    let test_data = "test data with some bytes: \x00\x01\x02";
    Arc::clone(&bus)
        .publish_string("binary.test", test_data)
        .unwrap();

    // Give event processing time
    std::thread::sleep(std::time::Duration::from_millis(100));

    let stored_data = captured_data.lock().unwrap();
    assert_eq!(stored_data.len(), test_data.len());
}

#[test]
fn test_concurrent_error_scenarios() {
    use std::sync::Barrier;
    use std::thread;

    let (bus, _handle) = create_test_bus();
    let barrier = Arc::new(Barrier::new(10));
    let error_counter = Arc::new(std::sync::atomic::AtomicUsize::new(0));

    let mut handles = vec![];

    for i in 0..10 {
        let bus_clone = Arc::clone(&bus);
        let barrier_clone = barrier.clone();
        let error_counter_clone = error_counter.clone();

        let handle = thread::spawn(move || {
            // Wait for all threads to be ready
            barrier_clone.wait();

            // Try to register listeners concurrently
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                Arc::clone(&bus_clone)
                    .on(&format!("concurrent.error.{}", i), move |_, _| {
                        // This might cause issues if not thread-safe
                        error_counter_clone.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    })
                    .unwrap();
            }));

            result.is_ok()
        });

        handles.push(handle);
    }

    // Wait for all threads to complete
    let results: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();

    // Give listener registration time to process
    std::thread::sleep(std::time::Duration::from_millis(100));

    // All registrations should succeed
    assert!(
        results.iter().all(|&x| x),
        "Some concurrent registrations failed"
    );
}

#[test]
fn test_event_type_collision_handling() {
    let (bus, _handle) = create_test_bus();
    let counter = Arc::new(std::sync::atomic::AtomicUsize::new(0));

    // Register multiple listeners with the same event type
    for i in 0..5 {
        let counter_clone = counter.clone();
        Arc::clone(&bus)
            .on("collision.test", move |_, data| {
                counter_clone.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                println!("Listener {} received: {}", i, data);
            })
            .unwrap();
    }

    // Give listener registration time to process
    std::thread::sleep(std::time::Duration::from_millis(100));

    // Publish to the colliding event type
    Arc::clone(&bus)
        .publish_string("collision.test", "collision data")
        .unwrap();

    // Give event processing time
    std::thread::sleep(std::time::Duration::from_millis(100));

    // All listeners should be called
    assert_eq!(counter.load(std::sync::atomic::Ordering::SeqCst), 5);
}

#[test]
fn test_zero_length_data_handling() {
    let (bus, _handle) = create_test_bus();

    let counter = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let data_length = Arc::new(std::sync::Mutex::new(Vec::new()));

    let counter_clone = counter.clone();
    let length_clone = data_length.clone();

    let _ = Arc::clone(&bus).on("zero.length.test", move |_, data| {
        counter_clone.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let mut lengths = length_clone.lock().unwrap();
        lengths.push(data.len());
    });

    // Give listener registration time to process
    std::thread::sleep(std::time::Duration::from_millis(100));

    // Test various zero-length scenarios
    Arc::clone(&bus)
        .publish_string("zero.length.test", "")
        .unwrap();
    Arc::clone(&bus)
        .publish_string("zero.length.test", "")
        .unwrap();

    // Give event processing time
    std::thread::sleep(std::time::Duration::from_millis(100));

    let lengths = data_length.lock().unwrap();
    assert_eq!(counter.load(std::sync::atomic::Ordering::SeqCst), 2);
    assert_eq!(lengths.len(), 2);
    assert_eq!(lengths[0], 0);
    assert_eq!(lengths[1], 0);
}
