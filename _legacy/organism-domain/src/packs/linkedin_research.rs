// Copyright 2024-2025 Aprio One AB, Sweden
// SPDX-License-Identifier: MIT

//! LinkedIn Research Pack invariants and data rules.
//!
//! This pack enforces provenance, verification, and approval gating for
//! LinkedIn-derived research outputs.

use converge_core::invariant::{Invariant, InvariantClass, InvariantResult, Violation};
use converge_core::{Agent, AgentEffect, Context, ContextKey, Fact};
use converge_provider::{LinkedInGetRequest, LinkedInProvider, ProviderCallContext};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

// ============================================================================
// Fact ID Prefixes
// ============================================================================

pub const LINKEDIN_SIGNAL_PREFIX: &str = "linkedin_signal:";
pub const LINKEDIN_EVIDENCE_PREFIX: &str = "linkedin_evidence:";
pub const LINKEDIN_PATH_PREFIX: &str = "linkedin_path:";
pub const LINKEDIN_DOSSIER_PREFIX: &str = "linkedin_dossier:";
pub const LINKEDIN_OUTREACH_PREFIX: &str = "linkedin_outreach:";
pub const LINKEDIN_APPROVAL_PREFIX: &str = "linkedin_approval:";
pub const LINKEDIN_TARGET_PREFIX: &str = "linkedin_target:";

// ============================================================================
// Helpers
// ============================================================================

fn has_prefix(ctx: &Context, key: ContextKey, prefix: &str) -> bool {
    ctx.get(key).iter().any(|fact| fact.id.starts_with(prefix))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TargetDiscoveryCriteria {
    region: Option<String>,
    level: Option<String>,
    capabilities: Option<Vec<String>>,
    keywords: Option<String>,
    endpoint: Option<String>,
}

// ============================================================================
// Agents (Stubs)
// ============================================================================

/// Captures initial LinkedIn signals from research seeds.
#[derive(Debug, Clone, Default)]
pub struct SignalIngestAgent;

impl Agent for SignalIngestAgent {
    fn name(&self) -> &str {
        "signal_ingest"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Seeds, ContextKey::Proposals]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        let has_seed = ctx.get(ContextKey::Seeds).iter().any(|seed| {
            seed.content
                .contains("\"intent_type\":\"linkedin.research\"")
        });
        let has_signal = has_prefix(ctx, ContextKey::Proposals, LINKEDIN_SIGNAL_PREFIX);
        has_seed && !has_signal
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let mut facts = Vec::new();
        for seed in ctx.get(ContextKey::Seeds).iter() {
            if seed
                .content
                .contains("\"intent_type\":\"linkedin.research\"")
            {
                let content = serde_json::json!({
                    "type": "linkedin_signal",
                    "seed_id": seed.id,
                    "source_url": "https://linkedin.com",
                    "captured_at": "2026-01-21T00:00:00Z",
                    "confidence": 0.7
                })
                .to_string();
                facts.push(Fact {
                    key: ContextKey::Proposals,
                    id: format!("{}{}", LINKEDIN_SIGNAL_PREFIX, seed.id),
                    content,
                });
            }
        }
        AgentEffect::with_facts(facts)
    }
}

/// Promotes signals into validated evidence.
#[derive(Debug, Clone, Default)]
pub struct EvidenceValidatorAgent;

impl Agent for EvidenceValidatorAgent {
    fn name(&self) -> &str {
        "evidence_validator"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Proposals, ContextKey::Evaluations]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        let has_signal = has_prefix(ctx, ContextKey::Proposals, LINKEDIN_SIGNAL_PREFIX);
        let has_evidence = has_prefix(ctx, ContextKey::Evaluations, LINKEDIN_EVIDENCE_PREFIX);
        has_signal && !has_evidence
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let mut facts = Vec::new();
        for signal in ctx.get(ContextKey::Proposals).iter() {
            if signal.id.starts_with(LINKEDIN_SIGNAL_PREFIX) {
                let content = serde_json::json!({
                    "type": "linkedin_evidence",
                    "signal_id": signal.id,
                    "source_url": "https://linkedin.com",
                    "captured_at": "2026-01-21T00:00:00Z",
                    "confidence": 0.8
                })
                .to_string();
                facts.push(Fact {
                    key: ContextKey::Evaluations,
                    id: format!("{}{}", LINKEDIN_EVIDENCE_PREFIX, signal.id),
                    content,
                });
            }
        }
        AgentEffect::with_facts(facts)
    }
}

/// Builds a research dossier from evidence.
#[derive(Debug, Clone, Default)]
pub struct DossierBuilderAgent;

impl Agent for DossierBuilderAgent {
    fn name(&self) -> &str {
        "dossier_builder"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Evaluations, ContextKey::Strategies]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        let has_evidence = has_prefix(ctx, ContextKey::Evaluations, LINKEDIN_EVIDENCE_PREFIX);
        let has_dossier = has_prefix(ctx, ContextKey::Strategies, LINKEDIN_DOSSIER_PREFIX);
        has_evidence && !has_dossier
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let evidence_ids: Vec<String> = ctx
            .get(ContextKey::Evaluations)
            .iter()
            .filter(|fact| fact.id.starts_with(LINKEDIN_EVIDENCE_PREFIX))
            .map(|fact| fact.id.clone())
            .collect();

        if evidence_ids.is_empty() {
            return AgentEffect::empty();
        }

        let content = serde_json::json!({
            "type": "linkedin_dossier",
            "evidence_ids": evidence_ids,
            "state": "draft",
            "confidence": 0.75
        })
        .to_string();

        AgentEffect::with_facts(vec![Fact {
            key: ContextKey::Strategies,
            id: format!("{}{}", LINKEDIN_DOSSIER_PREFIX, "draft"),
            content,
        }])
    }
}

/// Verifies network paths before they become actionable.
#[derive(Debug, Clone, Default)]
pub struct PathVerifierAgent;

impl Agent for PathVerifierAgent {
    fn name(&self) -> &str {
        "path_verifier"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Signals, ContextKey::Strategies]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        let has_signal = has_prefix(ctx, ContextKey::Signals, LINKEDIN_SIGNAL_PREFIX);
        let has_path = has_prefix(ctx, ContextKey::Strategies, LINKEDIN_PATH_PREFIX);
        has_signal && !has_path
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let has_any_signal = ctx
            .get(ContextKey::Signals)
            .iter()
            .any(|fact| fact.id.starts_with(LINKEDIN_SIGNAL_PREFIX));

        if !has_any_signal {
            return AgentEffect::empty();
        }

        let content = serde_json::json!({
            "type": "network_path",
            "state": "actionable",
            "verified": true,
            "evidence_ids": ["placeholder_evidence"]
        })
        .to_string();

        AgentEffect::with_facts(vec![Fact {
            key: ContextKey::Strategies,
            id: format!("{}{}", LINKEDIN_PATH_PREFIX, "verified"),
            content,
        }])
    }
}

/// Finds people matching discovery criteria using the LinkedIn provider.
#[derive(Clone)]
pub struct LinkedInTargetDiscoveryAgent {
    provider: Arc<dyn LinkedInProvider>,
}

impl std::fmt::Debug for LinkedInTargetDiscoveryAgent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LinkedInTargetDiscoveryAgent")
            .field("provider", &self.provider.name())
            .finish()
    }
}

impl LinkedInTargetDiscoveryAgent {
    #[must_use]
    pub fn new(provider: Arc<dyn LinkedInProvider>) -> Self {
        Self { provider }
    }
}

impl Agent for LinkedInTargetDiscoveryAgent {
    fn name(&self) -> &str {
        "linkedin_target_discovery"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Seeds, ContextKey::Proposals]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        let has_seed = ctx
            .get(ContextKey::Seeds)
            .iter()
            .any(|seed| seed.content.contains("\"target_discovery\""));
        let has_targets = has_prefix(ctx, ContextKey::Proposals, LINKEDIN_TARGET_PREFIX);
        has_seed && !has_targets
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let mut facts = Vec::new();
        for seed in ctx.get(ContextKey::Seeds).iter() {
            if !seed.content.contains("\"target_discovery\"") {
                continue;
            }

            let criteria: Option<TargetDiscoveryCriteria> = serde_json::from_str(&seed.content)
                .ok()
                .and_then(|value: serde_json::Value| {
                    value
                        .get("target_discovery")
                        .cloned()
                        .and_then(|v| serde_json::from_value(v).ok())
                });

            let Some(criteria) = criteria else {
                continue;
            };

            let endpoint = criteria
                .endpoint
                .clone()
                .unwrap_or_else(|| "/search".to_string());
            let mut request = LinkedInGetRequest::new(endpoint);
            request.query.insert("q".to_string(), "people".to_string());
            if let Some(region) = criteria.region.as_ref() {
                request.query.insert("region".to_string(), region.clone());
            }
            if let Some(level) = criteria.level.as_ref() {
                request.query.insert("level".to_string(), level.clone());
            }
            if let Some(keywords) = criteria.keywords.as_ref() {
                request
                    .query
                    .insert("keywords".to_string(), keywords.clone());
            }
            if let Some(caps) = criteria.capabilities.as_ref() {
                request
                    .query
                    .insert("capabilities".to_string(), caps.join(","));
            }

            let call_ctx = ProviderCallContext::default();
            let response = match self.provider.get(&request, &call_ctx) {
                Ok(resp) => resp,
                Err(err) => {
                    facts.push(Fact {
                        key: ContextKey::Diagnostic,
                        id: format!("linkedin_target_error:{}", seed.id),
                        content: err.to_string(),
                    });
                    continue;
                }
            };

            for record in response.records {
                let content = serde_json::json!({
                    "type": "linkedin_target",
                    "seed_id": seed.id,
                    "criteria": criteria,
                    "payload": record.content.payload,
                    "provenance": record.provenance(),
                })
                .to_string();
                facts.push(Fact {
                    key: ContextKey::Proposals,
                    id: format!("{}{}", LINKEDIN_TARGET_PREFIX, record.observation_id),
                    content,
                });
            }
        }
        AgentEffect::with_facts(facts)
    }
}

/// Records explicit approvals from seed facts.
#[derive(Debug, Clone, Default)]
pub struct ApprovalRecorderAgent;

impl Agent for ApprovalRecorderAgent {
    fn name(&self) -> &str {
        "approval_recorder"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Seeds, ContextKey::Constraints]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        let has_seed = ctx
            .get(ContextKey::Seeds)
            .iter()
            .any(|seed| seed.content.contains("linkedin.approval.granted"));
        let has_approval = has_prefix(ctx, ContextKey::Constraints, LINKEDIN_APPROVAL_PREFIX);
        has_seed && !has_approval
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let mut facts = Vec::new();
        for seed in ctx.get(ContextKey::Seeds).iter() {
            if seed.content.contains("linkedin.approval.granted") {
                let content = serde_json::json!({
                    "type": "linkedin_approval",
                    "seed_id": seed.id,
                    "approver_role": "research_owner",
                    "approved_at": "2026-01-21T00:00:00Z"
                })
                .to_string();
                facts.push(Fact {
                    key: ContextKey::Constraints,
                    id: format!("{}{}", LINKEDIN_APPROVAL_PREFIX, seed.id),
                    content,
                });
                let audit_content = serde_json::json!({
                    "type": "audit_entry",
                    "action": "linkedin_approval_recorded",
                    "reference_id": seed.id,
                    "timestamp": "2026-01-21T00:00:00Z",
                    "immutable": true
                })
                .to_string();
                facts.push(Fact {
                    key: ContextKey::Proposals,
                    id: format!("audit:linkedin_approval:{}", seed.id),
                    content: audit_content,
                });
            }
        }
        AgentEffect::with_facts(facts)
    }
}

// ============================================================================
// Invariants
// ============================================================================

/// Evidence must include provenance fields (source URL and capture timestamp).
#[derive(Debug, Clone, Default)]
pub struct EvidenceRequiresProvenanceInvariant;

impl Invariant for EvidenceRequiresProvenanceInvariant {
    fn name(&self) -> &str {
        "evidence_requires_provenance"
    }

    fn class(&self) -> InvariantClass {
        InvariantClass::Structural
    }

    fn check(&self, ctx: &Context) -> InvariantResult {
        for evidence in ctx.get(ContextKey::Evaluations).iter() {
            if evidence.id.starts_with(LINKEDIN_EVIDENCE_PREFIX) {
                let has_url = evidence.content.contains("\"source_url\"");
                let has_timestamp = evidence.content.contains("\"captured_at\"");
                if !has_url || !has_timestamp {
                    return InvariantResult::Violated(Violation::with_facts(
                        format!("Evidence {} missing provenance", evidence.id),
                        vec![evidence.id.clone()],
                    ));
                }
            }
        }
        InvariantResult::Ok
    }
}

/// Network paths must be verified before marked actionable.
#[derive(Debug, Clone, Default)]
pub struct NetworkPathRequiresVerificationInvariant;

impl Invariant for NetworkPathRequiresVerificationInvariant {
    fn name(&self) -> &str {
        "network_path_requires_verification"
    }

    fn class(&self) -> InvariantClass {
        InvariantClass::Semantic
    }

    fn check(&self, ctx: &Context) -> InvariantResult {
        for path in ctx.get(ContextKey::Strategies).iter() {
            if path.id.starts_with(LINKEDIN_PATH_PREFIX)
                && path.content.contains("\"state\":\"actionable\"")
                && !path.content.contains("\"verified\":true")
            {
                return InvariantResult::Violated(Violation::with_facts(
                    format!(
                        "Network path {} is actionable without verification",
                        path.id
                    ),
                    vec![path.id.clone()],
                ));
            }
        }
        InvariantResult::Ok
    }
}

/// External outreach proposals require explicit approval facts.
#[derive(Debug, Clone, Default)]
pub struct ApprovalRequiredForExternalActionInvariant;

impl Invariant for ApprovalRequiredForExternalActionInvariant {
    fn name(&self) -> &str {
        "approval_required_for_external_action"
    }

    fn class(&self) -> InvariantClass {
        InvariantClass::Acceptance
    }

    fn check(&self, ctx: &Context) -> InvariantResult {
        let has_approval = ctx
            .get(ContextKey::Constraints)
            .iter()
            .any(|fact| fact.id.starts_with(LINKEDIN_APPROVAL_PREFIX));

        if !has_approval {
            for proposal in ctx.get(ContextKey::Proposals).iter() {
                if proposal.id.starts_with(LINKEDIN_OUTREACH_PREFIX) {
                    return InvariantResult::Violated(Violation::with_facts(
                        "Outreach proposed without approval".to_string(),
                        vec![proposal.id.clone()],
                    ));
                }
            }
        }

        InvariantResult::Ok
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use converge_core::Fact;

    #[test]
    fn agents_have_correct_names() {
        assert_eq!(SignalIngestAgent.name(), "signal_ingest");
        assert_eq!(EvidenceValidatorAgent.name(), "evidence_validator");
        assert_eq!(DossierBuilderAgent.name(), "dossier_builder");
        assert_eq!(PathVerifierAgent.name(), "path_verifier");
        assert_eq!(ApprovalRecorderAgent.name(), "approval_recorder");
        let provider = Arc::new(converge_provider::StubLinkedInProvider::default());
        assert_eq!(
            LinkedInTargetDiscoveryAgent::new(provider).name(),
            "linkedin_target_discovery"
        );
    }

    #[test]
    fn evidence_requires_provenance_flags_missing_fields() {
        let mut ctx = Context::new();
        ctx.add_fact(Fact::new(
            ContextKey::Evaluations,
            "linkedin_evidence:001",
            r#"{"type":"evidence","source_url":null}"#,
        ))
        .unwrap();

        let result = EvidenceRequiresProvenanceInvariant.check(&ctx);
        assert!(matches!(result, InvariantResult::Violated(_)));
    }

    #[test]
    fn network_path_requires_verification_blocks_actionable_without_flag() {
        let mut ctx = Context::new();
        ctx.add_fact(Fact::new(
            ContextKey::Strategies,
            "linkedin_path:001",
            r#"{"type":"network_path","state":"actionable","verified":false}"#,
        ))
        .unwrap();

        let result = NetworkPathRequiresVerificationInvariant.check(&ctx);
        assert!(matches!(result, InvariantResult::Violated(_)));
    }

    #[test]
    fn approval_required_for_external_action_flags_missing_approval() {
        let mut ctx = Context::new();
        ctx.add_fact(Fact::new(
            ContextKey::Proposals,
            "linkedin_outreach:001",
            r#"{"type":"outreach_draft","channel":"linkedin"}"#,
        ))
        .unwrap();

        let result = ApprovalRequiredForExternalActionInvariant.check(&ctx);
        assert!(matches!(result, InvariantResult::Violated(_)));
    }

    #[test]
    fn target_discovery_emits_targets() {
        let provider = Arc::new(converge_provider::StubLinkedInProvider::default());
        let agent = LinkedInTargetDiscoveryAgent::new(provider);
        let mut ctx = Context::new();
        ctx.add_fact(Fact::new(
            ContextKey::Seeds,
            "seed:linkedin-001",
            r#"{"intent_type":"linkedin.research","target_discovery":{"region":"EU","level":"VP","capabilities":["GTM"],"keywords":"revops"}}"#,
        ))
        .unwrap();

        let effect = agent.execute(&ctx);
        assert!(
            effect
                .facts
                .iter()
                .any(|fact| fact.id.starts_with(LINKEDIN_TARGET_PREFIX)),
            "should emit linkedin target proposals"
        );
    }
}
