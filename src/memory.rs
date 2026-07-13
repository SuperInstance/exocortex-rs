//! Memory Layer — three-tier storage with exponential confidence decay.
//!
//! Hot (LRU) → Warm (active) → Cold (archive)
//!
//! Every memory has a half-life. Confidence decays exponentially.
//! Recall reinforces memories, bumping them back to the hot tier.

use crate::types::*;
use std::collections::HashMap;

/// Tier thresholds.
pub const HOT_WINDOW_SECONDS: f64 = 60.0;
pub const WARM_UNREINFORCED_HOURS: f64 = 24.0;
pub const COLD_CONFIDENCE_THRESHOLD: f64 = 0.1;
pub const LRU_MAX: usize = 500;

/// A local memory store with hot/warm/cold tiers.
///
/// In the Python version, this maps to SurrealDB or in-memory dicts.
/// Here, it's a pure in-memory store suitable for embedding in agents.
pub struct MemoryStore {
    /// Hot tier: recently accessed (LRU-style).
    hot: HashMap<String, MemoryEntry>,
    /// Warm tier: all active memories.
    warm: HashMap<String, MemoryEntry>,
    /// Cold tier: archived memories.
    cold: HashMap<String, MemoryEntry>,
    /// Embedding cache for fast similarity search.
    embed_cache: HashMap<String, Vec<f64>>,
    /// Hot tier access order (for LRU eviction).
    hot_order: Vec<String>,
}

impl MemoryStore {
    /// Create a new empty memory store.
    pub fn new() -> Self {
        Self {
            hot: HashMap::new(),
            warm: HashMap::new(),
            cold: HashMap::new(),
            embed_cache: HashMap::new(),
            hot_order: Vec::new(),
        }
    }

    /// Store a new memory. Goes into hot + warm tiers.
    ///
    /// Returns the memory ID.
    pub fn remember(
        &mut self,
        content: &str,
        embedding: Vec<f64>,
        agent_id: &str,
        tags: &[&str],
    ) -> String {
        let entry = MemoryEntry::new_tagged(content, embedding.clone(), agent_id, tags);

        let id = entry.id.clone();

        // Hot tier
        self.hot.insert(id.clone(), entry.clone());
        self.hot_order.push(id.clone());
        if self.hot.len() > LRU_MAX {
            if let Some(evicted) = self.hot_order.first().cloned() {
                self.hot.remove(&evicted);
                self.hot_order.retain(|x| x != &evicted);
            }
        }

        // Warm tier
        self.warm.insert(id.clone(), entry);

        // Embedding cache
        self.embed_cache.insert(id.clone(), embedding);

        id
    }

    /// Find similar memories by embedding cosine similarity.
    ///
    /// Returns (entry_ref, similarity) pairs, sorted by similarity descending.
    pub fn recall(&self, query_embedding: &[f64], top_k: usize) -> Vec<(&MemoryEntry, f64)> {
        let mut results: Vec<(&MemoryEntry, f64)> = Vec::new();

        for (mid, emb) in &self.embed_cache {
            let entry = self
                .hot
                .get(mid)
                .or_else(|| self.warm.get(mid))
                .or_else(|| self.cold.get(mid));

            if let Some(entry) = entry {
                let eff_conf = entry.effective_confidence();
                if eff_conf < 0.01 {
                    continue;
                }
                let sim = cosine_similarity(query_embedding, emb);
                results.push((entry, sim));
            }
        }

        // Sort by similarity descending
        results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        results.truncate(top_k);
        results
    }

    /// Recall and reinforce memories (mutable version).
    pub fn recall_and_reinforce(
        &mut self,
        query_embedding: &[f64],
        top_k: usize,
    ) -> Vec<(String, String, f64)> {
        // First, collect IDs and similarities
        let matches: Vec<(String, f64)> = {
            let results = self.recall(query_embedding, top_k);
            results
                .into_iter()
                .map(|(e, sim)| (e.id.clone(), sim))
                .collect()
        };

        // Then reinforce
        let mut output = Vec::new();
        for (id, sim) in matches {
            let content = self.reinforce_by_id(&id);
            if let Some(c) = content {
                output.push((id, c, sim));
            }
        }
        output
    }

    /// Tag-based query.
    pub fn query_by_tags(&self, tags: &[&str], top_k: usize) -> Vec<&MemoryEntry> {
        let mut results: Vec<&MemoryEntry> = Vec::new();

        for entry in self.warm.values() {
            if tags.iter().any(|t| entry.tags.iter().any(|et| et == t)) {
                results.push(entry);
            }
        }

        // Sort by effective confidence descending
        results.sort_by(|a, b| {
            b.effective_confidence()
                .partial_cmp(&a.effective_confidence())
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        results.truncate(top_k);
        results
    }

    /// Get a memory by ID (checks all tiers).
    pub fn get(&self, memory_id: &str) -> Option<&MemoryEntry> {
        self.hot
            .get(memory_id)
            .or_else(|| self.warm.get(memory_id))
            .or_else(|| self.cold.get(memory_id))
    }

    /// Get a mutable memory by ID.
    pub fn get_mut(&mut self, memory_id: &str) -> Option<&mut MemoryEntry> {
        // Check hot first, then warm, then cold
        if self.hot.contains_key(memory_id) {
            return self.hot.get_mut(memory_id);
        }
        if self.warm.contains_key(memory_id) {
            return self.warm.get_mut(memory_id);
        }
        if self.cold.contains_key(memory_id) {
            return self.cold.get_mut(memory_id);
        }
        None
    }

    /// Run cooling cycle. Move hot→warm→cold based on age and confidence.
    ///
    /// Returns stats about the cooling.
    pub fn tick(&mut self) -> MemoryTickStats {
        let now = current_time();

        // Hot → Warm (age > 60s)
        let mut cooled_to_warm = 0;
        let to_cool: Vec<String> = self
            .hot
            .iter()
            .filter(|(_, e)| now - e.last_reinforced > HOT_WINDOW_SECONDS)
            .map(|(id, _)| id.clone())
            .collect();

        for id in &to_cool {
            self.hot.remove(id);
            self.hot_order.retain(|x| x != id);
            cooled_to_warm += 1;
        }

        // Warm → Cold (unreinforced > 24h or low confidence)
        let mut cooled_to_cold = 0;
        let to_archive: Vec<String> = self
            .warm
            .iter()
            .filter(|(_, e)| {
                now - e.last_reinforced > WARM_UNREINFORCED_HOURS * 3600.0
                    || e.effective_confidence() < COLD_CONFIDENCE_THRESHOLD
            })
            .map(|(id, _)| id.clone())
            .collect();

        for id in to_archive {
            if let Some(entry) = self.warm.remove(&id) {
                self.cold.insert(id, entry);
                cooled_to_cold += 1;
            }
        }

        // Prune cold (confidence < 0.05)
        let mut pruned = 0;
        let to_prune: Vec<String> = self
            .cold
            .iter()
            .filter(|(_, e)| e.effective_confidence() < 0.05)
            .map(|(id, _)| id.clone())
            .collect();

        for id in &to_prune {
            self.cold.remove(id);
            self.embed_cache.remove(id);
            pruned += 1;
        }

        MemoryTickStats {
            hot: self.hot.len(),
            warm: self.warm.len(),
            cold: self.cold.len(),
            total: self.hot.len() + self.warm.len() + self.cold.len(),
            cooled_to_warm,
            cooled_to_cold,
            pruned,
        }
    }

    /// Total number of memories across all tiers.
    pub fn total(&self) -> usize {
        self.hot.len() + self.warm.len() + self.cold.len()
    }

    /// Get memory stats.
    pub fn stats(&self) -> MemoryStats {
        MemoryStats {
            hot: self.hot.len(),
            warm: self.warm.len(),
            cold: self.cold.len(),
            total: self.total(),
        }
    }

    /// Reinforce a memory by ID (bumps last_reinforced).
    fn reinforce_by_id(&mut self, memory_id: &str) -> Option<String> {
        let now = current_time();

        // Check each tier
        if let Some(entry) = self.hot.get_mut(memory_id) {
            entry.last_reinforced = now;
            return Some(entry.content.clone());
        }
        if let Some(entry) = self.warm.get_mut(memory_id) {
            entry.last_reinforced = now;
            let clone = entry.clone();
            // Reheat to hot
            self.hot.insert(memory_id.to_string(), clone);
            self.hot_order.push(memory_id.to_string());
            return Some(entry.content.clone());
        }
        if let Some(entry) = self.cold.get_mut(memory_id) {
            entry.last_reinforced = now;
            let clone = entry.clone();
            // Reheat cold → hot
            self.hot.insert(memory_id.to_string(), clone);
            self.hot_order.push(memory_id.to_string());
            return Some(entry.content.clone());
        }
        None
    }
}

impl Default for MemoryStore {
    fn default() -> Self {
        Self::new()
    }
}

/// Memory tier statistics.
#[derive(Debug, Clone, Default)]
pub struct MemoryStats {
    pub hot: usize,
    pub warm: usize,
    pub cold: usize,
    pub total: usize,
}

/// Memory cooling cycle statistics.
#[derive(Debug, Clone, Default)]
pub struct MemoryTickStats {
    pub hot: usize,
    pub warm: usize,
    pub cold: usize,
    pub total: usize,
    pub cooled_to_warm: usize,
    pub cooled_to_cold: usize,
    pub pruned: usize,
}

/// A standalone shared-memory helper — a thin wrapper around [`MemoryStore`].
///
/// **Status (honest):** This type is exported so callers can build a
/// process-wide memory pool, but `AgentSpace` does **not** currently wire
/// one up. Each [`Agent`](crate::Agent) owns a private `MemoryStore`. Code
/// that wants a shared layer must construct a `MemoryLayer` and route
/// remember/recall through it explicitly. The Python counterpart's
/// `MemoryLayer` is plumbed through the space; that wiring is not yet
/// ported.
pub struct MemoryLayer {
    store: MemoryStore,
}

impl MemoryLayer {
    pub fn new() -> Self {
        Self {
            store: MemoryStore::new(),
        }
    }

    pub fn remember(
        &mut self,
        content: &str,
        embedding: Vec<f64>,
        agent_id: &str,
        tags: &[&str],
    ) -> String {
        self.store.remember(content, embedding, agent_id, tags)
    }

    pub fn recall(&self, query_embedding: &[f64], top_k: usize) -> Vec<(&MemoryEntry, f64)> {
        self.store.recall(query_embedding, top_k)
    }

    pub fn query_by_tags(&self, tags: &[&str], top_k: usize) -> Vec<&MemoryEntry> {
        self.store.query_by_tags(tags, top_k)
    }

    pub fn get(&self, memory_id: &str) -> Option<&MemoryEntry> {
        self.store.get(memory_id)
    }

    pub fn tick(&mut self) -> MemoryTickStats {
        self.store.tick()
    }

    pub fn stats(&self) -> MemoryStats {
        self.store.stats()
    }

    pub fn total(&self) -> usize {
        self.store.total()
    }
}

impl Default for MemoryLayer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_remember_and_recall() {
        let mut store = MemoryStore::new();
        let emb = vec![0.1; 128];
        let id = store.remember("hello world", emb.clone(), "test", &["greeting"]);
        assert!(!id.is_empty());

        let results = store.recall(&emb, 5);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0.content, "hello world");
    }

    #[test]
    fn test_tag_query() {
        let mut store = MemoryStore::new();
        store.remember("garden data", vec![0.1; 64], "agent", &["garden", "iot"]);
        store.remember("dev data", vec![0.2; 64], "agent", &["devops"]);

        let results = store.query_by_tags(&["garden"], 10);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].content, "garden data");
    }

    #[test]
    fn test_tick_cooling() {
        let mut store = MemoryStore::new();
        store.remember("old memory", vec![0.0; 64], "test", &[]);

        // Artificially age the memory
        let keys: Vec<String> = store.warm.keys().cloned().collect();
        for key in keys {
            if let Some(entry) = store.warm.get_mut(&key) {
                entry.last_reinforced = current_time() - 86400.0 * 2.0;
            }
        }

        let stats = store.tick();
        assert!(stats.cooled_to_cold >= 1);
    }

    #[test]
    fn test_stats() {
        let mut store = MemoryStore::new();
        store.remember("m1", vec![0.1; 64], "a", &[]);
        store.remember("m2", vec![0.2; 64], "a", &[]);
        store.remember("m3", vec![0.3; 64], "a", &[]);

        let stats = store.stats();
        assert!(stats.total >= 3);
    }

    #[test]
    fn test_recall_and_reinforce_reheats_to_hot() {
        // recall_and_reinforce must (1) return matching memories with their
        // similarity, (2) bump last_reinforced, and (3) reheat warm/cold
        // entries back into the hot tier.
        let mut store = MemoryStore::new();
        let emb = vec![0.4; 32];
        let id = store.remember("fact", emb.clone(), "agent", &[]);

        // Force-cool the memory into the warm tier by aging it past the
        // hot window, then running a tick.
        if let Some(entry) = store.warm.get_mut(&id) {
            entry.last_reinforced = current_time() - (HOT_WINDOW_SECONDS + 10.0);
        }
        // Also need the hot-tier copy to be aged so tick() moves it out.
        if let Some(entry) = store.hot.get_mut(&id) {
            entry.last_reinforced = current_time() - (HOT_WINDOW_SECONDS + 10.0);
        }
        let tick_stats = store.tick();
        assert!(
            tick_stats.cooled_to_warm >= 1,
            "expected hot→warm transition, got {tick_stats:?}"
        );
        // After cooling, the memory is in warm only.
        assert!(!store.hot.contains_key(&id));
        assert!(store.warm.contains_key(&id));

        // recall_and_reinforce should bring it back to hot.
        let results = store.recall_and_reinforce(&emb, 5);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0, id);
        assert_eq!(results[0].1, "fact");
        assert!(
            store.hot.contains_key(&id),
            "memory should be reheated to hot"
        );
    }

    #[test]
    fn test_tick_prunes_decayed_cold_memories() {
        // Cold-tier memories whose effective confidence falls below 0.05
        // must be pruned on tick, and their embedding-cache entries removed.
        let mut store = MemoryStore::new();
        let id = store.remember("ephemeral", vec![0.5; 16], "a", &[]);

        // Move the memory straight into the cold tier with confidence low
        // enough that effective_confidence < 0.05 (the prune threshold).
        let ancient = current_time() - 86400.0 * 365.0; // ~1 year
        let mut entry = store.warm.remove(&id).unwrap();
        entry.last_reinforced = ancient;
        entry.confidence = 0.01;
        store.cold.insert(id.clone(), entry);
        // Clear the hot copy too so the tick has nothing else to do.
        store.hot.remove(&id);
        store.hot_order.retain(|x| x != &id);

        let before = store.total();
        assert!(store.cold.contains_key(&id));

        let prune_stats = store.tick();
        assert!(
            prune_stats.pruned >= 1,
            "expected at least one prune, got {prune_stats:?}"
        );
        assert_eq!(store.total(), before - prune_stats.pruned);
        assert!(store.get(&id).is_none(), "decayed memory should be gone");
        assert!(
            !store.embed_cache.contains_key(&id),
            "embedding cache entry should be removed on prune"
        );
    }

    #[test]
    fn test_get_mut_updates_are_visible_via_get() {
        let mut store = MemoryStore::new();
        let id = store.remember("orig", vec![0.1; 8], "a", &[]);

        if let Some(entry) = store.get_mut(&id) {
            entry.content = "edited".to_string();
        }
        assert_eq!(store.get(&id).unwrap().content, "edited");
    }
}
