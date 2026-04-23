//! Vendor-selection lifecycle fixtures for the first compiler proof wedge.

use converge_kernel::formation::{
    FormationCatalog, FormationTemplate, FormationTemplateMetadata, StaticFormationTemplate,
    SuggestorCapability, SuggestorRole,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VendorSelectionFlow {
    Frame,
    Source,
    Decide,
    Operate,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VendorSelectionFlowSpec {
    pub flow: VendorSelectionFlow,
    pub template_id: String,
    pub truth_id: String,
    pub business_job: String,
    pub covered_stages: Vec<String>,
    pub output_artifacts: Vec<String>,
    pub required_roles: Vec<SuggestorRole>,
    pub required_capabilities: Vec<SuggestorCapability>,
}

impl VendorSelectionFlowSpec {
    fn template_metadata(&self) -> FormationTemplateMetadata {
        let mut metadata = FormationTemplateMetadata::new(
            self.template_id.clone(),
            self.business_job.clone(),
            self.required_roles.clone(),
        )
        .with_keyword("vendor")
        .with_keyword("rfp")
        .with_keyword("procurement")
        .with_keyword(self.truth_id.clone())
        .with_keyword(match self.flow {
            VendorSelectionFlow::Frame => "frame",
            VendorSelectionFlow::Source => "source",
            VendorSelectionFlow::Decide => "decide",
            VendorSelectionFlow::Operate => "operate",
        })
        .with_entity("vendor")
        .with_entity("rubric")
        .with_entity("approval");

        for artifact in &self.output_artifacts {
            metadata = metadata.with_entity(artifact.clone());
        }

        for capability in &self.required_capabilities {
            metadata = metadata.with_required_capability(*capability);
        }

        metadata
    }
}

pub fn vendor_selection_lifecycle() -> Vec<VendorSelectionFlowSpec> {
    vec![frame_flow(), source_flow(), decide_flow(), operate_flow()]
}

fn frame_flow() -> VendorSelectionFlowSpec {
    VendorSelectionFlowSpec {
        flow: VendorSelectionFlow::Frame,
        template_id: "vendor-selection-frame".to_string(),
        truth_id: "frame-need-and-rubric".to_string(),
        business_job:
            "Define the business need, constraints, and scoring rubric as one governed artifact."
                .to_string(),
        covered_stages: vec!["need-and-strategy".to_string(), "requirements".to_string()],
        output_artifacts: vec!["ScoringRubric".to_string(), "ShortlistSeed".to_string()],
        required_roles: vec![
            SuggestorRole::Signal,
            SuggestorRole::Planning,
            SuggestorRole::Constraint,
            SuggestorRole::Synthesis,
        ],
        required_capabilities: common_capabilities(),
    }
}

fn source_flow() -> VendorSelectionFlowSpec {
    VendorSelectionFlowSpec {
        flow: VendorSelectionFlow::Source,
        template_id: "vendor-selection-source".to_string(),
        truth_id: "issue-and-ingest".to_string(),
        business_job: "Issue the RFP fairly and ingest vendor responses into one comparable shape."
            .to_string(),
        covered_stages: vec!["rfp-issuance".to_string(), "vendor-response".to_string()],
        output_artifacts: vec![
            "NormalizedVendorResponse".to_string(),
            "QALedger".to_string(),
            "EvidenceGapReport".to_string(),
        ],
        required_roles: vec![
            SuggestorRole::Signal,
            SuggestorRole::Analysis,
            SuggestorRole::Constraint,
            SuggestorRole::Synthesis,
        ],
        required_capabilities: common_capabilities(),
    }
}

fn decide_flow() -> VendorSelectionFlowSpec {
    VendorSelectionFlowSpec {
        flow: VendorSelectionFlow::Decide,
        template_id: "vendor-selection-decide".to_string(),
        truth_id: "diligence-evaluate-decide".to_string(),
        business_job: "Run diligence, weighted evaluation, contradiction handling, synthesis, and approval as one reasoning chain.".to_string(),
        covered_stages: vec![
            "due-diligence".to_string(),
            "evaluation".to_string(),
            "decision".to_string(),
        ],
        output_artifacts: vec![
            "VendorSelectionDecisionRecord".to_string(),
            "RubricBaseline".to_string(),
            "AuditEntry".to_string(),
        ],
        required_roles: vec![
            SuggestorRole::Signal,
            SuggestorRole::Evaluation,
            SuggestorRole::Constraint,
            SuggestorRole::Synthesis,
        ],
        required_capabilities: common_capabilities(),
    }
}

fn operate_flow() -> VendorSelectionFlowSpec {
    let mut capabilities = common_capabilities();
    capabilities.push(SuggestorCapability::ExperienceLearning);

    VendorSelectionFlowSpec {
        flow: VendorSelectionFlow::Operate,
        template_id: "vendor-selection-operate".to_string(),
        truth_id: "contract-operate-govern".to_string(),
        business_job:
            "Reconcile contract commitments, monitor obligations, and close the loop at renewal."
                .to_string(),
        covered_stages: vec![
            "contract-and-onboard".to_string(),
            "monitor-and-govern".to_string(),
        ],
        output_artifacts: vec![
            "ObligationLedger".to_string(),
            "AuditSnapshot".to_string(),
            "RenewalRecommendation".to_string(),
        ],
        required_roles: vec![
            SuggestorRole::Signal,
            SuggestorRole::Evaluation,
            SuggestorRole::Constraint,
            SuggestorRole::Synthesis,
        ],
        required_capabilities: capabilities,
    }
}

fn common_capabilities() -> Vec<SuggestorCapability> {
    vec![
        SuggestorCapability::KnowledgeRetrieval,
        SuggestorCapability::Analytics,
        SuggestorCapability::PolicyEnforcement,
        SuggestorCapability::LlmReasoning,
    ]
}

pub fn vendor_selection_formation_catalog() -> FormationCatalog {
    vendor_selection_lifecycle()
        .into_iter()
        .fold(FormationCatalog::new(), |catalog, spec| {
            catalog.with_template(FormationTemplate::static_template(
                StaticFormationTemplate::new(spec.template_metadata()),
            ))
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use converge_kernel::formation::FormationTemplateQuery;

    #[test]
    fn exposes_four_lifecycle_flows() {
        let lifecycle = vendor_selection_lifecycle();

        assert_eq!(lifecycle.len(), 4);
        assert_eq!(lifecycle[0].flow, VendorSelectionFlow::Frame);
        assert_eq!(lifecycle[1].flow, VendorSelectionFlow::Source);
        assert_eq!(lifecycle[2].flow, VendorSelectionFlow::Decide);
        assert_eq!(lifecycle[3].flow, VendorSelectionFlow::Operate);
    }

    #[test]
    fn catalog_selects_decide_flow_from_vendor_query() {
        let catalog = vendor_selection_formation_catalog();
        let query = FormationTemplateQuery::new()
            .with_keyword("vendor")
            .with_entity("vendor")
            .with_required_capability(SuggestorCapability::Analytics);

        let template = catalog
            .top_match(&query)
            .expect("vendor query should match a template");

        assert!(template.id().starts_with("vendor-selection-"));
    }
}
