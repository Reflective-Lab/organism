// Copyright 2024-2025 Aprio One AB, Sweden
// SPDX-License-Identifier: MIT

//! Lead-to-Cash Blueprint
//!
//! Implements the complete revenue cycle from lead generation through payment collection.
//!
//! # Workflow
//!
//! ```text
//! ┌─────────────┐    ┌─────────────┐    ┌─────────────┐    ┌─────────────┐
//! │  CUSTOMERS  │───▶│  DELIVERY   │───▶│    LEGAL    │───▶│    MONEY    │
//! │             │    │             │    │             │    │             │
//! │ Lead        │    │ Promise     │    │ Contract    │    │ Invoice     │
//! │ Qualify     │    │ Execute     │    │ Sign        │    │ Collect     │
//! │ Propose     │    │ Complete    │    │ Execute     │    │ Reconcile   │
//! │ Close       │    │             │    │             │    │             │
//! └─────────────┘    └─────────────┘    └─────────────┘    └─────────────┘
//! ```
//!
//! # Packs Involved
//!
//! - **Customers**: Lead management, qualification, proposal, deal close
//! - **Delivery**: Promise creation, execution, completion
//! - **Legal**: Contract generation, signature, execution
//! - **Money**: Invoicing, payment collection, reconciliation
//!
//! # Key Transitions
//!
//! 1. Closed-won deal triggers promise creation in Delivery
//! 2. Contract execution enables delivery start
//! 3. Delivery completion triggers invoicing in Money
//! 4. Payment receipt triggers reconciliation

use converge_core::{Budget, Engine};

use crate::blueprints::{Blueprint, default_blueprint_budget};
use converge_domain::eval_agent::EvalExecutionAgent;
// Organism evals (this crate)
use crate::evals::{
    ContractSignatureEval, IpAssignmentComplianceEval, LeadConversionQualityEval,
    PipelineVelocityEval,
};
// Kernel evals (converge-domain)
use converge_domain::evals::{
    InvoiceAccuracyEval, PaymentReconciliationEval, PromiseFulfillmentEval,
    ScopeCreepDetectionEval,
};
// Organism packs (this crate)
use crate::packs::customers::{
    ClosedWonTriggersHandoffInvariant, DealCloserAgent, HandoffSchedulerAgent, LeadEnrichmentAgent,
    LeadHasSourceInvariant, LeadRouterAgent, LeadScorerAgent, ProposalGeneratorAgent,
};
use crate::packs::legal::{
    ContractExecutorAgent, ContractGeneratorAgent, ContractReviewerAgent, SignatureRequestorAgent,
    SignatureRequiredInvariant,
};
// Kernel packs (converge-domain)
use converge_domain::packs::{
    AcceptanceRequestorAgent, BlockerHasResolutionPathInvariant,
    CompletedPromiseHasAcceptanceInvariant, InvoiceCreatorAgent, InvoiceHasCustomerInvariant,
    LegalActionsAuditedInvariant, OverdueDetectorAgent, PaymentAllocatorAgent,
    PromiseCreatorAgent, PromiseHasDealInvariant, ReconciliationMatcherAgent,
    ScopeChangeRequiresApprovalInvariant, ScopeExtractorAgent, StatusAggregatorAgent,
    WorkBreakdownAgent,
};

/// Lead-to-Cash workflow blueprint
#[derive(Debug, Clone, Default)]
pub struct LeadToCashBlueprint;

impl LeadToCashBlueprint {
    /// Creates a new Lead-to-Cash blueprint
    pub fn new() -> Self {
        Self
    }
}

impl Blueprint for LeadToCashBlueprint {
    fn name(&self) -> &str {
        "lead_to_cash"
    }

    fn description(&self) -> &str {
        "Complete revenue cycle from lead generation through payment collection"
    }

    fn packs(&self) -> &[&str] {
        &["customers", "delivery", "legal", "money"]
    }

    fn create_engine(&self) -> Engine {
        self.create_engine_with_budget(default_blueprint_budget())
    }

    fn create_engine_with_budget(&self, budget: Budget) -> Engine {
        let mut engine = Engine::with_budget(budget);

        // Customers Pack agents
        engine.register(LeadEnrichmentAgent);
        engine.register(LeadScorerAgent);
        engine.register(LeadRouterAgent);
        engine.register(ProposalGeneratorAgent);
        engine.register(DealCloserAgent);
        engine.register(HandoffSchedulerAgent);

        // Delivery Pack agents
        engine.register(PromiseCreatorAgent);
        engine.register(ScopeExtractorAgent);
        engine.register(WorkBreakdownAgent);
        engine.register(StatusAggregatorAgent);
        engine.register(AcceptanceRequestorAgent);

        // Legal Pack agents
        engine.register(ContractGeneratorAgent);
        engine.register(ContractReviewerAgent);
        engine.register(SignatureRequestorAgent);
        engine.register(ContractExecutorAgent);

        // Money Pack agents
        engine.register(InvoiceCreatorAgent);
        engine.register(PaymentAllocatorAgent);
        engine.register(ReconciliationMatcherAgent);
        engine.register(OverdueDetectorAgent);

        // Register invariants
        engine.register_invariant(LeadHasSourceInvariant);
        engine.register_invariant(ClosedWonTriggersHandoffInvariant);
        engine.register_invariant(PromiseHasDealInvariant);
        engine.register_invariant(BlockerHasResolutionPathInvariant);
        engine.register_invariant(ScopeChangeRequiresApprovalInvariant);
        engine.register_invariant(CompletedPromiseHasAcceptanceInvariant);
        engine.register_invariant(SignatureRequiredInvariant);
        engine.register_invariant(InvoiceHasCustomerInvariant);
        engine.register_invariant(LegalActionsAuditedInvariant); // Cross-pack: Trust ↔ Legal

        // Register eval agent with pack-specific evals
        let mut eval_agent = EvalExecutionAgent::new("lead_to_cash_evals");
        // Customers Pack evals
        eval_agent.register_eval(LeadConversionQualityEval);
        eval_agent.register_eval(PipelineVelocityEval);
        // Delivery Pack evals
        eval_agent.register_eval(PromiseFulfillmentEval);
        eval_agent.register_eval(ScopeCreepDetectionEval);
        // Legal Pack evals
        eval_agent.register_eval(ContractSignatureEval);
        eval_agent.register_eval(IpAssignmentComplianceEval);
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
        let blueprint = LeadToCashBlueprint::new();
        assert_eq!(blueprint.name(), "lead_to_cash");
        assert_eq!(
            blueprint.packs(),
            &["customers", "delivery", "legal", "money"]
        );
    }

    #[test]
    fn engine_can_be_created() {
        let blueprint = LeadToCashBlueprint::new();
        let _engine = blueprint.create_engine();
        // Engine creation should succeed
    }

    #[test]
    fn engine_respects_custom_budget() {
        let blueprint = LeadToCashBlueprint::new();
        let budget = Budget {
            max_cycles: 10,
            max_facts: 50,
        };
        let _engine = blueprint.create_engine_with_budget(budget);
        // Engine creation should succeed with custom budget
    }
}
