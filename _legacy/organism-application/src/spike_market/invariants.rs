// Copyright 2024-2025 Aprio One AB, Sweden
// SPDX-License-Identifier: LicenseRef-Proprietary

//! Invariants for the Nordic Market Expansion spike.
//!
//! - BudgetConstraintInvariant (Structural): No recommendation exceeds entry cost budget
//! - MinimumScoreInvariant (Structural): Selected city meets minimum talent threshold
//! - ConsensusRequiredInvariant (Acceptance): All 3 experiments must contribute before decision

use converge_core::{Context, ContextKey, Invariant, InvariantClass, InvariantResult, Violation};

use crate::spike_market::scenario::candidate_cities;

// ── BudgetConstraintInvariant ───────────────────────────────────────

/// No recommendation can exceed the entry cost budget.
pub struct BudgetConstraintInvariant {
    max_budget_k: u32,
}

impl BudgetConstraintInvariant {
    #[must_use]
    pub fn new(max_budget_k: u32) -> Self {
        Self { max_budget_k }
    }
}

impl Invariant for BudgetConstraintInvariant {
    fn name(&self) -> &str {
        "budget_constraint"
    }

    fn class(&self) -> InvariantClass {
        InvariantClass::Structural
    }

    fn check(&self, ctx: &Context) -> InvariantResult {
        let cities = candidate_cities();

        // Check optimization result
        for fact in ctx.get(ContextKey::Evaluations) {
            if fact.id == "optimization_result" || fact.id == "recommendation" {
                if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&fact.content) {
                    let city_name = parsed.get("selected_city").and_then(|v| v.as_str());

                    if let Some(name) = city_name {
                        if let Some(city) = cities.iter().find(|c| c.name == name) {
                            if city.entry_cost_k > self.max_budget_k {
                                return InvariantResult::Violated(Violation::with_facts(
                                    format!(
                                        "{} entry cost ({}K EUR) exceeds budget ({}K EUR)",
                                        city.name, city.entry_cost_k, self.max_budget_k
                                    ),
                                    vec![fact.id.clone()],
                                ));
                            }
                        }
                    }
                }
            }
        }

        InvariantResult::Ok
    }
}

// ── MinimumScoreInvariant ───────────────────────────────────────────

/// Selected city must meet minimum talent score.
pub struct MinimumScoreInvariant {
    min_talent: u32,
}

impl MinimumScoreInvariant {
    #[must_use]
    pub fn new(min_talent: u32) -> Self {
        Self { min_talent }
    }
}

impl Invariant for MinimumScoreInvariant {
    fn name(&self) -> &str {
        "minimum_score"
    }

    fn class(&self) -> InvariantClass {
        InvariantClass::Structural
    }

    fn check(&self, ctx: &Context) -> InvariantResult {
        let cities = candidate_cities();

        for fact in ctx.get(ContextKey::Evaluations) {
            if fact.id == "optimization_result" || fact.id == "recommendation" {
                if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&fact.content) {
                    let city_name = parsed.get("selected_city").and_then(|v| v.as_str());

                    if let Some(name) = city_name {
                        if let Some(city) = cities.iter().find(|c| c.name == name) {
                            if city.talent_score < self.min_talent {
                                return InvariantResult::Violated(Violation::with_facts(
                                    format!(
                                        "{} talent score ({}) below minimum ({})",
                                        city.name, city.talent_score, self.min_talent
                                    ),
                                    vec![fact.id.clone()],
                                ));
                            }
                        }
                    }
                }
            }
        }

        InvariantResult::Ok
    }
}

// ── ConsensusRequiredInvariant ──────────────────────────────────────

/// All 3 experiments must have produced analyses before a decision is accepted.
pub struct ConsensusRequiredInvariant;

impl Invariant for ConsensusRequiredInvariant {
    fn name(&self) -> &str {
        "consensus_required"
    }

    fn class(&self) -> InvariantClass {
        InvariantClass::Acceptance
    }

    fn check(&self, ctx: &Context) -> InvariantResult {
        // Only enforce if there's a recommendation
        let has_recommendation = ctx
            .get(ContextKey::Evaluations)
            .iter()
            .any(|f| f.id == "recommendation");

        if !has_recommendation {
            return InvariantResult::Ok;
        }

        let required = [
            "analysis:market_demand",
            "analysis:competitive_landscape",
            "analysis:go_to_market_cost",
        ];

        let hypotheses = ctx.get(ContextKey::Hypotheses);
        let missing: Vec<&&str> = required
            .iter()
            .filter(|id| !hypotheses.iter().any(|f| f.id == **id))
            .collect();

        if missing.is_empty() {
            InvariantResult::Ok
        } else {
            InvariantResult::Violated(Violation::new(format!(
                "Missing experiment analyses: {}. All 3 experiments must complete before decision.",
                missing
                    .iter()
                    .map(|id| id.strip_prefix("analysis:").unwrap_or(id))
                    .collect::<Vec<_>>()
                    .join(", ")
            )))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use converge_core::Fact;

    #[test]
    fn budget_invariant_passes_within_budget() {
        let inv = BudgetConstraintInvariant::new(500);
        let mut ctx = Context::new();
        ctx.add_fact(Fact::new(
            ContextKey::Evaluations,
            "optimization_result",
            r#"{"selected_city": "Dublin"}"#,
        ))
        .unwrap();
        assert!(inv.check(&ctx).is_ok());
    }

    #[test]
    fn budget_invariant_fails_over_budget() {
        let inv = BudgetConstraintInvariant::new(200);
        let mut ctx = Context::new();
        ctx.add_fact(Fact::new(
            ContextKey::Evaluations,
            "optimization_result",
            r#"{"selected_city": "Stockholm"}"#, // 350K > 200K
        ))
        .unwrap();
        assert!(inv.check(&ctx).is_violated());
    }

    #[test]
    fn score_invariant_passes_above_minimum() {
        let inv = MinimumScoreInvariant::new(75);
        let mut ctx = Context::new();
        ctx.add_fact(Fact::new(
            ContextKey::Evaluations,
            "optimization_result",
            r#"{"selected_city": "Stockholm"}"#, // talent 88 >= 75
        ))
        .unwrap();
        assert!(inv.check(&ctx).is_ok());
    }

    #[test]
    fn score_invariant_fails_below_minimum() {
        // Helsinki has talent 78, so pass with min 80
        let inv = MinimumScoreInvariant::new(80);
        let mut ctx = Context::new();
        ctx.add_fact(Fact::new(
            ContextKey::Evaluations,
            "optimization_result",
            r#"{"selected_city": "Helsinki"}"#, // talent 78 < 80
        ))
        .unwrap();
        assert!(inv.check(&ctx).is_violated());
    }

    #[test]
    fn consensus_invariant_passes_with_all_experiments() {
        let inv = ConsensusRequiredInvariant;
        let mut ctx = Context::new();

        // Add recommendation
        ctx.add_fact(Fact::new(ContextKey::Evaluations, "recommendation", "{}"))
            .unwrap();

        // Add all 3 analyses
        for topic in [
            "market_demand",
            "competitive_landscape",
            "go_to_market_cost",
        ] {
            ctx.add_fact(Fact::new(
                ContextKey::Hypotheses,
                format!("analysis:{topic}"),
                "{}",
            ))
            .unwrap();
        }

        assert!(inv.check(&ctx).is_ok());
    }

    #[test]
    fn consensus_invariant_fails_with_missing_experiments() {
        let inv = ConsensusRequiredInvariant;
        let mut ctx = Context::new();

        ctx.add_fact(Fact::new(ContextKey::Evaluations, "recommendation", "{}"))
            .unwrap();
        // Only add 1 of 3 experiments
        ctx.add_fact(Fact::new(
            ContextKey::Hypotheses,
            "analysis:market_demand",
            "{}",
        ))
        .unwrap();

        let result = inv.check(&ctx);
        assert!(result.is_violated());
    }

    #[test]
    fn consensus_invariant_ok_without_recommendation() {
        let inv = ConsensusRequiredInvariant;
        let ctx = Context::new();
        // No recommendation → invariant is vacuously satisfied
        assert!(inv.check(&ctx).is_ok());
    }
}
