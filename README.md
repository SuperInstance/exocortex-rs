# 🧠 Exocortex (Rust)

![Crates.io](https://img.shields.io/crates/v/si-exocortex)
![Rust](https://img.shields.io/badge/rust-stable-orange)
![Tests](https://img.shields.io/badge/tests-50%2B-brightgreen)
![no_std](https://img.shields.io/badge/no__std-compatible-blue)
![License](https://img.shields.io/badge/License-MIT-yellow)

**Persistent cognitive substrate for multi-agent systems** — conservation-law aware decision making, tiered memory with half-life decay, inter-agent messaging, and resonance detection.

Zero external dependencies. Bring your own async runtime, storage backend, and transport layer.

---

## Philosophy

Built on [Working Animal Architecture](https://github.com/SuperInstance/AI-Writings), where **γ + η = C** (genome + nurture = capability). The exocortex is the **η** — the nurture layer, the persistent memory and learned experience that turns a working animal from instinct into craft. Each agent is a working animal; the exocortex is the shared cognitive substrate that coordinates the flock.

> *The exocortex doesn't think. It remembers, coordinates, and conserves — so the agents can think.*

## What Is This?

The Rust port of [exocortex](https://github.com/SuperInstance/exocortex), rewritten for zero-dependency, deterministic computation. It provides:

- **Agent** — autonomous entity with capabilities, state, and message queue
- **AgentSpace** — coordination space managing multiple agents
- **Conservation-Law Aware Decisions** — five conservation laws govern every agent decision
- **Tiered Memory** — hot / warm / cold with exponential half-life decay
- **Cortical Bus** — priority-based pub/sub event spine
- **Resonance Engine** — cross-agent knowledge overlap detection
- **Shadow Rendering** — machine events → human-readable narratives

## Installation

```bash
cargo add si-exocortex
```

Or in `Cargo.toml`:

```toml
[dependencies]
si-exocortex = "0.1"
```

For `no_std` environments:

```toml
[dependencies]
si-exocortex = { version = "0.1", default-features = false }
```

## Quick Start

### Create agents and send messages

```rust
use exocortex::{AgentSpace, Agent, Operation, Message};

let mut space = AgentSpace::new();

// Create agents with capabilities
let researcher = Agent::builder("researcher")
    .capability(Operation::Remember)
    .capability(Operation::Recall)
    .capability(Operation::Query)
    .build();

let predictor = Agent::builder("predictor")
    .capability(Operation::Predict)
    .capability(Operation::Train)
    .build();

space.register(researcher);
space.register(predictor);

// Send messages between agents
space.send("researcher", "predictor", Message::remember("training data")).unwrap();
```

### Conservation-law aware decisions

```rust
use exocortex::{Agent, Operation, DecisionResult};

let mut agent = Agent::builder("worker")
    .capability(Operation::Predict)
    .energy_budget(100.0)
    .build();

// Each decision checks all five conservation laws
match agent.decide(Operation::Predict, 15.0, 0.2) {
    DecisionResult::Approved(decision) => {
        println!("Approved: {:?}", decision.operation);
    }
    DecisionResult::Denied(reason) => {
        println!("Denied: {}", reason);
    }
}

// Reset at the start of each cycle
agent.reset_cycle();
```

### Tiered memory with decay

```rust
use exocortex::memory::MemoryStore;

let mut store = MemoryStore::new();
let embedding = vec![0.1; 384];

// Store → goes to hot + warm tier
let id = store.remember("important fact", embedding.clone(), "agent-1", &["important"]);

// Recall → finds by cosine similarity, reinforces memories
let results = store.recall(&embedding, 5);

// Tag-based query
let tagged = store.query_by_tags(&["important"], 10);

// Cooling cycle → hot→warm→cold migration
let stats = store.tick();
```

## Conservation Laws

Every agent decision is governed by five conservation laws, analogous to thermodynamic principles:

| Law | Symbol | What It Enforces |
|-----|--------|-----------------|
| **Energy** | ΔE | Total decision energy is finite per cycle — prevents runaway computation |
| **Momentum** | Δp | Agents resist sudden priority flips — prevents thrashing |
| **Entropy** | ΔS | Novelty seeking is bounded — prevents random behavior |
| **Information** | ΔI | Memories cannot be silently lost — audit trail for state changes |
| **Symmetry** | Ψ | Agent identity is preserved through transforms — prevents identity collapse |

These aren't metaphors. Each law maps to a concrete numeric check in the `conservation` module that gates every `agent.decide()` call. A decision that violates a conservation law returns `DecisionResult::Denied(reason)`.

## API Reference

### Core Types

| Type | Description |
|------|-------------|
| `Agent` | Autonomous entity with capabilities, state, memory, and message queue |
| `AgentSpace` | Coordination space — agent registry, message routing, resonance detection |
| `Message` | Typed inter-agent communication (Remember, Recall, Predict, Train, Query, etc.) |
| `Operation` | Enum of agent capabilities |
| `DecisionResult` | Result of `agent.decide()` — Approved or Denied with reason |

### Memory Module

| Method | Description |
|--------|-------------|
| `MemoryStore::new()` | Create a three-tier memory store |
| `store.remember(text, embedding, agent, tags)` | Store a memory (→ hot tier) |
| `store.recall(embedding, k)` | Find k nearest by cosine similarity |
| `store.query_by_tags(tags, k)` | Retrieve by tag match |
| `store.tick()` | Run cooling cycle (hot → warm → cold) |
| `store.stats()` | Memory statistics per tier |

### Conservation Module

| Method | Description |
|--------|-------------|
| `ConservationState::new()` | Initialize conservation state with defaults |
| `state.check_energy(cost)` | Verify energy budget not exceeded |
| `state.check_momentum(op)` | Verify operation doesn't flip priority too fast |
| `state.check_entropy(novelty)` | Verify novelty within bounds |
| `state.check_information(old, new)` | Verify no silent memory loss |
| `state.check_symmetry(agent)` | Verify agent identity preserved |

### Bus Module (Cortical Bus)

```rust
use exocortex::{CorticalBus, MessageType};

let mut bus = CorticalBus::new();

// Subscribe
bus.subscribe("agent-1", MessageType::Predict);

// Publish
bus.publish(Message::predict("forecast", 0.95));

// Drain matching messages for an agent
let msgs = bus.drain_for("agent-1");
```

### Resonance Module

```rust
use exocortex::ResonanceEngine;

let mut engine = ResonanceEngine::new();
engine.add_knowledge("agent-1", &["rust", "systems", "memory"]);
engine.add_knowledge("agent-2", &["rust", "safety", "concurrency"]);

// Find overlapping knowledge between agents
let resonances = engine.detect_resonance(0.3); // 30% overlap threshold
```

## Architecture

```
┌─────────────────────────────────────────────────┐
│                  AgentSpace                       │
│                                                   │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐       │
│  │  Agent A  │←→│  Agent B  │←→│  Agent C  │       │
│  │           │  │           │  │           │       │
│  │ Capability│  │ Capability│  │ Capability│       │
│  │   State   │  │   State   │  │   State   │       │
│  │Conservation│ │Conservation│ │Conservation│      │
│  │  Memory   │  │  Memory   │  │  Memory   │       │
│  │ MsgQueue  │  │ MsgQueue  │  │ MsgQueue  │       │
│  └──────────┘  └──────────┘  └──────────┘       │
│                                                   │
│  ┌─────────────────────────────────────────┐     │
│  │            Cortical Bus                   │     │
│  │  (priority pub/sub event spine)           │     │
│  └─────────────────────────────────────────┘     │
│                                                   │
│  ┌──────────────────┐  ┌────────────────┐        │
│  │ Resonance Engine  │  │ Shadow Renderer │        │
│  │ (cross-agent      │  │ (event → glyph) │        │
│  │  overlap detector)│  │                 │        │
│  └──────────────────┘  └────────────────┘        │
└─────────────────────────────────────────────────┘
```

### Module Map

| Module | Description |
|--------|-------------|
| `agent` | Agent lifecycle, state, and decision-making |
| `space` | AgentSpace — multi-agent coordination |
| `message` | Inter-agent message types and priority queue |
| `memory` | Three-tier memory with half-life decay |
| `bus` | Priority-based pub/sub Cortical Bus |
| `resonance` | Cross-agent knowledge resonance detection |
| `conservation` | Five conservation laws governing decisions |
| `types` | Core types (Operation, CortexEvent, MemoryEntry, etc.) |
| `shadow` | Event-to-narrative rendering pipeline |

## Zero Dependencies

This crate has **zero external dependencies** by design. The exocortex is a substrate, not an application framework. Bring your own:

- **Async runtime** — tokio, async-std, smol, etc.
- **Storage backend** — SurrealDB, SQLite, S3, etc.
- **Transport layer** — gRPC, REST, WebSocket, etc.

## Testing

```bash
# Run all tests
cargo test

# Run specific test suites
cargo test --test core         # Core types and conservation laws
cargo test --test agent        # Agent lifecycle and decisions
cargo test --test conservation # Conservation law enforcement

# Run with output
cargo test -- --nocapture
```

## Cross-Implementation

| Aspect | Python | Rust |
|--------|--------|------|
| Package | `pip install si-exocortex` | `cargo add si-exocortex` |
| Repo | [exocortex](https://github.com/SuperInstance/exocortex) | [exocortex-rs](https://github.com/SuperInstance/exocortex-rs) (this) |
| Dependencies | stdlib + numpy | **zero** external deps |
| `no_std` | N/A | ✅ (`default-features = false`) |
| Spec compatibility | Reference implementation | Feature-complete port |

Both implementations share the same architecture specification. Agents, messages, and memory entries are structurally compatible.

## Ecosystem

### FLUX Policy Layer
- [conservation-enforcer](https://github.com/SuperInstance/conservation-enforcer-rs) — FLUX bytecode conservation enforcement
- [flux-policy-tester](https://github.com/SuperInstance/flux-policy-tester-rs) — Policy testing framework
- [flux-registry](https://github.com/SuperInstance/flux-registry-rs) — Pre-compiled policy registry
- [flux-compiler](https://github.com/SuperInstance/flux-compiler-rs) — Bytecode assembler/disassembler

### PLATO Protocol
- [plato-core](https://github.com/SuperInstance/plato-core-rs) — Room/Sensor/Actuator protocol
- [plato-room-security-audit](https://github.com/SuperInstance/plato-room-security-audit-rs) — Security audit room

### Theory
- [AI-Writings](https://github.com/SuperInstance/AI-Writings) — Paradigm essays

## License

MIT

---

*🦀 Part of the **SuperInstance Fleet** — The crab inherits the shell. The forge shapes the steel.*
