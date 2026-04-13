// Copyright 2024-2025 Aprio One AB, Sweden
// SPDX-License-Identifier: MIT

//! Patent research agents for governed search and analysis workflows.
//!
//! These agents emit proposals that must be validated before becoming facts.

use converge_core::validation::encode_proposal;
use converge_core::{
    Agent, AgentEffect, Context, ContextKey, Fact, ProposedFact,
    invariant::{Invariant, InvariantClass, InvariantResult, Violation},
};
use converge_provider::ProviderCallContext;
use converge_provider::patent::{PatentOperator, PatentSearchProvider, PatentSearchRequest};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

// =============================================================================
// Prefixes
// =============================================================================

pub const PATENT_QUERY_PREFIX: &str = "patent_query:";
pub const OPERATOR_PLAN_PREFIX: &str = "patent_operator_plan:";
pub const PATENT_RESULT_PREFIX: &str = "patent_result:";
pub const PRIOR_ART_PREFIX: &str = "prior_art:";
pub const CLAIM_CHART_PREFIX: &str = "claim_chart:";
pub const LANDSCAPE_PREFIX: &str = "patent_landscape:";
pub const PATENT_REPORT_PREFIX: &str = "patent_report:";
pub const PATENT_ALERT_PREFIX: &str = "patent_alert:";
pub const PAID_ACTION_PREFIX: &str = "paid_action:";
pub const PATENT_SUBMISSION_PREFIX: &str = "patent_submission:";
pub const APPROVAL_PREFIX: &str = "approval:";
pub const MATTER_POLICY_PREFIX: &str = "matter_policy:";
pub const MATTER_CONTEXT_PREFIX: &str = "matter_context:";
pub const DISCLOSURE_DRAFT_PREFIX: &str = "disclosure_draft:";
pub const INVENTION_SUMMARY_PREFIX: &str = "invention_summary:";
pub const CLAIM_SEED_PREFIX: &str = "claim_seed:";
pub const PRIOR_ART_SHORTLIST_PREFIX: &str = "prior_art_shortlist:";
pub const CLAIM_RISK_PREFIX: &str = "claim_risk:";
pub const EXPANDED_QUERY_PREFIX: &str = "expanded_query:";
pub const ALT_CLAIM_STRATEGY_PREFIX: &str = "alt_claim_strategy:";
pub const CLAIM_SET_PREFIX: &str = "claim_set:";
pub const SPEC_DRAFT_PREFIX: &str = "spec_draft:";
pub const SUPPORT_MATRIX_PREFIX: &str = "support_matrix:";
pub const DRAFT_PACK_PREFIX: &str = "draft_pack:";

// =============================================================================
// Data Shapes
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PatentQuery {
    query_text: String,
    operators: Vec<PatentOperator>,
    include_paid: bool,
    account_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OperatorPlan {
    query_text: String,
    operators: Vec<PatentOperator>,
    include_paid: bool,
    account_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PriorArtEvidence {
    source_result_id: String,
    provenance: String,
    receipt_logged: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ClaimChart {
    evidence_ids: Vec<String>,
    analysis_summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct LandscapeSummary {
    operators: Vec<String>,
    summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ResearchReport {
    query_id: String,
    operators: Vec<String>,
    evidence_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AlertConfig {
    report_id: String,
    watch_terms: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SubmissionAction {
    query_id: String,
    requires_approval: bool,
    evidence_required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MatterPolicy {
    client: String,
    jurisdiction: String,
    confidentiality_tier: String,
    allowed_backends: Vec<String>,
    budgets: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MatterContext {
    matter_id: String,
    tenant_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DisclosureDraft {
    draft_text: String,
    newness: String,
    how_it_works: String,
    embodiments: Vec<String>,
    missing_support: Vec<String>,
    local_replayable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct InventionSummary {
    summary_lines: Vec<String>,
    claim_terms: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ClaimSeed {
    claim_text: String,
    strategy: String,
    support_terms: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PriorArtShortlist {
    items: Vec<String>,
    citations: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ClaimRiskFlags {
    flags: Vec<String>,
    citations: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ExpandedQuerySet {
    queries: Vec<String>,
    audit_only: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AlternateClaimStrategy {
    description: String,
    audit_only: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ClaimSet {
    strategy: String,
    claims: Vec<String>,
    claim_terms: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SpecDraft {
    sections: Vec<String>,
    embodiments: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SupportMatrix {
    term_to_sections: Vec<(String, Vec<String>)>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DraftPack {
    report_id: String,
    claim_set_ids: Vec<String>,
    spec_id: String,
}

// =============================================================================
// Helpers
// =============================================================================

fn has_prefix(ctx: &Context, key: ContextKey, prefix: &str) -> bool {
    ctx.get(key).iter().any(|fact| fact.id.starts_with(prefix))
}

/// Check if a prefix exists within proposals (proposal ids contain the prefix).
/// Proposals have format: proposal:{target_key}:{original_id}
fn has_prefix_in_proposals(ctx: &Context, prefix: &str) -> bool {
    ctx.get(ContextKey::Proposals)
        .iter()
        .any(|fact| fact.id.contains(prefix))
}

fn proposal_fact(
    target: ContextKey,
    id: impl Into<String>,
    content: impl Into<String>,
    confidence: f64,
    provenance: impl Into<String>,
) -> Fact {
    encode_proposal(&ProposedFact {
        key: target,
        id: id.into(),
        content: content.into(),
        confidence,
        provenance: provenance.into(),
    })
}

fn default_operators() -> Vec<PatentOperator> {
    vec![
        PatentOperator::Uspto,
        PatentOperator::Epo,
        PatentOperator::Wipo,
        PatentOperator::GooglePatents,
        PatentOperator::Lens,
    ]
}

fn parse_json(content: &str) -> Option<serde_json::Value> {
    let trimmed = content.trim();
    if trimmed.starts_with('{') && trimmed.ends_with('}') {
        serde_json::from_str(trimmed).ok()
    } else {
        None
    }
}

// =============================================================================
// Agents
// =============================================================================

/// Build MatterPolicy from matter setup inputs.
#[derive(Debug, Clone, Default)]
pub struct MatterPolicyAgent;

impl Agent for MatterPolicyAgent {
    fn name(&self) -> &str {
        "matter_policy_builder"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[
            ContextKey::Seeds,
            ContextKey::Constraints,
            ContextKey::Proposals,
        ]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        let has_setup = ctx
            .get(ContextKey::Seeds)
            .iter()
            .any(|seed| seed.content.contains("matter.setup"));
        let has_policy = has_prefix(ctx, ContextKey::Constraints, MATTER_POLICY_PREFIX)
            || has_prefix(ctx, ContextKey::Proposals, MATTER_POLICY_PREFIX);
        has_setup && !has_policy
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let mut facts = Vec::new();
        for seed in ctx.get(ContextKey::Seeds).iter() {
            if !seed.content.contains("matter.setup") {
                continue;
            }

            let json = parse_json(&seed.content).unwrap_or_else(|| serde_json::json!({}));
            let confidentiality = json
                .get("confidentiality_tier")
                .and_then(|v| v.as_str())
                .unwrap_or("standard");
            let allowed_backends = if confidentiality.eq_ignore_ascii_case("restricted") {
                vec!["local".to_string()]
            } else {
                vec!["local".to_string(), "remote_audit".to_string()]
            };

            let policy = MatterPolicy {
                client: json
                    .get("client")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown")
                    .to_string(),
                jurisdiction: json
                    .get("jurisdiction")
                    .and_then(|v| v.as_str())
                    .unwrap_or("US")
                    .to_string(),
                confidentiality_tier: confidentiality.to_string(),
                allowed_backends,
                budgets: json
                    .get("budgets")
                    .cloned()
                    .unwrap_or_else(|| serde_json::json!({})),
            };

            let content = serde_json::to_string(&policy).unwrap_or_default();
            facts.push(proposal_fact(
                ContextKey::Constraints,
                format!("{}{}", MATTER_POLICY_PREFIX, seed.id),
                content,
                0.95,
                format!("seed:{}", seed.id),
            ));
        }
        AgentEffect::with_facts(facts)
    }
}

/// Build MatterContext (tenant scope).
#[derive(Debug, Clone, Default)]
pub struct MatterContextAgent;

impl Agent for MatterContextAgent {
    fn name(&self) -> &str {
        "matter_context_builder"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[
            ContextKey::Seeds,
            ContextKey::Constraints,
            ContextKey::Proposals,
        ]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        let has_setup = ctx
            .get(ContextKey::Seeds)
            .iter()
            .any(|seed| seed.content.contains("matter.setup"));
        let has_context = has_prefix(ctx, ContextKey::Constraints, MATTER_CONTEXT_PREFIX)
            || has_prefix(ctx, ContextKey::Proposals, MATTER_CONTEXT_PREFIX);
        has_setup && !has_context
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let mut facts = Vec::new();
        for seed in ctx.get(ContextKey::Seeds).iter() {
            if !seed.content.contains("matter.setup") {
                continue;
            }
            let json = parse_json(&seed.content).unwrap_or_else(|| serde_json::json!({}));
            let matter = MatterContext {
                matter_id: json
                    .get("matter_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or(&seed.id)
                    .to_string(),
                tenant_id: json
                    .get("tenant_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("default_tenant")
                    .to_string(),
            };
            let content = serde_json::to_string(&matter).unwrap_or_default();
            facts.push(proposal_fact(
                ContextKey::Constraints,
                format!("{}{}", MATTER_CONTEXT_PREFIX, seed.id),
                content,
                0.9,
                format!("seed:{}", seed.id),
            ));
        }
        AgentEffect::with_facts(facts)
    }
}

/// Draft disclosure packet from inventor intake.
#[derive(Debug, Clone, Default)]
pub struct InventionCaptureAgent;

impl Agent for InventionCaptureAgent {
    fn name(&self) -> &str {
        "invention_capture"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[
            ContextKey::Seeds,
            ContextKey::Strategies,
            ContextKey::Proposals,
        ]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        let has_intake = ctx
            .get(ContextKey::Seeds)
            .iter()
            .any(|seed| seed.content.contains("inventor.intake"));
        let has_disclosure = has_prefix(ctx, ContextKey::Strategies, DISCLOSURE_DRAFT_PREFIX)
            || has_prefix(ctx, ContextKey::Proposals, DISCLOSURE_DRAFT_PREFIX);
        has_intake && !has_disclosure
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let mut facts = Vec::new();
        for seed in ctx.get(ContextKey::Seeds).iter() {
            if !seed.content.contains("inventor.intake") {
                continue;
            }
            let disclosure = DisclosureDraft {
                draft_text: "Disclosure draft v0".to_string(),
                newness: "Novel mechanism described".to_string(),
                how_it_works: "Mechanism description".to_string(),
                embodiments: vec![
                    "Embodiment A".to_string(),
                    "Embodiment B".to_string(),
                    "Embodiment C".to_string(),
                ],
                missing_support: Vec::new(),
                local_replayable: true,
            };
            let content = serde_json::to_string(&disclosure).unwrap_or_default();
            facts.push(proposal_fact(
                ContextKey::Strategies,
                format!("{}{}", DISCLOSURE_DRAFT_PREFIX, seed.id),
                content,
                0.8,
                format!("seed:{}", seed.id),
            ));
        }
        AgentEffect::with_facts(facts)
    }
}

/// Summarize invention into structured lines and terms.
#[derive(Debug, Clone, Default)]
pub struct InventionSummaryAgent;

impl Agent for InventionSummaryAgent {
    fn name(&self) -> &str {
        "invention_summary"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[
            ContextKey::Strategies,
            ContextKey::Signals,
            ContextKey::Proposals,
        ]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        // Check for disclosure in Strategies or in Proposals (as pending proposal)
        let has_disclosure = has_prefix(ctx, ContextKey::Strategies, DISCLOSURE_DRAFT_PREFIX)
            || has_prefix_in_proposals(ctx, DISCLOSURE_DRAFT_PREFIX);
        let has_summary = has_prefix(ctx, ContextKey::Signals, INVENTION_SUMMARY_PREFIX)
            || has_prefix(ctx, ContextKey::Proposals, INVENTION_SUMMARY_PREFIX);
        has_disclosure && !has_summary
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        // Look for disclosure in Strategies first, then in Proposals
        let disclosure_id = ctx
            .get(ContextKey::Strategies)
            .iter()
            .find(|fact| fact.id.starts_with(DISCLOSURE_DRAFT_PREFIX))
            .or_else(|| {
                ctx.get(ContextKey::Proposals)
                    .iter()
                    .find(|fact| fact.id.contains(DISCLOSURE_DRAFT_PREFIX))
            })
            .map(|fact| fact.id.clone())
            .unwrap_or_else(|| "disclosure:unknown".to_string());
        let summary = InventionSummary {
            summary_lines: vec!["Summary line 1".to_string(), "Summary line 2".to_string()],
            claim_terms: vec!["term_a".to_string(), "term_b".to_string()],
        };
        let content = serde_json::to_string(&summary).unwrap_or_default();
        AgentEffect::with_facts(vec![proposal_fact(
            ContextKey::Signals,
            format!("{}{}", INVENTION_SUMMARY_PREFIX, disclosure_id),
            content,
            0.8,
            "analysis:summary".to_string(),
        )])
    }
}

/// Generate claim seeds from invention summary.
#[derive(Debug, Clone, Default)]
pub struct ClaimSeedAgent;

impl Agent for ClaimSeedAgent {
    fn name(&self) -> &str {
        "claim_seed_generator"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[
            ContextKey::Signals,
            ContextKey::Hypotheses,
            ContextKey::Proposals,
        ]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        // Check for summary in Signals or in Proposals (as pending proposal)
        let has_summary = has_prefix(ctx, ContextKey::Signals, INVENTION_SUMMARY_PREFIX)
            || has_prefix_in_proposals(ctx, INVENTION_SUMMARY_PREFIX);
        let has_seeds = has_prefix(ctx, ContextKey::Hypotheses, CLAIM_SEED_PREFIX)
            || has_prefix(ctx, ContextKey::Proposals, CLAIM_SEED_PREFIX);
        has_summary && !has_seeds
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        // Look for summary in Signals first, then in Proposals
        let summary_id = ctx
            .get(ContextKey::Signals)
            .iter()
            .find(|fact| fact.id.starts_with(INVENTION_SUMMARY_PREFIX))
            .or_else(|| {
                ctx.get(ContextKey::Proposals)
                    .iter()
                    .find(|fact| fact.id.contains(INVENTION_SUMMARY_PREFIX))
            })
            .map(|fact| fact.id.clone())
            .unwrap_or_else(|| "summary:unknown".to_string());

        let seed = ClaimSeed {
            claim_text: "An independent claim draft".to_string(),
            strategy: "broad".to_string(),
            support_terms: vec!["term_a".to_string()],
        };
        let content = serde_json::to_string(&seed).unwrap_or_default();
        AgentEffect::with_facts(vec![proposal_fact(
            ContextKey::Hypotheses,
            format!("{}{}", CLAIM_SEED_PREFIX, summary_id),
            content,
            0.75,
            "analysis:claim_seed".to_string(),
        )])
    }
}
/// Build a scoped patent query from seed requests.
#[derive(Debug, Clone, Default)]
pub struct PatentQueryBuilderAgent;

impl Agent for PatentQueryBuilderAgent {
    fn name(&self) -> &str {
        "patent_query_builder"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[
            ContextKey::Seeds,
            ContextKey::Constraints,
            ContextKey::Proposals,
        ]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        let has_request = ctx.get(ContextKey::Seeds).iter().any(|seed| {
            seed.content.contains("patent.research.request")
                || seed.content.contains("patent.query")
        });
        let has_query = has_prefix(ctx, ContextKey::Constraints, PATENT_QUERY_PREFIX)
            || has_prefix(ctx, ContextKey::Proposals, PATENT_QUERY_PREFIX);
        has_request && !has_query
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let mut facts = Vec::new();
        for seed in ctx.get(ContextKey::Seeds).iter() {
            if seed.content.contains("patent.research.request")
                || seed.content.contains("patent.query")
            {
                let query = PatentQuery {
                    query_text: seed.content.clone(),
                    operators: default_operators(),
                    include_paid: true,
                    account_id: None,
                };
                let content = serde_json::to_string(&query).unwrap_or_default();
                facts.push(proposal_fact(
                    ContextKey::Constraints,
                    format!("{}{}", PATENT_QUERY_PREFIX, seed.id),
                    content,
                    0.9,
                    format!("seed:{}", seed.id),
                ));
            }
        }
        AgentEffect::with_facts(facts)
    }
}

/// Plan operator mix and budgets from the query.
#[derive(Debug, Clone, Default)]
pub struct PatentOperatorPlannerAgent;

impl Agent for PatentOperatorPlannerAgent {
    fn name(&self) -> &str {
        "patent_operator_planner"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[
            ContextKey::Constraints,
            ContextKey::Strategies,
            ContextKey::Proposals,
        ]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        let has_query = has_prefix(ctx, ContextKey::Constraints, PATENT_QUERY_PREFIX);
        let has_plan = has_prefix(ctx, ContextKey::Strategies, OPERATOR_PLAN_PREFIX)
            || has_prefix(ctx, ContextKey::Proposals, OPERATOR_PLAN_PREFIX);
        has_query && !has_plan
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let mut facts = Vec::new();
        for query_fact in ctx.get(ContextKey::Constraints).iter() {
            if query_fact.id.starts_with(PATENT_QUERY_PREFIX) {
                let query: PatentQuery =
                    serde_json::from_str(&query_fact.content).unwrap_or(PatentQuery {
                        query_text: query_fact.content.clone(),
                        operators: default_operators(),
                        include_paid: true,
                        account_id: None,
                    });
                let plan = OperatorPlan {
                    query_text: query.query_text,
                    operators: query.operators,
                    include_paid: query.include_paid,
                    account_id: query.account_id,
                };
                let content = serde_json::to_string(&plan).unwrap_or_default();
                facts.push(proposal_fact(
                    ContextKey::Strategies,
                    format!("{}{}", OPERATOR_PLAN_PREFIX, query_fact.id),
                    content,
                    0.85,
                    format!("query:{}", query_fact.id),
                ));
            }
        }
        AgentEffect::with_facts(facts)
    }
}

/// Execute searches using a patent search provider.
#[derive(Clone)]
pub struct PatentSearchExecutorAgent {
    provider: Arc<dyn PatentSearchProvider>,
}

impl std::fmt::Debug for PatentSearchExecutorAgent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PatentSearchExecutorAgent")
            .field("provider", &self.provider.name())
            .finish()
    }
}

impl PatentSearchExecutorAgent {
    #[must_use]
    pub fn new(provider: Arc<dyn PatentSearchProvider>) -> Self {
        Self { provider }
    }
}

impl Agent for PatentSearchExecutorAgent {
    fn name(&self) -> &str {
        "patent_search_executor"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[
            ContextKey::Strategies,
            ContextKey::Signals,
            ContextKey::Proposals,
        ]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        let has_plan = has_prefix(ctx, ContextKey::Strategies, OPERATOR_PLAN_PREFIX);
        let has_results = has_prefix(ctx, ContextKey::Signals, PATENT_RESULT_PREFIX)
            || has_prefix(ctx, ContextKey::Proposals, PATENT_RESULT_PREFIX);
        has_plan && !has_results
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let mut facts = Vec::new();
        for plan_fact in ctx.get(ContextKey::Strategies).iter() {
            if plan_fact.id.starts_with(OPERATOR_PLAN_PREFIX) {
                let plan: OperatorPlan =
                    serde_json::from_str(&plan_fact.content).unwrap_or(OperatorPlan {
                        query_text: plan_fact.content.clone(),
                        operators: default_operators(),
                        include_paid: true,
                        account_id: None,
                    });

                let request = PatentSearchRequest {
                    query: plan.query_text.clone(),
                    operators: plan.operators.clone(),
                    include_paid: plan.include_paid,
                    account_id: plan.account_id.clone(),
                    filters: serde_json::json!({}),
                };

                let ctx_call = ProviderCallContext::default();
                let response = match self.provider.search(&request, &ctx_call) {
                    Ok(resp) => resp,
                    Err(err) => {
                        return AgentEffect::with_facts(vec![Fact {
                            key: ContextKey::Signals,
                            id: format!("patent_search_error:{}", plan_fact.id),
                            content: err.to_string(),
                        }]);
                    }
                };

                for obs in response.results {
                    let content = serde_json::json!({
                        "publication_id": obs.content.publication_id,
                        "title": obs.content.title,
                        "abstract": obs.content.abstract_text,
                        "operator": obs.content.operator.as_str(),
                        "url": obs.content.url,
                        "provenance": obs.provenance(),
                    })
                    .to_string();
                    facts.push(proposal_fact(
                        ContextKey::Signals,
                        format!("{}{}", PATENT_RESULT_PREFIX, obs.observation_id),
                        content,
                        0.8,
                        obs.provenance(),
                    ));
                }

                for paid in response.paid_actions {
                    let paid_content = serde_json::to_string(&paid).unwrap_or_default();
                    facts.push(proposal_fact(
                        ContextKey::Strategies,
                        format!("{}{}", PAID_ACTION_PREFIX, paid.action_id),
                        paid_content,
                        0.8,
                        format!("provider:{}", self.provider.name()),
                    ));
                }
            }
        }
        AgentEffect::with_facts(facts)
    }
}

/// Collect prior art evidence from search results.
#[derive(Debug, Clone, Default)]
pub struct PatentEvidenceCollectorAgent;

impl Agent for PatentEvidenceCollectorAgent {
    fn name(&self) -> &str {
        "patent_evidence_collector"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[
            ContextKey::Signals,
            ContextKey::Evaluations,
            ContextKey::Proposals,
        ]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        let has_results = has_prefix(ctx, ContextKey::Signals, PATENT_RESULT_PREFIX);
        let has_evidence = has_prefix(ctx, ContextKey::Evaluations, PRIOR_ART_PREFIX)
            || has_prefix(ctx, ContextKey::Proposals, PRIOR_ART_PREFIX);
        has_results && !has_evidence
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let mut facts = Vec::new();
        for result in ctx.get(ContextKey::Signals).iter() {
            if result.id.starts_with(PATENT_RESULT_PREFIX) {
                let evidence = PriorArtEvidence {
                    source_result_id: result.id.clone(),
                    provenance: format!("result:{}", result.id),
                    receipt_logged: true,
                };
                let content = serde_json::to_string(&evidence).unwrap_or_default();
                facts.push(proposal_fact(
                    ContextKey::Evaluations,
                    format!("{}{}", PRIOR_ART_PREFIX, result.id),
                    content,
                    0.85,
                    format!("result:{}", result.id),
                ));
            }
        }
        AgentEffect::with_facts(facts)
    }
}

/// Generate claim charts from evidence.
#[derive(Debug, Clone, Default)]
pub struct PatentClaimsAnalyzerAgent;

impl Agent for PatentClaimsAnalyzerAgent {
    fn name(&self) -> &str {
        "patent_claims_analyzer"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[
            ContextKey::Evaluations,
            ContextKey::Hypotheses,
            ContextKey::Proposals,
        ]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        let has_evidence = has_prefix(ctx, ContextKey::Evaluations, PRIOR_ART_PREFIX);
        let has_claims = has_prefix(ctx, ContextKey::Hypotheses, CLAIM_CHART_PREFIX)
            || has_prefix(ctx, ContextKey::Proposals, CLAIM_CHART_PREFIX);
        has_evidence && !has_claims
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let evidence_ids: Vec<String> = ctx
            .get(ContextKey::Evaluations)
            .iter()
            .filter(|fact| fact.id.starts_with(PRIOR_ART_PREFIX))
            .map(|fact| fact.id.clone())
            .collect();

        if evidence_ids.is_empty() {
            return AgentEffect::empty();
        }

        let chart = ClaimChart {
            evidence_ids: evidence_ids.clone(),
            analysis_summary: "Claim chart generated from evidence".to_string(),
        };
        let content = serde_json::to_string(&chart).unwrap_or_default();

        AgentEffect::with_facts(vec![proposal_fact(
            ContextKey::Hypotheses,
            format!("{}{}", CLAIM_CHART_PREFIX, evidence_ids[0]),
            content,
            0.8,
            "analysis:claims".to_string(),
        )])
    }
}

/// Summarize the landscape from search results.
#[derive(Debug, Clone, Default)]
pub struct PatentLandscapeAnalyzerAgent;

impl Agent for PatentLandscapeAnalyzerAgent {
    fn name(&self) -> &str {
        "patent_landscape_analyzer"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[
            ContextKey::Signals,
            ContextKey::Competitors,
            ContextKey::Proposals,
        ]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        let has_results = has_prefix(ctx, ContextKey::Signals, PATENT_RESULT_PREFIX);
        let has_landscape = has_prefix(ctx, ContextKey::Competitors, LANDSCAPE_PREFIX)
            || has_prefix(ctx, ContextKey::Proposals, LANDSCAPE_PREFIX);
        has_results && !has_landscape
    }

    fn execute(&self, _ctx: &Context) -> AgentEffect {
        let operators: Vec<String> = default_operators()
            .into_iter()
            .map(|op| op.as_str().to_string())
            .collect();
        let summary = LandscapeSummary {
            operators,
            summary: "Landscape summary derived from operator results".to_string(),
        };
        let content = serde_json::to_string(&summary).unwrap_or_default();

        AgentEffect::with_facts(vec![proposal_fact(
            ContextKey::Competitors,
            format!("{}summary", LANDSCAPE_PREFIX),
            content,
            0.75,
            "analysis:landscape".to_string(),
        )])
    }
}

/// Assemble a research report from analysis artifacts.
#[derive(Debug, Clone, Default)]
pub struct PatentReportAssemblerAgent;

impl Agent for PatentReportAssemblerAgent {
    fn name(&self) -> &str {
        "patent_report_assembler"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[
            ContextKey::Hypotheses,
            ContextKey::Evaluations,
            ContextKey::Competitors,
            ContextKey::Constraints,
            ContextKey::Strategies,
            ContextKey::Proposals,
        ]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        let has_analysis = has_prefix(ctx, ContextKey::Hypotheses, CLAIM_CHART_PREFIX)
            && has_prefix(ctx, ContextKey::Evaluations, PRIOR_ART_PREFIX);
        let has_report = has_prefix(ctx, ContextKey::Strategies, PATENT_REPORT_PREFIX)
            || has_prefix(ctx, ContextKey::Proposals, PATENT_REPORT_PREFIX);
        has_analysis && !has_report
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let query_id = ctx
            .get(ContextKey::Constraints)
            .iter()
            .find(|fact| fact.id.starts_with(PATENT_QUERY_PREFIX))
            .map(|fact| fact.id.clone())
            .unwrap_or_else(|| "unknown_query".to_string());

        let evidence_count = ctx
            .get(ContextKey::Evaluations)
            .iter()
            .filter(|fact| fact.id.starts_with(PRIOR_ART_PREFIX))
            .count();

        let report = ResearchReport {
            query_id: query_id.clone(),
            operators: default_operators()
                .into_iter()
                .map(|op| op.as_str().to_string())
                .collect(),
            evidence_count,
        };
        let content = serde_json::to_string(&report).unwrap_or_default();

        AgentEffect::with_facts(vec![proposal_fact(
            ContextKey::Strategies,
            format!("{}{}", PATENT_REPORT_PREFIX, query_id),
            content,
            0.8,
            "analysis:report".to_string(),
        )])
    }
}

/// Configure alerts/watchlists from the report.
#[derive(Debug, Clone, Default)]
pub struct PatentAlertAgent;

impl Agent for PatentAlertAgent {
    fn name(&self) -> &str {
        "patent_alert_agent"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[
            ContextKey::Strategies,
            ContextKey::Signals,
            ContextKey::Proposals,
        ]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        let has_report = has_prefix(ctx, ContextKey::Strategies, PATENT_REPORT_PREFIX);
        let has_alerts = has_prefix(ctx, ContextKey::Signals, PATENT_ALERT_PREFIX)
            || has_prefix(ctx, ContextKey::Proposals, PATENT_ALERT_PREFIX);
        has_report && !has_alerts
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let report_id = ctx
            .get(ContextKey::Strategies)
            .iter()
            .find(|fact| fact.id.starts_with(PATENT_REPORT_PREFIX))
            .map(|fact| fact.id.clone())
            .unwrap_or_else(|| "report:unknown".to_string());

        let alert = AlertConfig {
            report_id: report_id.clone(),
            watch_terms: vec!["new filings".to_string(), "priority claims".to_string()],
        };
        let content = serde_json::to_string(&alert).unwrap_or_default();

        AgentEffect::with_facts(vec![proposal_fact(
            ContextKey::Signals,
            format!("{}{}", PATENT_ALERT_PREFIX, report_id),
            content,
            0.75,
            "analysis:alerts".to_string(),
        )])
    }
}

/// Propose a patent submission (approval gated).
#[derive(Debug, Clone, Default)]
pub struct PatentSubmissionAgent;

impl Agent for PatentSubmissionAgent {
    fn name(&self) -> &str {
        "patent_submission_agent"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[
            ContextKey::Seeds,
            ContextKey::Strategies,
            ContextKey::Proposals,
        ]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        let has_request = ctx
            .get(ContextKey::Seeds)
            .iter()
            .any(|seed| seed.content.contains("patent.submission.request"));
        let has_submission = has_prefix(ctx, ContextKey::Strategies, PATENT_SUBMISSION_PREFIX)
            || has_prefix(ctx, ContextKey::Proposals, PATENT_SUBMISSION_PREFIX);
        has_request && !has_submission
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let query_id = ctx
            .get(ContextKey::Strategies)
            .iter()
            .find(|fact| fact.id.starts_with(PATENT_REPORT_PREFIX))
            .map(|fact| fact.id.clone())
            .unwrap_or_else(|| "report:unknown".to_string());

        let action = SubmissionAction {
            query_id: query_id.clone(),
            requires_approval: true,
            evidence_required: true,
        };
        let content = serde_json::to_string(&action).unwrap_or_default();

        let fact = proposal_fact(
            ContextKey::Strategies,
            format!("{}{}", PATENT_SUBMISSION_PREFIX, query_id),
            content,
            0.7,
            "submission:requested".to_string(),
        );

        AgentEffect::with_facts(vec![fact])
    }
}

/// Records explicit approvals from seed facts.
#[derive(Debug, Clone, Default)]
pub struct PatentApprovalRecorderAgent;

impl Agent for PatentApprovalRecorderAgent {
    fn name(&self) -> &str {
        "patent_approval_recorder"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Seeds, ContextKey::Constraints]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        let has_approval_seed = ctx
            .get(ContextKey::Seeds)
            .iter()
            .any(|seed| seed.content.contains("approval.granted"));
        let has_approval = has_prefix(ctx, ContextKey::Constraints, APPROVAL_PREFIX);
        has_approval_seed && !has_approval
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let mut facts = Vec::new();
        for seed in ctx.get(ContextKey::Seeds).iter() {
            if seed.content.contains("approval.granted") {
                let parsed = parse_json(&seed.content);
                let approver_role = parsed
                    .as_ref()
                    .and_then(|json| json.get("role").and_then(|v| v.as_str()))
                    .or_else(|| {
                        if seed.content.contains("inventor") {
                            Some("inventor")
                        } else if seed.content.contains("attorney") {
                            Some("attorney")
                        } else {
                            None
                        }
                    })
                    .unwrap_or("legal_lead");
                let scope = parsed
                    .as_ref()
                    .and_then(|json| json.get("scope").and_then(|v| v.as_str()))
                    .unwrap_or("general");
                let approval = serde_json::json!({
                    "approver_role": approver_role,
                    "scope": scope,
                    "approval_id": seed.id,
                    "granted_at": "2026-01-12",
                });
                facts.push(Fact {
                    key: ContextKey::Constraints,
                    id: format!("{}{}", APPROVAL_PREFIX, seed.id),
                    content: approval.to_string(),
                });
            }
        }
        AgentEffect::with_facts(facts)
    }
}

/// Build baseline prior art shortlist from search results.
#[derive(Debug, Clone, Default)]
pub struct PriorArtShortlistAgent;

impl Agent for PriorArtShortlistAgent {
    fn name(&self) -> &str {
        "prior_art_shortlist"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[
            ContextKey::Signals,
            ContextKey::Evaluations,
            ContextKey::Proposals,
        ]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        let has_results = has_prefix(ctx, ContextKey::Signals, PATENT_RESULT_PREFIX);
        let has_shortlist = has_prefix(ctx, ContextKey::Evaluations, PRIOR_ART_SHORTLIST_PREFIX)
            || has_prefix(ctx, ContextKey::Proposals, PRIOR_ART_SHORTLIST_PREFIX);
        has_results && !has_shortlist
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let mut items = Vec::new();
        for fact in ctx.get(ContextKey::Signals).iter() {
            if fact.id.starts_with(PATENT_RESULT_PREFIX) {
                items.push(fact.id.clone());
            }
        }
        let citations = items.iter().take(10).cloned().collect::<Vec<_>>();
        let shortlist = PriorArtShortlist { items, citations };
        let content = serde_json::to_string(&shortlist).unwrap_or_default();
        AgentEffect::with_facts(vec![proposal_fact(
            ContextKey::Evaluations,
            format!("{}baseline", PRIOR_ART_SHORTLIST_PREFIX),
            content,
            0.8,
            "analysis:shortlist".to_string(),
        )])
    }
}

/// Flag claim risks based on shortlist and claim seeds.
#[derive(Debug, Clone, Default)]
pub struct ClaimRiskFlaggerAgent;

impl Agent for ClaimRiskFlaggerAgent {
    fn name(&self) -> &str {
        "claim_risk_flagger"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[
            ContextKey::Hypotheses,
            ContextKey::Evaluations,
            ContextKey::Proposals,
        ]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        let has_seeds = has_prefix(ctx, ContextKey::Hypotheses, CLAIM_SEED_PREFIX);
        let has_shortlist = has_prefix(ctx, ContextKey::Evaluations, PRIOR_ART_SHORTLIST_PREFIX);
        let has_flags = has_prefix(ctx, ContextKey::Evaluations, CLAIM_RISK_PREFIX)
            || has_prefix(ctx, ContextKey::Proposals, CLAIM_RISK_PREFIX);
        has_seeds && has_shortlist && !has_flags
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let citations = ctx
            .get(ContextKey::Evaluations)
            .iter()
            .filter(|fact| fact.id.starts_with(PRIOR_ART_SHORTLIST_PREFIX))
            .map(|fact| fact.id.clone())
            .collect::<Vec<_>>();
        let flags = ClaimRiskFlags {
            flags: vec!["Potential anticipation".to_string()],
            citations,
        };
        let content = serde_json::to_string(&flags).unwrap_or_default();
        AgentEffect::with_facts(vec![proposal_fact(
            ContextKey::Evaluations,
            format!("{}baseline", CLAIM_RISK_PREFIX),
            content,
            0.7,
            "analysis:claim_risk".to_string(),
        )])
    }
}

/// Expand queries using remote models (audit-only).
#[derive(Debug, Clone, Default)]
pub struct EnrichmentLoopAgent;

impl Agent for EnrichmentLoopAgent {
    fn name(&self) -> &str {
        "enrichment_loop"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[
            ContextKey::Hypotheses,
            ContextKey::Strategies,
            ContextKey::Proposals,
        ]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        let has_seeds = has_prefix(ctx, ContextKey::Hypotheses, CLAIM_SEED_PREFIX);
        let has_expanded = has_prefix(ctx, ContextKey::Strategies, EXPANDED_QUERY_PREFIX)
            || has_prefix(ctx, ContextKey::Proposals, EXPANDED_QUERY_PREFIX);
        has_seeds && !has_expanded
    }

    fn execute(&self, _ctx: &Context) -> AgentEffect {
        let expanded = ExpandedQuerySet {
            queries: vec!["expanded keyword set".to_string()],
            audit_only: true,
        };
        let alt = AlternateClaimStrategy {
            description: "Alternate claim strategy".to_string(),
            audit_only: true,
        };
        let expanded_content = serde_json::to_string(&expanded).unwrap_or_default();
        let alt_content = serde_json::to_string(&alt).unwrap_or_default();
        AgentEffect::with_facts(vec![
            proposal_fact(
                ContextKey::Strategies,
                format!("{}remote", EXPANDED_QUERY_PREFIX),
                expanded_content,
                0.6,
                "remote:audit_only".to_string(),
            ),
            proposal_fact(
                ContextKey::Strategies,
                format!("{}remote", ALT_CLAIM_STRATEGY_PREFIX),
                alt_content,
                0.6,
                "remote:audit_only".to_string(),
            ),
        ])
    }
}

/// Generate claim sets A/B/C.
#[derive(Debug, Clone, Default)]
pub struct ClaimStrategyAgent;

impl Agent for ClaimStrategyAgent {
    fn name(&self) -> &str {
        "claim_strategy_builder"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[
            ContextKey::Hypotheses,
            ContextKey::Strategies,
            ContextKey::Proposals,
        ]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        let has_seeds = has_prefix(ctx, ContextKey::Hypotheses, CLAIM_SEED_PREFIX);
        let has_claims = has_prefix(ctx, ContextKey::Strategies, CLAIM_SET_PREFIX)
            || has_prefix(ctx, ContextKey::Proposals, CLAIM_SET_PREFIX);
        has_seeds && !has_claims
    }

    fn execute(&self, _ctx: &Context) -> AgentEffect {
        let strategies = ["broad", "medium", "narrow"];
        let mut facts = Vec::new();
        for strategy in strategies {
            let claim_set = ClaimSet {
                strategy: strategy.to_string(),
                claims: vec![format!("{strategy} claim 1"), format!("{strategy} claim 2")],
                claim_terms: vec!["term_a".to_string(), "term_b".to_string()],
            };
            let content = serde_json::to_string(&claim_set).unwrap_or_default();
            facts.push(proposal_fact(
                ContextKey::Strategies,
                format!("{}{}", CLAIM_SET_PREFIX, strategy),
                content,
                0.7,
                "analysis:claim_strategy".to_string(),
            ));
        }
        AgentEffect::with_facts(facts)
    }
}

/// Generate claim chart vs top references.
#[derive(Debug, Clone, Default)]
pub struct ClaimChartGeneratorAgent;

impl Agent for ClaimChartGeneratorAgent {
    fn name(&self) -> &str {
        "claim_chart_generator"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[
            ContextKey::Strategies,
            ContextKey::Evaluations,
            ContextKey::Proposals,
        ]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        let has_claims = has_prefix(ctx, ContextKey::Strategies, CLAIM_SET_PREFIX);
        let has_shortlist = has_prefix(ctx, ContextKey::Evaluations, PRIOR_ART_SHORTLIST_PREFIX);
        let has_chart = has_prefix(ctx, ContextKey::Evaluations, CLAIM_CHART_PREFIX)
            || has_prefix(ctx, ContextKey::Proposals, CLAIM_CHART_PREFIX);
        has_claims && has_shortlist && !has_chart
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let evidence_ids = ctx
            .get(ContextKey::Evaluations)
            .iter()
            .filter(|fact| fact.id.starts_with(PRIOR_ART_SHORTLIST_PREFIX))
            .map(|fact| fact.id.clone())
            .collect::<Vec<_>>();
        let chart = ClaimChart {
            evidence_ids,
            analysis_summary: "Claim chart vs top refs".to_string(),
        };
        let content = serde_json::to_string(&chart).unwrap_or_default();
        AgentEffect::with_facts(vec![proposal_fact(
            ContextKey::Evaluations,
            format!("{}top3", CLAIM_CHART_PREFIX),
            content,
            0.7,
            "analysis:claim_chart".to_string(),
        )])
    }
}

/// Draft specification skeleton and embodiments.
#[derive(Debug, Clone, Default)]
pub struct SpecDraftAgent;

impl Agent for SpecDraftAgent {
    fn name(&self) -> &str {
        "spec_draft_builder"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[
            ContextKey::Strategies,
            ContextKey::Hypotheses,
            ContextKey::Proposals,
        ]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        let has_claims = has_prefix(ctx, ContextKey::Strategies, CLAIM_SET_PREFIX);
        let has_spec = has_prefix(ctx, ContextKey::Strategies, SPEC_DRAFT_PREFIX)
            || has_prefix(ctx, ContextKey::Proposals, SPEC_DRAFT_PREFIX);
        has_claims && !has_spec
    }

    fn execute(&self, _ctx: &Context) -> AgentEffect {
        let spec = SpecDraft {
            sections: vec![
                "Field".to_string(),
                "Background".to_string(),
                "Summary".to_string(),
                "Brief Description".to_string(),
                "Detailed Description".to_string(),
            ],
            embodiments: vec!["Embodiment A".to_string(), "Embodiment B".to_string()],
        };
        let content = serde_json::to_string(&spec).unwrap_or_default();
        AgentEffect::with_facts(vec![proposal_fact(
            ContextKey::Strategies,
            format!("{}v0", SPEC_DRAFT_PREFIX),
            content,
            0.7,
            "analysis:spec_draft".to_string(),
        )])
    }
}

/// Build support matrix for claim terms.
#[derive(Debug, Clone, Default)]
pub struct SupportMatrixAgent;

impl Agent for SupportMatrixAgent {
    fn name(&self) -> &str {
        "support_matrix_builder"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[
            ContextKey::Strategies,
            ContextKey::Evaluations,
            ContextKey::Proposals,
        ]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        let has_spec = has_prefix(ctx, ContextKey::Strategies, SPEC_DRAFT_PREFIX);
        let has_matrix = has_prefix(ctx, ContextKey::Evaluations, SUPPORT_MATRIX_PREFIX)
            || has_prefix(ctx, ContextKey::Proposals, SUPPORT_MATRIX_PREFIX);
        has_spec && !has_matrix
    }

    fn execute(&self, _ctx: &Context) -> AgentEffect {
        let matrix = SupportMatrix {
            term_to_sections: vec![(
                "term_a".to_string(),
                vec!["Detailed Description".to_string()],
            )],
        };
        let content = serde_json::to_string(&matrix).unwrap_or_default();
        AgentEffect::with_facts(vec![proposal_fact(
            ContextKey::Evaluations,
            format!("{}v0", SUPPORT_MATRIX_PREFIX),
            content,
            0.8,
            "analysis:support_matrix".to_string(),
        )])
    }
}

/// Assemble final draft pack for attorney handoff.
#[derive(Debug, Clone, Default)]
pub struct DraftPackAssemblerAgent;

impl Agent for DraftPackAssemblerAgent {
    fn name(&self) -> &str {
        "draft_pack_assembler"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[
            ContextKey::Strategies,
            ContextKey::Evaluations,
            ContextKey::Proposals,
        ]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        let has_report = has_prefix(ctx, ContextKey::Strategies, PATENT_REPORT_PREFIX);
        let has_spec = has_prefix(ctx, ContextKey::Strategies, SPEC_DRAFT_PREFIX);
        let has_pack = has_prefix(ctx, ContextKey::Strategies, DRAFT_PACK_PREFIX)
            || has_prefix(ctx, ContextKey::Proposals, DRAFT_PACK_PREFIX);
        has_report && has_spec && !has_pack
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let report_id = ctx
            .get(ContextKey::Strategies)
            .iter()
            .find(|fact| fact.id.starts_with(PATENT_REPORT_PREFIX))
            .map(|fact| fact.id.clone())
            .unwrap_or_else(|| "report:unknown".to_string());
        let spec_id = ctx
            .get(ContextKey::Strategies)
            .iter()
            .find(|fact| fact.id.starts_with(SPEC_DRAFT_PREFIX))
            .map(|fact| fact.id.clone())
            .unwrap_or_else(|| "spec:unknown".to_string());
        let claim_set_ids = ctx
            .get(ContextKey::Strategies)
            .iter()
            .filter(|fact| fact.id.starts_with(CLAIM_SET_PREFIX))
            .map(|fact| fact.id.clone())
            .collect::<Vec<_>>();

        let pack = DraftPack {
            report_id,
            claim_set_ids,
            spec_id,
        };
        let content = serde_json::to_string(&pack).unwrap_or_default();
        AgentEffect::with_facts(vec![proposal_fact(
            ContextKey::Strategies,
            format!("{}v0", DRAFT_PACK_PREFIX),
            content,
            0.8,
            "analysis:draft_pack".to_string(),
        )])
    }
}

// =============================================================================
// Invariants
// =============================================================================

/// Enforce disclosure completeness: newness, how it works, >= 3 embodiments.
#[derive(Debug, Clone, Default)]
pub struct DisclosureCompletenessInvariant;

impl Invariant for DisclosureCompletenessInvariant {
    fn name(&self) -> &str {
        "disclosure_completeness"
    }

    fn class(&self) -> InvariantClass {
        InvariantClass::Semantic
    }

    fn check(&self, ctx: &Context) -> InvariantResult {
        for disclosure in ctx.get(ContextKey::Strategies).iter() {
            if !disclosure.id.starts_with(DISCLOSURE_DRAFT_PREFIX) {
                continue;
            }
            let parsed = serde_json::from_str::<DisclosureDraft>(&disclosure.content).ok();
            if let Some(draft) = parsed {
                if draft.newness.is_empty() || draft.how_it_works.is_empty() {
                    return InvariantResult::Violated(Violation::with_facts(
                        format!("Disclosure {} missing core sections", disclosure.id),
                        vec![disclosure.id.clone()],
                    ));
                }
                if draft.embodiments.len() < 3 {
                    return InvariantResult::Violated(Violation::with_facts(
                        format!("Disclosure {} lacks 3 embodiments", disclosure.id),
                        vec![disclosure.id.clone()],
                    ));
                }
            }
        }
        InvariantResult::Ok
    }
}

/// Remote backend restricted when confidentiality is restricted.
#[derive(Debug, Clone, Default)]
pub struct RemoteBackendRestrictedInvariant;

impl Invariant for RemoteBackendRestrictedInvariant {
    fn name(&self) -> &str {
        "remote_backend_restricted"
    }

    fn class(&self) -> InvariantClass {
        InvariantClass::Acceptance
    }

    fn check(&self, ctx: &Context) -> InvariantResult {
        let restricted = ctx
            .get(ContextKey::Constraints)
            .iter()
            .find(|fact| fact.id.starts_with(MATTER_POLICY_PREFIX))
            .and_then(|fact| serde_json::from_str::<MatterPolicy>(&fact.content).ok())
            .map(|policy| {
                policy
                    .confidentiality_tier
                    .eq_ignore_ascii_case("restricted")
            })
            .unwrap_or(false);

        if !restricted {
            return InvariantResult::Ok;
        }

        for key in [
            ContextKey::Strategies,
            ContextKey::Signals,
            ContextKey::Evaluations,
        ] {
            for fact in ctx.get(key) {
                if fact.content.contains("remote:audit_only") {
                    return InvariantResult::Violated(Violation::with_facts(
                        format!("Remote backend used in restricted matter: {}", fact.id),
                        vec![fact.id.clone()],
                    ));
                }
            }
        }
        InvariantResult::Ok
    }
}

/// Ensure evidence assertions include citations.
#[derive(Debug, Clone, Default)]
pub struct EvidenceCitationInvariant;

impl Invariant for EvidenceCitationInvariant {
    fn name(&self) -> &str {
        "evidence_citations_required"
    }

    fn class(&self) -> InvariantClass {
        InvariantClass::Structural
    }

    fn check(&self, ctx: &Context) -> InvariantResult {
        for fact in ctx.get(ContextKey::Evaluations) {
            if fact.id.starts_with(PRIOR_ART_SHORTLIST_PREFIX)
                || fact.id.starts_with(CLAIM_RISK_PREFIX)
                || fact.id.starts_with(CLAIM_CHART_PREFIX)
            {
                if !fact.content.contains("\"citations\"")
                    && !fact.content.contains("\"evidence_ids\"")
                {
                    return InvariantResult::Violated(Violation::with_facts(
                        format!("Evidence output {} missing citations", fact.id),
                        vec![fact.id.clone()],
                    ));
                }
            }
        }
        InvariantResult::Ok
    }
}

/// Ensure claim terms have support entries.
#[derive(Debug, Clone, Default)]
pub struct ClaimSupportInvariant;

impl Invariant for ClaimSupportInvariant {
    fn name(&self) -> &str {
        "claim_terms_supported"
    }

    fn class(&self) -> InvariantClass {
        InvariantClass::Acceptance
    }

    fn check(&self, ctx: &Context) -> InvariantResult {
        let has_matrix = ctx
            .get(ContextKey::Evaluations)
            .iter()
            .any(|fact| fact.id.starts_with(SUPPORT_MATRIX_PREFIX));
        let has_claims = ctx
            .get(ContextKey::Strategies)
            .iter()
            .any(|fact| fact.id.starts_with(CLAIM_SET_PREFIX));
        if has_claims && !has_matrix {
            return InvariantResult::Violated(Violation::with_facts(
                "Claim sets require support matrix".to_string(),
                ctx.get(ContextKey::Strategies)
                    .iter()
                    .filter(|fact| fact.id.starts_with(CLAIM_SET_PREFIX))
                    .map(|fact| fact.id.clone())
                    .collect(),
            ));
        }
        InvariantResult::Ok
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use converge_provider::patent::StubPatentProvider;

    #[test]
    fn agent_names_are_stable() {
        assert_eq!(MatterPolicyAgent.name(), "matter_policy_builder");
        assert_eq!(MatterContextAgent.name(), "matter_context_builder");
        assert_eq!(InventionCaptureAgent.name(), "invention_capture");
        assert_eq!(InventionSummaryAgent.name(), "invention_summary");
        assert_eq!(ClaimSeedAgent.name(), "claim_seed_generator");
        assert_eq!(PatentQueryBuilderAgent.name(), "patent_query_builder");
        assert_eq!(PatentOperatorPlannerAgent.name(), "patent_operator_planner");
        let provider = Arc::new(StubPatentProvider::new());
        assert_eq!(
            PatentSearchExecutorAgent::new(provider).name(),
            "patent_search_executor"
        );
        assert_eq!(
            PatentEvidenceCollectorAgent.name(),
            "patent_evidence_collector"
        );
        assert_eq!(PatentClaimsAnalyzerAgent.name(), "patent_claims_analyzer");
        assert_eq!(
            PatentLandscapeAnalyzerAgent.name(),
            "patent_landscape_analyzer"
        );
        assert_eq!(PatentReportAssemblerAgent.name(), "patent_report_assembler");
        assert_eq!(PatentAlertAgent.name(), "patent_alert_agent");
        assert_eq!(PatentSubmissionAgent.name(), "patent_submission_agent");
        assert_eq!(
            PatentApprovalRecorderAgent.name(),
            "patent_approval_recorder"
        );
        assert_eq!(PriorArtShortlistAgent.name(), "prior_art_shortlist");
        assert_eq!(ClaimRiskFlaggerAgent.name(), "claim_risk_flagger");
        assert_eq!(EnrichmentLoopAgent.name(), "enrichment_loop");
        assert_eq!(ClaimStrategyAgent.name(), "claim_strategy_builder");
        assert_eq!(ClaimChartGeneratorAgent.name(), "claim_chart_generator");
        assert_eq!(SpecDraftAgent.name(), "spec_draft_builder");
        assert_eq!(SupportMatrixAgent.name(), "support_matrix_builder");
        assert_eq!(DraftPackAssemblerAgent.name(), "draft_pack_assembler");
    }
}
