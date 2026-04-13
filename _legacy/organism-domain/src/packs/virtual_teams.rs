// Copyright 2024-2025 Aprio One AB, Sweden
// SPDX-License-Identifier: MIT

//! Virtual Teams Pack agents for teams, personas, and content publishing.
//!
//! # Philosophy: Agents are "unsafe-by-default" for external actions
//!
//! - Human approval required for external publishing
//! - Personas have explicit guardrails (topics, tone, forbidden claims)
//! - Complete provenance for all external content
//! - Every agent has a human owner
//!
//! # Team State Machine
//!
//! ```text
//! forming → active → reshaping → dissolved
//! ```
//!
//! # Persona State Machine
//!
//! ```text
//! draft → pending_approval → active → suspended/retired
//! ```
//!
//! # Content Draft State Machine
//!
//! ```text
//! drafted → pending_review → approved → published → retracted
//! ```
//!
//! # Key Invariants
//!
//! - Agents cannot publish externally without approval chain
//! - All agent actions auditable
//! - Personas require guardrails and owner
//! - External posts need provenance

use converge_core::{
    Agent, AgentEffect, Context, ContextKey, Fact,
    invariant::{Invariant, InvariantClass, InvariantResult, Violation},
};

// ============================================================================
// Fact ID Prefixes
// ============================================================================

pub const TEAM_PREFIX: &str = "team:";
pub const PERSONA_PREFIX: &str = "persona:";
pub const CONTENT_DRAFT_PREFIX: &str = "content_draft:";
pub const CHANNEL_PREFIX: &str = "channel:";
pub const PUBLISH_PREFIX: &str = "publish:";

// ============================================================================
// Agents
// ============================================================================

/// Creates and manages teams.
#[derive(Debug, Clone, Default)]
pub struct TeamFormationAgent;

impl Agent for TeamFormationAgent {
    fn name(&self) -> &str {
        "team_formation"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Seeds]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        ctx.get(ContextKey::Seeds).iter().any(|s| {
            s.content.contains("team.create_requested") || s.content.contains("team.forming")
        })
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let seeds = ctx.get(ContextKey::Seeds);
        let mut facts = Vec::new();

        for seed in seeds.iter() {
            if seed.content.contains("team.create_requested")
                || seed.content.contains("team.forming")
            {
                facts.push(Fact {
                    key: ContextKey::Proposals,
                    id: format!("{}{}", TEAM_PREFIX, seed.id),
                    content: serde_json::json!({
                        "type": "team",
                        "seed_id": seed.id,
                        "state": "forming",
                        "owner": "team_lead",
                        "charter": "pending",
                        "domain": "defined",
                        "members": [],
                        "created_at": "2026-01-12"
                    })
                    .to_string(),
                });
            }
        }

        AgentEffect::with_facts(facts)
    }
}

/// Manages team lifecycle transitions.
#[derive(Debug, Clone, Default)]
pub struct TeamLifecycleAgent;

impl Agent for TeamLifecycleAgent {
    fn name(&self) -> &str {
        "team_lifecycle"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Proposals, ContextKey::Signals]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        ctx.get(ContextKey::Signals).iter().any(|s| {
            s.content.contains("charter.approved")
                || s.content.contains("team.restructure")
                || s.content.contains("team.sunset")
        })
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let signals = ctx.get(ContextKey::Signals);
        let mut facts = Vec::new();

        for signal in signals.iter() {
            if signal.content.contains("charter.approved") {
                facts.push(Fact {
                    key: ContextKey::Evaluations,
                    id: format!("{}activation:{}", TEAM_PREFIX, signal.id),
                    content: serde_json::json!({
                        "type": "team_activation",
                        "signal_id": signal.id,
                        "state": "active",
                        "activated_at": "2026-01-12"
                    })
                    .to_string(),
                });
            }
        }

        AgentEffect::with_facts(facts)
    }
}

/// Creates and manages personas.
#[derive(Debug, Clone, Default)]
pub struct PersonaCreatorAgent;

impl Agent for PersonaCreatorAgent {
    fn name(&self) -> &str {
        "persona_creator"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Seeds]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        ctx.get(ContextKey::Seeds).iter().any(|s| {
            s.content.contains("persona.drafted") || s.content.contains("brand_voice.created")
        })
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let seeds = ctx.get(ContextKey::Seeds);
        let mut facts = Vec::new();

        for seed in seeds.iter() {
            if seed.content.contains("persona.drafted")
                || seed.content.contains("brand_voice.created")
            {
                facts.push(Fact {
                    key: ContextKey::Proposals,
                    id: format!("{}{}", PERSONA_PREFIX, seed.id),
                    content: serde_json::json!({
                        "type": "persona",
                        "seed_id": seed.id,
                        "state": "draft",
                        "owner": "brand_manager",
                        "name": "TechCo AI",
                        "guardrails": {
                            "allowed_topics": [],
                            "forbidden_topics": [],
                            "tone": "professional",
                            "banned_claims": []
                        },
                        "voice_documented": false,
                        "created_at": "2026-01-12"
                    })
                    .to_string(),
                });
            }
        }

        AgentEffect::with_facts(facts)
    }
}

/// Reviews personas before activation.
#[derive(Debug, Clone, Default)]
pub struct PersonaReviewerAgent;

impl Agent for PersonaReviewerAgent {
    fn name(&self) -> &str {
        "persona_reviewer"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Proposals, ContextKey::Signals]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        ctx.get(ContextKey::Proposals).iter().any(|p| {
            p.id.starts_with(PERSONA_PREFIX) && p.content.contains("\"state\":\"pending_approval\"")
        })
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let proposals = ctx.get(ContextKey::Proposals);
        let signals = ctx.get(ContextKey::Signals);
        let mut facts = Vec::new();

        for persona in proposals.iter() {
            if persona.id.starts_with(PERSONA_PREFIX)
                && persona.content.contains("\"state\":\"pending_approval\"")
            {
                // Check for brand/compliance approval
                let is_approved = signals.iter().any(|s| {
                    s.content.contains("brand_review.approved")
                        || s.content.contains("compliance.approved")
                });

                if is_approved {
                    facts.push(Fact {
                        key: ContextKey::Evaluations,
                        id: format!("{}review:{}", PERSONA_PREFIX, persona.id),
                        content: serde_json::json!({
                            "type": "persona_review",
                            "persona_id": persona.id,
                            "brand_approved": true,
                            "compliance_approved": true,
                            "reviewed_at": "2026-01-12"
                        })
                        .to_string(),
                    });
                }
            }
        }

        AgentEffect::with_facts(facts)
    }
}

/// Creates content drafts from agents/personas.
#[derive(Debug, Clone, Default)]
pub struct ContentDraftCreatorAgent;

impl Agent for ContentDraftCreatorAgent {
    fn name(&self) -> &str {
        "content_draft_creator"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Seeds, ContextKey::Proposals]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        ctx.get(ContextKey::Seeds)
            .iter()
            .any(|s| s.content.contains("content.generated") || s.content.contains("post.drafted"))
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let seeds = ctx.get(ContextKey::Seeds);
        let mut facts = Vec::new();

        for seed in seeds.iter() {
            if seed.content.contains("content.generated") || seed.content.contains("post.drafted") {
                facts.push(Fact {
                    key: ContextKey::Proposals,
                    id: format!("{}{}", CONTENT_DRAFT_PREFIX, seed.id),
                    content: serde_json::json!({
                        "type": "content_draft",
                        "seed_id": seed.id,
                        "state": "drafted",
                        "author_type": "agent",
                        "persona_id": "extracted",
                        "channel_classification": "pending",
                        "requires_review": true,
                        "provenance": {
                            "source": "ai_generated",
                            "model": "gpt-4",
                            "timestamp": "2026-01-12T12:00:00Z"
                        },
                        "created_at": "2026-01-12"
                    })
                    .to_string(),
                });
            }
        }

        AgentEffect::with_facts(facts)
    }
}

/// Reviews content drafts before publication.
#[derive(Debug, Clone, Default)]
pub struct ContentReviewerAgent;

impl Agent for ContentReviewerAgent {
    fn name(&self) -> &str {
        "content_reviewer"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Proposals]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        ctx.get(ContextKey::Proposals).iter().any(|p| {
            p.id.starts_with(CONTENT_DRAFT_PREFIX)
                && p.content.contains("\"state\":\"pending_review\"")
        })
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let proposals = ctx.get(ContextKey::Proposals);
        let mut facts = Vec::new();

        for draft in proposals.iter() {
            if draft.id.starts_with(CONTENT_DRAFT_PREFIX)
                && draft.content.contains("\"state\":\"pending_review\"")
            {
                facts.push(Fact {
                    key: ContextKey::Evaluations,
                    id: format!("{}review:{}", CONTENT_DRAFT_PREFIX, draft.id),
                    content: serde_json::json!({
                        "type": "content_review",
                        "draft_id": draft.id,
                        "reviewer": "human_reviewer_id",
                        "claims_verified": true,
                        "brand_compliant": true,
                        "review_recorded": true,
                        "reviewed_at": "2026-01-12"
                    })
                    .to_string(),
                });
            }
        }

        AgentEffect::with_facts(facts)
    }
}

/// Handles content publication with approval chain.
#[derive(Debug, Clone, Default)]
pub struct PublishApprovalAgent;

impl Agent for PublishApprovalAgent {
    fn name(&self) -> &str {
        "publish_approval"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Proposals, ContextKey::Evaluations]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        ctx.get(ContextKey::Evaluations).iter().any(|e| {
            e.id.contains("content_review") && e.content.contains("\"claims_verified\":true")
        })
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let evaluations = ctx.get(ContextKey::Evaluations);
        let mut facts = Vec::new();

        for review in evaluations.iter() {
            if review.id.contains("content_review")
                && review.content.contains("\"claims_verified\":true")
            {
                facts.push(Fact {
                    key: ContextKey::Proposals,
                    id: format!("{}{}", PUBLISH_PREFIX, review.id),
                    content: serde_json::json!({
                        "type": "publish_approval",
                        "review_id": review.id,
                        "state": "approved",
                        "approver": "human_approver_id",
                        "approval_recorded": true,
                        "provenance": {
                            "reviewer": "human_reviewer_id",
                            "timestamp": "2026-01-12T12:00:00Z"
                        },
                        "approved_at": "2026-01-12"
                    })
                    .to_string(),
                });
            }
        }

        AgentEffect::with_facts(facts)
    }
}

/// Manages channel access and classification.
#[derive(Debug, Clone, Default)]
pub struct ChannelManagerAgent;

impl Agent for ChannelManagerAgent {
    fn name(&self) -> &str {
        "channel_manager"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Seeds]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        ctx.get(ContextKey::Seeds).iter().any(|s| {
            s.content.contains("channel.created") || s.content.contains("channel.access_requested")
        })
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let seeds = ctx.get(ContextKey::Seeds);
        let mut facts = Vec::new();

        for seed in seeds.iter() {
            if seed.content.contains("channel.created")
                || seed.content.contains("channel.access_requested")
            {
                facts.push(Fact {
                    key: ContextKey::Proposals,
                    id: format!("{}{}", CHANNEL_PREFIX, seed.id),
                    content: serde_json::json!({
                        "type": "channel",
                        "seed_id": seed.id,
                        "state": "active",
                        "classification": "internal",
                        "external": false,
                        "agent_access_approved": false,
                        "created_at": "2026-01-12"
                    })
                    .to_string(),
                });
            }
        }

        AgentEffect::with_facts(facts)
    }
}

/// Audits all agent actions.
#[derive(Debug, Clone, Default)]
pub struct AgentAuditAgent;

impl Agent for AgentAuditAgent {
    fn name(&self) -> &str {
        "agent_audit"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Proposals]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        ctx.get(ContextKey::Proposals)
            .iter()
            .any(|p| p.content.contains("\"author_type\":\"agent\""))
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let proposals = ctx.get(ContextKey::Proposals);
        let mut facts = Vec::new();

        for proposal in proposals.iter() {
            if proposal.content.contains("\"author_type\":\"agent\"") {
                facts.push(Fact {
                    key: ContextKey::Evaluations,
                    id: format!("agent_audit:{}", proposal.id),
                    content: serde_json::json!({
                        "type": "agent_audit_entry",
                        "proposal_id": proposal.id,
                        "action_type": "content_creation",
                        "auditable": true,
                        "audit_trail_complete": true,
                        "recorded_at": "2026-01-12"
                    })
                    .to_string(),
                });
            }
        }

        AgentEffect::with_facts(facts)
    }
}

// ============================================================================
// Invariants
// ============================================================================

/// Ensures agents cannot publish externally without approval.
#[derive(Debug, Clone, Default)]
pub struct AgentsUnsafeByDefaultInvariant;

impl Invariant for AgentsUnsafeByDefaultInvariant {
    fn name(&self) -> &str {
        "agents_unsafe_by_default"
    }

    fn class(&self) -> InvariantClass {
        InvariantClass::Acceptance
    }

    fn check(&self, ctx: &Context) -> InvariantResult {
        let proposals = ctx.get(ContextKey::Proposals);
        let evaluations = ctx.get(ContextKey::Evaluations);

        for draft in proposals.iter() {
            if draft.id.starts_with(CONTENT_DRAFT_PREFIX)
                && draft.content.contains("\"author_type\":\"agent\"")
                && draft.content.contains("\"state\":\"published\"")
            {
                // Check for human approval
                let has_approval = evaluations
                    .iter()
                    .any(|e| e.id.contains(&draft.id) && e.content.contains("\"approver\":"));

                if !has_approval {
                    return InvariantResult::Violated(Violation::with_facts(
                        format!(
                            "Agent content {} published without human approval",
                            draft.id
                        ),
                        vec![draft.id.clone()],
                    ));
                }
            }
        }
        InvariantResult::Ok
    }
}

/// Ensures all agent actions are auditable.
#[derive(Debug, Clone, Default)]
pub struct AgentActionsAuditableInvariant;

impl Invariant for AgentActionsAuditableInvariant {
    fn name(&self) -> &str {
        "agent_actions_auditable"
    }

    fn class(&self) -> InvariantClass {
        InvariantClass::Structural
    }

    fn check(&self, ctx: &Context) -> InvariantResult {
        let proposals = ctx.get(ContextKey::Proposals);
        let evaluations = ctx.get(ContextKey::Evaluations);

        for proposal in proposals.iter() {
            if proposal.content.contains("\"author_type\":\"agent\"") {
                let has_audit = evaluations
                    .iter()
                    .any(|e| e.id.contains(&proposal.id) && e.content.contains("agent_audit"));

                if !has_audit {
                    return InvariantResult::Violated(Violation::with_facts(
                        format!("Agent action {} missing audit trail", proposal.id),
                        vec![proposal.id.clone()],
                    ));
                }
            }
        }
        InvariantResult::Ok
    }
}

/// Ensures personas have guardrails.
#[derive(Debug, Clone, Default)]
pub struct PersonaHasGuardrailsInvariant;

impl Invariant for PersonaHasGuardrailsInvariant {
    fn name(&self) -> &str {
        "persona_has_guardrails"
    }

    fn class(&self) -> InvariantClass {
        InvariantClass::Structural
    }

    fn check(&self, ctx: &Context) -> InvariantResult {
        for persona in ctx.get(ContextKey::Proposals).iter() {
            if persona.id.starts_with(PERSONA_PREFIX)
                && persona.content.contains("\"state\":\"active\"")
                && !persona.content.contains("\"guardrails\":")
            {
                return InvariantResult::Violated(Violation::with_facts(
                    format!("Active persona {} missing guardrails", persona.id),
                    vec![persona.id.clone()],
                ));
            }
        }
        InvariantResult::Ok
    }
}

/// Ensures personas have owner.
#[derive(Debug, Clone, Default)]
pub struct PersonaHasOwnerInvariant;

impl Invariant for PersonaHasOwnerInvariant {
    fn name(&self) -> &str {
        "persona_has_owner"
    }

    fn class(&self) -> InvariantClass {
        InvariantClass::Structural
    }

    fn check(&self, ctx: &Context) -> InvariantResult {
        for persona in ctx.get(ContextKey::Proposals).iter() {
            if persona.id.starts_with(PERSONA_PREFIX) && !persona.content.contains("\"owner\":") {
                return InvariantResult::Violated(Violation::with_facts(
                    format!("Persona {} missing owner", persona.id),
                    vec![persona.id.clone()],
                ));
            }
        }
        InvariantResult::Ok
    }
}

/// Ensures external posts have provenance.
#[derive(Debug, Clone, Default)]
pub struct ExternalPostProvenanceInvariant;

impl Invariant for ExternalPostProvenanceInvariant {
    fn name(&self) -> &str {
        "external_post_provenance"
    }

    fn class(&self) -> InvariantClass {
        InvariantClass::Structural
    }

    fn check(&self, ctx: &Context) -> InvariantResult {
        for draft in ctx.get(ContextKey::Proposals).iter() {
            if draft.id.starts_with(CONTENT_DRAFT_PREFIX)
                && draft.content.contains("\"state\":\"published\"")
                && !draft.content.contains("\"provenance\":")
            {
                return InvariantResult::Violated(Violation::with_facts(
                    format!("Published content {} missing provenance", draft.id),
                    vec![draft.id.clone()],
                ));
            }
        }
        InvariantResult::Ok
    }
}

/// Ensures teams have charter.
#[derive(Debug, Clone, Default)]
pub struct TeamHasCharterInvariant;

impl Invariant for TeamHasCharterInvariant {
    fn name(&self) -> &str {
        "team_has_charter"
    }

    fn class(&self) -> InvariantClass {
        InvariantClass::Acceptance
    }

    fn check(&self, ctx: &Context) -> InvariantResult {
        for team in ctx.get(ContextKey::Proposals).iter() {
            if team.id.starts_with(TEAM_PREFIX)
                && team.content.contains("\"state\":\"active\"")
                && team.content.contains("\"charter\":\"pending\"")
            {
                return InvariantResult::Violated(Violation::with_facts(
                    format!("Active team {} missing charter", team.id),
                    vec![team.id.clone()],
                ));
            }
        }
        InvariantResult::Ok
    }
}

/// Ensures teams have owner.
#[derive(Debug, Clone, Default)]
pub struct TeamHasOwnerInvariant;

impl Invariant for TeamHasOwnerInvariant {
    fn name(&self) -> &str {
        "team_has_owner"
    }

    fn class(&self) -> InvariantClass {
        InvariantClass::Structural
    }

    fn check(&self, ctx: &Context) -> InvariantResult {
        for team in ctx.get(ContextKey::Proposals).iter() {
            if team.id.starts_with(TEAM_PREFIX) && !team.content.contains("\"owner\":") {
                return InvariantResult::Violated(Violation::with_facts(
                    format!("Team {} missing owner", team.id),
                    vec![team.id.clone()],
                ));
            }
        }
        InvariantResult::Ok
    }
}

/// Ensures external channels require agent access approval.
#[derive(Debug, Clone, Default)]
pub struct ExternalChannelRequiresApprovalInvariant;

impl Invariant for ExternalChannelRequiresApprovalInvariant {
    fn name(&self) -> &str {
        "external_channel_requires_approval"
    }

    fn class(&self) -> InvariantClass {
        InvariantClass::Acceptance
    }

    fn check(&self, ctx: &Context) -> InvariantResult {
        for channel in ctx.get(ContextKey::Proposals).iter() {
            if channel.id.starts_with(CHANNEL_PREFIX)
                && channel.content.contains("\"external\":true")
                && !channel.content.contains("\"agent_access_approved\":true")
            {
                return InvariantResult::Violated(Violation::with_facts(
                    format!(
                        "External channel {} lacks agent access approval",
                        channel.id
                    ),
                    vec![channel.id.clone()],
                ));
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

    #[test]
    fn agents_have_correct_names() {
        assert_eq!(TeamFormationAgent.name(), "team_formation");
        assert_eq!(TeamLifecycleAgent.name(), "team_lifecycle");
        assert_eq!(PersonaCreatorAgent.name(), "persona_creator");
        assert_eq!(PersonaReviewerAgent.name(), "persona_reviewer");
        assert_eq!(ContentDraftCreatorAgent.name(), "content_draft_creator");
        assert_eq!(ContentReviewerAgent.name(), "content_reviewer");
        assert_eq!(PublishApprovalAgent.name(), "publish_approval");
        assert_eq!(ChannelManagerAgent.name(), "channel_manager");
        assert_eq!(AgentAuditAgent.name(), "agent_audit");
    }

    #[test]
    fn invariants_have_correct_names() {
        assert_eq!(
            AgentsUnsafeByDefaultInvariant.name(),
            "agents_unsafe_by_default"
        );
        assert_eq!(
            AgentActionsAuditableInvariant.name(),
            "agent_actions_auditable"
        );
        assert_eq!(
            PersonaHasGuardrailsInvariant.name(),
            "persona_has_guardrails"
        );
        assert_eq!(PersonaHasOwnerInvariant.name(), "persona_has_owner");
        assert_eq!(
            ExternalPostProvenanceInvariant.name(),
            "external_post_provenance"
        );
        assert_eq!(TeamHasCharterInvariant.name(), "team_has_charter");
        assert_eq!(TeamHasOwnerInvariant.name(), "team_has_owner");
        assert_eq!(
            ExternalChannelRequiresApprovalInvariant.name(),
            "external_channel_requires_approval"
        );
    }
}
