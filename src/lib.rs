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
//! use exocortex::{AgentSpace, Agent, Operation};
//!
//! let mut space = AgentSpace::new();
//! let agent = Agent::builder("agent-1")
//!     .capability(Operation::Remember)
//!     .capability(Operation::Recall)
//!     .build();
//! space.register(agent);
//!
//! space.send("agent-1", "agent-2", Message::remember("hello world"));
//! ```

#![cfg_attr(not(feature = "std"), no_std)]

pub mod agent;
pub mod space;
pub mod message;
pub mod memory;
pub mod bus;
pub mod resonance;
pub mod conservation;
pub mod types;
pub mod shadow;

pub use agent::Agent;
pub use space::AgentSpace;
pub use message::{Message, MessageType};
pub use memory::MemoryLayer;
pub use bus::CorticalBus;
pub use resonance::ResonanceEngine;
pub use conservation::{ConservationLaw, ConservationState, DECISION_ENERGY_BUDGET};
pub use types::*;

/// Current version of the exocortex crate.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
