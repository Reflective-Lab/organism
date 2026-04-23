// Copyright 2024-2026 Reflective Labs
// SPDX-License-Identifier: MIT

//! Formation Compiler — compile the vendor-selection F3 proof wedge.

use converge_kernel::formation::{
    FormationTemplateQuery, ProfileSnapshot, SuggestorCapability, SuggestorRole,
};
use converge_kernel::{AgentEffect, Context, ContextKey, ProposedFact, Suggestor};
use converge_provider_api::{
    BackendKind, BackendRequirements, Capability, ComplianceLevel, CostClass, DataSovereignty,
    LatencyClass,
};
use organism_pack::IntentPacket;
use organism_runtime::{
    BusinessQualitySignal, DataContract, ExecutableSuggestorCatalog, FormationCompileRequest,
    FormationCompilerCatalogs, GovernanceClass, ProviderDescriptor, QualityScoreBps, ReplayMode,
    Runtime, Seed, SuggestorDescriptor, vendor_selection_formation_catalog,
};
use uuid::Uuid;

#[tokio::main]
async fn main() {
    let plan_id = Uuid::from_u128(0x100);
    let correlation_id = Uuid::from_u128(0x200);
    let request = FormationCompileRequest::new(
        plan_id,
        correlation_id,
        FormationTemplateQuery::new()
            .with_keyword("vendor")
            .with_keyword("diligence-evaluate-decide")
            .with_entity("VendorSelectionDecisionRecord"),
    )
    .with_tenant_id("hackathon-buyer")
    .with_domain_tag("vendor-selection");
    let intent = IntentPacket::new(
        "select AI automation vendor for claims and invoice exceptions",
        chrono::Utc::now() + chrono::Duration::hours(1),
    );

    let policy_requirements = BackendRequirements::access_policy().with_replay();
    let synthesis_requirements = BackendRequirements::reasoning_llm()
        .with_capability(Capability::StructuredOutput)
        .with_data_sovereignty(DataSovereignty::EU)
        .with_compliance(ComplianceLevel::HighExplainability);

    let catalogs = FormationCompilerCatalogs::new(vendor_selection_formation_catalog())
        .with_suggestor(market_scan_descriptor())
        .with_suggestor(weighted_evaluator_descriptor())
        .with_suggestor(policy_gate_descriptor(policy_requirements.clone()))
        .with_suggestor(decision_synthesis_descriptor(
            synthesis_requirements.clone(),
        ))
        .with_provider(
            ProviderDescriptor::new(
                "cedar-local",
                "Cedar local policy engine",
                policy_requirements,
            )
            .with_role_affinity(SuggestorRole::Constraint)
            .with_domain_tag("vendor-selection"),
        )
        .with_provider(
            ProviderDescriptor::new(
                "eu-reasoning-llm",
                "EU reasoning LLM with structured output",
                synthesis_requirements,
            )
            .with_role_affinity(SuggestorRole::Synthesis)
            .with_domain_tag("vendor-selection"),
        );

    let seed = Seed {
        key: ContextKey::Seeds,
        id: "vendor-selection-f3".into(),
        content: "evaluate AI automation vendors for claims and invoice exceptions".to_string(),
        provenance: "hackathon-fixture".to_string(),
    };

    let record = Runtime::new()
        .compile_and_run_formation(
            &intent,
            &request,
            &catalogs,
            &executable_catalog(),
            vec![seed],
            None,
        )
        .await
        .expect("vendor-selection F3 should compile and run");

    let plan = &record.plan;
    let outcome = record
        .outcome
        .clone()
        .with_gate_trigger("dpo-iso-42001-gap-acceptance")
        .with_quality_signal(BusinessQualitySignal::new(
            "audit_completeness",
            QualityScoreBps::new(9_200).expect("valid bps"),
            "scorecard cells carry source evidence links",
        ))
        .with_writeback_target("decision://vendor-selection/hackathon-f3");

    println!("template: {}", plan.template_id);
    println!("formation: {}", record.result.label);
    println!("correlation: {}", plan.correlation_id);
    println!("tenant: {}", plan.tenant_id.as_deref().unwrap_or("<none>"));
    println!("roster:");
    for member in &plan.roster {
        println!("  - {} [{:?}]", member.suggestor_id, member.role);
    }
    println!("providers:");
    for assignment in &plan.provider_assignments {
        println!(
            "  - {} -> {}",
            assignment.suggestor_id, assignment.provider_id
        );
    }
    println!("outcome status: {:?}", outcome.status);
    println!(
        "writeback: {}",
        outcome.writeback_target.as_deref().unwrap_or("<none>")
    );
}

fn profile(
    name: &str,
    role: SuggestorRole,
    output_keys: Vec<ContextKey>,
    capabilities: Vec<SuggestorCapability>,
) -> ProfileSnapshot {
    ProfileSnapshot {
        name: name.to_string(),
        role,
        output_keys,
        cost_hint: CostClass::Low,
        latency_hint: LatencyClass::Interactive,
        capabilities,
        confidence_min: 0.7,
        confidence_max: 0.95,
    }
}

fn market_scan_descriptor() -> SuggestorDescriptor {
    SuggestorDescriptor::new(
        "market-scan",
        profile(
            "market-scan",
            SuggestorRole::Signal,
            vec![ContextKey::Signals],
            vec![SuggestorCapability::KnowledgeRetrieval],
        ),
    )
    .with_read(ContextKey::Seeds)
    .with_domain_tag("vendor-selection")
    .with_output_contract(DataContract::new("MarketEvidence", "1.0"))
}

fn weighted_evaluator_descriptor() -> SuggestorDescriptor {
    SuggestorDescriptor::new(
        "weighted-evaluator",
        profile(
            "weighted-evaluator",
            SuggestorRole::Evaluation,
            vec![ContextKey::Evaluations],
            vec![SuggestorCapability::Analytics],
        ),
    )
    .with_read(ContextKey::Signals)
    .with_domain_tag("vendor-selection")
    .with_input_contract(DataContract::new("NormalizedVendorResponse", "1.0"))
}

fn policy_gate_descriptor(requirements: BackendRequirements) -> SuggestorDescriptor {
    SuggestorDescriptor::new(
        "policy-gate",
        profile(
            "policy-gate",
            SuggestorRole::Constraint,
            vec![ContextKey::Constraints],
            vec![SuggestorCapability::PolicyEnforcement],
        ),
    )
    .with_read(ContextKey::Evaluations)
    .with_domain_tag("vendor-selection")
    .with_replay_mode(ReplayMode::Required)
    .with_governance_class(GovernanceClass::HumanApprovalRequired)
    .with_backend_requirements(requirements)
}

fn decision_synthesis_descriptor(requirements: BackendRequirements) -> SuggestorDescriptor {
    SuggestorDescriptor::new(
        "decision-synthesis",
        profile(
            "decision-synthesis",
            SuggestorRole::Synthesis,
            vec![ContextKey::Proposals],
            vec![SuggestorCapability::LlmReasoning],
        ),
    )
    .with_read(ContextKey::Evaluations)
    .with_read(ContextKey::Constraints)
    .with_domain_tag("vendor-selection")
    .with_output_contract(DataContract::new("VendorSelectionDecisionRecord", "1.0"))
    .with_backend_requirements(requirements)
}

#[allow(dead_code)]
fn offline_extraction_requirements() -> BackendRequirements {
    BackendRequirements::new(BackendKind::Analytics)
        .with_capability(Capability::StructuredOutput)
        .with_data_sovereignty(DataSovereignty::EU)
        .with_offline()
}

fn executable_catalog() -> ExecutableSuggestorCatalog {
    let mut catalog = ExecutableSuggestorCatalog::new();
    catalog
        .register_factory("market-scan", || {
            FixtureSuggestor::new("market-scan", vec![ContextKey::Seeds], ContextKey::Signals)
        })
        .expect("market-scan factory");
    catalog
        .register_factory("weighted-evaluator", || {
            FixtureSuggestor::new(
                "weighted-evaluator",
                vec![ContextKey::Signals],
                ContextKey::Evaluations,
            )
        })
        .expect("weighted-evaluator factory");
    catalog
        .register_factory("policy-gate", || {
            FixtureSuggestor::new(
                "policy-gate",
                vec![ContextKey::Evaluations],
                ContextKey::Constraints,
            )
        })
        .expect("policy-gate factory");
    catalog
        .register_factory("decision-synthesis", || {
            FixtureSuggestor::new(
                "decision-synthesis",
                vec![ContextKey::Evaluations, ContextKey::Constraints],
                ContextKey::Proposals,
            )
        })
        .expect("decision-synthesis factory");
    catalog
}

struct FixtureSuggestor {
    name: &'static str,
    dependencies: Vec<ContextKey>,
    output: ContextKey,
}

impl FixtureSuggestor {
    fn new(name: &'static str, dependencies: Vec<ContextKey>, output: ContextKey) -> Self {
        Self {
            name,
            dependencies,
            output,
        }
    }
}

#[async_trait::async_trait]
impl Suggestor for FixtureSuggestor {
    fn name(&self) -> &str {
        self.name
    }

    fn dependencies(&self) -> &[ContextKey] {
        &self.dependencies
    }

    fn accepts(&self, ctx: &dyn Context) -> bool {
        self.dependencies.iter().any(|key| ctx.has(*key)) && !ctx.has(self.output)
    }

    async fn execute(&self, _ctx: &dyn Context) -> AgentEffect {
        AgentEffect::with_proposal(ProposedFact::new(
            self.output,
            format!("{}-fixture-output", self.name),
            format!("{} fixture output", self.name),
            self.name,
        ))
    }
}
