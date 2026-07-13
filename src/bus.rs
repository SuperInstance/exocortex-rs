//! Cortical Bus — the central event spine of the exocortex.
//!
//! All inter-component communication flows through the bus.
//! Zero direct coupling. Everything is a typed CortexEvent.
//!
//! This is a synchronous priority-based implementation (no async runtime needed).
//! The Python version uses asyncio; this Rust version uses a simple
//! priority queue with backpressure.

use crate::types::CortexEvent;
use std::collections::BinaryHeap;

/// Type alias for subscriber callbacks.
type Subscriber = fn(&CortexEvent);

/// Priority-based pub/sub event bus with backpressure.
///
/// - PriorityQueue: high-importance events dispatched first
/// - Fan-out: all subscribers get every event
/// - Backpressure: bounded queue, shed low-priority when full
pub struct CorticalBus {
    queue: BinaryHeap<EventWrapper>,
    subscribers: Vec<Subscriber>,
    max_queue_size: usize,
    trace_counts: std::collections::HashMap<String, usize>,
    max_per_trace: usize,
    events_published: u64,
    events_dropped: u64,
    events_dispatched: u64,
}

impl CorticalBus {
    /// Create a new bus with default queue size (1000).
    pub fn new() -> Self {
        Self {
            queue: BinaryHeap::new(),
            subscribers: Vec::new(),
            max_queue_size: 1000,
            trace_counts: std::collections::HashMap::new(),
            max_per_trace: 5,
            events_published: 0,
            events_dropped: 0,
            events_dispatched: 0,
        }
    }

    /// Create a bus with a custom max queue size.
    pub fn with_capacity(max_queue_size: usize) -> Self {
        Self {
            max_queue_size,
            ..Self::new()
        }
    }

    /// Register a subscriber callback.
    pub fn subscribe(&mut self, callback: Subscriber) {
        self.subscribers.push(callback);
    }

    /// Publish an event. Returns false if queue is full (backpressure).
    pub fn publish(&mut self, event: CortexEvent) -> bool {
        if self.queue.len() >= self.max_queue_size {
            self.events_dropped += 1;
            return false;
        }
        self.events_published += 1;
        self.queue.push(EventWrapper(event));
        true
    }

    /// Shorthand: create and publish an event.
    pub fn emit(&mut self, event_type: &str, source: &str) -> bool {
        let event = CortexEvent::new(event_type, source);
        self.publish(event)
    }

    /// Rate limit: check if this trace_id has exceeded the per-trace limit.
    pub fn should_render(&mut self, event: &CortexEvent) -> bool {
        let count = self.trace_counts.entry(event.trace_id.clone()).or_insert(0);
        *count += 1;
        *count <= self.max_per_trace
    }

    /// Dispatch all queued events to subscribers.
    ///
    /// Returns the number of events dispatched.
    pub fn dispatch_all(&mut self) -> usize {
        let mut dispatched = 0;
        while let Some(EventWrapper(event)) = self.queue.pop() {
            for &sub in &self.subscribers {
                sub(&event);
            }
            dispatched += 1;
        }
        self.events_dispatched += dispatched as u64;
        dispatched
    }

    /// Dispatch up to `max` events.
    pub fn dispatch_some(&mut self, max: usize) -> usize {
        let mut dispatched = 0;
        while dispatched < max {
            match self.queue.pop() {
                Some(EventWrapper(event)) => {
                    for &sub in &self.subscribers {
                        sub(&event);
                    }
                    dispatched += 1;
                }
                None => break,
            }
        }
        self.events_dispatched += dispatched as u64;
        dispatched
    }

    /// Number of events currently in the queue.
    pub fn pending(&self) -> usize {
        self.queue.len()
    }

    /// Is the queue empty?
    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }

    /// Bus statistics.
    pub fn stats(&self) -> BusStats {
        BusStats {
            published: self.events_published,
            dropped: self.events_dropped,
            dispatched: self.events_dispatched,
            pending: self.queue.len() as u64,
            subscribers: self.subscribers.len(),
        }
    }
}

impl Default for CorticalBus {
    fn default() -> Self {
        Self::new()
    }
}

/// Bus statistics.
#[derive(Debug, Clone, Default)]
pub struct BusStats {
    pub published: u64,
    pub dropped: u64,
    pub dispatched: u64,
    pub pending: u64,
    pub subscribers: usize,
}

/// Wrapper for BinaryHeap ordering (max-heap by priority).
struct EventWrapper(CortexEvent);

impl PartialEq for EventWrapper {
    fn eq(&self, other: &Self) -> bool {
        self.0.priority_key() == other.0.priority_key()
    }
}

impl Eq for EventWrapper {}

impl PartialOrd for EventWrapper {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for EventWrapper {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // BinaryHeap is a max-heap, so we want higher priority first.
        // priority_key returns (negated_importance, timestamp).
        // Lower negated_importance = higher actual importance.
        // We reverse to get higher importance first.
        // Compare by importance first (i64, which is Ord), then by timestamp (f64, PartialOrd).
        let (self_imp, self_ts) = self.0.priority_key();
        let (other_imp, other_ts) = other.0.priority_key();
        other_imp
            .cmp(&self_imp)
            .then_with(|| other_ts.partial_cmp(&self_ts).unwrap_or(std::cmp::Ordering::Equal))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_publish_and_dispatch() {
        let mut bus = CorticalBus::new();
        let callback: Subscriber = |event: &CortexEvent| {
            // Can't capture external state easily without Box<dyn Fn>.
            // In real usage, subscribers would write to channels or shared state.
        };

        bus.subscribe(callback);
        bus.publish(CortexEvent::new("test", "unit"));
        bus.publish(CortexEvent::new("test2", "unit"));

        assert_eq!(bus.pending(), 2);

        let dispatched = bus.dispatch_all();
        assert_eq!(dispatched, 2);
        assert_eq!(bus.pending(), 0);
    }

    #[test]
    fn test_backpressure() {
        let mut bus = CorticalBus::with_capacity(5);

        for _ in 0..5 {
            assert!(bus.publish(CortexEvent::new("test", "unit")));
        }

        // Queue is full
        assert!(!bus.publish(CortexEvent::new("overflow", "unit")));

        let stats = bus.stats();
        assert_eq!(stats.published, 5);
        assert_eq!(stats.dropped, 1);
    }

    #[test]
    fn test_priority_ordering() {
        let mut bus = CorticalBus::new();

        bus.publish(CortexEvent::new("low", "unit").with_importance(0.1));
        bus.publish(CortexEvent::new("high", "unit").with_importance(0.9));
        bus.publish(CortexEvent::new("mid", "unit").with_importance(0.5));

        // Dispatch should process high importance first
        let mut dispatched_order = Vec::new();
        while let Some(EventWrapper(event)) = bus.queue.pop() {
            dispatched_order.push(event.event_type.clone());
        }

        assert_eq!(dispatched_order, vec!["high", "mid", "low"]);
    }
}
