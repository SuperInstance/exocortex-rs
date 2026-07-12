//! Resonance Engine — detects when agents' knowledge overlaps.
//!
//! When Agent A learns something that overlaps with Agent B's active queries,
//! the resonance engine detects this and emits a "resonance" event.
//! This enables serendipitous cross-agent knowledge sharing.

use crate::types::{cosine_similarity, current_time, CortexEvent};
use std::collections::HashMap;

/// Resonance threshold (default 0.8 cosine similarity).
pub const RESONANCE_THRESHOLD: f64 = 0.8;

/// Maximum learning events stored per agent.
pub const MAX_LEARNING_PER_AGENT: usize = 50;

/// Maximum active queries stored per agent.
pub const MAX_QUERIES_PER_AGENT: usize = 20;

/// Learning TTL in seconds (1 hour).
pub const LEARNING_TTL_SECONDS: f64 = 3600.0;

/// A learning event from an agent.
#[derive(Debug, Clone)]
pub struct LearningEvent {
    pub agent_id: String,
    pub content: String,
    pub embedding: Vec<f64>,
    pub timestamp: f64,
}

/// An active query from an agent.
#[derive(Debug, Clone)]
pub struct ActiveQuery {
    pub agent_id: String,
    pub content: String,
    pub embedding: Vec<f64>,
    pub timestamp: f64,
}

/// A detected resonance between two agents.
#[derive(Debug, Clone)]
pub struct ResonanceHit {
    pub source_agent: String,
    pub target_agent: String,
    pub learning_content: String,
    pub query_content: String,
    pub similarity: f64,
    pub timestamp: f64,
}

impl ResonanceHit {
    fn new(source: &str, target: &str, learning: &str, query: &str, similarity: f64) -> Self {
        Self {
            source_agent: source.to_string(),
            target_agent: target.to_string(),
            learning_content: learning.to_string(),
            query_content: query.to_string(),
            similarity,
            timestamp: current_time(),
        }
    }
}

/// Detects cross-agent knowledge resonance.
///
/// Tracks what agents are learning and what they're querying.
/// When one agent's learning overlaps another's active query,
/// a resonance event is emitted.
pub struct ResonanceEngine {
    threshold: f64,
    learnings: HashMap<String, Vec<LearningEvent>>,
    queries: HashMap<String, Vec<ActiveQuery>>,
    recent_resonances: Vec<ResonanceHit>,
    stats: ResonanceStats,
}

impl ResonanceEngine {
    /// Create a new resonance engine with default threshold.
    pub fn new() -> Self {
        Self {
            threshold: RESONANCE_THRESHOLD,
            learnings: HashMap::new(),
            queries: HashMap::new(),
            recent_resonances: Vec::new(),
            stats: ResonanceStats::default(),
        }
    }

    /// Create with a custom threshold.
    pub fn with_threshold(threshold: f64) -> Self {
        Self {
            threshold,
            ..Self::new()
        }
    }

    /// Record a learning event and check for resonances.
    ///
    /// Returns list of resonance hits detected.
    pub fn record_learning(
        &mut self,
        agent_id: &str,
        content: &str,
        embedding: &[f64],
    ) -> Vec<ResonanceHit> {
        let event = LearningEvent {
            agent_id: agent_id.to_string(),
            content: content.to_string(),
            embedding: embedding.to_vec(),
            timestamp: current_time(),
        };

        let learnings = self.learnings.entry(agent_id.to_string()).or_default();
        learnings.push(event);
        self.stats.learnings_tracked += 1;

        // Trim to max
        if learnings.len() > MAX_LEARNING_PER_AGENT {
            let excess = learnings.len() - MAX_LEARNING_PER_AGENT;
            learnings.drain(..excess);
        }

        // Check against all other agents' active queries
        self.check_resonances_for_learning(agent_id, content, embedding)
    }

    /// Record an active query and check for resonances.
    ///
    /// Returns list of resonance hits detected.
    pub fn record_query(
        &mut self,
        agent_id: &str,
        content: &str,
        embedding: &[f64],
    ) -> Vec<ResonanceHit> {
        let query = ActiveQuery {
            agent_id: agent_id.to_string(),
            content: content.to_string(),
            embedding: embedding.to_vec(),
            timestamp: current_time(),
        };

        let queries = self.queries.entry(agent_id.to_string()).or_default();
        queries.push(query);
        self.stats.queries_tracked += 1;

        if queries.len() > MAX_QUERIES_PER_AGENT {
            let excess = queries.len() - MAX_QUERIES_PER_AGENT;
            queries.drain(..excess);
        }

        // Check this query against all other agents' learnings
        let mut hits = Vec::new();
        for (other_agent, learnings) in &self.learnings {
            if other_agent == agent_id {
                continue;
            }
            for learning in learnings {
                let sim = cosine_similarity(embedding, &learning.embedding);
                if sim >= self.threshold {
                    hits.push(ResonanceHit::new(
                        other_agent,
                        agent_id,
                        &learning.content,
                        content,
                        sim,
                    ));
                    self.stats.resonances_detected += 1;
                }
            }
        }

        self.recent_resonances.extend(hits.clone());
        if self.recent_resonances.len() > 100 {
            let excess = self.recent_resonances.len() - 100;
            self.recent_resonances.drain(..excess);
        }

        hits
    }

    /// Check a learning event against all agents' active queries.
    fn check_resonances_for_learning(
        &self,
        agent_id: &str,
        content: &str,
        embedding: &[f64],
    ) -> Vec<ResonanceHit> {
        let mut hits = Vec::new();
        for (other_agent, queries) in &self.queries {
            if other_agent == agent_id {
                continue;
            }
            for query in queries {
                let sim = cosine_similarity(embedding, &query.embedding);
                if sim >= self.threshold {
                    hits.push(ResonanceHit::new(
                        agent_id,
                        other_agent,
                        content,
                        &query.content,
                        sim,
                    ));
                }
            }
        }
        hits
    }

    /// Remove stale learning events and queries.
    ///
    /// Returns the number of items pruned.
    pub fn prune_stale(&mut self) -> ResonancePruneStats {
        let now = current_time();
        let mut learnings_pruned = 0;
        let mut queries_pruned = 0;

        let learning_agents: Vec<String> = self.learnings.keys().cloned().collect();
        for agent_id in learning_agents {
            if let Some(learnings) = self.learnings.get_mut(&agent_id) {
                let before = learnings.len();
                learnings.retain(|e| now - e.timestamp < LEARNING_TTL_SECONDS);
                learnings_pruned += before - learnings.len();
                if learnings.is_empty() {
                    self.learnings.remove(&agent_id);
                }
            }
        }

        let query_agents: Vec<String> = self.queries.keys().cloned().collect();
        for agent_id in query_agents {
            if let Some(queries) = self.queries.get_mut(&agent_id) {
                let before = queries.len();
                queries.retain(|q| now - q.timestamp < LEARNING_TTL_SECONDS);
                queries_pruned += before - queries.len();
                if queries.is_empty() {
                    self.queries.remove(&agent_id);
                }
            }
        }

        ResonancePruneStats {
            learnings_pruned,
            queries_pruned,
        }
    }

    /// Get recent resonance hits (last 10).
    pub fn recent_resonances(&self) -> &[ResonanceHit] {
        let start = self.recent_resonances.len().saturating_sub(10);
        &self.recent_resonances[start..]
    }

    /// Get statistics.
    pub fn stats(&self) -> &ResonanceStats {
        &self.stats
    }

    /// Get the resonance threshold.
    pub fn threshold(&self) -> f64 {
        self.threshold
    }
}

impl Default for ResonanceEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// Resonance engine statistics.
#[derive(Debug, Clone, Default)]
pub struct ResonanceStats {
    pub learnings_tracked: usize,
    pub queries_tracked: usize,
    pub resonances_detected: usize,
}

/// Pruning statistics.
#[derive(Debug, Clone, Default)]
pub struct ResonancePruneStats {
    pub learnings_pruned: usize,
    pub queries_pruned: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_record_learning() {
        let mut engine = ResonanceEngine::new();
        let emb = vec![0.1; 64];

        let hits = engine.record_learning("agent-a", "learned about rust", &emb);
        assert!(hits.is_empty()); // No queries to match against
        assert_eq!(engine.stats().learnings_tracked, 1);
    }

    #[test]
    fn test_resonance_detection() {
        let mut engine = ResonanceEngine::with_threshold(0.5);
        let emb = vec![1.0, 0.0, 0.0, 0.0];

        // Agent A queries about something
        engine.record_query("agent-a", "what is rust?", &emb);

        // Agent B learns about rust → should resonate
        let hits = engine.record_learning("agent-b", "rust is a systems language", &emb);

        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].source_agent, "agent-b");
        assert_eq!(hits[0].target_agent, "agent-a");
        assert!((hits[0].similarity - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_no_self_resonance() {
        let mut engine = ResonanceEngine::with_threshold(0.5);
        let emb = vec![1.0; 64];

        engine.record_query("agent-a", "query", &emb);
        let hits = engine.record_learning("agent-a", "learning", &emb);

        assert!(hits.is_empty()); // Should not resonate with self
    }

    #[test]
    fn test_prune_stale() {
        let mut engine = ResonanceEngine::new();
        let emb = vec![0.1; 32];

        engine.record_learning("a", "content", &emb);
        engine.record_query("a", "query", &emb);

        // Artificially age the entries
        if let Some(learnings) = engine.learnings.get_mut("a") {
            for l in learnings.iter_mut() {
                l.timestamp = current_time() - LEARNING_TTL_SECONDS * 2.0;
            }
        }
        if let Some(queries) = engine.queries.get_mut("a") {
            for q in queries.iter_mut() {
                q.timestamp = current_time() - LEARNING_TTL_SECONDS * 2.0;
            }
        }

        let pruned = engine.prune_stale();
        assert!(pruned.learnings_pruned > 0);
        assert!(pruned.queries_pruned > 0);
    }
}
