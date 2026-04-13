// Copyright 2024-2025 Aprio One AB, Sweden
// SPDX-License-Identifier: MIT

//! Issue-to-Resolution Blueprint
//!
//! Implements the support lifecycle from issue intake through resolution and knowledge capture.
//!
//! # Workflow
//!
//! ```text
//! ┌─────────────┐    ┌─────────────┐    ┌─────────────┐    ┌─────────────┐
//! │ OPS SUPPORT │───▶│  PRODUCT    │───▶│  KNOWLEDGE  │───▶│ DATA METRICS│
//! │             │    │ ENGINEERING │    │             │    │             │
//! │ Intake      │    │ Incident    │    │ Capture     │    │ Track       │
//! │ Triage      │    │ Response    │    │ Document    │    │ Report      │
//! │ Resolve     │    │ Postmortem  │    │ Canonize    │    │ Alert       │
//! └─────────────┘    └─────────────┘    └─────────────┘    └─────────────┘
//! ```
//!
//! # Packs Involved
//!
//! - **Ops Support**: Ticket intake, triage, routing, resolution
//! - **Product Engineering**: Incident response, postmortems
//! - **Knowledge**: Learning capture, documentation, knowledge base
//! - **Data Metrics**: SLA tracking, reporting, alerting
//!
//! # Key Transitions
//!
//! 1. Critical ticket triggers incident in Product Engineering
//! 2. Resolution triggers knowledge capture
//! 3. Pattern detection feeds back to product improvements
//! 4. Metrics inform SLA and quality reporting

use converge_core::{Budget, Engine};

use crate::blueprints::{Blueprint, default_blueprint_budget};
use converge_domain::eval_agent::EvalExecutionAgent;
// Organism evals (this crate)
use crate::evals::{
    EscalationAppropriatenessEval, FeatureOwnershipEval, ReleaseRollbackReadinessEval,
    TicketResolutionEval,
};
// Kernel evals (converge-domain)
use converge_domain::evals::{
    ClaimProvenanceEval, DashboardSourceEval, ExperimentMetricsEval, MetricDefinitionQualityEval,
};
// Organism packs (this crate)
use crate::packs::ops_support::{
    EscalationHandlerAgent, EscalationHasReasonInvariant, KbUpdaterAgent, NoOrphanTicketsInvariant,
    PatternDetectorAgent, ResolutionTrackerAgent, SlaBreachEscalatesInvariant, SlaMonitorAgent,
    TicketIntakeAgent, TicketRouterAgent, TicketTriagerAgent,
};
use crate::packs::product_engineering::{
    IncidentHasSeverityInvariant, IncidentResponderAgent, PostmortemFacilitatorAgent,
};
// Kernel packs (converge-domain)
use converge_domain::packs::{
    AlertEvaluatorAgent, AlertHasOwnerInvariant, CanonicalKnowledgeAgent,
    ClaimHasProvenanceInvariant, DashboardBuilderAgent, DecisionMemoAgent,
    LegalActionsAuditedInvariant, MetricCalculatorAgent, MetricRegistrarAgent, SignalCaptureAgent,
};

/// Issue-to-Resolution workflow blueprint
#[derive(Debug, Clone, Default)]
pub struct IssueToResolutionBlueprint;

impl IssueToResolutionBlueprint {
    /// Creates a new Issue-to-Resolution blueprint
    pub fn new() -> Self {
        Self
    }
}

impl Blueprint for IssueToResolutionBlueprint {
    fn name(&self) -> &str {
        "issue_to_resolution"
    }

    fn description(&self) -> &str {
        "Support lifecycle from issue intake through resolution and knowledge capture"
    }

    fn packs(&self) -> &[&str] {
        &[
            "ops_support",
            "product_engineering",
            "knowledge",
            "data_metrics",
        ]
    }

    fn create_engine(&self) -> Engine {
        self.create_engine_with_budget(default_blueprint_budget())
    }

    fn create_engine_with_budget(&self, budget: Budget) -> Engine {
        let mut engine = Engine::with_budget(budget);

        // Ops Support Pack agents
        engine.register(TicketIntakeAgent);
        engine.register(TicketTriagerAgent);
        engine.register(TicketRouterAgent);
        engine.register(SlaMonitorAgent);
        engine.register(EscalationHandlerAgent);
        engine.register(ResolutionTrackerAgent);
        engine.register(PatternDetectorAgent);
        engine.register(KbUpdaterAgent);

        // Product Engineering Pack agents
        engine.register(IncidentResponderAgent);
        engine.register(PostmortemFacilitatorAgent);

        // Knowledge Pack agents
        engine.register(SignalCaptureAgent);
        engine.register(DecisionMemoAgent);
        engine.register(CanonicalKnowledgeAgent);

        // Data Metrics Pack agents
        engine.register(MetricRegistrarAgent);
        engine.register(MetricCalculatorAgent);
        engine.register(AlertEvaluatorAgent);
        engine.register(DashboardBuilderAgent);

        // Register invariants
        engine.register_invariant(NoOrphanTicketsInvariant);
        engine.register_invariant(EscalationHasReasonInvariant);
        engine.register_invariant(SlaBreachEscalatesInvariant);
        engine.register_invariant(IncidentHasSeverityInvariant);
        engine.register_invariant(ClaimHasProvenanceInvariant);
        engine.register_invariant(AlertHasOwnerInvariant);
        engine.register_invariant(LegalActionsAuditedInvariant); // Cross-pack: Trust ↔ Legal

        // Register eval agent with pack-specific evals
        let mut eval_agent = EvalExecutionAgent::new("issue_to_resolution_evals");
        // Ops Support Pack evals
        eval_agent.register_eval(TicketResolutionEval);
        eval_agent.register_eval(EscalationAppropriatenessEval);
        // Product Engineering Pack evals
        eval_agent.register_eval(FeatureOwnershipEval);
        eval_agent.register_eval(ReleaseRollbackReadinessEval);
        // Knowledge Pack evals
        eval_agent.register_eval(ClaimProvenanceEval);
        eval_agent.register_eval(ExperimentMetricsEval);
        // Data Metrics Pack evals
        eval_agent.register_eval(MetricDefinitionQualityEval);
        eval_agent.register_eval(DashboardSourceEval);
        engine.register(eval_agent);

        engine
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blueprint_has_correct_metadata() {
        let blueprint = IssueToResolutionBlueprint::new();
        assert_eq!(blueprint.name(), "issue_to_resolution");
        assert_eq!(
            blueprint.packs(),
            &[
                "ops_support",
                "product_engineering",
                "knowledge",
                "data_metrics"
            ]
        );
    }

    #[test]
    fn engine_can_be_created() {
        let blueprint = IssueToResolutionBlueprint::new();
        let _engine = blueprint.create_engine();
    }
}
