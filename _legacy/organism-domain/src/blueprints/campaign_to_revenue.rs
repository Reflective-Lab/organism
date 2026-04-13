// Copyright 2024-2025 Aprio One AB, Sweden
// SPDX-License-Identifier: MIT

//! Campaign-to-Revenue Blueprint
//!
//! Implements the marketing-to-sales cycle from campaign launch through revenue recognition.
//!
//! # Workflow
//!
//! ```text
//! ┌─────────────┐    ┌─────────────┐    ┌─────────────┐    ┌─────────────┐
//! │   GROWTH    │───▶│  CUSTOMERS  │───▶│  DELIVERY   │───▶│    MONEY    │
//! │  MARKETING  │    │             │    │             │    │             │
//! │ Campaign    │    │ Lead        │    │ Promise     │    │ Invoice     │
//! │ Acquire     │    │ Qualify     │    │ Execute     │    │ Collect     │
//! │ Attribute   │    │ Close       │    │ Complete    │    │ Recognize   │
//! └─────────────┘    └─────────────┘    └─────────────┘    └─────────────┘
//! ```
//!
//! # Packs Involved
//!
//! - **Growth Marketing**: Campaign planning, audience targeting, attribution
//! - **Customers**: Lead management, qualification, deal close
//! - **Delivery**: Service delivery, completion
//! - **Money**: Invoicing, collection, revenue recognition
//!
//! # Key Transitions
//!
//! 1. Campaign generates leads in Customers pack
//! 2. Closed deal triggers delivery promise
//! 3. Completed delivery triggers invoicing
//! 4. Revenue attributed back to campaign

use converge_core::{Budget, Engine};

use crate::blueprints::{Blueprint, default_blueprint_budget};
use converge_domain::eval_agent::EvalExecutionAgent;
// Organism evals (this crate)
use crate::evals::{
    AttributionCompletenessEval, CampaignHypothesisQualityEval, LeadConversionQualityEval,
    PipelineVelocityEval,
};
// Kernel evals (converge-domain)
use converge_domain::evals::{
    InvoiceAccuracyEval, PaymentReconciliationEval, PromiseFulfillmentEval,
    ScopeCreepDetectionEval,
};
// Organism packs (this crate)
use crate::packs::growth_marketing::{
    AttributionAnalyzerAgent, AudienceSegmenterAgent, BudgetAllocatorAgent,
    BudgetGuardrailsEnforcedInvariant, CampaignHasHypothesisInvariant, CampaignOptimizerAgent,
    CampaignPlannerAgent, ChannelConnectorAgent, ContentSchedulerAgent, PerformanceTrackerAgent,
};
use crate::packs::customers::{
    DealCloserAgent, LeadEnrichmentAgent, LeadHasSourceInvariant, LeadRouterAgent,
    LeadScorerAgent, ProposalGeneratorAgent,
};
// Kernel packs (converge-domain)
use converge_domain::packs::{
    AcceptanceRequestorAgent, BlockerHasResolutionPathInvariant,
    CompletedPromiseHasAcceptanceInvariant, InvoiceCreatorAgent, InvoiceHasCustomerInvariant,
    LegalActionsAuditedInvariant, PaymentAllocatorAgent, PromiseCreatorAgent,
    PromiseHasDealInvariant, ReconciliationMatcherAgent, ScopeChangeRequiresApprovalInvariant,
    StatusAggregatorAgent,
};

/// Campaign-to-Revenue workflow blueprint
#[derive(Debug, Clone, Default)]
pub struct CampaignToRevenueBlueprint;

impl CampaignToRevenueBlueprint {
    /// Creates a new Campaign-to-Revenue blueprint
    pub fn new() -> Self {
        Self
    }
}

impl Blueprint for CampaignToRevenueBlueprint {
    fn name(&self) -> &str {
        "campaign_to_revenue"
    }

    fn description(&self) -> &str {
        "Marketing-to-sales cycle from campaign launch through revenue recognition"
    }

    fn packs(&self) -> &[&str] {
        &["growth_marketing", "customers", "delivery", "money"]
    }

    fn create_engine(&self) -> Engine {
        self.create_engine_with_budget(default_blueprint_budget())
    }

    fn create_engine_with_budget(&self, budget: Budget) -> Engine {
        let mut engine = Engine::with_budget(budget);

        // Growth Marketing Pack agents
        engine.register(CampaignPlannerAgent);
        engine.register(AudienceSegmenterAgent);
        engine.register(ChannelConnectorAgent);
        engine.register(ContentSchedulerAgent);
        engine.register(CampaignOptimizerAgent);
        engine.register(AttributionAnalyzerAgent);
        engine.register(BudgetAllocatorAgent);
        engine.register(PerformanceTrackerAgent);

        // Customers Pack agents
        engine.register(LeadEnrichmentAgent);
        engine.register(LeadScorerAgent);
        engine.register(LeadRouterAgent);
        engine.register(ProposalGeneratorAgent);
        engine.register(DealCloserAgent);

        // Delivery Pack agents
        engine.register(PromiseCreatorAgent);
        engine.register(StatusAggregatorAgent);
        engine.register(AcceptanceRequestorAgent);

        // Money Pack agents
        engine.register(InvoiceCreatorAgent);
        engine.register(PaymentAllocatorAgent);
        engine.register(ReconciliationMatcherAgent);

        // Register invariants
        engine.register_invariant(CampaignHasHypothesisInvariant);
        engine.register_invariant(BudgetGuardrailsEnforcedInvariant);
        engine.register_invariant(LeadHasSourceInvariant);
        engine.register_invariant(PromiseHasDealInvariant);
        engine.register_invariant(BlockerHasResolutionPathInvariant);
        engine.register_invariant(ScopeChangeRequiresApprovalInvariant);
        engine.register_invariant(CompletedPromiseHasAcceptanceInvariant);
        engine.register_invariant(InvoiceHasCustomerInvariant);
        engine.register_invariant(LegalActionsAuditedInvariant); // Cross-pack: Trust ↔ Legal

        // Register eval agent with pack-specific evals
        let mut eval_agent = EvalExecutionAgent::new("campaign_to_revenue_evals");
        // Growth Marketing Pack evals
        eval_agent.register_eval(CampaignHypothesisQualityEval);
        eval_agent.register_eval(AttributionCompletenessEval);
        // Customers Pack evals
        eval_agent.register_eval(LeadConversionQualityEval);
        eval_agent.register_eval(PipelineVelocityEval);
        // Delivery Pack evals
        eval_agent.register_eval(PromiseFulfillmentEval);
        eval_agent.register_eval(ScopeCreepDetectionEval);
        // Money Pack evals
        eval_agent.register_eval(InvoiceAccuracyEval);
        eval_agent.register_eval(PaymentReconciliationEval);
        engine.register(eval_agent);

        engine
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blueprint_has_correct_metadata() {
        let blueprint = CampaignToRevenueBlueprint::new();
        assert_eq!(blueprint.name(), "campaign_to_revenue");
        assert_eq!(
            blueprint.packs(),
            &["growth_marketing", "customers", "delivery", "money"]
        );
    }

    #[test]
    fn engine_can_be_created() {
        let blueprint = CampaignToRevenueBlueprint::new();
        let _engine = blueprint.create_engine();
    }
}
