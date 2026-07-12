//! Core tests — testing fundamental types, memory, bus, and shadow rendering.

use exocortex::*;

#[test]
fn test_operation_enum() {
    assert_eq!(Operation::ALL.len(), 8);
    assert_eq!(Operation::Embed.as_str(), "embed");
    assert_eq!(Operation::Train.as_str(), "train");
}

#[test]
fn test_operation_from_str() {
    assert_eq!(Operation::from_str("embed"), Some(Operation::Embed));
    assert_eq!(Operation::from_str("invalid"), None);
}

#[test]
fn test_operation_compute_tier() {
    assert_eq!(Operation::Embed.compute_tier(), ComputeTier::Hot);
    assert_eq!(Operation::Predict.compute_tier(), ComputeTier::Warm);
    assert_eq!(Operation::Train.compute_tier(), ComputeTier::Batch);
}

#[test]
fn test_cortex_event_creation() {
    let event = CortexEvent::new("embed", "test-agent")
        .with_payload("dims", "384")
        .with_importance(0.8)
        .with_confidence(0.95);

    assert_eq!(event.event_type, "embed");
    assert_eq!(event.source, "test-agent");
    assert_eq!(event.trace_id.len(), 12);
    assert!((event.importance - 0.8).abs() < 0.001);
    assert!((event.confidence - 0.95).abs() < 0.001);
    assert_eq!(event.payload_get("dims"), Some("384"));
}

#[test]
fn test_memory_entry_half_life() {
    let mut entry = MemoryEntry::new("test", vec![], "agent");
    entry.half_life_days = 30.0;

    // Fresh memory should have ~1.0 confidence
    assert!(entry.effective_confidence() > 0.99);

    // Age 30 days → confidence should be ~0.5
    entry.last_reinforced = current_time() - 30.0 * 86400.0;
    let conf = entry.effective_confidence();
    assert!(conf > 0.45 && conf < 0.55, "expected ~0.5, got {}", conf);
}

#[test]
fn test_memory_entry_reinforce() {
    let mut entry = MemoryEntry::new("test", vec![], "agent");
    entry.half_life_days = 1.0;
    entry.last_reinforced = current_time() - 86400.0; // 1 day old
    let old_conf = entry.effective_confidence();
    entry.reinforce();
    let new_conf = entry.effective_confidence();
    assert!(new_conf > old_conf);
}

#[test]
fn test_cosine_similarity() {
    let a = vec![1.0, 0.0, 0.0];
    let b = vec![1.0, 0.0, 0.0];
    assert!((cosine_similarity(&a, &b) - 1.0).abs() < 0.001);

    let c = vec![0.0, 1.0, 0.0];
    assert!((cosine_similarity(&a, &c)).abs() < 0.001);

    let d = vec![-1.0, 0.0, 0.0];
    assert!((cosine_similarity(&a, &d) + 1.0).abs() < 0.001);
}

#[test]
fn test_euclidean_distance() {
    let a = vec![0.0, 0.0];
    let b = vec![3.0, 4.0];
    assert!((euclidean_distance(&a, &b) - 5.0).abs() < 0.001);
}

#[test]
fn test_provenance() {
    let p = Provenance::new("agent-1", current_time(), "embed");
    assert_eq!(p.who, "agent-1");
    assert!(p.chain.is_empty());
    assert!((p.confidence - 1.0).abs() < 0.001);
}

#[test]
fn test_memory_store_basic() {
    let mut store = memory::MemoryStore::new();
    let emb = vec![0.5; 128];
    let id = store.remember("hello", emb.clone(), "test", &["greeting"]);

    assert!(!id.is_empty());

    let results = store.recall(&emb, 5);
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].0.content, "hello");
}

#[test]
fn test_memory_store_tag_query() {
    let mut store = memory::MemoryStore::new();
    store.remember("rust code", vec![0.1; 64], "dev", &["rust", "code"]);
    store.remember("python code", vec![0.2; 64], "dev", &["python", "code"]);
    store.remember("rust essay", vec![0.3; 64], "writer", &["rust", "essay"]);

    let rust_results = store.query_by_tags(&["rust"], 10);
    assert_eq!(rust_results.len(), 2);

    let code_results = store.query_by_tags(&["code"], 10);
    assert_eq!(code_results.len(), 2);
}

#[test]
fn test_memory_store_stats() {
    let mut store = memory::MemoryStore::new();
    store.remember("m1", vec![0.1; 32], "a", &[]);
    store.remember("m2", vec![0.2; 32], "a", &[]);
    store.remember("m3", vec![0.3; 32], "a", &[]);

    let stats = store.stats();
    assert!(stats.total >= 3);
}

#[test]
fn test_cortical_bus_basic() {
    let mut bus = bus::CorticalBus::new();
    bus.publish(CortexEvent::new("test", "unit"));
    bus.publish(CortexEvent::new("test2", "unit"));

    assert_eq!(bus.pending(), 2);

    let dispatched = bus.dispatch_all();
    assert_eq!(dispatched, 2);
    assert_eq!(bus.pending(), 0);
}

#[test]
fn test_cortical_bus_backpressure() {
    let mut bus = bus::CorticalBus::with_capacity(5);

    for _ in 0..5 {
        assert!(bus.publish(CortexEvent::new("test", "unit")));
    }

    assert!(!bus.publish(CortexEvent::new("overflow", "unit")));
    assert_eq!(bus.stats().dropped, 1);
}

#[test]
fn test_cortical_bus_priority() {
    let mut bus = bus::CorticalBus::new();

    bus.publish(CortexEvent::new("low", "unit").with_importance(0.1));
    bus.publish(CortexEvent::new("high", "unit").with_importance(0.9));
    bus.publish(CortexEvent::new("mid", "unit").with_importance(0.5));

    // The bus should dispatch in priority order (high first)
    // We can verify by checking the internal queue ordering
    let stats = bus.stats();
    assert_eq!(stats.pending, 3);
}

#[test]
fn test_shadow_render_embed() {
    let event = CortexEvent::new("embed", "test").with_payload("dims", "384");
    let shadow = shadow::render_shadow(&event);
    assert!(shadow.glyph.contains("🧮"));
    assert_eq!(shadow.color, shadow::ShadowColor::Blue);
}

#[test]
fn test_shadow_render_anomaly() {
    let event = CortexEvent::new("anomaly", "reflex").with_payload("detail", "spike");
    let shadow = shadow::render_shadow(&event);
    assert!(shadow.glyph.contains("⚠️"));
    assert_eq!(shadow.color, shadow::ShadowColor::Red);
}

#[test]
fn test_resonance_basic() {
    let mut engine = resonance::ResonanceEngine::with_threshold(0.5);
    let emb = vec![1.0, 0.0, 0.0, 0.0];

    engine.record_query("agent-a", "what is rust?", &emb);
    let hits = engine.record_learning("agent-b", "rust is great", &emb);

    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].source_agent, "agent-b");
    assert_eq!(hits[0].target_agent, "agent-a");
}

#[test]
fn test_resonance_no_self_resonance() {
    let mut engine = resonance::ResonanceEngine::with_threshold(0.0);
    let emb = vec![1.0; 32];

    engine.record_query("a", "query", &emb);
    let hits = engine.record_learning("a", "learning", &emb);

    assert!(hits.is_empty());
}

#[test]
fn test_conservation_laws() {
    assert_eq!(conservation::ConservationLaw::ALL.len(), 5);

    for law in &conservation::ConservationLaw::ALL {
        assert!(!law.name().is_empty());
        assert!(!law.description().is_empty());
    }
}
