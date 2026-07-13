//! Conservation-law aware decision making.
//!
//! Every agent decision is governed by five conservation laws, analogous
//! to the laws of thermodynamics. These ensure agents don't runaway,
//! over-commit, or violate system invariants.
//!
//! The laws:
//! 1. **Energy Conservation** — total decision energy is finite per cycle
//! 2. **Momentum Conservation** — agents resist sudden priority flips
//! 3. **Entropy Conservation** — novelty seeking is bounded
//! 4. **Information Conservation** — memories cannot be silently lost
//! 5. **Symmetry Conservation** — agent identity is preserved through transforms

/// Total decision energy budget per agent per cycle.
pub const DECISION_ENERGY_BUDGET: f64 = 100.0;

/// Maximum priority shift per cycle (momentum conservation).
pub const MAX_PRIORITY_SHIFT: f64 = 0.3;

/// Minimum novelty budget remaining before an agent must rest.
pub const MIN_NOVELTY_BUDGET: f64 = 0.15;

/// Maximum information loss rate (memories that can be pruned per cycle).
pub const MAX_INFORMATION_LOSS_RATE: f64 = 0.05;

/// Maximum identity drift (cosine distance) per transform.
pub const MAX_IDENTITY_DRIFT: f64 = 0.5;

/// The five conservation laws.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ConservationLaw {
    /// Total decision energy is conserved (finite budget per cycle).
    Energy,
    /// Agents resist sudden priority changes (inertia).
    Momentum,
    /// Novelty seeking is bounded (prevents thrashing).
    Entropy,
    /// Memories cannot be silently lost (accounting for all state changes).
    Information,
    /// Agent identity is preserved through transforms.
    Symmetry,
}

impl ConservationLaw {
    pub const ALL: [ConservationLaw; 5] = [
        ConservationLaw::Energy,
        ConservationLaw::Momentum,
        ConservationLaw::Entropy,
        ConservationLaw::Information,
        ConservationLaw::Symmetry,
    ];

    pub fn name(&self) -> &'static str {
        match self {
            ConservationLaw::Energy => "Energy Conservation",
            ConservationLaw::Momentum => "Momentum Conservation",
            ConservationLaw::Entropy => "Entropy Conservation",
            ConservationLaw::Information => "Information Conservation",
            ConservationLaw::Symmetry => "Symmetry Conservation",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            ConservationLaw::Energy => "Total decision energy is finite per cycle",
            ConservationLaw::Momentum => "Agents resist sudden priority flips",
            ConservationLaw::Entropy => "Novelty seeking is bounded to prevent thrashing",
            ConservationLaw::Information => "Memories cannot be silently lost",
            ConservationLaw::Symmetry => "Agent identity is preserved through transforms",
        }
    }
}

/// Conservation state tracked per agent.
#[derive(Debug, Clone)]
pub struct ConservationState {
    /// Remaining decision energy for this cycle.
    pub energy_remaining: f64,
    /// Total energy budget.
    pub energy_budget: f64,
    /// Last cycle's priority vector (for momentum).
    pub last_priorities: Vec<f64>,
    /// Current novelty budget (decreases with novel actions, recharges over time).
    pub novelty_budget: f64,
    /// Maximum novelty budget.
    pub max_novelty: f64,
    /// Number of memories pruned this cycle (for information accounting).
    pub pruned_this_cycle: usize,
    /// Identity vector (agent's core embedding).
    pub identity_vector: Vec<f64>,
    /// Total conservation violations.
    pub violations: usize,
}

impl ConservationState {
    /// Create a new conservation state with defaults.
    pub fn new() -> Self {
        Self {
            energy_remaining: DECISION_ENERGY_BUDGET,
            energy_budget: DECISION_ENERGY_BUDGET,
            last_priorities: Vec::new(),
            novelty_budget: 1.0,
            max_novelty: 1.0,
            pruned_this_cycle: 0,
            identity_vector: Vec::new(),
            violations: 0,
        }
    }

    /// Create with a custom energy budget.
    pub fn with_budget(energy_budget: f64) -> Self {
        Self {
            energy_remaining: energy_budget,
            energy_budget,
            ..Self::new()
        }
    }

    /// Create with an identity vector.
    pub fn with_identity(identity_vector: Vec<f64>) -> Self {
        Self {
            identity_vector,
            ..Self::new()
        }
    }

    /// Check if the agent can afford a decision costing `cost` energy.
    pub fn can_afford(&self, cost: f64) -> bool {
        self.energy_remaining >= cost
    }

    /// Spend decision energy. Returns false if insufficient.
    pub fn spend(&mut self, cost: f64) -> bool {
        if self.energy_remaining < cost {
            self.violations += 1;
            false
        } else {
            self.energy_remaining -= cost;
            true
        }
    }

    /// Check momentum conservation: would a new priority vector be too big a shift?
    pub fn check_momentum(&self, new_priorities: &[f64]) -> bool {
        if self.last_priorities.is_empty() {
            return true;
        }
        let shift = total_variation_distance(&self.last_priorities, new_priorities);
        shift <= MAX_PRIORITY_SHIFT
    }

    /// Update the priority history (call after committing to new priorities).
    pub fn update_priorities(&mut self, new_priorities: Vec<f64>) {
        self.last_priorities = new_priorities;
    }

    /// Check entropy conservation: can the agent afford a novel action?
    pub fn check_novelty(&self, novelty_cost: f64) -> bool {
        if novelty_cost <= 0.0 {
            return true;
        }
        self.novelty_budget >= novelty_cost
    }

    /// Spend novelty budget.
    pub fn spend_novelty(&mut self, cost: f64) -> bool {
        if self.novelty_budget < cost {
            self.violations += 1;
            false
        } else {
            self.novelty_budget -= cost;
            true
        }
    }

    /// Recharge novelty budget (called periodically).
    pub fn recharge_novelty(&mut self, amount: f64) {
        self.novelty_budget = (self.novelty_budget + amount).min(self.max_novelty);
    }

    /// Check information conservation: are we pruning too many memories?
    pub fn check_information_loss(&self, total_memories: usize) -> bool {
        if total_memories == 0 {
            return true;
        }
        let loss_rate = self.pruned_this_cycle as f64 / total_memories as f64;
        loss_rate <= MAX_INFORMATION_LOSS_RATE
    }

    /// Record a memory pruning event.
    pub fn record_prune(&mut self) {
        self.pruned_this_cycle += 1;
    }

    /// Reset per-cycle counters.
    pub fn reset_cycle(&mut self) {
        self.energy_remaining = self.energy_budget;
        self.pruned_this_cycle = 0;
        // Partial novelty recharge each cycle.
        let recharge = 0.1 * self.max_novelty;
        self.recharge_novelty(recharge);
    }

    /// Check symmetry conservation: would a transform cause too much identity drift?
    pub fn check_identity_drift(&self, new_vector: &[f64]) -> bool {
        if self.identity_vector.is_empty() || new_vector.is_empty() {
            return true;
        }
        let drift = 1.0 - crate::types::cosine_similarity(&self.identity_vector, new_vector);
        drift <= MAX_IDENTITY_DRIFT
    }

    /// Get a summary of conservation state.
    pub fn summary(&self) -> ConservationSummary {
        ConservationSummary {
            energy_pct: (self.energy_remaining / self.energy_budget * 100.0) as u8,
            novelty_pct: (self.novelty_budget / self.max_novelty * 100.0) as u8,
            pruned_this_cycle: self.pruned_this_cycle,
            violations: self.violations,
            energy_sufficient: self.energy_remaining > 0.0,
            novelty_sufficient: self.novelty_budget >= MIN_NOVELTY_BUDGET,
        }
    }
}

impl Default for ConservationState {
    fn default() -> Self {
        Self::new()
    }
}

/// Read-only summary of conservation state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConservationSummary {
    pub energy_pct: u8,
    pub novelty_pct: u8,
    pub pruned_this_cycle: usize,
    pub violations: usize,
    pub energy_sufficient: bool,
    pub novelty_sufficient: bool,
}

/// Total variation distance between two probability vectors.
fn total_variation_distance(a: &[f64], b: &[f64]) -> f64 {
    let len = a.len().min(b.len());
    if len == 0 {
        return 0.0;
    }
    let mut sum = 0.0;
    for i in 0..len {
        sum += (a[i] - b[i]).abs();
    }
    // TV distance is half the L1 distance
    sum / 2.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_energy_conservation() {
        let mut state = ConservationState::new();
        assert!(state.can_afford(50.0));
        assert!(state.spend(50.0));
        assert_eq!(state.energy_remaining, 50.0);
        assert!(state.spend(50.0));
        assert_eq!(state.energy_remaining, 0.0);
        assert!(!state.spend(1.0));
        assert_eq!(state.violations, 1);
    }

    #[test]
    fn test_cycle_reset() {
        let mut state = ConservationState::new();
        state.spend(80.0);
        assert_eq!(state.energy_remaining, 20.0);
        state.reset_cycle();
        assert_eq!(state.energy_remaining, DECISION_ENERGY_BUDGET);
    }

    #[test]
    fn test_novelty_budget() {
        let mut state = ConservationState::new();
        assert!(state.check_novelty(0.5));
        assert!(state.spend_novelty(0.8));
        // Spending more than remains must fail and record a violation.
        assert!(!state.spend_novelty(0.3));
        assert_eq!(state.violations, 1);
    }

    #[test]
    fn test_information_conservation() {
        let mut state = ConservationState::new();
        // 100 memories, can prune up to 5% = 5
        for _ in 0..5 {
            state.record_prune();
        }
        assert!(state.check_information_loss(100));
        state.record_prune();
        assert!(!state.check_information_loss(100));
    }
}
