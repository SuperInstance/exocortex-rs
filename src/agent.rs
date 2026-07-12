//! Agent lifecycle — autonomous entities with capabilities, state, and message queue.

use crate::conservation::ConservationState;
use crate::message::{Message, MessageQueue};
use crate::types::*;
use crate::memory::MemoryStore;

/// Agent state — what an agent currently knows and feels.
#[derive(Debug, Clone, Default)]
pub struct AgentState {
    /// Current attention focus (what the agent is "looking at").
    pub focus: Option<String>,
    /// Current energy level (0..1).
    pub energy: f64,
    /// Current confidence in its world model (0..1).
    pub world_confidence: f64,
    /// Number of decisions made.
    pub decisions_made: u64,
    /// Number of messages sent.
    pub messages_sent: u64,
    /// Number of messages received.
    pub messages_received: u64,
}

impl AgentState {
    pub fn new() -> Self {
        Self {
            focus: None,
            energy: 1.0,
            world_confidence: 0.5,
            decisions_made: 0,
            messages_sent: 0,
            messages_received: 0,
        }
    }
}

/// Builder for creating agents with a fluent API.
pub struct AgentBuilder {
    id: String,
    capabilities: Vec<Operation>,
    protocol: Protocol,
    state: AgentState,
    conservation: ConservationState,
    memory: MemoryStore,
}

impl AgentBuilder {
    pub fn new(id: &str) -> Self {
        Self {
            id: id.to_string(),
            capabilities: Vec::new(),
            protocol: Protocol::Rest,
            state: AgentState::new(),
            conservation: ConservationState::new(),
            memory: MemoryStore::new(),
        }
    }

    pub fn capability(mut self, op: Operation) -> Self {
        if !self.capabilities.contains(&op) {
            self.capabilities.push(op);
        }
        self
    }

    pub fn capabilities(mut self, ops: &[Operation]) -> Self {
        for &op in ops {
            if !self.capabilities.contains(&op) {
                self.capabilities.push(op);
            }
        }
        self
    }

    pub fn protocol(mut self, protocol: Protocol) -> Self {
        self.protocol = protocol;
        self
    }

    pub fn energy(mut self, energy: f64) -> Self {
        self.state.energy = energy;
        self
    }

    pub fn world_confidence(mut self, conf: f64) -> Self {
        self.state.world_confidence = conf;
        self
    }

    pub fn energy_budget(mut self, budget: f64) -> Self {
        self.conservation = ConservationState::with_budget(budget);
        self
    }

    pub fn identity(mut self, vector: Vec<f64>) -> Self {
        self.conservation = ConservationState::with_identity(vector);
        self
    }

    pub fn build(self) -> Agent {
        Agent {
            id: self.id,
            capabilities: self.capabilities,
            protocol: self.protocol,
            state: self.state,
            conservation: self.conservation,
            memory: self.memory,
            message_queue: MessageQueue::new(),
        }
    }
}

/// An autonomous agent in the exocortex.
///
/// Agents have:
/// - **Capabilities** — the operations they can perform
/// - **State** — current focus, energy, confidence
/// - **Conservation** — five conservation laws governing decisions
/// - **Memory** — local memory store (hot/warm/cold tiers)
/// - **Message Queue** — pending messages from other agents
pub struct Agent {
    pub id: String,
    pub capabilities: Vec<Operation>,
    pub protocol: Protocol,
    pub state: AgentState,
    pub conservation: ConservationState,
    pub memory: MemoryStore,
    pub message_queue: MessageQueue,
}

impl Agent {
    /// Create a builder for a new agent.
    pub fn builder(id: &str) -> AgentBuilder {
        AgentBuilder::new(id)
    }

    /// Create a minimal agent with no capabilities.
    pub fn new(id: &str) -> Self {
        AgentBuilder::new(id).build()
    }

    /// Check if this agent has a given capability.
    pub fn has_capability(&self, op: Operation) -> bool {
        self.capabilities.contains(&op)
    }

    /// Attempt to perform an operation, respecting conservation laws.
    ///
    /// Returns the outcome or an error explaining which conservation law was violated.
    pub fn decide(&mut self, op: Operation, energy_cost: f64, novelty_cost: f64) -> DecisionResult {
        // Check capability
        if !self.has_capability(op) {
            return DecisionResult::Denied(DecisionDenialReason::MissingCapability(op));
        }

        // Check energy conservation
        if !self.conservation.can_afford(energy_cost) {
            return DecisionResult::Denied(DecisionDenialReason::EnergyExhausted);
        }

        // Check novelty (entropy conservation)
        if !self.conservation.check_novelty(novelty_cost) {
            return DecisionResult::Denied(DecisionDenialReason::NoveltyExhausted);
        }

        // Spend resources
        self.conservation.spend(energy_cost);
        self.conservation.spend_novelty(novelty_cost);

        // Update state
        self.state.decisions_made += 1;
        self.state.energy -= energy_cost / self.conservation.energy_budget;
        if self.state.energy < 0.0 {
            self.state.energy = 0.0;
        }

        DecisionResult::Approved(Decision {
            operation: op,
            energy_cost,
            novelty_cost,
            agent_id: self.id.clone(),
        })
    }

    /// Reset the agent's conservation cycle (called periodically).
    pub fn reset_cycle(&mut self) {
        self.conservation.reset_cycle();
    }

    /// Receive a message from another agent.
    pub fn receive(&mut self, message: Message) {
        self.state.messages_received += 1;
        self.message_queue.push(message);
    }

    /// Process the next message in the queue.
    pub fn process_message(&mut self) -> Option<Message> {
        let msg = self.message_queue.pop()?;
        Some(msg)
    }

    /// Remember something.
    pub fn remember(&mut self, content: &str, embedding: Vec<f64>, tags: &[&str]) -> String {
        self.memory.remember(content, embedding, &self.id, tags)
    }

    /// Recall similar memories.
    pub fn recall(&self, query_embedding: &[f64], top_k: usize) -> Vec<(&MemoryEntry, f64)> {
        self.memory.recall(query_embedding, top_k)
    }

    /// Get agent info summary.
    pub fn info(&self) -> AgentInfo {
        AgentInfo {
            agent_id: self.id.clone(),
            protocol: self.protocol,
            capabilities: self.capabilities.clone(),
            last_seen: current_time(),
        }
    }

    /// Get conservation summary.
    pub fn conservation_summary(&self) -> crate::conservation::ConservationSummary {
        self.conservation.summary()
    }

    /// Number of pending messages.
    pub fn pending_messages(&self) -> usize {
        self.message_queue.len()
    }
}

/// Result of a decision attempt.
#[derive(Debug, Clone)]
pub enum DecisionResult {
    /// Decision approved — the agent may proceed.
    Approved(Decision),
    /// Decision denied — a conservation law was violated.
    Denied(DecisionDenialReason),
}

/// A successful decision.
#[derive(Debug, Clone)]
pub struct Decision {
    pub operation: Operation,
    pub energy_cost: f64,
    pub novelty_cost: f64,
    pub agent_id: String,
}

/// Why a decision was denied.
#[derive(Debug, Clone)]
pub enum DecisionDenialReason {
    MissingCapability(Operation),
    EnergyExhausted,
    NoveltyExhausted,
    InformationLossExceeded,
    IdentityDriftExceeded,
    MomentumViolation,
}

impl std::fmt::Display for DecisionDenialReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingCapability(op) => {
                write!(f, "agent lacks capability: {}", op.as_str())
            }
            Self::EnergyExhausted => write!(f, "energy conservation law violated: budget exhausted"),
            Self::NoveltyExhausted => write!(f, "entropy conservation law violated: novelty budget exhausted"),
            Self::InformationLossExceeded => write!(f, "information conservation law violated: too many memories pruned"),
            Self::IdentityDriftExceeded => write!(f, "symmetry conservation law violated: identity drift exceeds threshold"),
            Self::MomentumViolation => write!(f, "momentum conservation law violated: priority shift too large"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_builder() {
        let agent = Agent::builder("test-1")
            .capability(Operation::Remember)
            .capability(Operation::Recall)
            .capability(Operation::Predict)
            .energy(0.8)
            .build();

        assert_eq!(agent.id, "test-1");
        assert!(agent.has_capability(Operation::Remember));
        assert!(!agent.has_capability(Operation::Train));
        assert!((agent.state.energy - 0.8).abs() < 0.001);
    }

    #[test]
    fn test_decision_approved() {
        let mut agent = Agent::builder("test-1")
            .capability(Operation::Remember)
            .build();

        let result = agent.decide(Operation::Remember, 10.0, 0.1);
        assert!(matches!(result, DecisionResult::Approved(_)));
        assert_eq!(agent.state.decisions_made, 1);
    }

    #[test]
    fn test_decision_denied_capability() {
        let mut agent = Agent::builder("test-1").build();

        let result = agent.decide(Operation::Train, 10.0, 0.1);
        assert!(matches!(result, DecisionResult::Denied(_)));
    }

    #[test]
    fn test_decision_denied_energy() {
        let mut agent = Agent::builder("test-1")
            .capability(Operation::Predict)
            .energy_budget(10.0)
            .build();

        // Exhaust energy
        agent.conservation.spend(90.0);

        let result = agent.decide(Operation::Predict, 50.0, 0.0);
        assert!(matches!(
            result,
            DecisionResult::Denied(DecisionDenialReason::EnergyExhausted)
        ));
    }

    #[test]
    fn test_agent_messaging() {
        let mut agent = Agent::new("receiver");

        let msg = Message::remember("hello");
        agent.receive(msg);

        assert_eq!(agent.pending_messages(), 1);
        assert_eq!(agent.state.messages_received, 1);

        let processed = agent.process_message();
        assert!(processed.is_some());
        assert_eq!(agent.pending_messages(), 0);
    }
}
