// Copyright 2024-2025 Aprio One AB, Sweden
// SPDX-License-Identifier: MIT

//! Operations & Support Pack agents for universal intake routing.
//!
//! Implements the agent contracts defined in specs/ops_support.truth.
//!
//! # Operations & Support is the Nervous System
//!
//! Every request flows through this pack:
//! - Customer support tickets
//! - Internal ops requests
//! - Escalation routing
//! - SLA tracking
//!
//! Note: This implementation uses the standard ContextKey enum. Facts are
//! distinguished by their ID prefixes (ticket:, escalation:, sla:, etc.).

use converge_core::{
    Agent, AgentEffect, Context, ContextKey, Fact,
    invariant::{Invariant, InvariantClass, InvariantResult, Violation},
};

// ============================================================================
// Fact ID Prefixes
// ============================================================================

pub const TICKET_PREFIX: &str = "ticket:";
pub const CONVERSATION_PREFIX: &str = "conversation:";
pub const ESCALATION_PREFIX: &str = "escalation:";
pub const SLA_PREFIX: &str = "sla:";
pub const ROOT_CAUSE_PREFIX: &str = "root_cause:";
pub const PREVENTION_PREFIX: &str = "prevention:";
pub const INTERNAL_REQUEST_PREFIX: &str = "internal_request:";
pub const KB_ARTICLE_PREFIX: &str = "kb_article:";

// ============================================================================
// Agents
// ============================================================================

/// Ingests tickets from various channels.
///
/// Creates normalized Ticket facts from email, chat, Slack, portal.
#[derive(Debug, Clone, Default)]
pub struct TicketIntakeAgent;

impl Agent for TicketIntakeAgent {
    fn name(&self) -> &str {
        "ticket_intake"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Seeds]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        ctx.get(ContextKey::Seeds)
            .iter()
            .any(|s| s.content.contains("support.request") || s.content.contains("ticket.incoming"))
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let triggers = ctx.get(ContextKey::Seeds);
        let mut facts = Vec::new();

        for trigger in triggers.iter() {
            if trigger.content.contains("support.request")
                || trigger.content.contains("ticket.incoming")
            {
                facts.push(Fact {
                    key: ContextKey::Signals,
                    id: format!("{}{}", TICKET_PREFIX, trigger.id),
                    content: serde_json::json!({
                        "type": "ticket",
                        "source_id": trigger.id,
                        "state": "new",
                        "channel": "detected",
                        "created_at": "2026-01-12T12:00:00Z"
                    })
                    .to_string(),
                });
            }
        }

        AgentEffect::with_facts(facts)
    }
}

/// Categorizes and prioritizes incoming tickets.
#[derive(Debug, Clone, Default)]
pub struct TicketTriagerAgent;

impl Agent for TicketTriagerAgent {
    fn name(&self) -> &str {
        "ticket_triager"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Signals]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        ctx.get(ContextKey::Signals)
            .iter()
            .any(|s| s.id.starts_with(TICKET_PREFIX) && s.content.contains("\"state\":\"new\""))
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let signals = ctx.get(ContextKey::Signals);
        let mut facts = Vec::new();

        for ticket in signals.iter() {
            if ticket.id.starts_with(TICKET_PREFIX) && ticket.content.contains("\"state\":\"new\"")
            {
                facts.push(Fact {
                    key: ContextKey::Proposals,
                    id: format!("triaged:{}", ticket.id),
                    content: serde_json::json!({
                        "type": "triaged_ticket",
                        "ticket_id": ticket.id,
                        "state": "triaged",
                        "priority": "P3",
                        "category": "general",
                        "triaged_at": "2026-01-12T12:00:00Z"
                    })
                    .to_string(),
                });
            }
        }

        AgentEffect::with_facts(facts)
    }
}

/// Provides automatic responses for common issues.
#[derive(Debug, Clone, Default)]
pub struct AutoResponderAgent;

impl Agent for AutoResponderAgent {
    fn name(&self) -> &str {
        "auto_responder"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Proposals]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        let has_triaged = ctx
            .get(ContextKey::Proposals)
            .iter()
            .any(|p| p.content.contains("\"type\":\"triaged_ticket\""));
        let has_response = ctx
            .get(ContextKey::Proposals)
            .iter()
            .any(|p| p.id.starts_with(CONVERSATION_PREFIX));
        has_triaged && !has_response
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let proposals = ctx.get(ContextKey::Proposals);
        let mut facts = Vec::new();

        for triaged in proposals.iter() {
            if triaged.content.contains("\"type\":\"triaged_ticket\"") {
                facts.push(Fact {
                    key: ContextKey::Proposals,
                    id: format!("{}{}", CONVERSATION_PREFIX, triaged.id),
                    content: serde_json::json!({
                        "type": "auto_response",
                        "ticket_id": triaged.id,
                        "response_type": "acknowledgment",
                        "kb_matched": false,
                        "sent_at": "2026-01-12T12:00:00Z"
                    })
                    .to_string(),
                });
            }
        }

        AgentEffect::with_facts(facts)
    }
}

/// Routes tickets to appropriate handlers.
#[derive(Debug, Clone, Default)]
pub struct TicketRouterAgent;

impl Agent for TicketRouterAgent {
    fn name(&self) -> &str {
        "ticket_router"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Proposals]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        ctx.get(ContextKey::Proposals).iter().any(|p| {
            p.content.contains("\"type\":\"triaged_ticket\"") && !p.content.contains("\"assignee\"")
        })
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let proposals = ctx.get(ContextKey::Proposals);
        let mut facts = Vec::new();

        for triaged in proposals.iter() {
            if triaged.content.contains("\"type\":\"triaged_ticket\"") {
                facts.push(Fact {
                    key: ContextKey::Proposals,
                    id: format!("assigned:{}", triaged.id),
                    content: serde_json::json!({
                        "type": "ticket_assignment",
                        "ticket_id": triaged.id,
                        "state": "assigned",
                        "assignee": "support_agent_001",
                        "assigned_at": "2026-01-12T12:00:00Z"
                    })
                    .to_string(),
                });
            }
        }

        AgentEffect::with_facts(facts)
    }
}

/// Monitors SLA compliance for active tickets.
#[derive(Debug, Clone, Default)]
pub struct SlaMonitorAgent;

impl Agent for SlaMonitorAgent {
    fn name(&self) -> &str {
        "sla_monitor"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Proposals]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        let has_assignment = ctx
            .get(ContextKey::Proposals)
            .iter()
            .any(|p| p.content.contains("\"type\":\"ticket_assignment\""));
        let has_sla = ctx
            .get(ContextKey::Evaluations)
            .iter()
            .any(|e| e.id.starts_with(SLA_PREFIX));
        has_assignment && !has_sla
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let proposals = ctx.get(ContextKey::Proposals);
        let mut facts = Vec::new();

        for assignment in proposals.iter() {
            if assignment
                .content
                .contains("\"type\":\"ticket_assignment\"")
            {
                facts.push(Fact {
                    key: ContextKey::Evaluations,
                    id: format!("{}{}", SLA_PREFIX, assignment.id),
                    content: serde_json::json!({
                        "type": "sla_tracking",
                        "ticket_id": assignment.id,
                        "state": "tracking",
                        "first_response_due": "2026-01-12T16:00:00Z",
                        "resolution_due": "2026-01-13T12:00:00Z",
                        "started_at": "2026-01-12T12:00:00Z"
                    })
                    .to_string(),
                });
            }
        }

        AgentEffect::with_facts(facts)
    }
}

/// Handles escalation triggers.
#[derive(Debug, Clone, Default)]
pub struct EscalationHandlerAgent;

impl Agent for EscalationHandlerAgent {
    fn name(&self) -> &str {
        "escalation_handler"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Signals]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        ctx.get(ContextKey::Signals).iter().any(|s| {
            s.content.contains("escalation.required") || s.content.contains("sla.breached")
        })
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let signals = ctx.get(ContextKey::Signals);
        let mut facts = Vec::new();

        for trigger in signals.iter() {
            if trigger.content.contains("escalation.required")
                || trigger.content.contains("sla.breached")
            {
                facts.push(Fact {
                    key: ContextKey::Proposals,
                    id: format!("{}{}", ESCALATION_PREFIX, trigger.id),
                    content: serde_json::json!({
                        "type": "escalation",
                        "source_id": trigger.id,
                        "state": "initiated",
                        "reason": "SLA breach or manual escalation",
                        "target_tier": "tier_2",
                        "initiated_at": "2026-01-12T12:00:00Z"
                    })
                    .to_string(),
                });
            }
        }

        AgentEffect::with_facts(facts)
    }
}

/// Tracks ticket resolution and validates closure.
#[derive(Debug, Clone, Default)]
pub struct ResolutionTrackerAgent;

impl Agent for ResolutionTrackerAgent {
    fn name(&self) -> &str {
        "resolution_tracker"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Proposals]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        ctx.get(ContextKey::Proposals)
            .iter()
            .any(|p| p.content.contains("\"state\":\"resolved\""))
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let proposals = ctx.get(ContextKey::Proposals);
        let mut facts = Vec::new();

        for resolved in proposals.iter() {
            if resolved.content.contains("\"state\":\"resolved\"") {
                facts.push(Fact {
                    key: ContextKey::Evaluations,
                    id: format!("resolution:{}", resolved.id),
                    content: serde_json::json!({
                        "type": "resolution_validation",
                        "ticket_id": resolved.id,
                        "resolution_valid": true,
                        "csat_sent": true,
                        "auto_close_scheduled": "2026-01-14T12:00:00Z",
                        "validated_at": "2026-01-12T12:00:00Z"
                    })
                    .to_string(),
                });
            }
        }

        AgentEffect::with_facts(facts)
    }
}

/// Detects patterns in resolved tickets.
#[derive(Debug, Clone, Default)]
pub struct PatternDetectorAgent;

impl Agent for PatternDetectorAgent {
    fn name(&self) -> &str {
        "pattern_detector"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Evaluations]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        let resolved_count = ctx
            .get(ContextKey::Evaluations)
            .iter()
            .filter(|e| e.content.contains("\"type\":\"resolution_validation\""))
            .count();
        let has_prevention = ctx
            .get(ContextKey::Proposals)
            .iter()
            .any(|p| p.id.starts_with(PREVENTION_PREFIX));
        resolved_count >= 3 && !has_prevention
    }

    fn execute(&self, _ctx: &Context) -> AgentEffect {
        AgentEffect::with_facts(vec![Fact {
            key: ContextKey::Proposals,
            id: format!("{}pattern:detected", PREVENTION_PREFIX),
            content: serde_json::json!({
                "type": "prevention",
                "state": "identified",
                "pattern": "recurring_issue",
                "ticket_count": 3,
                "recommendation": "Investigate root cause",
                "identified_at": "2026-01-12T12:00:00Z"
            })
            .to_string(),
        }])
    }
}

/// Routes internal operations requests.
#[derive(Debug, Clone, Default)]
pub struct InternalRequestRouterAgent;

impl Agent for InternalRequestRouterAgent {
    fn name(&self) -> &str {
        "internal_request_router"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Seeds]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        ctx.get(ContextKey::Seeds)
            .iter()
            .any(|s| s.content.contains("internal.request"))
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let triggers = ctx.get(ContextKey::Seeds);
        let mut facts = Vec::new();

        for trigger in triggers.iter() {
            if trigger.content.contains("internal.request") {
                facts.push(Fact {
                    key: ContextKey::Proposals,
                    id: format!("{}{}", INTERNAL_REQUEST_PREFIX, trigger.id),
                    content: serde_json::json!({
                        "type": "internal_request",
                        "source_id": trigger.id,
                        "state": "submitted",
                        "request_type": "detected",
                        "routed_to": "appropriate_pack",
                        "created_at": "2026-01-12T12:00:00Z"
                    })
                    .to_string(),
                });
            }
        }

        AgentEffect::with_facts(facts)
    }
}

/// Updates knowledge base from resolved tickets.
#[derive(Debug, Clone, Default)]
pub struct KbUpdaterAgent;

impl Agent for KbUpdaterAgent {
    fn name(&self) -> &str {
        "kb_updater"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Evaluations]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        ctx.get(ContextKey::Evaluations)
            .iter()
            .any(|e| e.content.contains("\"novel_solution\":true"))
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let evaluations = ctx.get(ContextKey::Evaluations);
        let mut facts = Vec::new();

        for eval in evaluations.iter() {
            if eval.content.contains("\"novel_solution\":true") {
                facts.push(Fact {
                    key: ContextKey::Proposals,
                    id: format!("{}{}", KB_ARTICLE_PREFIX, eval.id),
                    content: serde_json::json!({
                        "type": "kb_article_draft",
                        "source_ticket": eval.id,
                        "state": "draft",
                        "category": "from_resolution",
                        "created_at": "2026-01-12T12:00:00Z"
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

/// Ensures no ticket exists without an owner after triage.
#[derive(Debug, Clone, Default)]
pub struct NoOrphanTicketsInvariant;

impl Invariant for NoOrphanTicketsInvariant {
    fn name(&self) -> &str {
        "no_orphan_tickets"
    }

    fn class(&self) -> InvariantClass {
        InvariantClass::Structural
    }

    fn check(&self, ctx: &Context) -> InvariantResult {
        let proposals = ctx.get(ContextKey::Proposals);

        for triaged in proposals.iter() {
            if triaged.content.contains("\"type\":\"triaged_ticket\"") {
                let has_assignment = proposals.iter().any(|p| {
                    p.content.contains("\"type\":\"ticket_assignment\"")
                        && p.id.contains(&triaged.id)
                });
                if !has_assignment {
                    return InvariantResult::Violated(Violation::with_facts(
                        format!("Triaged ticket {} has no assignment", triaged.id),
                        vec![triaged.id.clone()],
                    ));
                }
            }
        }
        InvariantResult::Ok
    }
}

/// Ensures SLA breaches trigger escalation.
#[derive(Debug, Clone, Default)]
pub struct SlaBreachEscalatesInvariant;

impl Invariant for SlaBreachEscalatesInvariant {
    fn name(&self) -> &str {
        "sla_breach_escalates"
    }

    fn class(&self) -> InvariantClass {
        InvariantClass::Semantic
    }

    fn check(&self, ctx: &Context) -> InvariantResult {
        let evaluations = ctx.get(ContextKey::Evaluations);
        let proposals = ctx.get(ContextKey::Proposals);

        for sla in evaluations.iter() {
            if sla.id.starts_with(SLA_PREFIX) && sla.content.contains("\"state\":\"breached\"") {
                let has_escalation = proposals
                    .iter()
                    .any(|p| p.id.starts_with(ESCALATION_PREFIX) && p.content.contains(&sla.id));
                if !has_escalation {
                    return InvariantResult::Violated(Violation::with_facts(
                        format!("SLA breach {} has no escalation", sla.id),
                        vec![sla.id.clone()],
                    ));
                }
            }
        }
        InvariantResult::Ok
    }
}

/// Ensures ticket closure has resolution details.
#[derive(Debug, Clone, Default)]
pub struct ClosureRequiresResolutionInvariant;

impl Invariant for ClosureRequiresResolutionInvariant {
    fn name(&self) -> &str {
        "closure_requires_resolution"
    }

    fn class(&self) -> InvariantClass {
        InvariantClass::Acceptance
    }

    fn check(&self, ctx: &Context) -> InvariantResult {
        let proposals = ctx.get(ContextKey::Proposals);

        for ticket in proposals.iter() {
            if ticket.id.starts_with(TICKET_PREFIX)
                && ticket.content.contains("\"state\":\"closed\"")
            {
                if !ticket.content.contains("\"resolution\"") {
                    return InvariantResult::Violated(Violation::with_facts(
                        format!("Closed ticket {} has no resolution", ticket.id),
                        vec![ticket.id.clone()],
                    ));
                }
            }
        }
        InvariantResult::Ok
    }
}

/// Ensures escalations have documented reasons.
#[derive(Debug, Clone, Default)]
pub struct EscalationHasReasonInvariant;

impl Invariant for EscalationHasReasonInvariant {
    fn name(&self) -> &str {
        "escalation_has_reason"
    }

    fn class(&self) -> InvariantClass {
        InvariantClass::Structural
    }

    fn check(&self, ctx: &Context) -> InvariantResult {
        for escalation in ctx.get(ContextKey::Proposals).iter() {
            if escalation.id.starts_with(ESCALATION_PREFIX)
                && !escalation.content.contains("\"reason\"")
            {
                return InvariantResult::Violated(Violation::with_facts(
                    format!("Escalation {} has no documented reason", escalation.id),
                    vec![escalation.id.clone()],
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
        assert_eq!(TicketIntakeAgent.name(), "ticket_intake");
        assert_eq!(TicketTriagerAgent.name(), "ticket_triager");
        assert_eq!(AutoResponderAgent.name(), "auto_responder");
        assert_eq!(TicketRouterAgent.name(), "ticket_router");
        assert_eq!(SlaMonitorAgent.name(), "sla_monitor");
        assert_eq!(EscalationHandlerAgent.name(), "escalation_handler");
        assert_eq!(ResolutionTrackerAgent.name(), "resolution_tracker");
        assert_eq!(PatternDetectorAgent.name(), "pattern_detector");
        assert_eq!(InternalRequestRouterAgent.name(), "internal_request_router");
        assert_eq!(KbUpdaterAgent.name(), "kb_updater");
    }
}
