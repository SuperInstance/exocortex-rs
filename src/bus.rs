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
use std::sync::Arc;

/// Type alias for subscriber callbacks.
///
/// Subscribers are boxed `Fn` closures (held behind an `Arc` so they can be
/// cheaply shared and invoked from any context). This lets subscribers
/// capture external state — channels, atomics, shared cells — which a bare
/// `fn` pointer cannot do.
pub type Subscriber = Arc<dyn Fn(&CortexEvent) + Send + Sync>;

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
    ///
    /// Accepts any `Fn` closure (with optional captured state) that is
    /// `Send + Sync + 'static`. Plain `fn` pointers still work — they
    /// satisfy the bound trivially.
    pub fn subscribe<F>(&mut self, callback: F)
    where
        F: Fn(&CortexEvent) + Send + Sync + 'static,
    {
        self.subscribers.push(Arc::new(callback));
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

    /// Pop the single highest-priority event without dispatching it.
    ///
    /// Useful for callers that want to drain the bus in priority order under
    /// their own control, and for tests that need to assert on ordering
    /// without poking at private heap state.
    pub fn pop_next(&mut self) -> Option<CortexEvent> {
        self.queue.pop().map(|EventWrapper(event)| event)
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
            for sub in &self.subscribers {
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
                    for sub in &self.subscribers {
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
        // We compare BOTH components reversed so that:
        //   - higher importance (more negative i64) pops first, and
        //   - on equal importance, the earlier timestamp pops first (FIFO).
        // f64 only implements PartialOrd, so the timestamp leg uses
        // total_cmp to give a total ordering (required by Ord).
        let self_key = self.0.priority_key();
        let other_key = other.0.priority_key();
        other_key
            .0
            .cmp(&self_key.0)
            .then_with(|| other_key.1.total_cmp(&self_key.1))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    #[test]
    fn test_publish_and_dispatch() {
        let mut bus = CorticalBus::new();

        // Subscriber captures shared state — only possible now that the bus
        // accepts boxed closures instead of bare fn pointers.
        let received = Arc::new(AtomicUsize::new(0));
        let received_cb = Arc::clone(&received);
        bus.subscribe(move |_event: &CortexEvent| {
            received_cb.fetch_add(1, Ordering::SeqCst);
        });

        bus.publish(CortexEvent::new("test", "unit"));
        bus.publish(CortexEvent::new("test2", "unit"));

        assert_eq!(bus.pending(), 2);

        let dispatched = bus.dispatch_all();
        assert_eq!(dispatched, 2);
        assert_eq!(bus.pending(), 0);
        // Both events were fanned out to the subscriber.
        assert_eq!(received.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn test_dispatch_fans_out_to_all_subscribers() {
        let mut bus = CorticalBus::new();
        let seen_a = Arc::new(AtomicUsize::new(0));
        let seen_b = Arc::new(AtomicUsize::new(0));
        let seen_a_cb = Arc::clone(&seen_a);
        let seen_b_cb = Arc::clone(&seen_b);
        bus.subscribe(move |_e: &CortexEvent| {
            seen_a_cb.fetch_add(1, Ordering::SeqCst);
        });
        bus.subscribe(move |_e: &CortexEvent| {
            seen_b_cb.fetch_add(1, Ordering::SeqCst);
        });

        bus.publish(CortexEvent::new("x", "u"));
        bus.publish(CortexEvent::new("y", "u"));
        bus.publish(CortexEvent::new("z", "u"));

        assert_eq!(bus.dispatch_all(), 3);
        assert_eq!(seen_a.load(Ordering::SeqCst), 3);
        assert_eq!(seen_b.load(Ordering::SeqCst), 3);
    }

    #[test]
    fn test_subscriber_receives_event_payload() {
        let mut bus = CorticalBus::new();
        let captured = Arc::new(std::sync::Mutex::new(Vec::<String>::new()));
        let captured_cb = Arc::clone(&captured);
        bus.subscribe(move |event: &CortexEvent| {
            captured_cb.lock().unwrap().push(event.event_type.clone());
        });
        // Distinct importances so dispatch order is deterministic regardless
        // of timestamp collisions between tightly-spaced publish() calls.
        bus.publish(CortexEvent::new("alpha", "u").with_importance(0.9));
        bus.publish(CortexEvent::new("beta", "u").with_importance(0.1));
        bus.dispatch_all();

        let got = captured.lock().unwrap();
        assert_eq!(*got, vec!["alpha".to_string(), "beta".to_string()]);
    }

    #[test]
    fn test_equal_importance_drains_in_fifo_order() {
        // Regression: the priority_key timestamp tiebreaker must preserve
        // publish order for equal-importance events. An earlier version of
        // the Ord impl reversed only the importance leg and got FIFO wrong.
        let mut bus = CorticalBus::new();
        for i in 0..5u32 {
            let mut e = CortexEvent::new(&format!("e{i}"), "u").with_importance(0.5);
            // Pin timestamps to a strictly increasing sequence so the test
            // is independent of clock resolution.
            e.timestamp = 1_000_000.0 + i as f64;
            bus.publish(e);
        }

        let mut order = Vec::new();
        while let Some(e) = bus.pop_next() {
            order.push(e.event_type);
        }
        assert_eq!(order, vec!["e0", "e1", "e2", "e3", "e4"]);
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

        // Drain via the public pop_next API — high importance comes first.
        let mut dispatched_order = Vec::new();
        while let Some(event) = bus.pop_next() {
            dispatched_order.push(event.event_type);
        }

        assert_eq!(dispatched_order, vec!["high", "mid", "low"]);
    }

    #[test]
    fn fn_pointer_still_accepted_as_subscriber() {
        // Plain fn pointers must continue to work for backwards compat.
        fn count_event(event: &CortexEvent) {
            let _ = event.event_type.len();
        }
        let mut bus = CorticalBus::new();
        bus.subscribe(count_event);
        bus.publish(CortexEvent::new("ok", "u"));
        assert_eq!(bus.dispatch_all(), 1);
    }

    #[test]
    fn test_should_render_rate_limits_per_trace() {
        // should_render caps rendering at max_per_trace (default 5) events per
        // trace_id. Different trace_ids are tracked independently.
        let mut bus = CorticalBus::new();
        let trace_a = "trace-a-aaaa".to_string();
        let trace_b = "trace-b-bbbb".to_string();

        // First 5 of trace A should render; the 6th must be suppressed.
        for _ in 0..5 {
            let mut e = CortexEvent::new("predict", "u");
            e.trace_id = trace_a.clone();
            assert!(bus.should_render(&e), "expected render within cap");
        }
        let mut sixth = CortexEvent::new("predict", "u");
        sixth.trace_id = trace_a.clone();
        assert!(!bus.should_render(&sixth), "expected cap to be enforced");

        // Trace B has its own budget.
        let mut e_b = CortexEvent::new("predict", "u");
        e_b.trace_id = trace_b;
        assert!(bus.should_render(&e_b));
    }

    #[test]
    fn test_dispatch_some_bounds_dispatch_count() {
        let mut bus = CorticalBus::new();
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_cb = Arc::clone(&counter);
        bus.subscribe(move |_e: &CortexEvent| {
            counter_cb.fetch_add(1, Ordering::SeqCst);
        });
        for i in 0..10u32 {
            bus.publish(CortexEvent::new(&format!("e{i}"), "u"));
        }

        // dispatch_some must stop at the bound even when more are pending.
        assert_eq!(bus.dispatch_some(3), 3);
        assert_eq!(counter.load(Ordering::SeqCst), 3);
        assert_eq!(bus.pending(), 7);

        // dispatch_some with a bound larger than pending drains everything.
        assert_eq!(bus.dispatch_some(usize::MAX), 7);
        assert_eq!(bus.pending(), 0);
    }

    #[test]
    fn test_emit_shorthand_publishes() {
        let mut bus = CorticalBus::new();
        assert!(bus.emit("predict", "agent-1"));
        assert_eq!(bus.pending(), 1);
        assert_eq!(bus.pop_next().unwrap().event_type, "predict");
    }

    #[test]
    fn test_pop_next_on_empty_returns_none() {
        let mut bus = CorticalBus::new();
        assert!(bus.pop_next().is_none());
        assert!(bus.is_empty());
    }
}
