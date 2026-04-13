// Copyright 2024-2026 Aprio One AB, Sweden
// SPDX-License-Identifier: LicenseRef-Proprietary

//! Invariants for Spike 3.

use converge_core::{Context, ContextKey, Invariant, InvariantClass, InvariantResult, Violation};

pub struct ResearchCoverageInvariant;

impl Invariant for ResearchCoverageInvariant {
    fn name(&self) -> &str {
        "research_coverage"
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

        let required = [
            "analysis:demand_research",
            "analysis:delivery_deep_research",
            "analysis:workforce_research",
        ];

        let missing: Vec<_> = required
            .iter()
            .filter(|id| !ctx.get(ContextKey::Hypotheses).iter().any(|f| f.id == **id))
            .copied()
            .collect();

        if missing.is_empty() {
            InvariantResult::Ok
        } else {
            InvariantResult::Violated(Violation::new(format!(
                "missing research analyses: {}",
                missing.join(", ")
            )))
        }
    }
}

pub struct DatasetProvenanceInvariant;

impl Invariant for DatasetProvenanceInvariant {
    fn name(&self) -> &str {
        "dataset_provenance"
    }

    fn class(&self) -> InvariantClass {
        InvariantClass::Structural
    }

    fn check(&self, ctx: &Context) -> InvariantResult {
        for fact_id in ["analytics_scores", "feasible_plans"] {
            if let Some(fact) = ctx
                .get(ContextKey::Evaluations)
                .iter()
                .find(|f| f.id == fact_id)
            {
                if !fact.content.contains("dataset_version")
                    && !fact.content.contains("analytics_dataset_version")
                {
                    return InvariantResult::Violated(Violation::with_facts(
                        format!("{fact_id} missing dataset provenance"),
                        vec![fact.id.clone()],
                    ));
                }
            }
        }

        InvariantResult::Ok
    }
}

pub struct FeasibleRecommendationInvariant;

impl Invariant for FeasibleRecommendationInvariant {
    fn name(&self) -> &str {
        "feasible_recommendation"
    }

    fn class(&self) -> InvariantClass {
        InvariantClass::Acceptance
    }

    fn check(&self, ctx: &Context) -> InvariantResult {
        let recommendation = match ctx
            .get(ContextKey::Evaluations)
            .iter()
            .find(|f| f.id == "recommendation")
        {
            Some(recommendation) => recommendation,
            None => return InvariantResult::Ok,
        };

        let selected_plan_id = serde_json::from_str::<serde_json::Value>(&recommendation.content)
            .ok()
            .and_then(|v| {
                v.get("selected_plan_id")
                    .and_then(|selected| selected.as_str().map(ToString::to_string))
            });

        let selected_plan_id = match selected_plan_id {
            Some(id) => id,
            None => {
                return InvariantResult::Violated(Violation::with_facts(
                    "recommendation missing selected_plan_id",
                    vec![recommendation.id.clone()],
                ));
            }
        };

        let feasible_plans = match ctx
            .get(ContextKey::Evaluations)
            .iter()
            .find(|f| f.id == "feasible_plans")
        {
            Some(fact) => fact,
            None => {
                return InvariantResult::Violated(Violation::with_facts(
                    "recommendation exists without feasible plans",
                    vec![recommendation.id.clone()],
                ));
            }
        };

        let plans: Vec<serde_json::Value> =
            serde_json::from_str(&feasible_plans.content).unwrap_or_default();
        let selected = plans.iter().find(|plan| {
            plan.get("plan_id")
                .and_then(|v| v.as_str())
                .is_some_and(|id| id == selected_plan_id)
        });

        match selected {
            Some(plan)
                if plan
                    .get("gate_decision")
                    .and_then(|v| v.as_str())
                    .is_some_and(|decision| decision != "reject") =>
            {
                InvariantResult::Ok
            }
            _ => InvariantResult::Violated(Violation::with_facts(
                format!("recommended plan {selected_plan_id} is not optimization-backed"),
                vec![recommendation.id.clone(), feasible_plans.id.clone()],
            )),
        }
    }
}
