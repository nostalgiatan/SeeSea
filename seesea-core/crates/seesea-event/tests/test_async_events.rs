//! Asynchronous Event Tests
//!
//! Tests for the asynchronous event bus functionality, including
//! async event sending, request/response patterns, and concurrent operations.

use seesea_event::{EventPayload, StringAsyncEventOperations};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::timeout;

#[tokio::test]
async fn test_send_string_request() {
    // Create a dedicated test bus to avoid interference
    let bus = Arc::new(seesea_event::AsyncEventBus::new());

    // Start the event loop in background
    let bus_clone = Arc::clone(&bus);
    tokio::spawn(async move {
        let _ = bus_clone.run_event_loop().await;
    });

    // Give the event loop time to start
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Register a listener that returns data
    Arc::clone(&bus)
        .on("test.request", |event_type, data| {
            let event_type = event_type.to_string();
            let data = data.to_string();
            Box::pin(async move {
                assert_eq!(event_type, "test.request");
                assert_eq!(data, "request data");
                EventPayload::Data(b"response data".to_vec())
            })
        })
        .await
        .unwrap();

    // Give listener registration time to complete
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Send a request and wait for response
    let result = timeout(
        Duration::from_secs(5),
        bus.send_string_request("test.request", "request data"),
    )
    .await;

    assert!(result.is_ok(), "Request timed out");
    let payload = result.unwrap();

    match payload {
        Ok(EventPayload::Data(data)) => {
            assert_eq!(data, b"response data");
        }
        _ => panic!("Expected Data payload, got {:?}", payload),
    }
}

#[tokio::test]
async fn test_send_string_notification() {
    // Create a dedicated test bus to avoid interference
    let bus = Arc::new(seesea_event::AsyncEventBus::new());

    // Start the event loop in background
    let bus_clone = Arc::clone(&bus);
    tokio::spawn(async move {
        let _ = bus_clone.run_event_loop().await;
    });

    // Give the event loop time to start
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Counter to track notification delivery
    let counter = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let counter_clone = counter.clone();

    // Register a notification listener
    Arc::clone(&bus)
        .on("test.notification", move |event_type, data| {
            let event_type = event_type.to_string();
            let data = data.to_string();
            let counter = counter_clone.clone();
            Box::pin(async move {
                assert_eq!(event_type, "test.notification");
                assert_eq!(data, "notification data");
                counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                EventPayload::Empty
            })
        })
        .await
        .unwrap();

    // Give listener registration time to complete
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Send notification
    let result =
        Arc::clone(&bus).send_string_notification("test.notification", "notification data");
    assert!(result.is_ok(), "Failed to send notification: {:?}", result);

    // Give some time for async processing
    tokio::time::sleep(Duration::from_millis(200)).await;

    // Verify notification was received
    assert_eq!(counter.load(std::sync::atomic::Ordering::SeqCst), 1);
}

#[tokio::test]
async fn test_send_string_error() {
    // Create a dedicated test bus to avoid interference
    let bus = Arc::new(seesea_event::AsyncEventBus::new());

    // Start the event loop in background
    let bus_clone = Arc::clone(&bus);
    tokio::spawn(async move {
        let _ = bus_clone.run_event_loop().await;
    });

    // Give the event loop time to start
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Register an error handler
    Arc::clone(&bus)
        .on("test.error", |event_type, error_msg| {
            let event_type = event_type.to_string();
            let error_msg = error_msg.to_string();
            Box::pin(async move {
                assert_eq!(event_type, "test.error");
                assert_eq!(error_msg, "test error message");
                EventPayload::Error("error acknowledged".to_string())
            })
        })
        .await
        .unwrap();

    // Give handler time to register
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Send error
    let result = Arc::clone(&bus).send_string_error("test error message");
    assert!(result.is_ok(), "Failed to send error: {:?}", result);
}

#[tokio::test]
async fn test_async_request_response_pattern() {
    // Create a dedicated test bus to avoid interference
    let bus = Arc::new(seesea_event::AsyncEventBus::new());

    // Start the event loop in background
    let bus_clone = Arc::clone(&bus);
    tokio::spawn(async move {
        let _ = bus_clone.run_event_loop().await;
    });

    // Give the event loop time to start
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Register multiple handlers for different request types
    println!("Registering user.get handler");
    Arc::clone(&bus)
        .on("user.get", |event_type, user_id| {
            println!(
                "user.get handler called with event_type: {}, user_id: {}",
                event_type, user_id
            );
            let user_id = user_id.to_string();
            Box::pin(async move {
                // Simulate database lookup
                tokio::time::sleep(Duration::from_millis(50)).await;
                let result = EventPayload::Data(format!("user_data_for_{}", user_id).into_bytes());
                println!("user.get handler returning: user_data_for_{}", user_id);
                result
            })
        })
        .await
        .unwrap();

    println!("Registering product.get handler");
    Arc::clone(&bus)
        .on("product.get", |event_type, product_id| {
            println!(
                "product.get handler called with event_type: {}, product_id: {}",
                event_type, product_id
            );
            let product_id = product_id.to_string();
            Box::pin(async move {
                // Simulate different processing
                tokio::time::sleep(Duration::from_millis(30)).await;
                let result =
                    EventPayload::Data(format!("product_data_for_{}", product_id).into_bytes());
                println!(
                    "product.get handler returning: product_data_for_{}",
                    product_id
                );
                result
            })
        })
        .await
        .unwrap();

    // Give handlers time to register
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Send concurrent requests
    let bus_clone1 = Arc::clone(&bus);
    let bus_clone2 = Arc::clone(&bus);
    let user_future = bus_clone1.send_string_request("user.get", "123");
    let product_future = bus_clone2.send_string_request("product.get", "456");

    let (user_result, product_result) = tokio::join!(user_future, product_future);

    // Verify user data response
    match user_result {
        Ok(EventPayload::Data(data)) => {
            println!("User data: {:?}", String::from_utf8_lossy(&data));
            assert_eq!(data, b"user_data_for_123");
        }
        Ok(EventPayload::Empty) => {
            println!("User result is empty");
            panic!("Expected user data response");
        }
        Ok(EventPayload::Error(e)) => {
            println!("User result error payload: {:?}", e);
            panic!("Expected user data response");
        }
        Err(e) => {
            println!("User result error: {:?}", e);
            panic!("Expected user data response");
        }
    }

    // Verify product data response
    match product_result {
        Ok(EventPayload::Data(data)) => {
            println!("Product data: {:?}", String::from_utf8_lossy(&data));
            assert_eq!(data, b"product_data_for_456");
        }
        Ok(EventPayload::Empty) => {
            println!("Product result is empty");
            panic!("Expected product data response");
        }
        Ok(EventPayload::Error(e)) => {
            println!("Product result error payload: {:?}", e);
            panic!("Expected product data response");
        }
        Err(e) => {
            println!("Product result error: {:?}", e);
            panic!("Expected product data response");
        }
    }
}

#[tokio::test]
async fn test_async_concurrent_events() {
    // Create a dedicated test bus to avoid interference
    let bus = Arc::new(seesea_event::AsyncEventBus::new());

    // Start the event loop in background
    let bus_clone = Arc::clone(&bus);
    tokio::spawn(async move {
        let _ = bus_clone.run_event_loop().await;
    });

    // Give the event loop time to start
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Counter for concurrent event processing
    let counter = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let counter_clone = counter.clone();

    // Register a listener that simulates some processing time
    Arc::clone(&bus)
        .on("concurrent.test", move |_, data| {
            let counter = counter_clone.clone();
            let data = data.to_string();
            Box::pin(async move {
                // Simulate processing time based on data
                let delay = data.len() as u64 * 10;
                tokio::time::sleep(Duration::from_millis(delay)).await;
                counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                EventPayload::Data(format!("processed_{}", data).into_bytes())
            })
        })
        .await
        .unwrap();

    // Give handler time to register
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Send multiple concurrent requests
    let bus_clone1 = Arc::clone(&bus);
    let bus_clone2 = Arc::clone(&bus);
    let bus_clone3 = Arc::clone(&bus);
    let futures = vec![
        bus_clone1.send_string_request("concurrent.test", "short"),
        bus_clone2.send_string_request("concurrent.test", "medium_length"),
        bus_clone3.send_string_request("concurrent.test", "quite_long_data_string"),
    ];

    // Wait for all requests to complete
    let results = futures::future::join_all(futures.into_iter()).await;

    // Verify all requests succeeded
    assert_eq!(results.len(), 3);
    for result in &results {
        assert!(result.is_ok(), "Request failed: {:?}", result);
    }

    // Verify all events were processed
    assert_eq!(counter.load(std::sync::atomic::Ordering::SeqCst), 3);
}

#[tokio::test]
async fn test_async_error_handling() {
    // Create a dedicated test bus to avoid interference
    let bus = Arc::new(seesea_event::AsyncEventBus::new());

    // Start the event loop in background
    let bus_clone = Arc::clone(&bus);
    tokio::spawn(async move {
        let _ = bus_clone.run_event_loop().await;
    });

    // Give the event loop time to start
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Register a listener that returns an error
    Arc::clone(&bus)
        .on("error.test", |_, data| {
            let data = data.to_string();
            Box::pin(async move {
                if data == "trigger_error" {
                    EventPayload::Error("intentional error".to_string())
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
    let bus_clone1 = Arc::clone(&bus);
    let success_result = bus_clone1
        .send_string_request("error.test", "valid_data")
        .await;
    match success_result {
        Ok(EventPayload::Data(data)) => {
            assert_eq!(data, b"success");
        }
        _ => panic!("Expected successful data response"),
    }

    // Test error case
    let bus_clone2 = Arc::clone(&bus);
    let error_result = bus_clone2
        .send_string_request("error.test", "trigger_error")
        .await;
    match error_result {
        Ok(EventPayload::Error(msg)) => {
            assert_eq!(msg, "intentional error");
        }
        _ => panic!("Expected error response"),
    }
}

#[tokio::test]
async fn test_async_timeout_behavior() {
    // Create a dedicated test bus to avoid interference
    let bus = Arc::new(seesea_event::AsyncEventBus::new());

    // Start the event loop in background
    let bus_clone = Arc::clone(&bus);
    tokio::spawn(async move {
        let _ = bus_clone.run_event_loop().await;
    });

    // Give the event loop time to start
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Register a listener that takes longer than timeout
    Arc::clone(&bus)
        .on("timeout.test", |_, _| {
            Box::pin(async move {
                // This will cause a timeout in the request
                tokio::time::sleep(Duration::from_secs(10)).await;
                EventPayload::Data(b"too late".to_vec())
            })
        })
        .await
        .unwrap();

    // Give handler time to register
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Send request with short timeout
    let bus_clone = Arc::clone(&bus);
    let result = timeout(
        Duration::from_millis(100),
        bus_clone.send_string_request("timeout.test", "data"),
    )
    .await;

    // Verify timeout occurred
    assert!(result.is_err(), "Expected timeout, but request succeeded");
}

#[tokio::test]
async fn test_async_empty_payload() {
    // Create a dedicated test bus to avoid interference
    let bus = Arc::new(seesea_event::AsyncEventBus::new());

    // Start the event loop in background
    let bus_clone = Arc::clone(&bus);
    tokio::spawn(async move {
        let _ = bus_clone.run_event_loop().await;
    });

    // Give the event loop time to start
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Register a listener that returns empty payload
    Arc::clone(&bus)
        .on("empty.test", |_, _| {
            Box::pin(async move { EventPayload::Empty })
        })
        .await
        .unwrap();

    // Give handler time to register
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Send request that will get empty response
    let bus_clone = Arc::clone(&bus);
    let result = bus_clone
        .send_string_request("empty.test", "any_data")
        .await;
    println!("Empty test result: {:?}", result);
    match result {
        Ok(EventPayload::Empty) => {
            // Expected empty payload
        }
        Ok(EventPayload::Data(data)) => {
            println!(
                "Got data instead of empty: {:?}",
                String::from_utf8_lossy(&data)
            );
            panic!("Expected empty payload");
        }
        Ok(EventPayload::Error(e)) => {
            println!("Got error instead of empty: {:?}", e);
            panic!("Expected empty payload");
        }
        Err(e) => {
            println!("Got error result: {:?}", e);
            panic!("Expected empty payload");
        }
    }
}
