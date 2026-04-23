//! Formation compiler — Organism-owned selection before Converge execution.
//!
//! The compiler turns a business intent classification into an executable
//! formation plan. Converge still owns execution, promotion, gates, and audit.

use converge_kernel::ContextKey;
use converge_kernel::formation::{
    FormationCatalog, FormationKind, FormationTemplateQuery, ProfileSnapshot, SuggestorCapability,
    SuggestorRole,
};
use converge_provider_api::{BackendRequirements, ComplianceLevel, DataSovereignty};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DataContract {
    pub name: String,
    pub version: String,
}

impl DataContract {
    pub fn new(name: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            version: version.into(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReplayMode {
    Required,
    Preferred,
    NotRequired,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GovernanceClass {
    LowRisk,
    BusinessDecision,
    RegulatedDecision,
    HumanApprovalRequired,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuggestorDescriptor {
    pub id: String,
    pub profile: ProfileSnapshot,
    pub reads: Vec<ContextKey>,
    pub domain_tags: Vec<String>,
    pub input_contracts: Vec<DataContract>,
    pub output_contracts: Vec<DataContract>,
    pub replay_mode: ReplayMode,
    pub governance_class: GovernanceClass,
    pub backend_requirements: Option<BackendRequirements>,
}

impl SuggestorDescriptor {
    pub fn new(id: impl Into<String>, profile: ProfileSnapshot) -> Self {
        Self {
            id: id.into(),
            profile,
            reads: Vec::new(),
            domain_tags: Vec::new(),
            input_contracts: Vec::new(),
            output_contracts: Vec::new(),
            replay_mode: ReplayMode::NotRequired,
            governance_class: GovernanceClass::BusinessDecision,
            backend_requirements: None,
        }
    }

    pub fn with_read(mut self, key: ContextKey) -> Self {
        self.reads.push(key);
        self
    }

    pub fn with_domain_tag(mut self, tag: impl Into<String>) -> Self {
        self.domain_tags.push(tag.into());
        self
    }

    pub fn with_input_contract(mut self, contract: DataContract) -> Self {
        self.input_contracts.push(contract);
        self
    }

    pub fn with_output_contract(mut self, contract: DataContract) -> Self {
        self.output_contracts.push(contract);
        self
    }

    pub fn with_replay_mode(mut self, mode: ReplayMode) -> Self {
        self.replay_mode = mode;
        self
    }

    pub fn with_governance_class(mut self, class: GovernanceClass) -> Self {
        self.governance_class = class;
        self
    }

    pub fn with_backend_requirements(mut self, requirements: BackendRequirements) -> Self {
        self.backend_requirements = Some(requirements);
        self
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SuggestorDescriptorCatalog {
    descriptors: Vec<SuggestorDescriptor>,
}

impl SuggestorDescriptorCatalog {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_descriptor(mut self, descriptor: SuggestorDescriptor) -> Self {
        self.register(descriptor);
        self
    }

    pub fn register(&mut self, descriptor: SuggestorDescriptor) {
        self.descriptors.push(descriptor);
    }

    pub fn iter(&self) -> std::slice::Iter<'_, SuggestorDescriptor> {
        self.descriptors.iter()
    }
}

impl<'a> IntoIterator for &'a SuggestorDescriptorCatalog {
    type IntoIter = std::slice::Iter<'a, SuggestorDescriptor>;
    type Item = &'a SuggestorDescriptor;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderDescriptor {
    pub id: String,
    pub label: String,
    pub requirements: BackendRequirements,
    pub role_affinity: Vec<SuggestorRole>,
    pub domain_tags: Vec<String>,
}

impl ProviderDescriptor {
    pub fn new(
        id: impl Into<String>,
        label: impl Into<String>,
        requirements: BackendRequirements,
    ) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            requirements,
            role_affinity: Vec::new(),
            domain_tags: Vec::new(),
        }
    }

    pub fn with_role_affinity(mut self, role: SuggestorRole) -> Self {
        self.role_affinity.push(role);
        self
    }

    pub fn with_domain_tag(mut self, tag: impl Into<String>) -> Self {
        self.domain_tags.push(tag.into());
        self
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProviderDescriptorCatalog {
    descriptors: Vec<ProviderDescriptor>,
}

impl ProviderDescriptorCatalog {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_descriptor(mut self, descriptor: ProviderDescriptor) -> Self {
        self.register(descriptor);
        self
    }

    pub fn register(&mut self, descriptor: ProviderDescriptor) {
        self.descriptors.push(descriptor);
    }

    pub fn iter(&self) -> std::slice::Iter<'_, ProviderDescriptor> {
        self.descriptors.iter()
    }
}

impl<'a> IntoIterator for &'a ProviderDescriptorCatalog {
    type IntoIter = std::slice::Iter<'a, ProviderDescriptor>;
    type Item = &'a ProviderDescriptor;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormationCompilerCatalogs {
    pub formation_templates: FormationCatalog,
    pub suggestors: SuggestorDescriptorCatalog,
    pub providers: ProviderDescriptorCatalog,
}

impl FormationCompilerCatalogs {
    pub fn new(formation_templates: FormationCatalog) -> Self {
        Self {
            formation_templates,
            suggestors: SuggestorDescriptorCatalog::new(),
            providers: ProviderDescriptorCatalog::new(),
        }
    }

    pub fn with_suggestor(mut self, descriptor: SuggestorDescriptor) -> Self {
        self.suggestors.register(descriptor);
        self
    }

    pub fn with_provider(mut self, descriptor: ProviderDescriptor) -> Self {
        self.providers.register(descriptor);
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormationCompileRequest {
    pub plan_id: Uuid,
    pub correlation_id: Uuid,
    pub tenant_id: Option<String>,
    pub query: FormationTemplateQuery,
    pub domain_tags: Vec<String>,
}

impl FormationCompileRequest {
    pub fn new(plan_id: Uuid, correlation_id: Uuid, query: FormationTemplateQuery) -> Self {
        Self {
            plan_id,
            correlation_id,
            tenant_id: None,
            query,
            domain_tags: Vec::new(),
        }
    }

    pub fn with_tenant_id(mut self, tenant_id: impl Into<String>) -> Self {
        self.tenant_id = Some(tenant_id.into());
        self
    }

    pub fn with_domain_tag(mut self, tag: impl Into<String>) -> Self {
        self.domain_tags.push(tag.into());
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompiledSuggestorRole {
    pub suggestor_id: String,
    pub role: SuggestorRole,
    pub capabilities: Vec<SuggestorCapability>,
    pub reads: Vec<ContextKey>,
    pub writes: Vec<ContextKey>,
    pub input_contracts: Vec<DataContract>,
    pub output_contracts: Vec<DataContract>,
    pub replay_mode: ReplayMode,
    pub governance_class: GovernanceClass,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoleProviderAssignment {
    pub suggestor_id: String,
    pub role: SuggestorRole,
    pub provider_id: String,
    pub requirements: BackendRequirements,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompiledFormationPlan {
    pub plan_id: Uuid,
    pub correlation_id: Uuid,
    pub tenant_id: Option<String>,
    pub template_id: String,
    pub template_kind: FormationKind,
    pub roster: Vec<CompiledSuggestorRole>,
    pub provider_assignments: Vec<RoleProviderAssignment>,
    pub trace: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum FormationCompileError {
    #[error("no formation template matched the compile request")]
    NoTemplate,
    #[error("formation requirements were not covered")]
    UncoveredRequirements {
        unmatched_roles: Vec<SuggestorRole>,
        unmatched_capabilities: Vec<SuggestorCapability>,
    },
    #[error("no provider matched backend requirements for suggestor '{suggestor_id}'")]
    MissingProvider {
        suggestor_id: String,
        role: SuggestorRole,
    },
}

#[derive(Debug, Default)]
pub struct FormationCompiler;

impl FormationCompiler {
    pub fn new() -> Self {
        Self
    }

    pub fn compile(
        &self,
        request: &FormationCompileRequest,
        catalogs: &FormationCompilerCatalogs,
    ) -> Result<CompiledFormationPlan, FormationCompileError> {
        let template = catalogs
            .formation_templates
            .top_match(&request.query)
            .ok_or(FormationCompileError::NoTemplate)?;
        let metadata = template.metadata();

        let mut unmatched_roles = metadata.required_roles.clone();
        let mut unmatched_capabilities = unique_capabilities(
            metadata
                .required_capabilities
                .iter()
                .chain(request.query.required_capabilities.iter())
                .copied(),
        );
        let mut selected: Vec<&SuggestorDescriptor> = Vec::new();
        let mut trace = vec![format!("selected template '{}'", metadata.id)];

        while !unmatched_roles.is_empty() || !unmatched_capabilities.is_empty() {
            let Some(next) = best_suggestor(
                (&catalogs.suggestors).into_iter(),
                &selected,
                &unmatched_roles,
                &unmatched_capabilities,
                &request.domain_tags,
            ) else {
                return Err(FormationCompileError::UncoveredRequirements {
                    unmatched_roles,
                    unmatched_capabilities,
                });
            };

            trace.push(format!(
                "selected suggestor '{}' for role {:?}",
                next.id, next.profile.role
            ));
            remove_role(&mut unmatched_roles, next.profile.role);
            remove_capabilities(&mut unmatched_capabilities, &next.profile.capabilities);
            selected.push(next);
        }

        let mut provider_assignments = Vec::new();
        for descriptor in &selected {
            let Some(requirements) = &descriptor.backend_requirements else {
                continue;
            };
            let Some(provider) =
                best_provider((&catalogs.providers).into_iter(), descriptor, requirements)
            else {
                return Err(FormationCompileError::MissingProvider {
                    suggestor_id: descriptor.id.clone(),
                    role: descriptor.profile.role,
                });
            };
            trace.push(format!(
                "assigned provider '{}' to suggestor '{}'",
                provider.id, descriptor.id
            ));
            provider_assignments.push(RoleProviderAssignment {
                suggestor_id: descriptor.id.clone(),
                role: descriptor.profile.role,
                provider_id: provider.id.clone(),
                requirements: requirements.clone(),
            });
        }

        let roster = selected
            .into_iter()
            .map(|descriptor| CompiledSuggestorRole {
                suggestor_id: descriptor.id.clone(),
                role: descriptor.profile.role,
                capabilities: descriptor.profile.capabilities.clone(),
                reads: descriptor.reads.clone(),
                writes: descriptor.profile.output_keys.clone(),
                input_contracts: descriptor.input_contracts.clone(),
                output_contracts: descriptor.output_contracts.clone(),
                replay_mode: descriptor.replay_mode,
                governance_class: descriptor.governance_class,
            })
            .collect();

        Ok(CompiledFormationPlan {
            plan_id: request.plan_id,
            correlation_id: request.correlation_id,
            tenant_id: request.tenant_id.clone(),
            template_id: metadata.id.clone(),
            template_kind: template.kind(),
            roster,
            provider_assignments,
            trace,
        })
    }
}

fn best_suggestor<'a>(
    candidates: impl Iterator<Item = &'a SuggestorDescriptor>,
    selected: &[&SuggestorDescriptor],
    unmatched_roles: &[SuggestorRole],
    unmatched_capabilities: &[SuggestorCapability],
    domain_tags: &[String],
) -> Option<&'a SuggestorDescriptor> {
    candidates
        .filter(|candidate| !selected.iter().any(|chosen| chosen.id == candidate.id))
        .map(|candidate| {
            let coverage = suggestor_coverage(candidate, unmatched_roles, unmatched_capabilities);
            let domain_hits = domain_overlap(&candidate.domain_tags, domain_tags);
            (candidate, coverage, domain_hits)
        })
        .filter(|(_, coverage, _)| *coverage > 0)
        .max_by(
            |(left, left_coverage, left_domain), (right, right_coverage, right_domain)| {
                left_coverage
                    .cmp(right_coverage)
                    .then_with(|| left_domain.cmp(right_domain))
                    .then_with(|| right.profile.cost_hint.cmp(&left.profile.cost_hint))
                    .then_with(|| right.profile.latency_hint.cmp(&left.profile.latency_hint))
                    .then_with(|| right.id.cmp(&left.id))
            },
        )
        .map(|(candidate, _, _)| candidate)
}

fn suggestor_coverage(
    candidate: &SuggestorDescriptor,
    unmatched_roles: &[SuggestorRole],
    unmatched_capabilities: &[SuggestorCapability],
) -> usize {
    let role_score = usize::from(unmatched_roles.contains(&candidate.profile.role));
    let capability_score = unmatched_capabilities
        .iter()
        .filter(|capability| candidate.profile.capabilities.contains(capability))
        .count();
    role_score + capability_score
}

fn best_provider<'a>(
    candidates: impl Iterator<Item = &'a ProviderDescriptor>,
    descriptor: &SuggestorDescriptor,
    requirements: &BackendRequirements,
) -> Option<&'a ProviderDescriptor> {
    candidates
        .filter(|candidate| provider_satisfies(candidate, requirements))
        .map(|candidate| {
            let role_hit = usize::from(candidate.role_affinity.contains(&descriptor.profile.role));
            let domain_hits = domain_overlap(&candidate.domain_tags, &descriptor.domain_tags);
            (candidate, role_hit, domain_hits)
        })
        .max_by(
            |(left, left_role, left_domain), (right, right_role, right_domain)| {
                left_role
                    .cmp(right_role)
                    .then_with(|| left_domain.cmp(right_domain))
                    .then_with(|| {
                        right
                            .requirements
                            .max_cost_class
                            .cmp(&left.requirements.max_cost_class)
                    })
                    .then_with(|| {
                        right
                            .requirements
                            .max_latency_ms
                            .cmp(&left.requirements.max_latency_ms)
                    })
                    .then_with(|| right.id.cmp(&left.id))
            },
        )
        .map(|(candidate, _, _)| candidate)
}

fn provider_satisfies(provider: &ProviderDescriptor, requirements: &BackendRequirements) -> bool {
    provider.requirements.kind == requirements.kind
        && requirements.required_capabilities.iter().all(|capability| {
            provider
                .requirements
                .required_capabilities
                .contains(capability)
        })
        && provider.requirements.max_cost_class <= requirements.max_cost_class
        && latency_satisfies(
            provider.requirements.max_latency_ms,
            requirements.max_latency_ms,
        )
        && sovereignty_satisfies(
            provider.requirements.data_sovereignty,
            requirements.data_sovereignty,
        )
        && compliance_satisfies(provider.requirements.compliance, requirements.compliance)
        && (!requirements.requires_replay || provider.requirements.requires_replay)
        && (!requirements.requires_offline || provider.requirements.requires_offline)
}

fn latency_satisfies(provider_ms: u32, required_ms: u32) -> bool {
    required_ms == 0 || provider_ms <= required_ms
}

fn sovereignty_satisfies(provider: DataSovereignty, required: DataSovereignty) -> bool {
    match required {
        DataSovereignty::Any => true,
        _ => provider == required || provider == DataSovereignty::OnPremises,
    }
}

fn compliance_satisfies(provider: ComplianceLevel, required: ComplianceLevel) -> bool {
    required == ComplianceLevel::None || provider == required
}

fn domain_overlap(left: &[String], right: &[String]) -> usize {
    left.iter().filter(|tag| right.contains(tag)).count()
}

fn unique_capabilities(
    capabilities: impl IntoIterator<Item = SuggestorCapability>,
) -> Vec<SuggestorCapability> {
    let mut unique = Vec::new();
    for capability in capabilities {
        if !unique.contains(&capability) {
            unique.push(capability);
        }
    }
    unique
}

fn remove_role(roles: &mut Vec<SuggestorRole>, role: SuggestorRole) {
    if let Some(index) = roles.iter().position(|candidate| *candidate == role) {
        roles.remove(index);
    }
}

fn remove_capabilities(
    capabilities: &mut Vec<SuggestorCapability>,
    covered: &[SuggestorCapability],
) {
    capabilities.retain(|capability| !covered.contains(capability));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vendor_selection::vendor_selection_formation_catalog;
    use converge_provider_api::{BackendKind, Capability, CostClass, LatencyClass};

    fn id(n: u128) -> Uuid {
        Uuid::from_u128(n)
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

    fn policy_gate_descriptor(policy_requirements: BackendRequirements) -> SuggestorDescriptor {
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
        .with_backend_requirements(policy_requirements)
    }

    fn decision_synthesis_descriptor() -> SuggestorDescriptor {
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
    }

    fn cedar_provider(policy_requirements: BackendRequirements) -> ProviderDescriptor {
        ProviderDescriptor::new(
            "cedar-local",
            "Cedar local policy engine",
            policy_requirements,
        )
        .with_role_affinity(SuggestorRole::Constraint)
        .with_domain_tag("vendor-selection")
    }

    fn complete_vendor_selection_catalogs(
        policy_requirements: BackendRequirements,
    ) -> FormationCompilerCatalogs {
        FormationCompilerCatalogs::new(vendor_selection_formation_catalog())
            .with_suggestor(market_scan_descriptor())
            .with_suggestor(weighted_evaluator_descriptor())
            .with_suggestor(policy_gate_descriptor(policy_requirements.clone()))
            .with_suggestor(decision_synthesis_descriptor())
            .with_provider(cedar_provider(policy_requirements))
    }

    #[test]
    fn compiles_complementary_vendor_selection_team() {
        let request = FormationCompileRequest::new(
            id(1),
            id(2),
            FormationTemplateQuery::new()
                .with_keyword("vendor")
                .with_keyword("diligence-evaluate-decide")
                .with_entity("VendorSelectionDecisionRecord"),
        )
        .with_tenant_id("tenant-a")
        .with_domain_tag("vendor-selection");

        let policy_requirements = BackendRequirements::access_policy().with_replay();
        let catalogs = complete_vendor_selection_catalogs(policy_requirements);

        let plan = FormationCompiler::new()
            .compile(&request, &catalogs)
            .expect("vendor selection should compile");

        assert_eq!(plan.template_id, "vendor-selection-decide");
        assert_eq!(plan.correlation_id, id(2));
        assert_eq!(plan.tenant_id.as_deref(), Some("tenant-a"));
        assert_eq!(plan.roster.len(), 4);
        assert_eq!(plan.provider_assignments.len(), 1);
        assert_eq!(plan.provider_assignments[0].provider_id, "cedar-local");
        assert!(
            plan.roster
                .iter()
                .any(|role| role.suggestor_id == "market-scan")
        );
        assert!(
            plan.roster
                .iter()
                .any(|role| role.suggestor_id == "weighted-evaluator")
        );
        assert!(
            plan.roster
                .iter()
                .any(|role| role.suggestor_id == "policy-gate")
        );
        assert!(
            plan.roster
                .iter()
                .any(|role| role.suggestor_id == "decision-synthesis")
        );
    }

    #[test]
    fn reports_uncovered_requirements_instead_of_over_filtering() {
        let request = FormationCompileRequest::new(
            id(3),
            id(4),
            FormationTemplateQuery::new()
                .with_keyword("vendor")
                .with_keyword("diligence-evaluate-decide"),
        );
        let catalogs = FormationCompilerCatalogs::new(vendor_selection_formation_catalog())
            .with_suggestor(SuggestorDescriptor::new(
                "analytics-only",
                profile(
                    "analytics-only",
                    SuggestorRole::Evaluation,
                    vec![ContextKey::Evaluations],
                    vec![SuggestorCapability::Analytics],
                ),
            ));

        let error = FormationCompiler::new()
            .compile(&request, &catalogs)
            .expect_err("missing roles and capabilities should be explicit");

        match error {
            FormationCompileError::UncoveredRequirements {
                unmatched_roles,
                unmatched_capabilities,
            } => {
                assert!(unmatched_roles.contains(&SuggestorRole::Signal));
                assert!(unmatched_roles.contains(&SuggestorRole::Constraint));
                assert!(unmatched_roles.contains(&SuggestorRole::Synthesis));
                assert!(unmatched_capabilities.contains(&SuggestorCapability::KnowledgeRetrieval));
                assert!(unmatched_capabilities.contains(&SuggestorCapability::PolicyEnforcement));
                assert!(unmatched_capabilities.contains(&SuggestorCapability::LlmReasoning));
            }
            other => panic!("unexpected compile error: {other:?}"),
        }
    }

    #[test]
    fn requires_role_level_provider_match_when_backend_is_declared() {
        let request = FormationCompileRequest::new(
            id(5),
            id(6),
            FormationTemplateQuery::new()
                .with_keyword("vendor")
                .with_keyword("diligence-evaluate-decide"),
        );
        let policy_requirements = BackendRequirements::access_policy().with_replay();
        let catalogs = FormationCompilerCatalogs::new(vendor_selection_formation_catalog())
            .with_suggestor(market_scan_descriptor())
            .with_suggestor(weighted_evaluator_descriptor())
            .with_suggestor(policy_gate_descriptor(policy_requirements))
            .with_suggestor(decision_synthesis_descriptor())
            .with_provider(ProviderDescriptor::new(
                "generic-llm",
                "Generic LLM",
                BackendRequirements::reasoning_llm(),
            ));

        let error = FormationCompiler::new()
            .compile(&request, &catalogs)
            .expect_err("policy role should not route to an LLM provider");

        assert_eq!(
            error,
            FormationCompileError::MissingProvider {
                suggestor_id: "policy-gate".to_string(),
                role: SuggestorRole::Constraint,
            }
        );
    }

    #[test]
    fn carries_rich_provider_requirements_per_role() {
        let requirements = BackendRequirements::new(BackendKind::Llm)
            .with_capability(Capability::TextGeneration)
            .with_capability(Capability::Reasoning)
            .with_data_sovereignty(DataSovereignty::EU)
            .with_compliance(ComplianceLevel::HighExplainability)
            .with_capability(Capability::StructuredOutput);

        let descriptor = SuggestorDescriptor::new(
            "decision-synthesis",
            profile(
                "decision-synthesis",
                SuggestorRole::Synthesis,
                vec![ContextKey::Proposals],
                vec![SuggestorCapability::LlmReasoning],
            ),
        )
        .with_backend_requirements(requirements.clone());

        assert_eq!(
            descriptor
                .backend_requirements
                .as_ref()
                .expect("requirements should be present")
                .data_sovereignty,
            DataSovereignty::EU
        );
        assert!(
            descriptor
                .backend_requirements
                .as_ref()
                .expect("requirements should be present")
                .required_capabilities
                .contains(&Capability::StructuredOutput)
        );
    }
}
