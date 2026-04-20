//! Topology transitions — data-driven rules for mid-run shape changes.
//!
//! The collaboration topology isn't fixed for the lifetime of a run.
//! Transition rules define conditions under which the shape should change:
//! swarm → huddle when evidence clusters, huddle → panel when contradictions
//! spike, panel → synthesis when stability is reached.
//!
//! Rules are data, not code — serializable, inspectable, configurable.
//! The runtime evaluates them each cycle and applies the first match.

use serde::{Deserialize, Serialize};

use crate::collaboration::{
    CollaborationCharter, CollaborationDiscipline, CollaborationRole, CollaborationTopology,
    ConsensusRule, TurnCadence,
};

/// A named condition under which the collaboration shape should change.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransitionTrigger {
    EvidenceClustering {
        stable_fact_ratio: f64,
        min_stable_cycles: u32,
    },
    ContradictionSpike {
        contradiction_ratio: f64,
        min_contradictions: usize,
    },
    StabilityReached {
        min_stable_cycles: u32,
        min_hypotheses: usize,
    },
    BudgetPressure {
        remaining_fraction: f64,
    },
    ConsensusDeadlock {
        failed_vote_count: u32,
    },
}

/// What shape to transition to, and why.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransitionRule {
    pub name: String,
    pub trigger: TransitionTrigger,
    pub from: Option<CollaborationTopology>,
    pub to: CollaborationTopology,
    pub charter_adjustments: CharterAdjustments,
    pub rationale: String,
}

/// Partial charter override applied during a transition.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CharterAdjustments {
    pub consensus_rule: Option<ConsensusRule>,
    pub discipline: Option<CollaborationDiscipline>,
    pub turn_cadence: Option<TurnCadence>,
    pub require_dissent_map: Option<bool>,
    pub require_done_gate: Option<bool>,
    pub add_roles: Vec<CollaborationRole>,
    pub minimum_members: Option<usize>,
}

/// Observable convergence state fed to transition evaluation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConvergenceSignals {
    pub current_topology: CollaborationTopology,
    pub cycle_count: u32,
    pub hypothesis_count: usize,
    pub stable_hypothesis_count: usize,
    pub contradiction_count: usize,
    pub failed_vote_count: u32,
    pub budget_remaining_fraction: f64,
    pub stable_cycles: u32,
}

/// The result of a matched transition rule.
#[derive(Debug, Clone)]
pub struct TransitionDecision {
    pub rule: TransitionRule,
    pub new_charter: CollaborationCharter,
    pub signals_snapshot: ConvergenceSignals,
}

/// Evaluate transition rules against current signals.
/// Returns the first matching rule (rules are priority-ordered).
pub fn evaluate_transitions(
    current_charter: &CollaborationCharter,
    signals: &ConvergenceSignals,
    rules: &[TransitionRule],
) -> Option<TransitionDecision> {
    for rule in rules {
        if let Some(from) = rule.from
            && from != signals.current_topology
        {
            continue;
        }

        if signals.current_topology == rule.to {
            continue;
        }

        if trigger_matches(&rule.trigger, signals) {
            let new_charter =
                apply_adjustments(current_charter, &rule.charter_adjustments, rule.to);
            return Some(TransitionDecision {
                rule: rule.clone(),
                new_charter,
                signals_snapshot: signals.clone(),
            });
        }
    }

    None
}

/// Apply charter adjustments to produce a new charter.
pub fn apply_adjustments(
    base: &CollaborationCharter,
    adjustments: &CharterAdjustments,
    new_topology: CollaborationTopology,
) -> CollaborationCharter {
    let mut charter = base.clone();
    charter.topology = new_topology;

    if let Some(rule) = adjustments.consensus_rule {
        charter.consensus_rule = rule;
    }
    if let Some(discipline) = adjustments.discipline {
        charter.discipline = discipline;
    }
    if let Some(cadence) = adjustments.turn_cadence {
        charter.turn_cadence = cadence;
    }
    if let Some(dissent) = adjustments.require_dissent_map {
        charter.require_dissent_map = dissent;
    }
    if let Some(gate) = adjustments.require_done_gate {
        charter.require_done_gate = gate;
    }
    if let Some(min) = adjustments.minimum_members {
        charter.minimum_members = min;
    }

    for role in &adjustments.add_roles {
        if !charter.expected_roles.contains(role) {
            charter.expected_roles.push(*role);
        }
    }

    charter
}

#[allow(clippy::cast_precision_loss)]
fn trigger_matches(trigger: &TransitionTrigger, signals: &ConvergenceSignals) -> bool {
    match trigger {
        TransitionTrigger::EvidenceClustering {
            stable_fact_ratio,
            min_stable_cycles,
        } => {
            if signals.hypothesis_count == 0 {
                return false;
            }
            let ratio = signals.stable_hypothesis_count as f64 / signals.hypothesis_count as f64;
            ratio >= *stable_fact_ratio && signals.stable_cycles >= *min_stable_cycles
        }
        TransitionTrigger::ContradictionSpike {
            contradiction_ratio,
            min_contradictions,
        } => {
            if signals.hypothesis_count == 0 {
                return false;
            }
            let ratio = signals.contradiction_count as f64 / signals.hypothesis_count as f64;
            ratio >= *contradiction_ratio && signals.contradiction_count >= *min_contradictions
        }
        TransitionTrigger::StabilityReached {
            min_stable_cycles,
            min_hypotheses,
        } => {
            signals.stable_cycles >= *min_stable_cycles
                && signals.hypothesis_count >= *min_hypotheses
        }
        TransitionTrigger::BudgetPressure { remaining_fraction } => {
            signals.budget_remaining_fraction <= *remaining_fraction
        }
        TransitionTrigger::ConsensusDeadlock { failed_vote_count } => {
            signals.failed_vote_count >= *failed_vote_count
        }
    }
}

/// Default transition rules for common patterns.
pub fn default_transition_rules() -> Vec<TransitionRule> {
    vec![
        TransitionRule {
            name: "swarm-to-huddle".into(),
            trigger: TransitionTrigger::EvidenceClustering {
                stable_fact_ratio: 0.6,
                min_stable_cycles: 2,
            },
            from: Some(CollaborationTopology::SelfOrganizing),
            to: CollaborationTopology::Huddle,
            charter_adjustments: CharterAdjustments {
                discipline: Some(CollaborationDiscipline::Enforced),
                turn_cadence: Some(TurnCadence::RoundRobin),
                require_done_gate: Some(true),
                add_roles: vec![
                    CollaborationRole::Lead,
                    CollaborationRole::Critic,
                    CollaborationRole::Synthesizer,
                ],
                minimum_members: Some(3),
                ..CharterAdjustments::default()
            },
            rationale: "Evidence is clustering — tighten into a huddle for focused synthesis"
                .into(),
        },
        TransitionRule {
            name: "huddle-to-panel".into(),
            trigger: TransitionTrigger::ContradictionSpike {
                contradiction_ratio: 0.2,
                min_contradictions: 3,
            },
            from: Some(CollaborationTopology::Huddle),
            to: CollaborationTopology::Panel,
            charter_adjustments: CharterAdjustments {
                consensus_rule: Some(ConsensusRule::Supermajority),
                require_dissent_map: Some(true),
                add_roles: vec![CollaborationRole::Judge],
                ..CharterAdjustments::default()
            },
            rationale: "Contradictions spiking — escalate to panel for adversarial review".into(),
        },
        TransitionRule {
            name: "panel-to-synthesis".into(),
            trigger: TransitionTrigger::StabilityReached {
                min_stable_cycles: 3,
                min_hypotheses: 5,
            },
            from: Some(CollaborationTopology::Panel),
            to: CollaborationTopology::DiscussionGroup,
            charter_adjustments: CharterAdjustments {
                turn_cadence: Some(TurnCadence::SynthesisOnly),
                consensus_rule: Some(ConsensusRule::AdvisoryOnly),
                discipline: Some(CollaborationDiscipline::Moderated),
                require_done_gate: Some(true),
                ..CharterAdjustments::default()
            },
            rationale: "Stability reached — shift to synthesis mode for final report".into(),
        },
        TransitionRule {
            name: "budget-tighten".into(),
            trigger: TransitionTrigger::BudgetPressure {
                remaining_fraction: 0.2,
            },
            from: None,
            to: CollaborationTopology::Huddle,
            charter_adjustments: CharterAdjustments {
                discipline: Some(CollaborationDiscipline::Enforced),
                consensus_rule: Some(ConsensusRule::LeadDecides),
                turn_cadence: Some(TurnCadence::RoundRobin),
                require_done_gate: Some(true),
                ..CharterAdjustments::default()
            },
            rationale: "Budget pressure — tighten to huddle, lead decides to avoid waste".into(),
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_signals() -> ConvergenceSignals {
        ConvergenceSignals {
            current_topology: CollaborationTopology::SelfOrganizing,
            cycle_count: 5,
            hypothesis_count: 20,
            stable_hypothesis_count: 0,
            contradiction_count: 0,
            failed_vote_count: 0,
            budget_remaining_fraction: 0.8,
            stable_cycles: 0,
        }
    }

    fn base_charter() -> CollaborationCharter {
        CollaborationCharter::self_organizing()
    }

    #[test]
    fn evidence_clustering_fires_at_threshold() {
        let rules = default_transition_rules();
        let mut signals = base_signals();
        signals.stable_hypothesis_count = 14; // 14/20 = 0.7 >= 0.6
        signals.stable_cycles = 2;

        let decision = evaluate_transitions(&base_charter(), &signals, &rules);

        assert!(decision.is_some());
        let d = decision.unwrap();
        assert_eq!(d.rule.name, "swarm-to-huddle");
        assert_eq!(d.new_charter.topology, CollaborationTopology::Huddle);
    }

    #[test]
    fn evidence_clustering_below_threshold_does_not_fire() {
        let rules = default_transition_rules();
        let mut signals = base_signals();
        signals.stable_hypothesis_count = 10; // 10/20 = 0.5 < 0.6
        signals.stable_cycles = 2;

        assert!(evaluate_transitions(&base_charter(), &signals, &rules).is_none());
    }

    #[test]
    fn contradiction_spike_fires_from_huddle() {
        let rules = default_transition_rules();
        let mut signals = base_signals();
        signals.current_topology = CollaborationTopology::Huddle;
        signals.contradiction_count = 5; // 5/20 = 0.25 >= 0.2
        signals.hypothesis_count = 20;

        let decision = evaluate_transitions(&CollaborationCharter::huddle(), &signals, &rules);

        assert!(decision.is_some());
        let d = decision.unwrap();
        assert_eq!(d.rule.name, "huddle-to-panel");
        assert_eq!(d.new_charter.topology, CollaborationTopology::Panel);
        assert!(d.new_charter.require_dissent_map);
    }

    #[test]
    fn contradiction_spike_does_not_fire_from_swarm() {
        let rules = default_transition_rules();
        let mut signals = base_signals();
        signals.contradiction_count = 10;

        assert!(
            evaluate_transitions(&base_charter(), &signals, &rules).is_none()
                || evaluate_transitions(&base_charter(), &signals, &rules)
                    .unwrap()
                    .rule
                    .name
                    != "huddle-to-panel"
        );
    }

    #[test]
    fn stability_fires_panel_to_synthesis() {
        let rules = default_transition_rules();
        let mut signals = base_signals();
        signals.current_topology = CollaborationTopology::Panel;
        signals.stable_cycles = 3;
        signals.hypothesis_count = 10;

        let decision = evaluate_transitions(&CollaborationCharter::panel(), &signals, &rules);

        assert!(decision.is_some());
        let d = decision.unwrap();
        assert_eq!(d.rule.name, "panel-to-synthesis");
        assert_eq!(d.new_charter.turn_cadence, TurnCadence::SynthesisOnly);
    }

    #[test]
    fn budget_pressure_fires_from_any_topology() {
        let rules = default_transition_rules();
        let mut signals = base_signals();
        signals.current_topology = CollaborationTopology::Panel;
        signals.budget_remaining_fraction = 0.15;

        let decision = evaluate_transitions(&CollaborationCharter::panel(), &signals, &rules);

        assert!(decision.is_some());
        let d = decision.unwrap();
        assert_eq!(d.rule.name, "budget-tighten");
        assert_eq!(d.new_charter.consensus_rule, ConsensusRule::LeadDecides);
    }

    #[test]
    fn budget_pressure_does_not_fire_if_already_huddle() {
        let rules = default_transition_rules();
        let mut signals = base_signals();
        signals.current_topology = CollaborationTopology::Huddle;
        signals.budget_remaining_fraction = 0.15;

        // budget-tighten targets Huddle, and current is already Huddle → skipped
        // But swarm-to-huddle won't fire either (wrong source topology for clustering)
        let decision = evaluate_transitions(&CollaborationCharter::huddle(), &signals, &rules);

        // Should be None since all rules either don't match source or target == current
        assert!(decision.is_none());
    }

    #[test]
    fn first_matching_rule_wins() {
        let rules = vec![
            TransitionRule {
                name: "first".into(),
                trigger: TransitionTrigger::BudgetPressure {
                    remaining_fraction: 0.5,
                },
                from: None,
                to: CollaborationTopology::Huddle,
                charter_adjustments: CharterAdjustments::default(),
                rationale: "first rule".into(),
            },
            TransitionRule {
                name: "second".into(),
                trigger: TransitionTrigger::BudgetPressure {
                    remaining_fraction: 0.8,
                },
                from: None,
                to: CollaborationTopology::Panel,
                charter_adjustments: CharterAdjustments::default(),
                rationale: "second rule".into(),
            },
        ];

        let mut signals = base_signals();
        signals.budget_remaining_fraction = 0.3;

        let decision = evaluate_transitions(&base_charter(), &signals, &rules).unwrap();
        assert_eq!(decision.rule.name, "first");
    }

    #[test]
    fn apply_adjustments_merges_roles() {
        let base = CollaborationCharter::self_organizing();
        let adjustments = CharterAdjustments {
            add_roles: vec![CollaborationRole::Critic, CollaborationRole::Judge],
            ..CharterAdjustments::default()
        };

        let result = apply_adjustments(&base, &adjustments, CollaborationTopology::Panel);

        assert!(result.expected_roles.contains(&CollaborationRole::Critic));
        assert!(result.expected_roles.contains(&CollaborationRole::Judge));
        assert!(
            result
                .expected_roles
                .contains(&CollaborationRole::Generalist)
        );
    }

    #[test]
    fn apply_adjustments_does_not_duplicate_roles() {
        let base = CollaborationCharter::huddle();
        let adjustments = CharterAdjustments {
            add_roles: vec![CollaborationRole::Lead, CollaborationRole::Critic],
            ..CharterAdjustments::default()
        };

        let result = apply_adjustments(&base, &adjustments, CollaborationTopology::Panel);

        let lead_count = result
            .expected_roles
            .iter()
            .filter(|r| **r == CollaborationRole::Lead)
            .count();
        assert_eq!(lead_count, 1);
    }

    #[test]
    fn default_rules_produce_four_canonical_transitions() {
        let rules = default_transition_rules();
        assert_eq!(rules.len(), 4);
        assert_eq!(rules[0].name, "swarm-to-huddle");
        assert_eq!(rules[1].name, "huddle-to-panel");
        assert_eq!(rules[2].name, "panel-to-synthesis");
        assert_eq!(rules[3].name, "budget-tighten");
    }

    // ── Negative tests ────────────────────────────────────────────

    #[test]
    fn empty_rules_returns_none() {
        let signals = base_signals();
        assert!(evaluate_transitions(&base_charter(), &signals, &[]).is_none());
    }

    #[test]
    fn no_match_returns_none() {
        let rules = default_transition_rules();
        let signals = base_signals(); // nothing triggered

        assert!(evaluate_transitions(&base_charter(), &signals, &rules).is_none());
    }

    #[test]
    fn zero_hypotheses_never_triggers_ratio_based_rules() {
        let rules = default_transition_rules();
        let mut signals = base_signals();
        signals.hypothesis_count = 0;
        signals.contradiction_count = 100; // would be high ratio, but div by zero guarded

        assert!(evaluate_transitions(&base_charter(), &signals, &rules).is_none());
    }

    #[test]
    fn consensus_deadlock_trigger() {
        let rules = vec![TransitionRule {
            name: "deadlock-escalate".into(),
            trigger: TransitionTrigger::ConsensusDeadlock {
                failed_vote_count: 3,
            },
            from: None,
            to: CollaborationTopology::Panel,
            charter_adjustments: CharterAdjustments {
                consensus_rule: Some(ConsensusRule::LeadDecides),
                ..CharterAdjustments::default()
            },
            rationale: "Deadlocked — escalate to panel, let lead decide".into(),
        }];

        let mut signals = base_signals();
        signals.failed_vote_count = 3;

        let decision = evaluate_transitions(&base_charter(), &signals, &rules).unwrap();
        assert_eq!(decision.rule.name, "deadlock-escalate");
    }

    // ── Proptests ─────────────────────────────────────────────────

    #[allow(clippy::cast_precision_loss)]
    mod proptests {
        use super::*;
        use proptest::prelude::*;

        fn arb_topology() -> impl Strategy<Value = CollaborationTopology> {
            prop_oneof![
                Just(CollaborationTopology::Huddle),
                Just(CollaborationTopology::DiscussionGroup),
                Just(CollaborationTopology::Panel),
                Just(CollaborationTopology::SelfOrganizing),
            ]
        }

        fn arb_signals() -> impl Strategy<Value = ConvergenceSignals> {
            (
                arb_topology(),
                0_u32..100,
                0_usize..200,
                0_usize..200,
                0_usize..100,
                0_u32..20,
                0.0..=1.0_f64,
                0_u32..50,
            )
                .prop_map(
                    |(topo, cycles, hyp, stable, contra, failed, budget, stable_c)| {
                        ConvergenceSignals {
                            current_topology: topo,
                            cycle_count: cycles,
                            hypothesis_count: hyp,
                            stable_hypothesis_count: stable.min(hyp),
                            contradiction_count: contra.min(hyp),
                            failed_vote_count: failed,
                            budget_remaining_fraction: budget,
                            stable_cycles: stable_c,
                        }
                    },
                )
        }

        proptest! {
            #[test]
            fn evaluate_never_panics(signals in arb_signals()) {
                let rules = default_transition_rules();
                let charter = CollaborationCharter::self_organizing();
                let _ = evaluate_transitions(&charter, &signals, &rules);
            }

            #[test]
            fn apply_adjustments_never_panics(topology in arb_topology()) {
                let base = CollaborationCharter::self_organizing();
                let adjustments = CharterAdjustments {
                    consensus_rule: Some(ConsensusRule::Unanimous),
                    discipline: Some(CollaborationDiscipline::Enforced),
                    add_roles: vec![CollaborationRole::Judge, CollaborationRole::Critic],
                    ..CharterAdjustments::default()
                };
                let result = apply_adjustments(&base, &adjustments, topology);
                prop_assert_eq!(result.topology, topology);
            }
        }
    }
}
