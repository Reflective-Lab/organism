// Copyright 2024-2025 Aprio One AB, Sweden
// SPDX-License-Identifier: LicenseRef-Proprietary

//! Planning huddle — multi-model reasoning that produces converge Proposals.
//!
//! The huddle is a machine strategy room where multiple reasoning systems
//! collaborate to produce candidate plans. Its outputs are
//! [`converge_core::types::Proposal<Draft>`] instances that enter the
//! standard converge pipeline: validation → promotion → fact.
//!
//! The huddle does NOT bypass the PromotionGate. It produces proposals
//! that the engine governs like any other agent output.
//!
//! ## Participants
//!
//! - LLM reasoning (strategic synthesis)
//! - Constraint solvers (feasibility checking)
//! - ML prediction models (forecasting)
//! - Causal analysis (correlation vs causation)
//! - Cost estimation (resource envelope)
//! - Domain models (business-specific knowledge)

use converge_core::Context;
use serde::{Deserialize, Serialize};

use crate::intent::OrganismIntent;

/// Metadata attached to a candidate plan produced by the huddle.
///
/// This is organism-level annotation — the plan itself flows through
/// converge as a standard Proposal.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanAnnotation {
    /// Human-readable description of the plan.
    pub description: String,
    /// Expected impact assessment.
    pub expected_impact: Impact,
    /// Cost estimate for execution.
    pub cost_estimate: CostEstimate,
    /// Risks identified during planning.
    pub risks: Vec<Risk>,
    /// Which reasoning systems contributed.
    pub contributors: Vec<ReasoningSystem>,
}

/// Expected impact of a plan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Impact {
    pub description: String,
    /// Confidence in the impact estimate (0.0 to 1.0).
    pub confidence: f64,
}

/// Cost estimate for plan execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostEstimate {
    pub compute_cost: f64,
    pub time_cost: Option<String>,
    pub unit: String,
}

/// A risk identified during planning.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Risk {
    pub description: String,
    pub likelihood: Likelihood,
    pub impact: RiskImpact,
    pub mitigation: Option<String>,
}

/// Risk likelihood.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Likelihood {
    Low,
    Medium,
    High,
}

/// Risk impact severity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RiskImpact {
    Low,
    Medium,
    High,
    Critical,
}

/// Types of reasoning systems that participate in the huddle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReasoningSystem {
    LlmReasoning,
    ConstraintSolver,
    MlPrediction,
    CausalAnalysis,
    CostEstimation,
    DomainModel,
}

/// Trait for huddle participants.
///
/// Each participant contributes to plan generation by producing
/// suggestions, constraints, and risks. Their output feeds into
/// converge Proposal creation.
pub trait HuddleParticipant: Send + Sync {
    /// The type of reasoning system this participant represents.
    fn system_type(&self) -> ReasoningSystem;

    /// Contribute to planning for a given intent.
    fn contribute(&self, intent: &OrganismIntent, ctx: &Context) -> PlanContribution;
}

/// A contribution from a huddle participant.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanContribution {
    pub system: ReasoningSystem,
    pub suggestions: Vec<String>,
    pub constraints_identified: Vec<String>,
    pub risks_identified: Vec<Risk>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plan_annotation_roundtrips() {
        let annotation = PlanAnnotation {
            description: "Expand into Nordic enterprise market".into(),
            expected_impact: Impact {
                description: "15% revenue increase".into(),
                confidence: 0.72,
            },
            cost_estimate: CostEstimate {
                compute_cost: 500.0,
                time_cost: Some("4 weeks".into()),
                unit: "USD".into(),
            },
            risks: vec![Risk {
                description: "Long sales cycles".into(),
                likelihood: Likelihood::High,
                impact: RiskImpact::Medium,
                mitigation: Some("Partner with local firms".into()),
            }],
            contributors: vec![ReasoningSystem::LlmReasoning, ReasoningSystem::CostEstimation],
        };

        let json = serde_json::to_string(&annotation).unwrap();
        let _: PlanAnnotation = serde_json::from_str(&json).unwrap();
    }
}
