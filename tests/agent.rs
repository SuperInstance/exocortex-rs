//! Agent tests — testing agent lifecycle, decisions, and messaging.

use exocortex::*;

#[test]
fn test_agent_creation() {
    let agent = Agent::builder("test-1")
        .capability(Operation::Remember)
        .capability(Operation::Recall)
        .capability(Operation::Predict)
        .protocol(Protocol::A2a)
        .energy(0.9)
        .world_confidence(0.7)
        .build();

    assert_eq!(agent.id, "test-1");
    assert_eq!(agent.protocol, Protocol::A2a);
    assert!(agent.has_capability(Operation::Remember));
    assert!(agent.has_capability(Operation::Recall));
    assert!(agent.has_capability(Operation::Predict));
    assert!(!agent.has_capability(Operation::Train));
    assert!((agent.state.energy - 0.9).abs() < 0.001);
    assert!((agent.state.world_confidence - 0.7).abs() < 0.001);
}

#[test]
fn test_agent_default_capabilities() {
    let agent = Agent::new("bare");
    assert_eq!(agent.id, "bare");
    assert!(agent.capabilities.is_empty());
    assert!(!agent.has_capability(Operation::Embed));
}

#[test]
fn test_agent_info() {
    let agent = Agent::builder("info-test")
        .capability(Operation::Remember)
        .protocol(Protocol::Mcp)
        .build();

    let info = agent.info();
    assert_eq!(info.agent_id, "info-test");
    assert_eq!(info.protocol, Protocol::Mcp);
    assert!(info.capabilities.contains(&Operation::Remember));
}

#[test]
fn test_agent_decision_approved() {
    let mut agent = Agent::builder("decider")
        .capability(Operation::Remember)
        .capability(Operation::Predict)
        .build();

    let result = agent.decide(Operation::Remember, 10.0, 0.1);
    assert!(matches!(result, DecisionResult::Approved(_)));
    assert_eq!(agent.state.decisions_made, 1);
}

#[test]
fn test_agent_decision_denied_missing_capability() {
    let mut agent = Agent::builder("limited")
        .capability(Operation::Remember)
        .build();

    let result = agent.decide(Operation::Train, 10.0, 0.1);
    assert!(matches!(
        result,
        DecisionResult::Denied(DecisionDenialReason::MissingCapability(Operation::Train))
    ));
}

#[test]
fn test_agent_decision_denied_energy() {
    let mut agent = Agent::builder("tired")
        .capability(Operation::Predict)
        .energy_budget(20.0)
        .build();

    // Exhaust most energy
    agent.conservation.spend(15.0);

    let result = agent.decide(Operation::Predict, 10.0, 0.0);
    assert!(matches!(
        result,
        DecisionResult::Denied(DecisionDenialReason::EnergyExhausted)
    ));
}

#[test]
fn test_agent_decision_denied_novelty() {
    let mut agent = Agent::builder("bored")
        .capability(Operation::Predict)
        .build();

    // Exhaust novelty budget
    agent.conservation.spend_novelty(0.95);

    let result = agent.decide(Operation::Predict, 1.0, 0.1);
    assert!(matches!(
        result,
        DecisionResult::Denied(DecisionDenialReason::NoveltyExhausted)
    ));
}

#[test]
fn test_agent_cycle_reset() {
    let mut agent = Agent::builder("cycler")
        .capability(Operation::Remember)
        .energy_budget(50.0)
        .build();

    // Spend energy
    agent.decide(Operation::Remember, 40.0, 0.5);
    assert!(agent.conservation.energy_remaining < 15.0);

    // Reset cycle
    agent.reset_cycle();
    assert!((agent.conservation.energy_remaining - 50.0).abs() < 0.001);
}

#[test]
fn test_agent_messaging() {
    let mut agent = Agent::new("receiver");

    let msg = Message::remember("hello world");
    agent.receive(msg);

    assert_eq!(agent.pending_messages(), 1);
    assert_eq!(agent.state.messages_received, 1);

    let processed = agent.process_message();
    assert!(processed.is_some());
    assert_eq!(processed.unwrap().content, "hello world");
    assert_eq!(agent.pending_messages(), 0);
}

#[test]
fn test_agent_local_memory() {
    let mut agent = Agent::builder("mem-agent")
        .capability(Operation::Remember)
        .capability(Operation::Recall)
        .build();

    let emb = vec![0.5; 64];
    let id = agent.remember("important fact", emb.clone(), &["important"]);
    assert!(!id.is_empty());

    let results = agent.recall(&emb, 5);
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].0.content, "important fact");
}

#[test]
fn test_agent_message_types() {
    let remember_msg = Message::remember("data");
    assert_eq!(remember_msg.operation, Some(Operation::Remember));

    let recall_msg = Message::recall("query");
    assert_eq!(recall_msg.operation, Some(Operation::Recall));
    assert!(recall_msg.expects_response());

    let predict_msg = Message::predict(vec![0.5; 128]);
    assert_eq!(predict_msg.operation, Some(Operation::Predict));
    assert!(predict_msg.expects_response());

    let custom_msg = Message::new("custom content")
        .with_priority(0.9)
        .with_tags(&["urgent"]);
    assert_eq!(custom_msg.content, "custom content");
    assert!((custom_msg.priority - 0.9).abs() < 0.001);
    assert_eq!(custom_msg.tags, vec!["urgent"]);
}

#[test]
fn test_agent_message_queue() {
    use exocortex::message::MessageQueue;

    let mut q = MessageQueue::new();

    q.push(Message::new("low").with_priority(0.1));
    q.push(Message::new("high").with_priority(0.9));
    q.push(Message::new("medium").with_priority(0.5));

    assert_eq!(q.len(), 3);

    let first = q.pop().unwrap();
    assert_eq!(first.content, "high");

    let second = q.pop().unwrap();
    assert_eq!(second.content, "medium");

    let third = q.pop().unwrap();
    assert_eq!(third.content, "low");

    assert!(q.is_empty());
}

#[test]
fn test_agent_message_queue_backpressure() {
    use exocortex::message::MessageQueue;

    let mut q = MessageQueue::with_capacity(3);
    assert!(q.push(Message::new("1").with_priority(0.1)));
    assert!(q.push(Message::new("2").with_priority(0.5)));
    assert!(q.push(Message::new("3").with_priority(0.9)));

    // Queue full, pushing should drop lowest priority and accept new
    assert!(q.push(Message::new("4").with_priority(0.7)));

    // "1" (lowest) should be gone; "3" (highest) should be first out
    let first = q.pop().unwrap();
    assert_eq!(first.content, "3");
}

// --- AgentSpace tests ---

#[test]
fn test_space_register() {
    let mut space = AgentSpace::new();
    assert!(space.is_empty());

    let agent = Agent::new("a1");
    assert!(space.register(agent));
    assert_eq!(space.len(), 1);
    assert!(space.get("a1").is_some());
}

#[test]
fn test_space_duplicate_register() {
    let mut space = AgentSpace::new();
    space.register(Agent::new("a1"));
    assert!(!space.register(Agent::new("a1")));
    assert_eq!(space.len(), 1);
}

#[test]
fn test_space_deregister() {
    let mut space = AgentSpace::new();
    space.register(Agent::new("a1"));
    assert!(space.deregister("a1"));
    assert!(space.is_empty());
    assert!(!space.deregister("a1"));
}

#[test]
fn test_space_send_message() {
    let mut space = AgentSpace::new();
    space.register(Agent::new("sender"));
    space.register(Agent::new("receiver"));

    let msg = Message::remember("hello");
    let result = space.send("sender", "receiver", msg);
    assert!(result.is_ok());

    let receiver = space.get("receiver").unwrap();
    assert_eq!(receiver.pending_messages(), 1);

    let sender = space.get("sender").unwrap();
    assert_eq!(sender.state.messages_sent, 1);
}

#[test]
fn test_space_send_nonexistent() {
    let mut space = AgentSpace::new();
    space.register(Agent::new("a1"));

    let result = space.send("a1", "ghost", Message::new("hi"));
    assert!(result.is_err());

    let result = space.send("ghost", "a1", Message::new("hi"));
    assert!(result.is_err());
}

#[test]
fn test_space_broadcast() {
    let mut space = AgentSpace::new();
    space.register(Agent::new("a1"));
    space.register(Agent::new("a2"));
    space.register(Agent::new("a3"));
    space.register(Agent::new("a4"));

    let delivered = space.broadcast("a1", Message::remember("news")).unwrap();
    assert_eq!(delivered, 3);

    assert_eq!(space.get("a2").unwrap().pending_messages(), 1);
    assert_eq!(space.get("a3").unwrap().pending_messages(), 1);
    assert_eq!(space.get("a4").unwrap().pending_messages(), 1);
    assert_eq!(space.get("a1").unwrap().pending_messages(), 0);
}

#[test]
fn test_space_process_all() {
    let mut space = AgentSpace::new();
    space.register(Agent::new("a1"));
    space.register(Agent::new("a2"));

    space.send("a1", "a2", Message::remember("msg1"));
    space.send("a2", "a1", Message::remember("msg2"));

    let processed = space.process_all();
    assert_eq!(processed.len(), 2);
}

#[test]
fn test_space_event_emission() {
    let mut space = AgentSpace::new();
    space.register(Agent::new("a1"));
    assert!(space.event_count() > 0); // connect event

    space.register(Agent::new("a2"));
    space.send("a1", "a2", Message::new("hello"));
    assert!(space.event_count() >= 3); // 2 connects + 1 message routed
}

#[test]
fn test_space_agent_ids() {
    let mut space = AgentSpace::new();
    space.register(Agent::new("alpha"));
    space.register(Agent::new("beta"));
    space.register(Agent::new("gamma"));

    let ids = space.agent_ids();
    assert_eq!(ids.len(), 3);
}

#[test]
fn test_space_reset_cycles() {
    let mut space = AgentSpace::new();
    let mut agent = Agent::builder("a1")
        .capability(Operation::Remember)
        .energy_budget(50.0)
        .build();

    // Agent spends energy
    agent.decide(Operation::Remember, 30.0, 0.5);
    assert!(agent.conservation.energy_remaining < 25.0);

    space.register(agent);
    space.reset_all_cycles();

    let a = space.get("a1").unwrap();
    assert!((a.conservation.energy_remaining - 50.0).abs() < 0.001);
}
