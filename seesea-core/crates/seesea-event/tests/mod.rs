//! SeeSea Event System Tests
//!
//! This module contains comprehensive tests for the event system,
//! including synchronous events, asynchronous events, error handling,
//! and integration tests.

pub mod test_async_events;
pub mod test_error_handling;
pub mod test_event_payloads;
pub mod test_sync_events;
pub mod test_utils;

use seesea_event::EventPayload;

/// Common test utilities and helpers
pub mod utils {
    use super::*;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    /// Test event counter for verifying event delivery
    pub struct EventCounter {
        count: AtomicUsize,
    }

    impl EventCounter {
        pub fn new() -> Arc<Self> {
            Arc::new(Self {
                count: AtomicUsize::new(0),
            })
        }

        pub fn increment(&self) {
            self.count.fetch_add(1, Ordering::SeqCst);
        }

        pub fn get(&self) -> usize {
            self.count.load(Ordering::SeqCst)
        }

        pub fn reset(&self) {
            self.count.store(0, Ordering::SeqCst);
        }
    }

    /// Create a test event listener that increments a counter
    pub fn create_counting_listener(
        counter: Arc<EventCounter>,
    ) -> impl Fn(&str, &str) + Send + Sync + 'static {
        move |_event_type, _data| {
            counter.increment();
        }
    }

    /// Create a test async event listener that increments a counter
    pub fn create_async_counting_listener(
        counter: Arc<EventCounter>,
    ) -> impl Fn(
        &str,
        &str,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = EventPayload> + Send>>
    + Send
    + Sync
    + 'static {
        move |_event_type, _data| {
            let counter = counter.clone();
            Box::pin(async move {
                counter.increment();
                EventPayload::Empty
            })
        }
    }

    /// Test timeout duration for async operations
    pub const TEST_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);
}
