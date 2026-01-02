//! Event Payload Tests
//!
//! Tests for EventPayload enum variants and their proper handling
//! in both synchronous and asynchronous contexts.

use seesea_event::{EventPayload, StringAsyncEventOperations};
use std::sync::Arc;

#[test]
fn test_event_payload_data_variant() {
    // Test Data payload creation
    let data = vec![1, 2, 3, 4, 5];
    let payload = EventPayload::Data(data.clone());

    match payload {
        EventPayload::Data(received_data) => {
            assert_eq!(received_data, data);
        }
        _ => panic!("Expected Data payload"),
    }
}

#[test]
fn test_event_payload_error_variant() {
    // Test Error payload creation
    let error_msg = "Test error message".to_string();
    let payload = EventPayload::Error(error_msg.clone());

    match payload {
        EventPayload::Error(received_error) => {
            assert_eq!(received_error, error_msg);
        }
        _ => panic!("Expected Error payload"),
    }
}

#[test]
fn test_event_payload_empty_variant() {
    // Test Empty payload
    let payload = EventPayload::Empty;

    match payload {
        EventPayload::Empty => {
            // Expected empty payload
        }
        _ => panic!("Expected Empty payload"),
    }
}

#[test]
fn test_event_payload_data_with_string() {
    // Test Data payload with string data
    let test_string = "Hello, Event System!";
    let payload = EventPayload::Data(test_string.as_bytes().to_vec());

    match payload {
        EventPayload::Data(data) => {
            let received_string = String::from_utf8(data).unwrap();
            assert_eq!(received_string, test_string);
        }
        _ => panic!("Expected Data payload"),
    }
}

#[test]
fn test_event_payload_data_with_json() {
    // Test Data payload with JSON data
    let json_data = r#"{"key": "value", "number": 42}"#;
    let payload = EventPayload::Data(json_data.as_bytes().to_vec());

    match payload {
        EventPayload::Data(data) => {
            let received_json = String::from_utf8(data).unwrap();
            assert_eq!(received_json, json_data);
            // Verify it's valid JSON
            let parsed: serde_json::Value = serde_json::from_str(&received_json).unwrap();
            assert_eq!(parsed["key"], "value");
            assert_eq!(parsed["number"], 42);
        }
        _ => panic!("Expected Data payload"),
    }
}

#[test]
fn test_event_payload_data_with_binary() {
    // Test Data payload with binary data
    let binary_data = vec![0x00, 0xFF, 0x42, 0x13, 0x37];
    let payload = EventPayload::Data(binary_data.clone());

    match payload {
        EventPayload::Data(data) => {
            assert_eq!(data, binary_data);
            // Verify binary data is preserved correctly
            assert_eq!(data[0], 0x00);
            assert_eq!(data[1], 0xFF);
            assert_eq!(data[2], 0x42);
        }
        _ => panic!("Expected Data payload"),
    }
}

#[test]
fn test_event_payload_error_with_details() {
    // Test Error payload with detailed error information
    let error_type = "DatabaseError";
    let error_message = "Connection timeout after 30 seconds";
    let full_error = format!("{}: {}", error_type, error_message);

    let payload = EventPayload::Error(full_error.clone());

    match payload {
        EventPayload::Error(error) => {
            assert!(error.contains(error_type));
            assert!(error.contains(error_message));
            assert_eq!(error, full_error);
        }
        _ => panic!("Expected Error payload"),
    }
}

#[tokio::test]
async fn test_async_payload_handling() {
    // Create a dedicated test bus to avoid interference
    let bus = Arc::new(seesea_event::AsyncEventBus::new());

    // Start the event loop in background
    let bus_clone = Arc::clone(&bus);
    tokio::spawn(async move {
        let _ = bus_clone.run_event_loop().await;
    });

    // Give the event loop time to start
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Test Data payload in async context
    Arc::clone(&bus)
        .on("async.data.test", |_, data| {
            let data = data.to_string();
            Box::pin(async move {
                let expected_data = b"async test data";
                if data.as_bytes() == expected_data {
                    EventPayload::Data(b"data received".to_vec())
                } else {
                    EventPayload::Error("unexpected data".to_string())
                }
            })
        })
        .await
        .unwrap();

    // Give handler time to register
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let bus_clone = Arc::clone(&bus);
    let result = bus_clone
        .send_string_request("async.data.test", "async test data")
        .await;
    match result {
        Ok(EventPayload::Data(data)) => {
            assert_eq!(data, b"data received");
        }
        _ => panic!("Expected successful data response"),
    }
}

#[tokio::test]
async fn test_async_error_payload_handling() {
    // Create a dedicated test bus to avoid interference
    let bus = Arc::new(seesea_event::AsyncEventBus::new());

    // Start the event loop in background
    let bus_clone = Arc::clone(&bus);
    tokio::spawn(async move {
        let _ = bus_clone.run_event_loop().await;
    });

    // Give the event loop time to start
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Test Error payload in async context
    Arc::clone(&bus)
        .on("async.error.test", |_, data| {
            let data = data.to_string();
            Box::pin(async move {
                if data == "trigger_error" {
                    EventPayload::Error("async processing failed".to_string())
                } else {
                    EventPayload::Data(b"success".to_vec())
                }
            })
        })
        .await
        .unwrap();

    // Give handler time to register
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Test error case
    let bus_clone = Arc::clone(&bus);
    let result = bus_clone
        .send_string_request("async.error.test", "trigger_error")
        .await;
    match result {
        Ok(EventPayload::Error(error)) => {
            assert_eq!(error, "async processing failed");
        }
        _ => panic!("Expected error payload"),
    }
}

#[tokio::test]
async fn test_async_empty_payload_handling() {
    // Create a dedicated test bus to avoid interference
    let bus = Arc::new(seesea_event::AsyncEventBus::new());

    // Start the event loop in background
    let bus_clone = Arc::clone(&bus);
    tokio::spawn(async move {
        let _ = bus_clone.run_event_loop().await;
    });

    // Give the event loop time to start
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Test Empty payload in async context
    Arc::clone(&bus)
        .on("async.empty.test", |_, _| {
            Box::pin(async move { EventPayload::Empty })
        })
        .await
        .unwrap();

    // Give handler time to register
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let bus_clone = Arc::clone(&bus);
    let result = bus_clone
        .send_string_request("async.empty.test", "any data")
        .await;
    match result {
        Ok(EventPayload::Empty) => {
            // Expected empty payload
        }
        _ => panic!("Expected empty payload"),
    }
}

#[test]
fn test_payload_size_limits() {
    // Test with very large data payload
    let large_data = vec![0x42; 1024 * 1024]; // 1MB of data
    let payload = EventPayload::Data(large_data.clone());

    match payload {
        EventPayload::Data(data) => {
            assert_eq!(data.len(), 1024 * 1024);
            assert_eq!(data[0], 0x42);
            assert_eq!(data[data.len() - 1], 0x42);
        }
        _ => panic!("Expected Data payload"),
    }
}

#[test]
fn test_payload_clone_behavior() {
    // Test that payloads can be cloned properly
    let original_data = b"test data for cloning";
    let payload = EventPayload::Data(original_data.to_vec());

    let cloned_payload = payload.clone();

    match (payload, cloned_payload) {
        (EventPayload::Data(original), EventPayload::Data(cloned)) => {
            assert_eq!(original, cloned);
            assert_eq!(original, original_data.to_vec());
        }
        _ => panic!("Expected Data payloads"),
    }
}

#[test]
fn test_payload_debug_formatting() {
    // Test Debug formatting for payloads
    let data_payload = EventPayload::Data(b"test".to_vec());
    let error_payload = EventPayload::Error("test error".to_string());
    let empty_payload = EventPayload::Empty;

    let data_debug = format!("{:?}", data_payload);
    let error_debug = format!("{:?}", error_payload);
    let empty_debug = format!("{:?}", empty_payload);

    // Verify debug output contains expected information
    assert!(data_debug.contains("Data"));
    assert!(error_debug.contains("Error"));
    assert!(empty_debug.contains("Empty"));
}
