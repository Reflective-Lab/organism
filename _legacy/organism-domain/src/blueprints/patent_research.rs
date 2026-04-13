// Copyright 2024-2025 Aprio One AB, Sweden
// SPDX-License-Identifier: MIT

//! Patent Research Blueprint
//!
//! Implements governed patent research across operators with explicit approval gates.

use converge_core::{Budget, Engine};

use crate::blueprints::{Blueprint, default_blueprint_budget};
// Organism packs (this crate)
use crate::packs::legal::{
    PaidActionRequiresApprovalInvariant, SubmissionRequiresApprovalInvariant,
    SubmissionRequiresEvidenceInvariant,
};
// Kernel packs (converge-domain)
use converge_domain::packs::{
    AuditWriterAgent, ClaimHasProvenanceInvariant, LegalActionsAuditedInvariant,
    PatentEvidenceHasProvenanceInvariant, ProvenanceTrackerAgent,
};
use crate::use_cases::patent_research::{
    ClaimChartGeneratorAgent, ClaimRiskFlaggerAgent, ClaimSeedAgent, ClaimStrategyAgent,
    ClaimSupportInvariant, DisclosureCompletenessInvariant, DraftPackAssemblerAgent,
    EnrichmentLoopAgent, EvidenceCitationInvariant, InventionCaptureAgent, InventionSummaryAgent,
    MatterContextAgent, MatterPolicyAgent, PatentAlertAgent, PatentApprovalRecorderAgent,
    PatentClaimsAnalyzerAgent, PatentEvidenceCollectorAgent, PatentLandscapeAnalyzerAgent,
    PatentOperatorPlannerAgent, PatentQueryBuilderAgent, PatentReportAssemblerAgent,
    PatentSearchExecutorAgent, PatentSubmissionAgent, PriorArtShortlistAgent,
    RemoteBackendRestrictedInvariant, SpecDraftAgent, SupportMatrixAgent,
};
use converge_core::validation::ValidationAgent;
use converge_provider::{CompositePatentProvider, PatentOperator, StubPatentProvider};
use std::sync::Arc;

/// Patent research workflow blueprint.
#[derive(Debug, Clone, Default)]
pub struct PatentResearchBlueprint;

impl PatentResearchBlueprint {
    /// Creates a new Patent Research blueprint.
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl Blueprint for PatentResearchBlueprint {
    fn name(&self) -> &str {
        "patent_research"
    }

    fn description(&self) -> &str {
        "Patent research pipeline with governed search, evidence, and approvals"
    }

    fn packs(&self) -> &[&str] {
        &["legal", "knowledge", "trust"]
    }

    fn create_engine(&self) -> Engine {
        self.create_engine_with_budget(default_blueprint_budget())
    }

    fn create_engine_with_budget(&self, budget: Budget) -> Engine {
        let mut engine = Engine::with_budget(budget);

        let provider = Arc::new(CompositePatentProvider::from_env().unwrap_or_else(|_| {
            CompositePatentProvider::new()
                .with_provider(PatentOperator::Uspto, Arc::new(StubPatentProvider::new()))
        }));

        // Patent research agents (phase 0-6)
        engine.register(MatterPolicyAgent);
        engine.register(MatterContextAgent);
        engine.register(InventionCaptureAgent);
        engine.register(InventionSummaryAgent);
        engine.register(ClaimSeedAgent);
        engine.register(PatentQueryBuilderAgent);
        engine.register(PatentOperatorPlannerAgent);
        engine.register(PatentSearchExecutorAgent::new(provider));
        engine.register(PatentEvidenceCollectorAgent);
        engine.register(PriorArtShortlistAgent);
        engine.register(ClaimRiskFlaggerAgent);
        engine.register(EnrichmentLoopAgent);
        engine.register(ClaimStrategyAgent);
        engine.register(ClaimChartGeneratorAgent);
        engine.register(PatentClaimsAnalyzerAgent);
        engine.register(PatentLandscapeAnalyzerAgent);
        engine.register(PatentReportAssemblerAgent);
        engine.register(PatentAlertAgent);
        engine.register(SpecDraftAgent);
        engine.register(SupportMatrixAgent);
        engine.register(DraftPackAssemblerAgent);
        engine.register(PatentSubmissionAgent);
        engine.register(PatentApprovalRecorderAgent);

        // Trust pack agents for audit and provenance
        engine.register(AuditWriterAgent);
        engine.register(ProvenanceTrackerAgent);

        // Validation
        engine.register(ValidationAgent::with_defaults());

        // Invariants
        engine.register_invariant(DisclosureCompletenessInvariant);
        engine.register_invariant(RemoteBackendRestrictedInvariant);
        engine.register_invariant(EvidenceCitationInvariant);
        engine.register_invariant(ClaimSupportInvariant);
        engine.register_invariant(PatentEvidenceHasProvenanceInvariant);
        engine.register_invariant(ClaimHasProvenanceInvariant);
        engine.register_invariant(PaidActionRequiresApprovalInvariant);
        engine.register_invariant(SubmissionRequiresApprovalInvariant);
        engine.register_invariant(SubmissionRequiresEvidenceInvariant);
        engine.register_invariant(LegalActionsAuditedInvariant);

        engine
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blueprint_has_correct_metadata() {
        let blueprint = PatentResearchBlueprint::new();
        assert_eq!(blueprint.name(), "patent_research");
        assert_eq!(blueprint.packs(), &["legal", "knowledge", "trust"]);
    }

    #[test]
    fn engine_can_be_created() {
        let blueprint = PatentResearchBlueprint::new();
        let _engine = blueprint.create_engine();
    }
}
