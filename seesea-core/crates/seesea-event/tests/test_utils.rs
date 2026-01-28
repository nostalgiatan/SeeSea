//! Test utilities for event system testing

use seesea_event::{EventBus, get_global_event_bus};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::timeout;

/// Start the event bus event loop in a background task
pub fn start_event_loop() {
    let bus = get_global_event_bus();

    // Spawn a background task to run the event loop
    tokio::spawn(async move {
        // Clone the bus to move into the async block
        let bus_clone = bus.clone();

        // Run the event loop
        if let Err(e) = bus_clone.run().await {
            eprintln!("Event loop error: {}", e);
        }
    });
}

/// Wait for events to be processed with a timeout
pub async fn wait_for_events(
    timeout_duration: Duration,
) -> Result<(), tokio::time::error::Elapsed> {
    timeout(timeout_duration, async {
        // Give the event loop time to process events
        tokio::time::sleep(Duration::from_millis(100)).await;
    })
    .await
}

/// Create a test event bus with a running event loop
pub fn create_test_event_bus() -> Arc<EventBus> {
    let bus = Arc::new(EventBus::new());

    // Start the event loop
    let bus_clone = bus.clone();
    tokio::spawn(async move {
        if let Err(e) = bus_clone.run().await {
            eprintln!("Test event loop error: {}", e);
        }
    });

    bus
}
