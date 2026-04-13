// Copyright 2024-2025 Aprio One AB, Sweden
// SPDX-License-Identifier: MIT

//! Idea-to-Launch Blueprint
//!
//! Implements the product development cycle from ideation through market launch.
//!
//! # Workflow
//!
//! ```text
//! ┌─────────────┐    ┌─────────────┐    ┌─────────────┐    ┌─────────────┐
//! │  KNOWLEDGE  │───▶│  PRODUCT    │───▶│  DELIVERY   │───▶│   GROWTH    │
//! │             │    │ ENGINEERING │    │             │    │  MARKETING  │
//! │ Signal      │    │ Plan        │    │ Execute     │    │ Launch      │
//! │ Hypothesis  │    │ Build       │    │ Test        │    │ Campaign    │
//! │ Experiment  │    │ Release     │    │ Deploy      │    │ Measure     │
//! └─────────────┘    └─────────────┘    └─────────────┘    └─────────────┘
//! ```
//!
//! # Packs Involved
//!
//! - **Knowledge**: Signal capture, hypothesis generation, experimentation
//! - **Product Engineering**: Roadmap planning, feature development, release
//! - **Delivery**: Promise execution, testing, deployment
//! - **Growth Marketing**: Launch campaigns, customer acquisition
//!
//! # Key Transitions
//!
//! 1. Validated hypothesis triggers feature planning
//! 2. Feature completion triggers release coordination
//! 3. Successful release triggers marketing campaign
//! 4. Campaign results feed back to Knowledge for iteration

use converge_core::{Budget, Engine};

use crate::blueprints::{Blueprint, default_blueprint_budget};
use converge_domain::eval_agent::EvalExecutionAgent;
// Organism evals (this crate)
use crate::evals::{
    AttributionCompletenessEval, CampaignHypothesisQualityEval, FeatureOwnershipEval,
    ReleaseRollbackReadinessEval,
};
// Kernel evals (converge-domain)
use converge_domain::evals::{
    ClaimProvenanceEval, ExperimentMetricsEval, PromiseFulfillmentEval, ScopeCreepDetectionEval,
};
// Organism packs (this crate)
use crate::packs::growth_marketing::{
    CampaignHasHypothesisInvariant, CampaignOptimizerAgent, CampaignPlannerAgent,
    ContentSchedulerAgent, PerformanceTrackerAgent,
};
use crate::packs::product_engineering::{
    CanaryAnalyzerAgent, FeatureHasOwnerInvariant, FeatureSpecifierAgent, ReleaseCoordinatorAgent,
    ReleaseHasRollbackPlanInvariant, RoadmapPlannerAgent, TaskDecomposerAgent,
};
// Kernel packs (converge-domain)
use converge_domain::packs::{
    AcceptanceRequestorAgent, BlockerHasResolutionPathInvariant, ClaimHasProvenanceInvariant,
    CompletedPromiseHasAcceptanceInvariant, DecisionMemoAgent, ExperimentHasMetricsInvariant,
    ExperimentRunnerAgent, ExperimentSchedulerAgent, HypothesisGeneratorAgent,
    LegalActionsAuditedInvariant, PromiseCreatorAgent, PromiseHasDealInvariant,
    ScopeChangeRequiresApprovalInvariant, SignalCaptureAgent, StatusAggregatorAgent,
    WorkBreakdownAgent,
};

/// Idea-to-Launch workflow blueprint
#[derive(Debug, Clone, Default)]
pub struct IdeaToLaunchBlueprint;

impl IdeaToLaunchBlueprint {
    /// Creates a new Idea-to-Launch blueprint
    pub fn new() -> Self {
        Self
    }
}

impl Blueprint for IdeaToLaunchBlueprint {
    fn name(&self) -> &str {
        "idea_to_launch"
    }

    fn description(&self) -> &str {
        "Product development cycle from ideation through market launch"
    }

    fn packs(&self) -> &[&str] {
        &[
            "knowledge",
            "product_engineering",
            "delivery",
            "growth_marketing",
        ]
    }

    fn create_engine(&self) -> Engine {
        self.create_engine_with_budget(default_blueprint_budget())
    }

    fn create_engine_with_budget(&self, budget: Budget) -> Engine {
        let mut engine = Engine::with_budget(budget);

        // Knowledge Pack agents
        engine.register(SignalCaptureAgent);
        engine.register(HypothesisGeneratorAgent);
        engine.register(ExperimentSchedulerAgent);
        engine.register(ExperimentRunnerAgent);
        engine.register(DecisionMemoAgent);

        // Product Engineering Pack agents
        engine.register(RoadmapPlannerAgent);
        engine.register(FeatureSpecifierAgent);
        engine.register(TaskDecomposerAgent);
        engine.register(ReleaseCoordinatorAgent);
        engine.register(CanaryAnalyzerAgent);

        // Delivery Pack agents
        engine.register(PromiseCreatorAgent);
        engine.register(WorkBreakdownAgent);
        engine.register(StatusAggregatorAgent);
        engine.register(AcceptanceRequestorAgent);

        // Growth Marketing Pack agents
        engine.register(CampaignPlannerAgent);
        engine.register(ContentSchedulerAgent);
        engine.register(CampaignOptimizerAgent);
        engine.register(PerformanceTrackerAgent);

        // Register invariants
        engine.register_invariant(ClaimHasProvenanceInvariant);
        engine.register_invariant(ExperimentHasMetricsInvariant);
        engine.register_invariant(FeatureHasOwnerInvariant);
        engine.register_invariant(ReleaseHasRollbackPlanInvariant);
        engine.register_invariant(PromiseHasDealInvariant);
        engine.register_invariant(BlockerHasResolutionPathInvariant);
        engine.register_invariant(ScopeChangeRequiresApprovalInvariant);
        engine.register_invariant(CompletedPromiseHasAcceptanceInvariant);
        engine.register_invariant(CampaignHasHypothesisInvariant);
        engine.register_invariant(LegalActionsAuditedInvariant); // Cross-pack: Trust ↔ Legal

        // Register eval agent with pack-specific evals
        let mut eval_agent = EvalExecutionAgent::new("idea_to_launch_evals");
        // Knowledge Pack evals
        eval_agent.register_eval(ClaimProvenanceEval);
        eval_agent.register_eval(ExperimentMetricsEval);
        // Product Engineering Pack evals
        eval_agent.register_eval(FeatureOwnershipEval);
        eval_agent.register_eval(ReleaseRollbackReadinessEval);
        // Delivery Pack evals
        eval_agent.register_eval(PromiseFulfillmentEval);
        eval_agent.register_eval(ScopeCreepDetectionEval);
        // Growth Marketing Pack evals
        eval_agent.register_eval(CampaignHypothesisQualityEval);
        eval_agent.register_eval(AttributionCompletenessEval);
        engine.register(eval_agent);

        engine
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blueprint_has_correct_metadata() {
        let blueprint = IdeaToLaunchBlueprint::new();
        assert_eq!(blueprint.name(), "idea_to_launch");
        assert_eq!(
            blueprint.packs(),
            &[
                "knowledge",
                "product_engineering",
                "delivery",
                "growth_marketing"
            ]
        );
    }

    #[test]
    fn engine_can_be_created() {
        let blueprint = IdeaToLaunchBlueprint::new();
        let _engine = blueprint.create_engine();
    }
}
