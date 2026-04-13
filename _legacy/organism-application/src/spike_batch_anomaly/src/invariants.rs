// Copyright 2024-2026 Aprio One AB, Sweden
// SPDX-License-Identifier: LicenseRef-Proprietary

//! Convergence invariants for Spike 4.

use converge_core::{Context, ContextKey, Invariant, InvariantClass, InvariantResult, Violation};

/// Every user must have scores from all 4 scoring agents.
pub struct AllAgentsScoredInvariant;

const REQUIRED_SCORES: &[&str] = &[
    "temporal_scores",
    "burst_scores",
    "behavior_scores",
    "ml_scores",
];

impl Invariant for AllAgentsScoredInvariant {
    fn name(&self) -> &str {
        "all_agents_scored"
    }

    fn class(&self) -> InvariantClass {
        InvariantClass::Acceptance
    }

    fn check(&self, ctx: &Context) -> InvariantResult {
        // Only check once all agents have had a chance to run.
        let has_any = REQUIRED_SCORES.iter().any(|id| {
            ctx.get(ContextKey::Hypotheses)
                .iter()
                .any(|f| f.id == *id)
        });
        if !has_any {
            return InvariantResult::Ok; // too early to check
        }

        let missing: Vec<_> = REQUIRED_SCORES
            .iter()
            .filter(|id| {
                !ctx.get(ContextKey::Hypotheses)
                    .iter()
                    .any(|f| f.id == **id)
            })
            .copied()
            .collect();

        if missing.is_empty() {
            InvariantResult::Ok
        } else {
            InvariantResult::Violated(Violation::new(format!(
                "missing agent scores: {}",
                missing.join(", ")
            )))
        }
    }
}

/// All anomalous users must be triaged (when recommendation exists).
pub struct TriageCompleteInvariant;

impl Invariant for TriageCompleteInvariant {
    fn name(&self) -> &str {
        "triage_complete"
    }

    fn class(&self) -> InvariantClass {
        InvariantClass::Acceptance
    }

    fn check(&self, ctx: &Context) -> InvariantResult {
        let has_recommendation = ctx
            .get(ContextKey::Evaluations)
            .iter()
            .any(|f| f.id == "recommendation");
        if !has_recommendation {
            return InvariantResult::Ok;
        }

        let has_triage = ctx
            .get(ContextKey::Evaluations)
            .iter()
            .any(|f| f.id == "anomaly_triage");

        if has_triage {
            InvariantResult::Ok
        } else {
            InvariantResult::Violated(Violation::new(
                "recommendation exists without triage results".to_string(),
            ))
        }
    }
}

/// Recommendation must reference triage-backed results.
pub struct ConsistentRankingInvariant;

impl Invariant for ConsistentRankingInvariant {
    fn name(&self) -> &str {
        "consistent_ranking"
    }

    fn class(&self) -> InvariantClass {
        InvariantClass::Structural
    }

    fn check(&self, ctx: &Context) -> InvariantResult {
        let recommendation = match ctx
            .get(ContextKey::Evaluations)
            .iter()
            .find(|f| f.id == "recommendation")
        {
            Some(r) => r,
            None => return InvariantResult::Ok,
        };

        // Recommendation must have flagged_count and top_flagged fields.
        let parsed: serde_json::Value =
            serde_json::from_str(&recommendation.content).unwrap_or_default();

        if parsed.get("flagged_count").is_none() || parsed.get("top_flagged").is_none() {
            return InvariantResult::Violated(Violation::with_facts(
                "recommendation missing flagged_count or top_flagged",
                vec![recommendation.id.clone()],
            ));
        }

        InvariantResult::Ok
    }
}
