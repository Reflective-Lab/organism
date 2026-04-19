// Copyright 2024-2026 Reflective Labs
// SPDX-License-Identifier: MIT

//! Shape Competition — the collaboration shape itself is a hypothesis.
//!
//! Multiple candidate shapes compete for the same intent. Each is scored
//! by evidence quality. The winner is selected, and the learning layer
//! calibrates priors so future derivations are informed by past outcomes.
//!
//! Over episodes, the system discovers which shapes work for which
//! problem classes — without any human design.

use chrono::{Duration, Utc};
use organism_intent::{IntentPacket, Reversibility};
use organism_pack::{
    CollaborationTopology, ShapeCompetition, ShapeMetric, ShapeObservation,
    calibrate_shape, classify_problem, generate_candidates, score_observation, select_winner,
};
use uuid::Uuid;

fn main() {
    let now = Utc::now();

    println!("=== Shape Competition: The Shape Is a Hypothesis ===\n");

    // ── Step 1: Generate candidates for an irreversible intent ────
    let mut intent = IntentPacket::new("Acquire target company for €100M", now + Duration::days(30));
    intent.reversibility = Reversibility::Irreversible;
    intent.authority = vec!["board".into(), "cfo".into(), "legal".into()];
    intent.constraints = vec!["regulatory".into(), "financing".into(), "due_diligence".into()];

    let problem_class = classify_problem(&intent);
    println!("Problem class: {problem_class}\n");

    let candidates = generate_candidates(&intent, now, &[]);

    println!("--- Candidates (no priors) ---\n");
    for (i, c) in candidates.iter().enumerate() {
        println!("  Candidate {}: {:?}", i + 1, c.charter.topology);
        println!("    Rationale:   {}", c.rationale);
        println!("    Prior score: {:.2}", c.prior_score);
        println!();
    }

    // ── Step 2: Simulate observations for each candidate ──────────
    println!("--- Simulated Trial Runs ---\n");

    let observations: Vec<ShapeObservation> = candidates
        .iter()
        .map(|c| {
            let obs = simulate_trial(c.charter.topology);
            println!(
                "  {:?}: {} hypotheses, {:.2} avg confidence, {:.1}% contradictions, {} cycles, {:.0}% budget",
                c.charter.topology,
                obs.hypothesis_count,
                obs.avg_confidence,
                obs.contradiction_rate * 100.0,
                obs.cycles_to_stability,
                obs.budget_used_fraction * 100.0,
            );
            ShapeObservation {
                candidate_id: c.id,
                ..obs
            }
        })
        .collect();

    println!();

    // ── Step 3: Score and select winner ───────────────────────────
    println!("--- Scoring (all metrics) ---\n");

    let metrics = [
        ShapeMetric::EvidenceQuality,
        ShapeMetric::ConvergenceSpeed,
        ShapeMetric::ContradictionMinimization,
        ShapeMetric::Balanced,
    ];

    for metric in &metrics {
        println!("  {metric:?}:");
        for (c, obs) in candidates.iter().zip(&observations) {
            let score = score_observation(obs, *metric);
            println!("    {:?}: {score:.3}", c.charter.topology);
        }
    }
    println!();

    let competition = ShapeCompetition {
        intent_id: intent.id,
        candidates: candidates.clone(),
        evaluation_metric: ShapeMetric::Balanced,
        winner: None,
    };

    let winner_id = select_winner(&competition, &observations);
    let winner = candidates.iter().find(|c| Some(c.id) == winner_id).unwrap();
    let winner_obs = observations.iter().find(|o| Some(o.candidate_id) == winner_id).unwrap();
    let winner_score = score_observation(winner_obs, ShapeMetric::Balanced);

    println!(
        "  Winner: {:?} (balanced score: {winner_score:.3})",
        winner.charter.topology
    );
    println!();

    // ── Step 4: Calibrate priors ─────────────────────────────────
    println!("--- Prior Calibration ---\n");

    let mut calibrations = Vec::new();
    for (c, obs) in candidates.iter().zip(&observations) {
        let score = score_observation(obs, ShapeMetric::Balanced);
        let cal = calibrate_shape(&problem_class, c.charter.topology, score, &calibrations);
        println!(
            "  {:?}: prior={:.3} → posterior={:.3} (obs={})",
            cal.topology, cal.prior_score, cal.posterior_score, cal.observation_count
        );
        calibrations.push(cal);
    }
    println!();

    // ── Step 5: Run 5 more episodes, show convergence ────────────
    println!("--- Calibration Over 5 Episodes ---\n");

    for episode in 1..=5 {
        let mut episode_cals = Vec::new();
        for (c, obs) in candidates.iter().zip(&observations) {
            let score = score_observation(obs, ShapeMetric::Balanced);
            let cal = calibrate_shape(&problem_class, c.charter.topology, score, &calibrations);
            episode_cals.push(cal);
        }

        print!("  Episode {episode}:");
        for cal in &episode_cals {
            print!("  {:?}={:.3}", cal.topology, cal.posterior_score);
        }
        println!();

        calibrations = episode_cals;
    }
    println!();

    // ── Step 6: Re-derive with calibrated priors ─────────────────
    println!("--- Re-Derive With Priors ---\n");

    let new_candidates = generate_candidates(&intent, now, &calibrations);
    println!("  Candidates with priors ({} total):", new_candidates.len());
    for c in &new_candidates {
        println!(
            "    {:?} — prior_score={:.3}, rationale: {}",
            c.charter.topology, c.prior_score, c.rationale
        );
    }
    println!();

    // Show that prior-informed candidates include the best performer.
    let has_prior_informed = new_candidates.iter().any(|c| c.rationale.contains("Prior-informed"));
    if has_prior_informed {
        println!("  The system learned from past episodes and included a prior-informed candidate.");
    } else {
        println!("  With only {} observations, the system hasn't yet accumulated enough data", calibrations[0].observation_count);
        println!("  to override the derivation. It needs >= 3 observations per topology.");
    }
}

fn simulate_trial(topology: CollaborationTopology) -> ShapeObservation {
    match topology {
        CollaborationTopology::Panel => ShapeObservation {
            candidate_id: Uuid::nil(),
            hypothesis_count: 35,
            avg_confidence: 0.85,
            contradiction_rate: 0.08,
            cycles_to_stability: 8,
            budget_used_fraction: 0.7,
        },
        CollaborationTopology::Huddle => ShapeObservation {
            candidate_id: Uuid::nil(),
            hypothesis_count: 25,
            avg_confidence: 0.78,
            contradiction_rate: 0.12,
            cycles_to_stability: 5,
            budget_used_fraction: 0.45,
        },
        CollaborationTopology::DiscussionGroup => ShapeObservation {
            candidate_id: Uuid::nil(),
            hypothesis_count: 20,
            avg_confidence: 0.72,
            contradiction_rate: 0.15,
            cycles_to_stability: 6,
            budget_used_fraction: 0.5,
        },
        CollaborationTopology::SelfOrganizing => ShapeObservation {
            candidate_id: Uuid::nil(),
            hypothesis_count: 40,
            avg_confidence: 0.65,
            contradiction_rate: 0.22,
            cycles_to_stability: 12,
            budget_used_fraction: 0.85,
        },
    }
}
