// Copyright 2024-2025 Aprio One AB, Sweden
// SPDX-License-Identifier: LicenseRef-Proprietary

//! Budget Approval Decision spike — thin vertical slice proving converge → organism architecture.
//!
//! A department requests $50K marketing spend, and the organism decides whether to
//! approve, reject, or modify — using all 8 organism truths on top of converge's 9 axioms.
//!
//! # Agent Pipeline
//!
//! ```text
//! OrganismIntent → admission check → Seeds
//!   ↓
//! [1] IntentDecompositionAgent     → Seeds (amount, dept, purpose, reversibility)
//!   ↓
//! [2] AuthorityVerificationAgent   → Signals (authority:status, authority:remaining)
//!   ↓
//! [3] PlanningAgent                → Strategies (3 candidates: approve/phase/reject)
//!   ↓
//! [4] AdversarialAgent             → Constraints (blocking ROI challenge)
//!   ↓
//! [5] PlanRevisionAgent            → Strategies (revised plan addressing challenge)
//!   ↓
//! [6] SimulationAgent              → Evaluations (cost/outcome/policy confidence)
//!   ↓
//! [7] DecisionAgent                → Evaluations (final decision:recommendation)
//! ```

pub mod admission;
pub mod agents;
pub mod invariants;
pub mod learning;
pub mod scenario;

use converge_core::{ContextKey, ConvergeError, ConvergeResult};
use organism_core::intent::OrganismIntent;

use crate::spike::admission::BudgetAdmissionController;
use crate::spike::learning::extract_learning_episode;
use crate::spike::scenario::build_budget_approval_engine;
use organism_core::intent::AdmissionController;
use organism_core::learning::LearningEpisode;

/// Run the full budget approval decision spike end-to-end (quiet).
///
/// # Errors
///
/// Returns `BudgetApprovalError` if admission is rejected, the engine
/// fails to converge, or a convergence error occurs.
pub fn run_budget_approval(
    intent: &OrganismIntent,
) -> Result<(ConvergeResult, LearningEpisode), BudgetApprovalError> {
    run_budget_approval_inner(intent, false)
}

/// Run the full budget approval decision spike with step-by-step output.
///
/// Prints each phase of the pipeline so you can follow what the organism
/// is doing: admission, convergence, adversarial debate, decision, learning.
///
/// # Errors
///
/// Returns `BudgetApprovalError` if admission is rejected, the engine
/// fails to converge, or a convergence error occurs.
pub fn run_budget_approval_verbose(
    intent: &OrganismIntent,
) -> Result<(ConvergeResult, LearningEpisode), BudgetApprovalError> {
    run_budget_approval_inner(intent, true)
}

fn run_budget_approval_inner(
    intent: &OrganismIntent,
    verbose: bool,
) -> Result<(ConvergeResult, LearningEpisode), BudgetApprovalError> {
    if verbose {
        println!("\n========================================================");
        println!("  BUDGET APPROVAL DECISION — Organism Spike");
        println!("========================================================\n");
        println!(
            "Intent: {}",
            intent
                .root_intent()
                .objective
                .as_ref()
                .map_or("(no objective)", |o| match o {
                    converge_core::Objective::Custom(s) => s.as_str(),
                    _ => "(standard)",
                })
        );
        println!("Reversibility: {:?}", intent.reversibility);
        println!();
    }

    // ── Step 1: Admission Control ──────────────────────────────────
    if verbose {
        println!("── Step 1: Admission Control ──────────────────────────");
        println!("  Checking 4 dimensions: Capability, Context, Resources, Authority");
    }

    let controller = BudgetAdmissionController::new(100_000);
    let admission = controller.evaluate(intent);

    if verbose {
        for dim in &admission.dimensions {
            let icon = if dim.feasible { "OK" } else { "FAIL" };
            println!("  [{icon}] {:?}: {}", dim.kind, dim.reason);
        }
    }

    if !admission.admitted {
        if verbose {
            println!("  REJECTED: {}", admission.blockers.join("; "));
        }
        return Err(BudgetApprovalError::AdmissionRejected(admission.blockers));
    }

    if verbose {
        println!("  ADMITTED — intent enters the system\n");
    }

    // ── Step 2: Build Engine ───────────────────────────────────────
    if verbose {
        println!("── Step 2: Build Convergence Engine ───────────────────");
        println!("  Registering 7 agents:");
        println!("    [1] IntentDecompositionAgent   -> Seeds");
        println!("    [2] AuthorityVerificationAgent -> Signals");
        println!("    [3] PlanningAgent              -> Strategies (3 candidates)");
        println!("    [4] BudgetAdversarialAgent     -> Constraints (ROI challenge)");
        println!("    [5] PlanRevisionAgent          -> Strategies (revised plan)");
        println!("    [6] SimulationAgent            -> Evaluations (scoring)");
        println!("    [7] DecisionAgent              -> Evaluations (final decision)");
        println!("  Registering 4 invariants:");
        println!("    [S] BudgetEnvelopeInvariant    — no strategy exceeds budget");
        println!("    [A] ChallengeResolutionInvariant — challenges must be addressed");
        println!("    [A] DecisionRequiredInvariant  — must produce a decision");
        println!("    [S] CommitBarrierInvariant     — re-derive authority at commit");
        println!();
    }

    let mut engine = build_budget_approval_engine(intent);

    // ── Step 3: Run Convergence ────────────────────────────────────
    if verbose {
        println!("── Step 3: Convergence Loop ──────────────────────────");
        println!("  Running until fixed point...\n");
    }

    let result = engine
        .run(converge_core::Context::new())
        .map_err(BudgetApprovalError::Convergence)?;

    if !result.converged {
        if verbose {
            println!("  FAILED — did not reach fixed point");
        }
        return Err(BudgetApprovalError::DidNotConverge);
    }

    if verbose {
        println!("  Converged in {} cycles\n", result.cycles);

        // Show what happened in the context
        println!("── Pipeline Results ─────────────────────────────────");

        // Seeds
        let seeds = result.context.get(ContextKey::Seeds);
        println!("  Seeds ({}):", seeds.len());
        for s in seeds {
            println!("    {} = {}", s.id, truncate(&s.content, 60));
        }

        // Signals
        let signals = result.context.get(ContextKey::Signals);
        println!("  Signals ({}):", signals.len());
        for s in signals {
            println!("    {} = {}", s.id, s.content);
        }

        // Strategies
        let strategies = result.context.get(ContextKey::Strategies);
        println!("  Strategies ({}):", strategies.len());
        for s in strategies {
            let action =
                extract_json_field(&s.content, "action").unwrap_or_else(|| s.content.clone());
            let desc = extract_json_field(&s.content, "description").unwrap_or_default();
            let marker = if s.id.starts_with("revised:") {
                " ** REVISED **"
            } else {
                ""
            };
            println!("    {}: {}{}", s.id, truncate(&desc, 60), marker);
            let _ = action; // used implicitly via desc
        }

        // Constraints (challenges)
        let constraints = result.context.get(ContextKey::Constraints);
        println!("  Adversarial Challenges ({}):", constraints.len());
        for c in constraints {
            let desc =
                extract_json_field(&c.content, "description").unwrap_or_else(|| c.content.clone());
            let severity = extract_json_field(&c.content, "severity").unwrap_or_else(|| "?".into());
            println!("    {} [{}]: {}", c.id, severity, truncate(&desc, 55));
        }

        // Evaluations
        let evaluations = result.context.get(ContextKey::Evaluations);
        println!("  Evaluations ({}):", evaluations.len());
        for e in evaluations {
            if e.id.starts_with("decision:") {
                let rec =
                    extract_json_field(&e.content, "recommendation").unwrap_or_else(|| "?".into());
                let rationale = extract_json_field(&e.content, "rationale").unwrap_or_default();
                println!("    >>> DECISION: {} — {}", rec, truncate(&rationale, 50));
            } else {
                let total =
                    extract_json_field(&e.content, "total_score").unwrap_or_else(|| "?".into());
                println!("    {} score={}", e.id, total);
            }
        }
        println!();
    }

    // ── Step 4: Extract Learning Episode ───────────────────────────
    let episode = extract_learning_episode(intent, &result.context);

    if verbose {
        println!("── Step 4: Learning Episode ─────────────────────────");
        println!("  Intent: {}", episode.intent_id);
        println!("  Selected plan: {}", episode.plan_fact_id);
        println!(
            "  Adversarial signals: {}",
            episode.adversarial_signals.len()
        );
        for signal in &episode.adversarial_signals {
            println!(
                "    {:?} on '{}': {}",
                signal.skepticism,
                signal.challenge.target,
                truncate(&signal.challenge.description, 50)
            );
        }
        println!("  Lessons learned: {}", episode.lessons.len());
        for lesson in &episode.lessons {
            println!(
                "    - {} (confidence: {:.0}%)",
                truncate(&lesson.insight, 60),
                lesson.confidence * 100.0
            );
        }
        println!("\n========================================================");
        println!("  SPIKE COMPLETE — all 8 organism truths exercised");
        println!("========================================================\n");
    }

    Ok((result, episode))
}

/// Truncate a string to max length, adding "..." if truncated.
fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}...", &s[..max])
    }
}

/// Extract a value from a JSON-like string (handles both "string" and numeric values).
fn extract_json_field(json: &str, field: &str) -> Option<String> {
    // Try string value first: "field":"value"
    let string_pattern = format!("\"{field}\":\"");
    if let Some(pos) = json.find(&string_pattern) {
        let start = pos + string_pattern.len();
        let end = json[start..].find('"')? + start;
        return Some(json[start..end].to_string());
    }
    // Try numeric/other value: "field":value
    let num_pattern = format!("\"{field}\":");
    let start = json.find(&num_pattern)? + num_pattern.len();
    let end = json[start..].find([',', '}'])? + start;
    Some(json[start..end].trim().to_string())
}

/// Errors from the budget approval spike.
#[derive(Debug)]
pub enum BudgetApprovalError {
    /// Intent was rejected by admission control.
    AdmissionRejected(Vec<String>),
    /// Engine returned a convergence error.
    Convergence(ConvergeError),
    /// Engine did not reach fixed point.
    DidNotConverge,
}

impl std::fmt::Display for BudgetApprovalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AdmissionRejected(blockers) => {
                write!(f, "Admission rejected: {}", blockers.join("; "))
            }
            Self::Convergence(e) => write!(f, "Convergence error: {e}"),
            Self::DidNotConverge => write!(f, "Engine did not converge"),
        }
    }
}

impl std::error::Error for BudgetApprovalError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spike::scenario::test_budget_intent;

    #[test]
    fn budget_approval_converges() {
        let intent = test_budget_intent(50_000);
        let (result, _episode) = run_budget_approval(&intent).expect("should converge");
        assert!(result.converged);
    }

    #[test]
    fn budget_approval_verbose_output() {
        let intent = test_budget_intent(50_000);
        let (result, _episode) = run_budget_approval_verbose(&intent).expect("should converge");
        assert!(result.converged);
    }

    #[test]
    fn budget_approval_is_deterministic() {
        let intent = test_budget_intent(50_000);
        let (r1, _) = run_budget_approval(&intent).expect("run 1");
        let (r2, _) = run_budget_approval(&intent).expect("run 2");

        assert_eq!(r1.cycles, r2.cycles);
        assert_eq!(
            r1.context.get(ContextKey::Evaluations),
            r2.context.get(ContextKey::Evaluations)
        );
        assert_eq!(
            r1.context.get(ContextKey::Strategies),
            r2.context.get(ContextKey::Strategies)
        );
    }

    #[test]
    fn adversarial_challenge_forces_revision() {
        let intent = test_budget_intent(50_000);
        let (result, _) = run_budget_approval(&intent).expect("should converge");

        // Challenge must exist
        let constraints = result.context.get(ContextKey::Constraints);
        assert!(
            constraints.iter().any(|c| c.id.starts_with("challenge:")),
            "expected adversarial challenge in constraints"
        );

        // Revised strategy must exist
        let strategies = result.context.get(ContextKey::Strategies);
        assert!(
            strategies.iter().any(|s| s.id.starts_with("revised:")),
            "expected revised strategy addressing challenge"
        );
    }

    #[test]
    fn admission_rejects_over_authority() {
        let intent = test_budget_intent(200_000);
        let result = run_budget_approval(&intent);
        assert!(
            matches!(result, Err(BudgetApprovalError::AdmissionRejected(_))),
            "expected admission rejection for $200K request"
        );
    }

    #[test]
    fn decision_is_made() {
        let intent = test_budget_intent(50_000);
        let (result, _) = run_budget_approval(&intent).expect("should converge");

        let evaluations = result.context.get(ContextKey::Evaluations);
        assert!(
            evaluations.iter().any(|e| e.id.starts_with("decision:")),
            "expected decision: evaluation in final context"
        );
    }

    #[test]
    fn learning_episode_captures_signals() {
        let intent = test_budget_intent(50_000);
        let (_, episode) = run_budget_approval(&intent).expect("should converge");

        assert!(
            !episode.adversarial_signals.is_empty(),
            "expected adversarial signals in learning episode"
        );
    }

    #[test]
    fn invariant_blocks_over_budget_strategy() {
        // This test verifies that the BudgetEnvelopeInvariant prevents
        // strategies that exceed the declared budget from persisting.
        // The pipeline naturally produces strategies within budget,
        // so we verify by checking the invariant directly.
        use crate::spike::invariants::BudgetEnvelopeInvariant;
        use converge_core::{Context, Fact, Invariant, InvariantResult};

        let invariant = BudgetEnvelopeInvariant::new(50_000);
        let mut ctx = Context::new();

        // Add a seed with budget info
        ctx.add_fact(Fact::new(ContextKey::Seeds, "budget:amount", "50000"))
            .unwrap();

        // Add an over-budget strategy
        ctx.add_fact(Fact::new(
            ContextKey::Strategies,
            "strategy:over",
            r#"{"total_cost":75000,"description":"Over budget plan"}"#,
        ))
        .unwrap();

        match invariant.check(&ctx) {
            InvariantResult::Violated(v) => {
                assert!(v.reason.contains("exceeds"), "violation: {}", v.reason);
            }
            InvariantResult::Ok => panic!("should have violated for over-budget strategy"),
        }
    }
}
