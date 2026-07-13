//! Message types and routing — inter-agent communication.

use crate::types::current_time;
use crate::types::Operation;

/// Message types for inter-agent communication.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum MessageType {
    /// Direct point-to-point message.
    #[default]
    Direct,
    /// Broadcast to all agents.
    Broadcast,
    /// Query — expects a response.
    Query,
    /// Response to a query.
    Response,
    /// Event notification (fire and forget).
    Event,
    /// Conservation enforcement (system-level).
    Conservation,
}

impl MessageType {
    pub fn as_str(&self) -> &'static str {
        match self {
            MessageType::Direct => "direct",
            MessageType::Broadcast => "broadcast",
            MessageType::Query => "query",
            MessageType::Response => "response",
            MessageType::Event => "event",
            MessageType::Conservation => "conservation",
        }
    }
}

/// A message passed between agents.
#[derive(Debug, Clone)]
pub struct Message {
    /// Sender agent ID (stamped by AgentSpace).
    pub from: String,
    /// Receiver agent ID (stamped by AgentSpace).
    pub to: String,
    /// Message type.
    pub message_type: MessageType,
    /// Content payload (free-form text).
    pub content: String,
    /// Associated operation (if relevant).
    pub operation: Option<Operation>,
    /// Embedding data (for vector messages).
    pub embedding: Vec<f64>,
    /// Tags for categorization.
    pub tags: Vec<String>,
    /// Priority 0..1 (higher = more important).
    pub priority: f64,
    /// Timestamp.
    pub timestamp: f64,
    /// Trace ID for request tracking.
    pub trace_id: String,
}

impl Message {
    /// Create a remember message.
    pub fn remember(content: &str) -> Self {
        Self {
            from: String::new(),
            to: String::new(),
            message_type: MessageType::Direct,
            content: content.to_string(),
            operation: Some(Operation::Remember),
            embedding: Vec::new(),
            tags: Vec::new(),
            priority: 0.5,
            timestamp: current_time(),
            trace_id: crate::types::generate_trace_id(),
        }
    }

    /// Create a recall message.
    pub fn recall(query: &str) -> Self {
        Self {
            from: String::new(),
            to: String::new(),
            message_type: MessageType::Query,
            content: query.to_string(),
            operation: Some(Operation::Recall),
            embedding: Vec::new(),
            tags: Vec::new(),
            priority: 0.5,
            timestamp: current_time(),
            trace_id: crate::types::generate_trace_id(),
        }
    }

    /// Create a predict message.
    pub fn predict(input: Vec<f64>) -> Self {
        Self {
            from: String::new(),
            to: String::new(),
            message_type: MessageType::Query,
            content: String::new(),
            operation: Some(Operation::Predict),
            embedding: input,
            tags: Vec::new(),
            priority: 0.6,
            timestamp: current_time(),
            trace_id: crate::types::generate_trace_id(),
        }
    }

    /// Create a custom message.
    pub fn new(content: &str) -> Self {
        Self {
            from: String::new(),
            to: String::new(),
            message_type: MessageType::Direct,
            content: content.to_string(),
            operation: None,
            embedding: Vec::new(),
            tags: Vec::new(),
            priority: 0.5,
            timestamp: current_time(),
            trace_id: crate::types::generate_trace_id(),
        }
    }

    /// Set the message type.
    pub fn with_type(mut self, msg_type: MessageType) -> Self {
        self.message_type = msg_type;
        self
    }

    /// Set the priority.
    pub fn with_priority(mut self, priority: f64) -> Self {
        self.priority = priority;
        self
    }

    /// Add tags.
    pub fn with_tags(mut self, tags: &[&str]) -> Self {
        self.tags = tags.iter().map(|s| s.to_string()).collect();
        self
    }

    /// Check if this is a query expecting a response.
    pub fn expects_response(&self) -> bool {
        matches!(self.message_type, MessageType::Query)
    }
}

/// A bounded message queue for an agent.
pub struct MessageQueue {
    messages: std::collections::VecDeque<Message>,
    max_size: usize,
}

impl MessageQueue {
    pub fn new() -> Self {
        Self {
            messages: std::collections::VecDeque::with_capacity(64),
            max_size: 1024,
        }
    }

    pub fn with_capacity(max_size: usize) -> Self {
        Self {
            messages: std::collections::VecDeque::with_capacity(max_size.min(64)),
            max_size,
        }
    }

    /// Push a message to the back of the queue.
    /// Returns false if the queue is full (backpressure).
    pub fn push(&mut self, message: Message) -> bool {
        if self.messages.len() >= self.max_size {
            // Backpressure: drop lowest priority message
            if let Some(min_idx) = self.find_lowest_priority() {
                self.messages.remove(min_idx);
            } else {
                return false;
            }
        }
        self.messages.push_back(message);
        true
    }

    /// Pop the highest priority message from the queue.
    pub fn pop(&mut self) -> Option<Message> {
        if self.messages.is_empty() {
            return None;
        }

        // Find highest priority message
        let mut best_idx = 0;
        let mut best_priority = self.messages[0].priority;
        for (i, msg) in self.messages.iter().enumerate().skip(1) {
            if msg.priority > best_priority {
                best_priority = msg.priority;
                best_idx = i;
            }
        }

        self.messages.remove(best_idx)
    }

    /// Peek at the highest priority message without removing it.
    pub fn peek(&self) -> Option<&Message> {
        self.messages.iter().max_by(|a, b| {
            a.priority
                .partial_cmp(&b.priority)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
    }

    /// Number of messages in the queue.
    pub fn len(&self) -> usize {
        self.messages.len()
    }

    /// Is the queue empty?
    pub fn is_empty(&self) -> bool {
        self.messages.is_empty()
    }

    /// Clear all messages.
    pub fn clear(&mut self) {
        self.messages.clear();
    }

    fn find_lowest_priority(&self) -> Option<usize> {
        self.messages
            .iter()
            .enumerate()
            .min_by(|(_, a), (_, b)| {
                a.priority
                    .partial_cmp(&b.priority)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|(i, _)| i)
    }
}

impl Default for MessageQueue {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_remember() {
        let msg = Message::remember("hello world");
        assert_eq!(msg.content, "hello world");
        assert_eq!(msg.operation, Some(Operation::Remember));
    }

    #[test]
    fn test_message_priority_queue() {
        let mut q = MessageQueue::new();

        q.push(Message::new("low").with_priority(0.1));
        q.push(Message::new("high").with_priority(0.9));
        q.push(Message::new("mid").with_priority(0.5));

        let first = q.pop().unwrap();
        assert_eq!(first.content, "high");

        let second = q.pop().unwrap();
        assert_eq!(second.content, "mid");

        let third = q.pop().unwrap();
        assert_eq!(third.content, "low");
    }

    #[test]
    fn test_queue_backpressure() {
        let mut q = MessageQueue::with_capacity(3);

        assert!(q.push(Message::new("1").with_priority(0.1)));
        assert!(q.push(Message::new("2").with_priority(0.5)));
        assert!(q.push(Message::new("3").with_priority(0.9)));

        // Queue is full, but we can still push (drops lowest priority)
        assert!(q.push(Message::new("4").with_priority(0.7)));

        // "1" (lowest priority) should have been dropped
        let first = q.pop().unwrap();
        assert_eq!(first.content, "3"); // priority 0.9
    }

    #[test]
    fn test_message_expects_response() {
        assert!(Message::recall("test").expects_response());
        assert!(!Message::remember("test").expects_response());
    }

    #[test]
    fn test_peek_returns_highest_priority_without_removing() {
        let mut q = MessageQueue::new();
        assert!(q.peek().is_none());

        q.push(Message::new("low").with_priority(0.1));
        q.push(Message::new("high").with_priority(0.9));
        q.push(Message::new("mid").with_priority(0.5));

        // peek() must surface the highest-priority message without draining.
        assert_eq!(q.peek().unwrap().content, "high");
        assert_eq!(q.len(), 3);

        // Repeated peeks are stable.
        assert_eq!(q.peek().unwrap().content, "high");
        assert_eq!(q.len(), 3);
    }

    #[test]
    fn test_clear_empties_queue() {
        let mut q = MessageQueue::new();
        q.push(Message::new("a"));
        q.push(Message::new("b"));
        assert_eq!(q.len(), 2);

        q.clear();
        assert!(q.is_empty());
        assert_eq!(q.len(), 0);
        assert!(q.pop().is_none());
    }

    #[test]
    fn test_with_capacity_zero_drops_everything() {
        // A zero-capacity queue cannot accept anything — every push must
        // find a lower-priority message to evict, and when the queue is
        // empty there is nothing to evict, so push returns false.
        let mut q = MessageQueue::with_capacity(0);
        assert!(!q.push(Message::new("first")));
        assert!(q.is_empty());
    }

    #[test]
    fn test_with_capacity_one_evicts_on_push() {
        let mut q = MessageQueue::with_capacity(1);
        assert!(q.push(Message::new("low").with_priority(0.1)));
        // Full — the new message is higher priority, so the old one is evicted.
        assert!(q.push(Message::new("high").with_priority(0.9)));
        assert_eq!(q.len(), 1);
        assert_eq!(q.pop().unwrap().content, "high");
    }
}
