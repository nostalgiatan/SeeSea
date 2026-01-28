//! Simple Sync Test
//!
//! Test to verify basic sync event functionality

use seesea_event::{EventBus, StringEventOperations};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

#[test]
fn test_simple_sync_event() {
    let bus = Arc::new(EventBus::new());

    let counter = Arc::new(AtomicUsize::new(0));
    let counter_clone = counter.clone();

    // Start event loop in background
    let bus_clone = Arc::clone(&bus);
    let _handle = std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async { bus_clone.run().await })
    });

    // Give event loop time to start
    std::thread::sleep(std::time::Duration::from_millis(100));

    // Register listener
    Arc::clone(&bus)
        .on("simple.test", move |_, _| {
            counter_clone.fetch_add(1, Ordering::SeqCst);
        })
        .unwrap();

    // Give listener registration time to process
    std::thread::sleep(std::time::Duration::from_millis(100));

    // Publish event
    Arc::clone(&bus)
        .publish_string("simple.test", "test data")
        .unwrap();

    // Give event processing time
    std::thread::sleep(std::time::Duration::from_millis(100));

    // Check if event was processed
    assert_eq!(counter.load(Ordering::SeqCst), 1, "Event was not processed");
}
