// Copyright 2024-2025 Aprio One AB, Sweden
// SPDX-License-Identifier: MIT

//! Procure-to-Pay Blueprint
//!
//! Implements the complete procurement cycle from request through payment.
//!
//! # Workflow
//!
//! ```text
//! ┌─────────────┐    ┌─────────────┐    ┌─────────────┐    ┌─────────────┐
//! │ PROCUREMENT │───▶│ PARTNERSHIPS│───▶│    LEGAL    │───▶│    MONEY    │
//! │             │    │             │    │             │    │             │
//! │ Request     │    │ Vendor      │    │ Contract    │    │ Invoice     │
//! │ Approve     │    │ Assess      │    │ Negotiate   │    │ Pay         │
//! │ Purchase    │    │ Onboard     │    │ Execute     │    │ Reconcile   │
//! └─────────────┘    └─────────────┘    └─────────────┘    └─────────────┘
//! ```
//!
//! # Packs Involved
//!
//! - **Procurement Assets**: Purchase requests, approvals, asset tracking
//! - **Partnerships Vendors**: Vendor assessment, onboarding, management
//! - **Legal**: Vendor contracts, terms negotiation
//! - **Money**: Invoice processing, payment, reconciliation
//!
//! # Key Transitions
//!
//! 1. Purchase request triggers vendor assessment if new vendor
//! 2. Approved vendor enables purchase execution
//! 3. Purchase order generates invoice in Money
//! 4. Payment triggers reconciliation

use converge_core::{Budget, Engine};

use crate::blueprints::{Blueprint, default_blueprint_budget};
use converge_domain::eval_agent::EvalExecutionAgent;
// Organism evals (this crate)
use crate::evals::{
    AssetTrackingEval, ContractSignatureEval, IpAssignmentComplianceEval,
    PartnerAgreementCoverageEval, SpendApprovalComplianceEval, VendorAssessmentCompletenessEval,
};
// Kernel evals (converge-domain)
use converge_domain::evals::{InvoiceAccuracyEval, PaymentReconciliationEval};
// Organism packs (this crate)
use crate::packs::procurement_assets::{
    ApprovalRouterAgent, AssetTrackerAgent, BudgetMonitorAgent, PurchaseExecutorAgent,
    RequestIntakeAgent, SpendNeedsBudgetInvariant, SpendNeedsOwnerInvariant,
};
use crate::packs::partnerships_vendors::{
    ContractNegotiatorAgent, RelationshipManagerAgent, VendorAssessorAgent,
    VendorHasAssessmentInvariant,
};
use crate::packs::legal::{
    ContractGeneratorAgent, ContractReviewerAgent, SignatureRequestorAgent,
    SignatureRequiredInvariant,
};
// Kernel packs (converge-domain)
use converge_domain::packs::{
    InvoiceCreatorAgent, InvoiceHasCustomerInvariant, LegalActionsAuditedInvariant,
    PaymentAllocatorAgent, ReconciliationMatcherAgent,
};

/// Procure-to-Pay workflow blueprint
#[derive(Debug, Clone, Default)]
pub struct ProcureToPayBlueprint;

impl ProcureToPayBlueprint {
    /// Creates a new Procure-to-Pay blueprint
    pub fn new() -> Self {
        Self
    }
}

impl Blueprint for ProcureToPayBlueprint {
    fn name(&self) -> &str {
        "procure_to_pay"
    }

    fn description(&self) -> &str {
        "Complete procurement cycle from purchase request through vendor payment"
    }

    fn packs(&self) -> &[&str] {
        &[
            "procurement_assets",
            "partnerships_vendors",
            "legal",
            "money",
        ]
    }

    fn create_engine(&self) -> Engine {
        self.create_engine_with_budget(default_blueprint_budget())
    }

    fn create_engine_with_budget(&self, budget: Budget) -> Engine {
        let mut engine = Engine::with_budget(budget);

        // Procurement Assets Pack agents
        engine.register(RequestIntakeAgent);
        engine.register(ApprovalRouterAgent);
        engine.register(PurchaseExecutorAgent);
        engine.register(AssetTrackerAgent);
        engine.register(BudgetMonitorAgent);

        // Partnerships Vendors Pack agents
        engine.register(VendorAssessorAgent);
        engine.register(ContractNegotiatorAgent);
        engine.register(RelationshipManagerAgent);

        // Legal Pack agents
        engine.register(ContractGeneratorAgent);
        engine.register(ContractReviewerAgent);
        engine.register(SignatureRequestorAgent);

        // Money Pack agents
        engine.register(InvoiceCreatorAgent);
        engine.register(PaymentAllocatorAgent);
        engine.register(ReconciliationMatcherAgent);

        // Register invariants
        engine.register_invariant(SpendNeedsOwnerInvariant);
        engine.register_invariant(SpendNeedsBudgetInvariant);
        engine.register_invariant(VendorHasAssessmentInvariant);
        engine.register_invariant(SignatureRequiredInvariant);
        engine.register_invariant(InvoiceHasCustomerInvariant);
        engine.register_invariant(LegalActionsAuditedInvariant); // Cross-pack: Trust ↔ Legal

        // Register eval agent with pack-specific evals
        let mut eval_agent = EvalExecutionAgent::new("procure_to_pay_evals");
        // Procurement Assets Pack evals
        eval_agent.register_eval(SpendApprovalComplianceEval);
        eval_agent.register_eval(AssetTrackingEval);
        // Partnerships Vendors Pack evals
        eval_agent.register_eval(PartnerAgreementCoverageEval);
        eval_agent.register_eval(VendorAssessmentCompletenessEval);
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
        let blueprint = ProcureToPayBlueprint::new();
        assert_eq!(blueprint.name(), "procure_to_pay");
        assert_eq!(
            blueprint.packs(),
            &[
                "procurement_assets",
                "partnerships_vendors",
                "legal",
                "money"
            ]
        );
    }

    #[test]
    fn engine_can_be_created() {
        let blueprint = ProcureToPayBlueprint::new();
        let _engine = blueprint.create_engine();
    }
}
