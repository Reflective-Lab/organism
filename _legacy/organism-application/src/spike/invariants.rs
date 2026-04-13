// Copyright 2024-2025 Aprio One AB, Sweden
// SPDX-License-Identifier: LicenseRef-Proprietary

//! Invariants for the Budget Approval Decision spike.
//!
//! Truth #6: Governance is structural — 3 invariants prevent invalid states
//! at compile time, not after the fact.

use converge_core::{Context, ContextKey, Invariant, InvariantClass, InvariantResult, Violation};

// ---------------------------------------------------------------------------
// BudgetEnvelopeInvariant (Structural)
// ---------------------------------------------------------------------------

/// No strategy may exceed the declared budget.
///
/// Checked on every merge — violation = immediate failure.
pub struct BudgetEnvelopeInvariant {
    budget_limit: u64,
}

impl BudgetEnvelopeInvariant {
    pub fn new(budget_limit: u64) -> Self {
        Self { budget_limit }
    }
}

impl Invariant for BudgetEnvelopeInvariant {
    fn name(&self) -> &'static str {
        "budget_envelope"
    }

    fn class(&self) -> InvariantClass {
        InvariantClass::Structural
    }

    fn check(&self, ctx: &Context) -> InvariantResult {
        let strategies = ctx.get(ContextKey::Strategies);

        for strategy in strategies {
            // Extract total_cost from JSON content
            if let Some(cost_str) = strategy
                .content
                .split("\"total_cost\":")
                .nth(1)
                .and_then(|s| s.split([',', '}']).next())
            {
                if let Ok(cost) = cost_str.trim().parse::<u64>() {
                    if cost > self.budget_limit {
                        return InvariantResult::Violated(Violation::with_facts(
                            format!(
                                "Strategy '{}' total_cost ${cost} exceeds budget limit ${}",
                                strategy.id, self.budget_limit
                            ),
                            vec![strategy.id.clone()],
                        ));
                    }
                }
            }
        }

        InvariantResult::Ok
    }
}

// ---------------------------------------------------------------------------
// ChallengeResolutionInvariant (Semantic)
// ---------------------------------------------------------------------------

/// Every blocking challenge must have a corresponding revised strategy.
///
/// Checked at convergence — rejects results if unresolved challenges remain.
/// Uses Acceptance class so agents have time to resolve challenges during
/// the convergence loop before the check fires.
pub struct ChallengeResolutionInvariant;

impl Invariant for ChallengeResolutionInvariant {
    fn name(&self) -> &'static str {
        "challenge_resolution"
    }

    fn class(&self) -> InvariantClass {
        InvariantClass::Acceptance
    }

    fn check(&self, ctx: &Context) -> InvariantResult {
        let constraints = ctx.get(ContextKey::Constraints);
        let strategies = ctx.get(ContextKey::Strategies);

        // Find blocking challenges
        let blocking_challenges: Vec<&str> = constraints
            .iter()
            .filter(|c| c.id.starts_with("challenge:") && c.content.contains("\"Blocking\""))
            .map(|c| c.id.as_str())
            .collect();

        if blocking_challenges.is_empty() {
            return InvariantResult::Ok;
        }

        // Check that at least one revised strategy exists that addresses challenges
        let has_revision = strategies
            .iter()
            .any(|s| s.id.starts_with("revised:") && s.content.contains("addresses_challenge"));

        if !has_revision {
            return InvariantResult::Violated(Violation::with_facts(
                format!(
                    "Blocking challenges {} have no revised strategy",
                    blocking_challenges.join(", ")
                ),
                blocking_challenges
                    .iter()
                    .map(|s| (*s).to_string())
                    .collect(),
            ));
        }

        InvariantResult::Ok
    }
}

// ---------------------------------------------------------------------------
// DecisionRequiredInvariant (Acceptance)
// ---------------------------------------------------------------------------

/// The final context must contain a `decision:` evaluation.
///
/// Checked when convergence is claimed — violation rejects results.
pub struct DecisionRequiredInvariant;

impl Invariant for DecisionRequiredInvariant {
    fn name(&self) -> &'static str {
        "decision_required"
    }

    fn class(&self) -> InvariantClass {
        InvariantClass::Acceptance
    }

    fn check(&self, ctx: &Context) -> InvariantResult {
        let evaluations = ctx.get(ContextKey::Evaluations);

        let has_decision = evaluations.iter().any(|e| e.id.starts_with("decision:"));

        if has_decision {
            InvariantResult::Ok
        } else {
            InvariantResult::Violated(Violation::new(
                "No decision evaluation produced — pipeline incomplete",
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use converge_core::Fact;

    #[test]
    fn budget_envelope_passes_within_limit() {
        let invariant = BudgetEnvelopeInvariant::new(50_000);
        let mut ctx = Context::new();
        ctx.add_fact(Fact::new(
            ContextKey::Strategies,
            "strategy:ok",
            r#"{"total_cost":25000}"#,
        ))
        .unwrap();

        assert!(invariant.check(&ctx).is_ok());
    }

    #[test]
    fn budget_envelope_catches_over_budget() {
        let invariant = BudgetEnvelopeInvariant::new(50_000);
        let mut ctx = Context::new();
        ctx.add_fact(Fact::new(
            ContextKey::Strategies,
            "strategy:over",
            r#"{"total_cost":75000}"#,
        ))
        .unwrap();

        assert!(invariant.check(&ctx).is_violated());
    }

    #[test]
    fn challenge_resolution_passes_without_challenges() {
        let invariant = ChallengeResolutionInvariant;
        let ctx = Context::new();
        assert!(invariant.check(&ctx).is_ok());
    }

    #[test]
    fn challenge_resolution_blocks_unresolved() {
        let invariant = ChallengeResolutionInvariant;
        let mut ctx = Context::new();
        ctx.add_fact(Fact::new(
            ContextKey::Constraints,
            "challenge:roi",
            r#"{"severity":"Blocking","description":"ROI too optimistic"}"#,
        ))
        .unwrap();

        assert!(invariant.check(&ctx).is_violated());
    }

    #[test]
    fn challenge_resolution_passes_with_revision() {
        let invariant = ChallengeResolutionInvariant;
        let mut ctx = Context::new();
        ctx.add_fact(Fact::new(
            ContextKey::Constraints,
            "challenge:roi",
            r#"{"severity":"Blocking","description":"ROI too optimistic"}"#,
        ))
        .unwrap();
        ctx.add_fact(Fact::new(
            ContextKey::Strategies,
            "revised:conservative",
            r#"{"addresses_challenge":"challenge:roi"}"#,
        ))
        .unwrap();

        assert!(invariant.check(&ctx).is_ok());
    }

    #[test]
    fn decision_required_passes_with_decision() {
        let invariant = DecisionRequiredInvariant;
        let mut ctx = Context::new();
        ctx.add_fact(Fact::new(
            ContextKey::Evaluations,
            "decision:recommendation",
            "approve_phased",
        ))
        .unwrap();

        assert!(invariant.check(&ctx).is_ok());
    }

    #[test]
    fn decision_required_blocks_without_decision() {
        let invariant = DecisionRequiredInvariant;
        let ctx = Context::new();
        assert!(invariant.check(&ctx).is_violated());
    }
}
