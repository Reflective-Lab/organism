// Copyright 2024-2025 Aprio One AB, Sweden
// SPDX-License-Identifier: MIT

//! Hire-to-Retire Blueprint
//!
//! Implements the complete employee lifecycle from hiring through offboarding.
//!
//! # Workflow
//!
//! ```text
//! ┌─────────────┐    ┌─────────────┐    ┌─────────────┐    ┌─────────────┐
//! │    LEGAL    │───▶│   PEOPLE    │───▶│    TRUST    │───▶│    MONEY    │
//! │             │    │             │    │             │    │             │
//! │ Offer       │    │ Onboard     │    │ Identity    │    │ Payroll     │
//! │ Contract    │    │ Provision   │    │ Access      │    │ Expenses    │
//! │ IP Assign   │    │ Offboard    │    │ Audit       │    │ Final Pay   │
//! └─────────────┘    └─────────────┘    └─────────────┘    └─────────────┘
//! ```
//!
//! # Packs Involved
//!
//! - **Legal**: Employment contracts, IP assignment, equity grants
//! - **People**: Onboarding, access provisioning, offboarding
//! - **Trust**: Identity management, access control, audit
//! - **Money**: Payroll, expenses, final pay processing
//!
//! # Key Transitions
//!
//! 1. Signed offer letter triggers onboarding in People
//! 2. IP assignment must complete before first payroll
//! 3. Identity provisioning enables access provisioning
//! 4. Termination triggers access revocation and final pay

use converge_core::{Budget, Engine};

use crate::blueprints::{Blueprint, default_blueprint_budget};
use converge_domain::eval_agent::EvalExecutionAgent;
// Organism evals (this crate)
use crate::evals::{ContractSignatureEval, IpAssignmentComplianceEval, OnboardingCompletenessEval};
// Kernel evals (converge-domain)
use converge_domain::evals::{
    AccessComplianceEval, AuditCoverageEval, InvoiceAccuracyEval, PaymentReconciliationEval,
    RbacEnforcementEval,
};
// Organism packs (this crate)
use crate::packs::legal::{
    ContractGeneratorAgent, EquityGrantProcessorAgent, IpAssignmentAgent,
    IpAssignmentBeforePaymentInvariant, SignatureRequestorAgent,
};
use crate::packs::people::{
    AccessEvaluatorAgent, AccessProvisionerAgent, ExpenseEvaluatorAgent, FinalPaySchedulerAgent,
    IdentityBeforeAccessInvariant, IdentityProvisionerAgent, OffboardingCoordinatorAgent,
    OnboardingCoordinatorAgent, PayrollCollectorAgent, PayrollValidatorAgent,
    TerminationRevokesAccessInvariant,
};
// Kernel packs (converge-domain)
use converge_domain::packs::{
    AllActionsAuditedInvariant, AuditImmutabilityInvariant, AuditWriterAgent,
    InvoiceCreatorAgent, LegalActionsAuditedInvariant, PaymentAllocatorAgent, RbacEnforcerAgent,
    SessionValidatorAgent,
};

/// Hire-to-Retire workflow blueprint
#[derive(Debug, Clone, Default)]
pub struct HireToRetireBlueprint;

impl HireToRetireBlueprint {
    /// Creates a new Hire-to-Retire blueprint
    pub fn new() -> Self {
        Self
    }
}

impl Blueprint for HireToRetireBlueprint {
    fn name(&self) -> &str {
        "hire_to_retire"
    }

    fn description(&self) -> &str {
        "Complete employee lifecycle from hiring through retirement or termination"
    }

    fn packs(&self) -> &[&str] {
        &["legal", "people", "trust", "money"]
    }

    fn create_engine(&self) -> Engine {
        self.create_engine_with_budget(default_blueprint_budget())
    }

    fn create_engine_with_budget(&self, budget: Budget) -> Engine {
        let mut engine = Engine::with_budget(budget);

        // Legal Pack agents
        engine.register(ContractGeneratorAgent);
        engine.register(SignatureRequestorAgent);
        engine.register(IpAssignmentAgent);
        engine.register(EquityGrantProcessorAgent);

        // People Pack agents
        engine.register(IdentityProvisionerAgent);
        engine.register(AccessProvisionerAgent);
        engine.register(AccessEvaluatorAgent);
        engine.register(OnboardingCoordinatorAgent);
        engine.register(OffboardingCoordinatorAgent);
        engine.register(PayrollCollectorAgent);
        engine.register(PayrollValidatorAgent);
        engine.register(FinalPaySchedulerAgent);
        engine.register(ExpenseEvaluatorAgent);

        // Trust Pack agents
        engine.register(SessionValidatorAgent);
        engine.register(RbacEnforcerAgent);
        engine.register(AuditWriterAgent);

        // Money Pack agents
        engine.register(InvoiceCreatorAgent);
        engine.register(PaymentAllocatorAgent);

        // Register invariants
        engine.register_invariant(IpAssignmentBeforePaymentInvariant);
        engine.register_invariant(IdentityBeforeAccessInvariant);
        engine.register_invariant(TerminationRevokesAccessInvariant);
        engine.register_invariant(AllActionsAuditedInvariant);
        engine.register_invariant(AuditImmutabilityInvariant);
        engine.register_invariant(LegalActionsAuditedInvariant); // Cross-pack: Trust ↔ Legal

        // Register eval agent with pack-specific evals
        let mut eval_agent = EvalExecutionAgent::new("hire_to_retire_evals");
        // Legal Pack evals
        eval_agent.register_eval(ContractSignatureEval);
        eval_agent.register_eval(IpAssignmentComplianceEval);
        // People Pack evals
        eval_agent.register_eval(OnboardingCompletenessEval);
        eval_agent.register_eval(AccessComplianceEval);
        // Trust Pack evals
        eval_agent.register_eval(AuditCoverageEval);
        eval_agent.register_eval(RbacEnforcementEval);
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
        let blueprint = HireToRetireBlueprint::new();
        assert_eq!(blueprint.name(), "hire_to_retire");
        assert_eq!(blueprint.packs(), &["legal", "people", "trust", "money"]);
    }

    #[test]
    fn engine_can_be_created() {
        let blueprint = HireToRetireBlueprint::new();
        let _engine = blueprint.create_engine();
    }
}
