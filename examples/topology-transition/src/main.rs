// Copyright 2024-2026 Reflective Labs
// SPDX-License-Identifier: MIT

//! Topology Transition — mid-run shape changes driven by convergence signals.
//!
//! Simulates a convergence loop where signals evolve over 8 cycles.
//! The collaboration shape adapts: swarm → huddle → panel → synthesis.
//! Each transition fires when the data warrants it, not on a schedule.

use organism_pack::{
    CollaborationCharter, ConvergenceSignals, CollaborationTopology,
    default_transition_rules, evaluate_transitions,
};

fn main() {
    println!("=== Topology Transitions: Simulated Convergence Loop ===\n");

    let rules = default_transition_rules();

    println!("Loaded {} transition rules:", rules.len());
    for rule in &rules {
        println!(
            "  {} — {:?} → {:?}",
            rule.name,
            rule.from.map_or("any".to_string(), |t| format!("{t:?}")),
            rule.to,
        );
        println!("    {}", rule.rationale);
    }
    println!();

    // Simulate evolving signals over 8 cycles.
    let cycles = vec![
        // Cycle 1-2: Early exploration, swarm is discovering
        ConvergenceSignals {
            current_topology: CollaborationTopology::SelfOrganizing,
            cycle_count: 1,
            hypothesis_count: 5,
            stable_hypothesis_count: 1,
            contradiction_count: 0,
            failed_vote_count: 0,
            budget_remaining_fraction: 0.95,
            stable_cycles: 0,
        },
        ConvergenceSignals {
            current_topology: CollaborationTopology::SelfOrganizing,
            cycle_count: 2,
            hypothesis_count: 12,
            stable_hypothesis_count: 3,
            contradiction_count: 1,
            failed_vote_count: 0,
            budget_remaining_fraction: 0.85,
            stable_cycles: 0,
        },
        // Cycle 3: Evidence clusters — triggers swarm→huddle
        ConvergenceSignals {
            current_topology: CollaborationTopology::SelfOrganizing,
            cycle_count: 3,
            hypothesis_count: 20,
            stable_hypothesis_count: 14, // 70% stable
            contradiction_count: 2,
            failed_vote_count: 0,
            budget_remaining_fraction: 0.75,
            stable_cycles: 2,
        },
        // Cycle 4: Now in huddle, contradictions start appearing
        ConvergenceSignals {
            current_topology: CollaborationTopology::Huddle,
            cycle_count: 4,
            hypothesis_count: 25,
            stable_hypothesis_count: 18,
            contradiction_count: 3,
            failed_vote_count: 0,
            budget_remaining_fraction: 0.65,
            stable_cycles: 1,
        },
        // Cycle 5: Contradictions spike — triggers huddle→panel
        ConvergenceSignals {
            current_topology: CollaborationTopology::Huddle,
            cycle_count: 5,
            hypothesis_count: 28,
            stable_hypothesis_count: 20,
            contradiction_count: 7, // 25% contradiction rate
            failed_vote_count: 0,
            budget_remaining_fraction: 0.55,
            stable_cycles: 1,
        },
        // Cycle 6: Panel resolves contradictions, stability builds
        ConvergenceSignals {
            current_topology: CollaborationTopology::Panel,
            cycle_count: 6,
            hypothesis_count: 30,
            stable_hypothesis_count: 25,
            contradiction_count: 2,
            failed_vote_count: 0,
            budget_remaining_fraction: 0.40,
            stable_cycles: 2,
        },
        // Cycle 7: Stability reached — triggers panel→synthesis
        ConvergenceSignals {
            current_topology: CollaborationTopology::Panel,
            cycle_count: 7,
            hypothesis_count: 30,
            stable_hypothesis_count: 28,
            contradiction_count: 1,
            failed_vote_count: 0,
            budget_remaining_fraction: 0.30,
            stable_cycles: 3,
        },
        // Cycle 8: Now in synthesis mode, winding down
        ConvergenceSignals {
            current_topology: CollaborationTopology::DiscussionGroup,
            cycle_count: 8,
            hypothesis_count: 30,
            stable_hypothesis_count: 29,
            contradiction_count: 0,
            failed_vote_count: 0,
            budget_remaining_fraction: 0.20,
            stable_cycles: 4,
        },
    ];

    let mut charter = CollaborationCharter::self_organizing();
    let mut transition_log = Vec::new();

    println!("--- Convergence Loop ---\n");

    for signals in &cycles {
        let decision = evaluate_transitions(&charter, signals, &rules);

        let topo_label = format!("{:?}", signals.current_topology);
        print!(
            "  Cycle {:>2} | {:<18} | hyp={:>2} stable={:>2} contra={} budget={:.0}%",
            signals.cycle_count,
            topo_label,
            signals.hypothesis_count,
            signals.stable_hypothesis_count,
            signals.contradiction_count,
            signals.budget_remaining_fraction * 100.0,
        );

        if let Some(d) = decision {
            println!(" → {:?}", d.new_charter.topology);
            println!("           TRANSITION: {} — {}", d.rule.name, d.rule.rationale);
            transition_log.push((signals.cycle_count, d.rule.name.clone(), d.rule.rationale.clone()));
            charter = d.new_charter;
        } else {
            println!(" (stable)");
        }
    }

    println!();
    println!("--- Transition Log ---\n");
    if transition_log.is_empty() {
        println!("  No transitions fired.");
    } else {
        for (cycle, name, rationale) in &transition_log {
            println!("  Cycle {cycle}: {name}");
            println!("    {rationale}");
        }
    }

    // Show budget pressure override
    println!();
    println!("--- Budget Pressure Demo ---\n");
    println!("  What happens when budget drops to 15% in any topology?\n");

    for topo in [
        CollaborationTopology::SelfOrganizing,
        CollaborationTopology::DiscussionGroup,
        CollaborationTopology::Panel,
    ] {
        let signals = ConvergenceSignals {
            current_topology: topo,
            cycle_count: 10,
            hypothesis_count: 20,
            stable_hypothesis_count: 10,
            contradiction_count: 2,
            failed_vote_count: 0,
            budget_remaining_fraction: 0.15,
            stable_cycles: 1,
        };

        let base = match topo {
            CollaborationTopology::SelfOrganizing => CollaborationCharter::self_organizing(),
            CollaborationTopology::DiscussionGroup => CollaborationCharter::discussion_group(),
            CollaborationTopology::Panel => CollaborationCharter::panel(),
            _ => CollaborationCharter::huddle(),
        };

        let decision = evaluate_transitions(&base, &signals, &rules);
        if let Some(d) = decision {
            println!(
                "  {:?} → {:?} ({}, {:?})",
                topo, d.new_charter.topology, d.rule.name, d.new_charter.consensus_rule
            );
        }
    }
}
