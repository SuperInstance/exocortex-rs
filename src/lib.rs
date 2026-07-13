//! # Exocortex — Persistent Cognitive Substrate for Multi-Agent Systems
//!
//! A zero-dependency Rust implementation of the exocortex agent framework.
//! Provides conservation-law aware decision making, tiered memory with
//! half-life decay, inter-agent messaging, and resonance detection.
//!
//! ## Architecture
//!
//! - **Agent** — autonomous entity with capabilities, state, and message queue
//! - **AgentSpace** — coordination space managing multiple agents
//! - **Message** — typed inter-agent communication
//! - **Memory** — three-tier (hot/warm/cold) with exponential confidence decay
//! - **CorticalBus** — priority-based pub/sub event spine
//! - **Resonance** — cross-agent knowledge overlap detection
//! - **Conservation** — five conservation laws governing agent decisions
//!
//! ## Quick Start
//!
//! ```rust
//! use exocortex::{AgentSpace, Agent, Operation, Message};
//!
//! let mut space = AgentSpace::new();
//! let agent = Agent::builder("agent-1")
//!     .capability(Operation::Remember)
//!     .capability(Operation::Recall)
//!     .build();
//! space.register(agent);
//! space.register(Agent::new("agent-2"));
//!
//! // Deliver a message (send validates both endpoints exist).
//! let result = space.send("agent-1", "agent-2", Message::remember("hello world"));
//! assert!(result.is_ok());
//! ```

//! *🦀 Part of the **SuperInstance Fleet** — zero-dependency, std-only for now.
//! A future `no_std` port would require replacing `std::collections::*` and
//! `SystemTime` with `alloc` + an injectable clock.*

pub mod agent;
pub mod bus;
pub mod conservation;
pub mod memory;
pub mod message;
pub mod resonance;
pub mod shadow;
pub mod space;
pub mod types;

pub use agent::{Agent, AgentBuilder, AgentState, Decision, DecisionDenialReason, DecisionResult};
pub use bus::CorticalBus;
pub use conservation::{ConservationLaw, ConservationState, DECISION_ENERGY_BUDGET};
pub use memory::MemoryLayer;
pub use message::{Message, MessageType};
pub use resonance::ResonanceEngine;
pub use space::AgentSpace;
pub use types::*;

/// Current version of the exocortex crate.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
