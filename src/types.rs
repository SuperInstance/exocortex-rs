//! Core types — the canonical vocabulary every cortex speaks.

/// The 8 canonical operations every cortex speaks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Operation {
    Embed = 0,
    Query = 1,
    Train = 2,
    Predict = 3,
    Analyze = 4,
    Remember = 5,
    Recall = 6,
    Transform = 7,
}

impl Operation {
    /// All operations as a slice.
    pub const ALL: [Operation; 8] = [
        Operation::Embed,
        Operation::Query,
        Operation::Train,
        Operation::Predict,
        Operation::Analyze,
        Operation::Remember,
        Operation::Recall,
        Operation::Transform,
    ];

    /// String representation.
    pub fn as_str(&self) -> &'static str {
        match self {
            Operation::Embed => "embed",
            Operation::Query => "query",
            Operation::Train => "train",
            Operation::Predict => "predict",
            Operation::Analyze => "analyze",
            Operation::Remember => "remember",
            Operation::Recall => "recall",
            Operation::Transform => "transform",
        }
    }

    /// Determine the compute tier for this operation.
    pub fn compute_tier(&self) -> ComputeTier {
        match self {
            Operation::Embed | Operation::Remember | Operation::Recall | Operation::Query => {
                ComputeTier::Hot
            }
            Operation::Predict | Operation::Analyze | Operation::Transform => ComputeTier::Warm,
            Operation::Train => ComputeTier::Batch,
        }
    }
}

/// Parse an [`Operation`] from a string (case-sensitive).
///
/// Implemented as the standard `FromStr` trait so that `str::parse` works:
///
/// ```
/// use exocortex::Operation;
/// use std::str::FromStr;
///
/// assert_eq!(Operation::from_str("embed"), Ok(Operation::Embed));
/// assert_eq!("train".parse::<Operation>(), Ok(Operation::Train));
/// assert!("nope".parse::<Operation>().is_err());
/// ```
impl std::str::FromStr for Operation {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "embed" => Ok(Operation::Embed),
            "query" => Ok(Operation::Query),
            "train" => Ok(Operation::Train),
            "predict" => Ok(Operation::Predict),
            "analyze" => Ok(Operation::Analyze),
            "remember" => Ok(Operation::Remember),
            "recall" => Ok(Operation::Recall),
            "transform" => Ok(Operation::Transform),
            _ => Err(()),
        }
    }
}

/// Tiered compute — hot/warm/batch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ComputeTier {
    /// <5ms, sync, in-memory
    Hot,
    /// 5-500ms, FFI / small model
    Warm,
    /// >500ms, background
    Batch,
}

impl ComputeTier {
    pub fn as_str(&self) -> &'static str {
        match self {
            ComputeTier::Hot => "hot",
            ComputeTier::Warm => "warm",
            ComputeTier::Batch => "batch",
        }
    }
}

/// Supported protocols.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Protocol {
    A2a,
    Mcp,
    Rest,
    Tap,
}

impl Protocol {
    pub fn as_str(&self) -> &'static str {
        match self {
            Protocol::A2a => "a2a",
            Protocol::Mcp => "mcp",
            Protocol::Rest => "rest",
            Protocol::Tap => "tap",
        }
    }
}

/// Who/when/how for every memory — the "nutrition label" for AI decisions.
#[derive(Debug, Clone)]
pub struct Provenance {
    /// Agent that created this.
    pub who: String,
    /// Unix timestamp.
    pub when: f64,
    /// Operation that created this.
    pub how: String,
    /// Confidence factor 0..1.
    pub confidence: f64,
    /// Source identifier.
    pub source: String,
    /// Parent memory IDs (chain of derivation).
    pub chain: Vec<String>,
}

impl Provenance {
    pub fn new(who: &str, when: f64, how: &str) -> Self {
        Self {
            who: who.to_string(),
            when,
            how: how.to_string(),
            confidence: 1.0,
            source: String::new(),
            chain: Vec::new(),
        }
    }
}

/// Typed event on the Cortical Bus.
#[derive(Debug, Clone)]
pub struct CortexEvent {
    /// Event type name (operation name or "anomaly", "dream", "resonance").
    pub event_type: String,
    /// Source agent_id or "system".
    pub source: String,
    /// Links request → compute → memory → shadow.
    pub trace_id: String,
    /// Unix timestamp.
    pub timestamp: f64,
    /// Event payload.
    pub payload: Vec<(String, String)>,
    /// Importance 0..1, for priority queue.
    pub importance: f64,
    /// Novelty 0..1, how new/unusual.
    pub novelty: f64,
    /// Model certainty 0..1.
    pub confidence: f64,
    /// Provenance info.
    pub provenance: Option<Provenance>,
}

impl CortexEvent {
    /// Create a new event with auto-generated trace_id.
    pub fn new(event_type: &str, source: &str) -> Self {
        Self {
            event_type: event_type.to_string(),
            source: source.to_string(),
            trace_id: generate_trace_id(),
            timestamp: current_time(),
            payload: Vec::new(),
            importance: 0.5,
            novelty: 0.5,
            confidence: 1.0,
            provenance: None,
        }
    }

    /// Set payload key-value pair.
    pub fn with_payload(mut self, key: &str, value: &str) -> Self {
        self.payload.push((key.to_string(), value.to_string()));
        self
    }

    /// Set importance.
    pub fn with_importance(mut self, importance: f64) -> Self {
        self.importance = importance;
        self
    }

    /// Set confidence.
    pub fn with_confidence(mut self, confidence: f64) -> Self {
        self.confidence = confidence;
        self
    }

    /// Ordering: higher importance = higher priority (reversed for min-heap compat).
    /// Uses total order: first by importance (descending), then by timestamp (ascending).
    pub fn priority_key(&self) -> (i64, f64) {
        // Negate importance * 1000 as integer for stable ordering
        let importance_neg = -((self.importance * 1000.0) as i64);
        (importance_neg, self.timestamp)
    }

    /// Get a payload value by key.
    pub fn payload_get(&self, key: &str) -> Option<&str> {
        self.payload
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.as_str())
    }
}

/// Canonical request format — all protocols normalize to this.
#[derive(Debug, Clone)]
pub struct CortexRequest {
    pub operation: Operation,
    pub agent_id: String,
    pub payload: Vec<(String, String)>,
    pub protocol: Protocol,
    pub trace_id: String,
    /// 0=low, 1=critical.
    pub priority: f64,
}

/// Canonical response.
#[derive(Debug, Clone)]
pub struct CortexResponse {
    pub trace_id: String,
    pub operation: Operation,
    pub status: ResponseStatus,
    pub payload: Vec<(String, String)>,
    pub shadow_glyph: String,
    pub latency_ms: f64,
}

/// Response status.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResponseStatus {
    Ok,
    Error,
    Partial,
}

impl ResponseStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            ResponseStatus::Ok => "ok",
            ResponseStatus::Error => "error",
            ResponseStatus::Partial => "partial",
        }
    }
}

/// A single memory in the cortex.
#[derive(Debug, Clone)]
pub struct MemoryEntry {
    pub id: String,
    pub content: String,
    pub embedding: Vec<f64>,
    pub agent_id: String,
    pub confidence: f64,
    pub created_at: f64,
    pub last_reinforced: f64,
    pub half_life_days: f64,
    pub provenance: Option<Provenance>,
    pub tags: Vec<String>,
}

impl MemoryEntry {
    /// Create a new memory entry with auto-generated ID.
    pub fn new(content: &str, embedding: Vec<f64>, agent_id: &str) -> Self {
        let now = current_time();
        Self {
            id: generate_id(),
            content: content.to_string(),
            embedding,
            agent_id: agent_id.to_string(),
            confidence: 1.0,
            created_at: now,
            last_reinforced: now,
            half_life_days: 30.0,
            provenance: None,
            tags: Vec::new(),
        }
    }

    /// Create a new memory entry with tags.
    pub fn new_tagged(content: &str, embedding: Vec<f64>, agent_id: &str, tags: &[&str]) -> Self {
        let mut entry = Self::new(content, embedding, agent_id);
        entry.tags = tags.iter().map(|s| s.to_string()).collect();
        entry
    }

    /// Confidence after half-life decay.
    pub fn effective_confidence(&self) -> f64 {
        let age_days = (current_time() - self.last_reinforced) / 86400.0;
        // decay = 0.5 ^ (age / half_life) = exp(-ln(2) * age / half_life)
        let decay = (-std::f64::consts::LN_2 * age_days / self.half_life_days).exp();
        self.confidence * decay
    }

    /// Recall reinforces the memory (bumps last_reinforced).
    pub fn reinforce(&mut self) {
        self.last_reinforced = current_time();
    }
}

/// Connected agent metadata.
#[derive(Debug, Clone)]
pub struct AgentInfo {
    pub agent_id: String,
    pub protocol: Protocol,
    pub capabilities: Vec<Operation>,
    pub last_seen: f64,
}

impl AgentInfo {
    pub fn new(agent_id: &str, protocol: Protocol) -> Self {
        Self {
            agent_id: agent_id.to_string(),
            protocol,
            capabilities: Vec::new(),
            last_seen: current_time(),
        }
    }
}

// ── Utility functions ──

/// Cosine similarity between two vectors.
pub fn cosine_similarity(a: &[f64], b: &[f64]) -> f64 {
    if a.is_empty() || b.is_empty() || a.len() != b.len() {
        return 0.0;
    }
    let dot: f64 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let mag_a: f64 = a.iter().map(|x| x * x).sum::<f64>().sqrt();
    let mag_b: f64 = b.iter().map(|x| x * x).sum::<f64>().sqrt();
    if mag_a == 0.0 || mag_b == 0.0 {
        return 0.0;
    }
    dot / (mag_a * mag_b)
}

/// Euclidean distance between two vectors.
pub fn euclidean_distance(a: &[f64], b: &[f64]) -> f64 {
    a.iter()
        .zip(b.iter())
        .map(|(x, y)| (x - y).powi(2))
        .sum::<f64>()
        .sqrt()
}

/// Generate a short trace ID (12 hex chars).
pub fn generate_trace_id() -> String {
    generate_id()[..12].to_string()
}

/// Generate a random-ish ID (16 hex chars).
/// Uses a simple LCG seeded by time — not cryptographically secure.
pub fn generate_id() -> String {
    static mut STATE: u64 = 0;
    unsafe {
        if STATE == 0 {
            STATE = current_time() as u64 ^ 0x5DEECE66D;
        }
        let mut result = String::with_capacity(16);
        for _ in 0..16 {
            STATE = STATE
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            let nibble = ((STATE >> 56) & 0xF) as u8;
            let c = if nibble < 10 {
                b'0' + nibble
            } else {
                b'a' + nibble - 10
            };
            result.push(c as char);
        }
        result
    }
}

/// Current unix timestamp as f64.
pub fn current_time() -> f64 {
    // For no_std, we'd use a provided clock. For now, use std.
    #[cfg(feature = "std")]
    {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs_f64())
            .unwrap_or(0.0)
    }
    #[cfg(not(feature = "std"))]
    {
        0.0
    }
}
