//! AgentSpace — coordination space managing multiple agents.
//!
//! The AgentSpace is the container for all agents, handling:
//! - Agent registration and deregistration
//! - Message routing between agents
//! - Broadcast messaging
//! - Global conservation state tracking

use crate::agent::Agent;
use crate::message::{Message, MessageType};
use crate::types::*;

/// The AgentSpace — a coordination environment for multiple agents.
pub struct AgentSpace {
    agents: Vec<Agent>,
    /// Global event log (simplified cortical bus).
    events: Vec<CortexEvent>,
}

impl AgentSpace {
    /// Create an empty AgentSpace.
    pub fn new() -> Self {
        Self {
            agents: Vec::new(),
            events: Vec::new(),
        }
    }

    /// Register a new agent.
    ///
    /// Returns false if an agent with the same ID already exists.
    pub fn register(&mut self, agent: Agent) -> bool {
        if self.find_index(&agent.id).is_some() {
            return false;
        }
        let id = agent.id.clone();
        self.agents.push(agent);

        // Emit connect event
        let event = CortexEvent::new("agent_connect", "system")
            .with_payload("agent_id", &id)
            .with_importance(0.6);
        self.events.push(event);

        true
    }

    /// Deregister an agent by ID.
    pub fn deregister(&mut self, agent_id: &str) -> bool {
        if let Some(idx) = self.find_index(agent_id) {
            self.agents.swap_remove(idx);
            let event =
                CortexEvent::new("agent_disconnect", "system").with_payload("agent_id", agent_id);
            self.events.push(event);
            true
        } else {
            false
        }
    }

    /// Get an agent by ID.
    pub fn get(&self, agent_id: &str) -> Option<&Agent> {
        self.agents.iter().find(|a| a.id == agent_id)
    }

    /// Get a mutable agent by ID.
    pub fn get_mut(&mut self, agent_id: &str) -> Option<&mut Agent> {
        self.agents.iter_mut().find(|a| a.id == agent_id)
    }

    /// Number of registered agents.
    pub fn len(&self) -> usize {
        self.agents.len()
    }

    /// Is the space empty?
    pub fn is_empty(&self) -> bool {
        self.agents.is_empty()
    }

    /// List all agent IDs.
    pub fn agent_ids(&self) -> Vec<&str> {
        self.agents.iter().map(|a| a.id.as_str()).collect()
    }

    /// Get all agent info.
    pub fn agent_infos(&self) -> Vec<AgentInfo> {
        self.agents.iter().map(|a| a.info()).collect()
    }

    /// Send a message from one agent to another.
    ///
    /// Returns `Ok(())` if delivered, `Err` if sender or receiver not found.
    pub fn send(&mut self, from: &str, to: &str, mut message: Message) -> Result<(), SpaceError> {
        // Validate sender exists
        if self.find_index(from).is_none() {
            return Err(SpaceError::AgentNotFound(from.to_string()));
        }
        // Validate receiver exists
        let to_idx = self
            .find_index(to)
            .ok_or(SpaceError::AgentNotFound(to.to_string()))?;

        // Stamp the message
        message.from = from.to_string();
        message.to = to.to_string();
        let msg_type = message.message_type.clone();

        // Deliver. `Agent::receive` increments messages_received and pushes
        // onto the queue — do not double-count here.
        if let Some(ref mut a) = self.agents.get_mut(to_idx) {
            a.receive(message);
        }

        // Emit routing event
        let event = CortexEvent::new("message_routed", "space")
            .with_payload("from", from)
            .with_payload("to", to)
            .with_payload("type", msg_type.as_str());
        self.events.push(event);

        // Bump sender's sent counter
        if let Some(sender) = self.get_mut(from) {
            sender.state.messages_sent += 1;
        }

        Ok(())
    }

    /// Broadcast a message to all agents except the sender.
    pub fn broadcast(&mut self, from: &str, mut message: Message) -> Result<usize, SpaceError> {
        if self.find_index(from).is_none() {
            return Err(SpaceError::AgentNotFound(from.to_string()));
        }

        message.from = from.to_string();
        message.message_type = MessageType::Broadcast;
        let content = message.content.clone();

        let mut delivered = 0;
        for agent in &mut self.agents {
            if agent.id != from {
                let mut msg_copy = message.clone();
                msg_copy.to = agent.id.clone();
                agent.receive(msg_copy);
                delivered += 1;
            }
        }

        // Update sender stats
        if let Some(sender) = self.get_mut(from) {
            sender.state.messages_sent += 1;
        }

        // Emit broadcast event
        let event = CortexEvent::new("broadcast", "space")
            .with_payload("from", from)
            .with_payload("content", &content)
            .with_payload("delivered", &delivered.to_string());
        self.events.push(event);

        Ok(delivered)
    }

    /// Reset all agents' conservation cycles.
    pub fn reset_all_cycles(&mut self) {
        for agent in &mut self.agents {
            agent.reset_cycle();
        }
    }

    /// Get recent events from the space.
    pub fn recent_events(&self, n: usize) -> &[CortexEvent] {
        let start = self.events.len().saturating_sub(n);
        &self.events[start..]
    }

    /// Total number of events emitted.
    pub fn event_count(&self) -> usize {
        self.events.len()
    }

    /// Process messages for all agents (pop one message per agent).
    ///
    /// Returns a map of agent_id → processed message (if any).
    pub fn process_all(&mut self) -> Vec<(String, Message)> {
        let mut results = Vec::new();
        for agent in &mut self.agents {
            if let Some(msg) = agent.process_message() {
                results.push((agent.id.clone(), msg));
            }
        }
        results
    }

    /// Find the index of an agent by ID.
    fn find_index(&self, agent_id: &str) -> Option<usize> {
        self.agents.iter().position(|a| a.id == agent_id)
    }

    /// Iterate over all agents.
    pub fn iter(&self) -> impl Iterator<Item = &Agent> {
        self.agents.iter()
    }

    /// Iterate mutably over all agents.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Agent> {
        self.agents.iter_mut()
    }
}

impl Default for AgentSpace {
    fn default() -> Self {
        Self::new()
    }
}

/// Errors that can occur in the AgentSpace.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpaceError {
    AgentNotFound(String),
    CapabilityDenied(String),
    ConservationViolation(String),
}

impl std::fmt::Display for SpaceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SpaceError::AgentNotFound(id) => write!(f, "agent not found: {}", id),
            SpaceError::CapabilityDenied(reason) => write!(f, "capability denied: {}", reason),
            SpaceError::ConservationViolation(reason) => {
                write!(f, "conservation violation: {}", reason)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_and_get() {
        let mut space = AgentSpace::new();
        let agent = Agent::new("a1");
        assert!(space.register(agent));
        assert!(space.get("a1").is_some());
        assert!(space.get("nonexistent").is_none());
    }

    #[test]
    fn test_duplicate_register() {
        let mut space = AgentSpace::new();
        space.register(Agent::new("a1"));
        assert!(!space.register(Agent::new("a1"))); // duplicate
    }

    #[test]
    fn test_deregister() {
        let mut space = AgentSpace::new();
        space.register(Agent::new("a1"));
        assert!(space.deregister("a1"));
        assert!(space.get("a1").is_none());
        assert!(!space.deregister("a1"));
    }

    #[test]
    fn test_send_message() {
        let mut space = AgentSpace::new();
        space.register(Agent::new("a1"));
        space.register(Agent::new("a2"));

        let msg = Message::remember("hello world");
        assert!(space.send("a1", "a2", msg).is_ok());

        let a2 = space.get("a2").unwrap();
        assert_eq!(a2.pending_messages(), 1);
    }

    #[test]
    fn test_send_nonexistent() {
        let mut space = AgentSpace::new();
        space.register(Agent::new("a1"));

        let msg = Message::remember("hello");
        let result = space.send("a1", "ghost", msg);
        assert!(result.is_err());
    }

    #[test]
    fn test_broadcast() {
        let mut space = AgentSpace::new();
        space.register(Agent::new("a1"));
        space.register(Agent::new("a2"));
        space.register(Agent::new("a3"));

        let delivered = space.broadcast("a1", Message::remember("news")).unwrap();
        assert_eq!(delivered, 2);

        assert_eq!(space.get("a2").unwrap().pending_messages(), 1);
        assert_eq!(space.get("a3").unwrap().pending_messages(), 1);
        assert_eq!(space.get("a1").unwrap().pending_messages(), 0);
    }

    #[test]
    fn test_event_emission() {
        let mut space = AgentSpace::new();
        space.register(Agent::new("a1"));

        assert!(space.event_count() > 0); // connect event
    }
}
