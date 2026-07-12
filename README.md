# exocortex-rs

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
![Rust](https://img.shields.io/badge/Rust-🦀-orange)

🦀 **Rust implementation of the exocortex agent framework** — persistent cognitive substrate for multi-agent systems with conservation-law aware decision making.

Part of the [SuperInstance](https://github.com/SuperInstance) fleet ecosystem.

---

## Overview

This is the Rust port of [exocortex](https://github.com/SuperInstance/exocortex), rewritten for zero-dependency, deterministic computation. It provides:

- **Agent** — autonomous entity with capabilities, state, and message queue
- **AgentSpace** — coordination space managing multiple agents
- **Conservation-Law Aware Decisions** — five conservation laws govern every agent decision
- **Tiered Memory** — hot/warm/cold with exponential half-life decay
- **Cortical Bus** — priority-based pub/sub event spine
- **Resonance Engine** — cross-agent knowledge overlap detection
- **Shadow Rendering** — machine events → human-readable narratives

## Quick Start

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

## Conservation Laws

Every agent decision is governed by five conservation laws, analogous to thermodynamic laws:

| Law | What It Enforces |
|-----|-----------------|
| **Energy** | Total decision energy is finite per cycle (prevents runaway computation) |
| **Momentum** | Agents resist sudden priority flips (prevents thrashing) |
| **Entropy** | Novelty seeking is bounded (prevents random behavior) |
| **Information** | Memories cannot be silently lost (audit trail for state changes) |
| **Symmetry** | Agent identity is preserved through transforms (prevents identity collapse) |

### Using Conservation

```rust
use exocortex::{Agent, Operation, DecisionResult};

let mut agent = Agent::builder("worker")
    .capability(Operation::Predict)
    .energy_budget(100.0)
    .build();

// Each decision checks all conservation laws
match agent.decide(Operation::Predict, 15.0, 0.2) {
    DecisionResult::Approved(decision) => {
        println!("Decision approved: {:?}", decision.operation);
    }
    DecisionResult::Denied(reason) => {
        println!("Denied: {}", reason);
    }
}

// Reset at the start of each cycle
agent.reset_cycle();
```

## Memory

Three-tier memory with exponential confidence decay:

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

// Cooling cycle → hot→warm→cold
let stats = store.tick();
```

## Cross-Implementation

This component exists in two languages:

- **Python** (`pip install si-exocortex`) — [SuperInstance/exocortex](https://github.com/SuperInstance/exocortex)
- **Rust** (`cargo add exocortex`) — [SuperInstance/exocortex-rs](https://github.com/SuperInstance/exocortex-rs)

> **Parity status (honest):** This Rust crate implements the **core
> in-memory substrate**: agents, conservation laws, tiered memory,
> pub/sub bus, resonance, and shadow rendering. It does **not** yet
> match the Python implementation's breadth — there is no persistence
> backend (SurrealDB/SQLite/S3 are bring-your-own stubs here, not
> wired-in integrations), no async runtime, no transport layer, and
> `MemoryLayer` is exported but not yet plumbed through `AgentSpace`.
> The shared data structures and conservation semantics are intended
> to match the Python spec; do not assume behavioral parity for
> non-core features without checking the source.

## Status & Scope

| Area | Status |
|------|--------|
| Agent / AgentSpace / decisions | ✅ Implemented, in-memory |
| Conservation laws (5) | ✅ Implemented |
| Tiered memory (hot/warm/cold) | ✅ Implemented, in-memory only |
| Cortical Bus (priority pub/sub) | ✅ Implemented, sync only |
| Resonance engine | ✅ Implemented |
| Shadow rendering | ✅ Implemented |
| `MemoryLayer` (shared pool) | ⚠️  Exported but not wired into `AgentSpace` |
| Persistence (SurrealDB/SQLite/S3) | ❌ Not implemented — bring your own |
| Async transport (gRPC/REST/WebSocket) | ❌ Not implemented — bring your own |
| `no_std` | ❌ Removed — crate requires `std` (see commit history) |

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

## Modules

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
- Async runtime (tokio, async-std, etc.)
- Storage backend (SurrealDB, SQLite, S3, etc.)
- Transport layer (gRPC, REST, WebSocket, etc.)

## License

MIT

---

*🦀 Part of the **SuperInstance Fleet** — The crab inherits the shell. The forge shapes the steel.*
