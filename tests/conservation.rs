//! Conservation tests — verifying the five conservation laws.

use exocortex::*;

#[test]
fn test_all_five_conservation_laws_exist() {
    use exocortex::conservation::ConservationLaw;

    let laws = ConservationLaw::ALL;
    assert_eq!(laws.len(), 5);

    assert_eq!(laws[0], ConservationLaw::Energy);
    assert_eq!(laws[1], ConservationLaw::Momentum);
    assert_eq!(laws[2], ConservationLaw::Entropy);
    assert_eq!(laws[3], ConservationLaw::Information);
    assert_eq!(laws[4], ConservationLaw::Symmetry);
}

#[test]
fn test_law_descriptions() {
    use exocortex::conservation::ConservationLaw;

    for law in &ConservationLaw::ALL {
        assert!(!law.name().is_empty(), "Law has empty name");
        assert!(!law.description().is_empty(), "Law {:?} has empty description", law);
    }
}

// --- Energy Conservation ---

#[test]
fn test_energy_budget_default() {
    let state = ConservationState::new();
    assert!((state.energy_budget - conservation::DECISION_ENERGY_BUDGET).abs() < 0.001);
    assert!((state.energy_remaining - conservation::DECISION_ENERGY_BUDGET).abs() < 0.001);
}

#[test]
fn test_energy_spend() {
    let mut state = ConservationState::new();
    assert!(state.spend(50.0));
    assert!((state.energy_remaining - 50.0).abs() < 0.001);
}

#[test]
fn test_energy_exhaustion() {
    let mut state = ConservationState::new();
    state.spend(90.0);
    assert!(!state.can_afford(20.0));
    assert!(!state.spend(20.0));
    assert_eq!(state.violations, 1);
}

#[test]
fn test_energy_cycle_reset() {
    let mut state = ConservationState::new();
    state.spend(80.0);
    assert!((state.energy_remaining - 20.0).abs() < 0.001);

    state.reset_cycle();
    assert!((state.energy_remaining - conservation::DECISION_ENERGY_BUDGET).abs() < 0.001);
}

#[test]
fn test_custom_energy_budget() {
    let state = ConservationState::with_budget(500.0);
    assert!((state.energy_budget - 500.0).abs() < 0.001);
    assert!((state.energy_remaining - 500.0).abs() < 0.001);
}

// --- Entropy / Novelty Conservation ---

#[test]
fn test_novelty_budget_starts_full() {
    let state = ConservationState::new();
    assert!((state.novelty_budget - 1.0).abs() < 0.001);
}

#[test]
fn test_novelty_spend() {
    let mut state = ConservationState::new();
    assert!(state.check_novelty(0.5));
    state.spend_novelty(0.5);
    assert!((state.novelty_budget - 0.5).abs() < 0.001);
}

#[test]
fn test_novelty_exhaustion() {
    let mut state = ConservationState::new();
    state.spend_novelty(0.9);
    assert!(!state.check_novelty(0.2));
    assert_eq!(state.violations, 1);
}

#[test]
fn test_novelty_recharge() {
    let mut state = ConservationState::new();
    state.spend_novelty(0.8);
    state.recharge_novelty(0.5);
    // Should be min(0.2 + 0.5, 1.0) = 0.7
    assert!((state.novelty_budget - 0.7).abs() < 0.001);
}

#[test]
fn test_novelty_recharge_capped() {
    let mut state = ConservationState::new();
    state.spend_novelty(0.3);
    state.recharge_novelty(1.0); // over-recharge
    assert!((state.novelty_budget - 1.0).abs() < 0.001);
}

// --- Momentum Conservation ---

#[test]
fn test_momentum_first_decision() {
    let state = ConservationState::new();
    let priorities = vec![0.3, 0.3, 0.4];
    assert!(state.check_momentum(&priorities)); // No history → always allowed
}

#[test]
fn test_momentum_small_shift() {
    let mut state = ConservationState::new();
    state.update_priorities(vec![0.3, 0.3, 0.4]);
    // Small shift should be allowed
    assert!(state.check_momentum(&[0.35, 0.3, 0.35]));
}

#[test]
fn test_momentum_large_shift() {
    let mut state = ConservationState::new();
    state.update_priorities(vec![0.8, 0.1, 0.1]);
    // Large shift should violate momentum
    assert!(!state.check_momentum(&[0.1, 0.1, 0.8]));
}

// --- Information Conservation ---

#[test]
fn test_information_loss_within_bounds() {
    let mut state = ConservationState::new();
    for _ in 0..3 {
        state.record_prune();
    }
    // 3 pruned out of 100 → 3% loss rate
    assert!(state.check_information_loss(100));
}

#[test]
fn test_information_loss_exceeded() {
    let mut state = ConservationState::new();
    for _ in 0..10 {
        state.record_prune();
    }
    // 10 pruned out of 100 → 10% loss rate (exceeds 5%)
    assert!(!state.check_information_loss(100));
}

#[test]
fn test_information_loss_empty_store() {
    let state = ConservationState::new();
    assert!(state.check_information_loss(0));
}

#[test]
fn test_information_prune_resets_on_cycle() {
    let mut state = ConservationState::new();
    for _ in 0..5 {
        state.record_prune();
    }
    assert_eq!(state.pruned_this_cycle, 5);
    state.reset_cycle();
    assert_eq!(state.pruned_this_cycle, 0);
}

// --- Symmetry Conservation ---

#[test]
fn test_identity_drift_no_identity() {
    let state = ConservationState::new();
    // No identity vector set → no drift limit
    assert!(state.check_identity_drift(&[1.0, 0.0, 0.0]));
}

#[test]
fn test_identity_drift_small() {
    let mut state = ConservationState::new();
    state.identity_vector = vec![1.0, 0.0, 0.0, 0.0];

    // Very similar vector
    let new_vec = vec![0.99, 0.01, 0.0, 0.0];
    assert!(state.check_identity_drift(&new_vec));
}

#[test]
fn test_identity_drift_large() {
    let mut state = ConservationState::new();
    state.identity_vector = vec![1.0, 0.0, 0.0, 0.0];

    // Very different vector (cosine similarity ≈ 0)
    let new_vec = vec![0.0, 1.0, 0.0, 0.0];
    assert!(!state.check_identity_drift(&new_vec));
}

// --- Integration: Agent + Conservation ---

#[test]
fn test_agent_respects_energy_conservation() {
    let mut agent = Agent::builder("energy-test")
        .capability(Operation::Remember)
        .capability(Operation::Predict)
        .energy_budget(30.0)
        .build();

    // Make decisions until energy runs out
    let mut decisions = 0;
    loop {
        let result = agent.decide(Operation::Remember, 10.0, 0.0);
        if matches!(result, DecisionResult::Denied(_)) {
            break;
        }
        decisions += 1;
    }

    // Should have made 3 decisions (30/10 = 3)
    assert_eq!(decisions, 3);
}

#[test]
fn test_agent_respects_novelty_conservation() {
    let mut agent = Agent::builder("novelty-test")
        .capability(Operation::Predict)
        .build();

    // Make high-novelty decisions until novelty runs out
    let mut decisions = 0;
    loop {
        let result = agent.decide(Operation::Predict, 1.0, 0.3);
        if matches!(result, DecisionResult::Denied(_)) {
            break;
        }
        decisions += 1;
    }

    // Should have made 3 decisions (1.0 / 0.3 ≈ 3.33)
    assert_eq!(decisions, 3);
    assert_eq!(agent.conservation.violations, 0); // No violations, just clean denials
}

#[test]
fn test_agent_cycle_reset_restores_energy() {
    let mut agent = Agent::builder("cycle-test")
        .capability(Operation::Remember)
        .energy_budget(50.0)
        .build();

    // Exhaust energy
    agent.decide(Operation::Remember, 50.0, 0.0);
    assert!(!agent.conservation.can_afford(1.0));

    // Reset cycle
    agent.reset_cycle();

    // Should be able to decide again
    let result = agent.decide(Operation::Remember, 10.0, 0.0);
    assert!(matches!(result, DecisionResult::Approved(_)));
}

#[test]
fn test_agent_violation_tracking() {
    let mut agent = Agent::builder("violation-test")
        .capability(Operation::Remember)
        .energy_budget(10.0)
        .build();

    // Exhaust energy
    agent.conservation.spend(10.0);

    // Try to decide (should fail)
    let result = agent.decide(Operation::Remember, 5.0, 0.0);
    assert!(matches!(result, DecisionResult::Denied(_)));

    // Spend is not called on denial from can_afford check, so spend manually
    agent.conservation.spend(5.0); // This triggers a violation
    assert!(agent.conservation.violations >= 1);
}

#[test]
fn test_conservation_summary() {
    let mut state = ConservationState::new();
    state.spend(60.0);
    state.spend_novelty(0.5);

    let summary = state.summary();
    assert_eq!(summary.energy_pct, 40);
    assert_eq!(summary.novelty_pct, 50);
    assert!(summary.energy_sufficient);
    assert!(summary.novelty_sufficient);
}

#[test]
fn test_conservation_summary_depleted() {
    let mut state = ConservationState::new();
    state.spend(100.0);
    state.spend_novelty(0.95);

    let summary = state.summary();
    assert_eq!(summary.energy_pct, 0);
    assert!(!summary.energy_sufficient);
    assert!(!summary.novelty_sufficient);
}
