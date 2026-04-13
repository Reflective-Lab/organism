// Copyright 2024-2025 Aprio One AB, Sweden
// SPDX-License-Identifier: MIT

//! Partner-to-Value Blueprint
//!
//! Implements the partnership lifecycle from sourcing through value realization.
//!
//! # Workflow
//!
//! ```text
//! ┌─────────────┐    ┌─────────────┐    ┌─────────────┐    ┌─────────────┐
//! │ PARTNERSHIPS│───▶│    LEGAL    │───▶│  DELIVERY   │───▶│    MONEY    │
//! │   VENDORS   │    │             │    │             │    │             │
//! │ Source      │    │ Contract    │    │ Integrate   │    │ Revenue     │
//! │ Evaluate    │    │ Negotiate   │    │ Execute     │    │ Share       │
//! │ Onboard     │    │ Execute     │    │ Review      │    │ Reconcile   │
//! └─────────────┘    └─────────────┘    └─────────────┘    └─────────────┘
//! ```
//!
//! # Packs Involved
//!
//! - **Partnerships Vendors**: Partner sourcing, evaluation, relationship management
//! - **Legal**: Partnership agreements, terms negotiation
//! - **Delivery**: Integration, joint deliverables
//! - **Money**: Revenue share, payments, reconciliation
//!
//! # Key Transitions
//!
//! 1. Partner evaluation triggers contract drafting
//! 2. Executed agreement enables integration
//! 3. Joint deliverables generate revenue
//! 4. Revenue share calculated and paid

use converge_core::{Budget, Engine};

use crate::blueprints::{Blueprint, default_blueprint_budget};
use converge_domain::eval_agent::EvalExecutionAgent;
// Organism evals (this crate)
use crate::evals::{
    ContractSignatureEval, IpAssignmentComplianceEval, PartnerAgreementCoverageEval,
    VendorAssessmentCompletenessEval,
};
// Kernel evals (converge-domain)
use converge_domain::evals::{
    InvoiceAccuracyEval, PaymentReconciliationEval, PromiseFulfillmentEval,
    ScopeCreepDetectionEval,
};
// Organism packs (this crate)
use crate::packs::partnerships_vendors::{
    ContractNegotiatorAgent, DueDiligenceCoordinatorAgent, IntegrationCoordinatorAgent,
    IntegrationHasOwnerInvariant, PartnerHasAgreementInvariant, PartnerSourcerAgent,
    PartnershipRenewalTrackerAgent, PerformanceReviewerAgent, RelationshipManagerAgent,
    VendorAssessorAgent,
};
use crate::packs::legal::{
    ContractExecutorAgent, ContractGeneratorAgent, ContractReviewerAgent, SignatureRequestorAgent,
    SignatureRequiredInvariant,
};
// Kernel packs (converge-domain)
use converge_domain::packs::{
    AcceptanceRequestorAgent, BlockerHasResolutionPathInvariant,
    CompletedPromiseHasAcceptanceInvariant, InvoiceCreatorAgent, InvoiceHasCustomerInvariant,
    LegalActionsAuditedInvariant, PaymentAllocatorAgent, PromiseCreatorAgent,
    PromiseHasDealInvariant, ReconciliationMatcherAgent, ScopeChangeRequiresApprovalInvariant,
    StatusAggregatorAgent, WorkBreakdownAgent,
};

/// Partner-to-Value workflow blueprint
#[derive(Debug, Clone, Default)]
pub struct PartnerToValueBlueprint;

impl PartnerToValueBlueprint {
    /// Creates a new Partner-to-Value blueprint
    pub fn new() -> Self {
        Self
    }
}

impl Blueprint for PartnerToValueBlueprint {
    fn name(&self) -> &str {
        "partner_to_value"
    }

    fn description(&self) -> &str {
        "Partnership lifecycle from sourcing through value realization"
    }

    fn packs(&self) -> &[&str] {
        &["partnerships_vendors", "legal", "delivery", "money"]
    }

    fn create_engine(&self) -> Engine {
        self.create_engine_with_budget(default_blueprint_budget())
    }

    fn create_engine_with_budget(&self, budget: Budget) -> Engine {
        let mut engine = Engine::with_budget(budget);

        // Partnerships Vendors Pack agents
        engine.register(PartnerSourcerAgent);
        engine.register(VendorAssessorAgent);
        engine.register(ContractNegotiatorAgent);
        engine.register(RelationshipManagerAgent);
        engine.register(PerformanceReviewerAgent);
        engine.register(IntegrationCoordinatorAgent);
        engine.register(DueDiligenceCoordinatorAgent);
        engine.register(PartnershipRenewalTrackerAgent);

        // Legal Pack agents
        engine.register(ContractGeneratorAgent);
        engine.register(ContractReviewerAgent);
        engine.register(SignatureRequestorAgent);
        engine.register(ContractExecutorAgent);

        // Delivery Pack agents
        engine.register(PromiseCreatorAgent);
        engine.register(WorkBreakdownAgent);
        engine.register(StatusAggregatorAgent);
        engine.register(AcceptanceRequestorAgent);

        // Money Pack agents
        engine.register(InvoiceCreatorAgent);
        engine.register(PaymentAllocatorAgent);
        engine.register(ReconciliationMatcherAgent);

        // Register invariants
        engine.register_invariant(PartnerHasAgreementInvariant);
        engine.register_invariant(IntegrationHasOwnerInvariant);
        engine.register_invariant(SignatureRequiredInvariant);
        engine.register_invariant(PromiseHasDealInvariant);
        engine.register_invariant(BlockerHasResolutionPathInvariant);
        engine.register_invariant(ScopeChangeRequiresApprovalInvariant);
        engine.register_invariant(CompletedPromiseHasAcceptanceInvariant);
        engine.register_invariant(InvoiceHasCustomerInvariant);
        engine.register_invariant(LegalActionsAuditedInvariant); // Cross-pack: Trust ↔ Legal

        // Register eval agent with pack-specific evals
        let mut eval_agent = EvalExecutionAgent::new("partner_to_value_evals");
        // Partnerships Vendors Pack evals
        eval_agent.register_eval(PartnerAgreementCoverageEval);
        eval_agent.register_eval(VendorAssessmentCompletenessEval);
        // Legal Pack evals
        eval_agent.register_eval(ContractSignatureEval);
        eval_agent.register_eval(IpAssignmentComplianceEval);
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
        let blueprint = PartnerToValueBlueprint::new();
        assert_eq!(blueprint.name(), "partner_to_value");
        assert_eq!(
            blueprint.packs(),
            &["partnerships_vendors", "legal", "delivery", "money"]
        );
    }

    #[test]
    fn engine_can_be_created() {
        let blueprint = PartnerToValueBlueprint::new();
        let _engine = blueprint.create_engine();
    }
}
