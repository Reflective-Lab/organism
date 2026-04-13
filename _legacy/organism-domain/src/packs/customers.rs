// Copyright 2024-2025 Aprio One AB, Sweden
// SPDX-License-Identifier: MIT

//! Customers Pack agents for revenue operations.
//!
//! Implements the agent contracts defined in specs/customers.feature.
//!
//! # Lifecycle: Lead → Qualify → Offer → Close → Handoff
//!
//! Note: This implementation uses the standard ContextKey enum. Facts are
//! distinguished by their ID prefixes (lead:, opportunity:, deal:, etc.).

use converge_core::{
    Agent, AgentEffect, Context, ContextKey, Fact,
    invariant::{Invariant, InvariantClass, InvariantResult, Violation},
};

// ============================================================================
// Fact ID Prefixes
// ============================================================================

pub const LEAD_PREFIX: &str = "lead:";
pub const OPPORTUNITY_PREFIX: &str = "opportunity:";
pub const PROPOSAL_PREFIX: &str = "proposal:";
pub const DEAL_PREFIX: &str = "deal:";
pub const HANDOFF_PREFIX: &str = "handoff:";
pub const SEQUENCE_PREFIX: &str = "sequence:";

// ============================================================================
// Agents
// ============================================================================

/// Enriches leads with company and contact data.
#[derive(Debug, Clone, Default)]
pub struct LeadEnrichmentAgent;

impl Agent for LeadEnrichmentAgent {
    fn name(&self) -> &str {
        "lead_enrichment"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Seeds]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        ctx.get(ContextKey::Seeds).iter().any(|lead| {
            lead.id.starts_with(LEAD_PREFIX)
                && (lead.content.contains("\"state\":\"new\"")
                    || lead.content.contains("\"state\":\"enriching\""))
        })
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let seeds = ctx.get(ContextKey::Seeds);
        let mut facts = Vec::new();

        for lead in seeds.iter() {
            if lead.id.starts_with(LEAD_PREFIX)
                && (lead.content.contains("\"state\":\"new\"")
                    || lead.content.contains("\"state\":\"enriching\""))
            {
                facts.push(Fact {
                    key: ContextKey::Signals,
                    id: format!("{}enriched:{}", LEAD_PREFIX, lead.id),
                    content: serde_json::json!({
                        "type": "enriched_lead",
                        "lead_id": lead.id,
                        "company_name": "Acme Corp",
                        "company_size": "50-100",
                        "industry": "Technology",
                        "title": "VP Engineering",
                        "technologies": ["AWS", "Kubernetes"]
                    })
                    .to_string(),
                });
            }
        }

        AgentEffect::with_facts(facts)
    }
}

/// Scores leads based on ICP fit.
#[derive(Debug, Clone, Default)]
pub struct LeadScorerAgent;

impl Agent for LeadScorerAgent {
    fn name(&self) -> &str {
        "lead_scorer"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Signals]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        let has_enriched = ctx
            .get(ContextKey::Signals)
            .iter()
            .any(|s| s.content.contains("\"type\":\"enriched_lead\""));
        let has_scores = ctx
            .get(ContextKey::Evaluations)
            .iter()
            .any(|e| e.id.starts_with(LEAD_PREFIX) && e.id.contains("score"));
        has_enriched && !has_scores
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let signals = ctx.get(ContextKey::Signals);
        let mut facts = Vec::new();

        for enriched in signals.iter() {
            if enriched.content.contains("\"type\":\"enriched_lead\"") {
                let score = 85;
                facts.push(Fact {
                    key: ContextKey::Evaluations,
                    id: format!("{}score:{}", LEAD_PREFIX, enriched.id),
                    content: serde_json::json!({
                        "type": "lead_score",
                        "lead_id": enriched.id,
                        "score": score,
                        "score_breakdown": {
                            "company_size": 25,
                            "industry_fit": 30,
                            "title_match": 20,
                            "technology_fit": 10
                        },
                        "icp_match": score >= 70
                    })
                    .to_string(),
                });
            }
        }

        AgentEffect::with_facts(facts)
    }
}

/// Routes qualified leads to owners.
#[derive(Debug, Clone, Default)]
pub struct LeadRouterAgent;

impl Agent for LeadRouterAgent {
    fn name(&self) -> &str {
        "lead_router"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Evaluations, ContextKey::Seeds]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        ctx.get(ContextKey::Evaluations)
            .iter()
            .any(|s| s.content.contains("\"icp_match\":true"))
            && ctx.get(ContextKey::Seeds).iter().any(|l| {
                l.id.starts_with(LEAD_PREFIX) && l.content.contains("\"state\":\"qualified\"")
            })
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let seeds = ctx.get(ContextKey::Seeds);
        let mut facts = Vec::new();

        for lead in seeds.iter() {
            if lead.id.starts_with(LEAD_PREFIX) && lead.content.contains("\"state\":\"qualified\"")
            {
                facts.push(Fact {
                    key: ContextKey::Proposals,
                    id: format!("{}assignment:{}", LEAD_PREFIX, lead.id),
                    content: serde_json::json!({
                        "type": "lead_assignment",
                        "lead_id": lead.id,
                        "owner_id": "user_123",
                        "routing_method": "territory_match",
                        "new_state": "assigned"
                    })
                    .to_string(),
                });
            }
        }

        AgentEffect::with_facts(facts)
    }
}

/// Selects outreach sequences for assigned leads.
#[derive(Debug, Clone, Default)]
pub struct SequenceSelectorAgent;

impl Agent for SequenceSelectorAgent {
    fn name(&self) -> &str {
        "sequence_selector"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Proposals, ContextKey::Signals]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        let has_assigned = ctx.get(ContextKey::Proposals).iter().any(|l| {
            l.id.contains(LEAD_PREFIX) && l.content.contains("\"new_state\":\"assigned\"")
        });
        let has_sequences = ctx
            .get(ContextKey::Proposals)
            .iter()
            .any(|p| p.id.starts_with(SEQUENCE_PREFIX));
        has_assigned && !has_sequences
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let proposals = ctx.get(ContextKey::Proposals);
        let mut facts = Vec::new();

        for assignment in proposals.iter() {
            if assignment.id.contains(LEAD_PREFIX)
                && assignment.content.contains("\"new_state\":\"assigned\"")
            {
                facts.push(Fact {
                    key: ContextKey::Proposals,
                    id: format!("{}{}", SEQUENCE_PREFIX, assignment.id),
                    content: serde_json::json!({
                        "type": "sequence",
                        "lead_id": assignment.id,
                        "sequence_template": "saas_outbound_v2",
                        "selection_factors": {
                            "lead_source": 0.3,
                            "industry": 0.3,
                            "company_size": 0.2,
                            "past_engagement": 0.2
                        },
                        "state": "active"
                    })
                    .to_string(),
                });
            }
        }

        AgentEffect::with_facts(facts)
    }
}

/// Generates proposals for opportunities.
#[derive(Debug, Clone, Default)]
pub struct ProposalGeneratorAgent;

impl Agent for ProposalGeneratorAgent {
    fn name(&self) -> &str {
        "proposal_generator"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Proposals]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        let has_opp_proposal_state = ctx.get(ContextKey::Proposals).iter().any(|o| {
            o.id.starts_with(OPPORTUNITY_PREFIX) && o.content.contains("\"state\":\"proposal\"")
        });
        let has_proposals = ctx
            .get(ContextKey::Proposals)
            .iter()
            .any(|p| p.id.starts_with(PROPOSAL_PREFIX));
        has_opp_proposal_state && !has_proposals
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let proposals = ctx.get(ContextKey::Proposals);
        let mut facts = Vec::new();

        for opp in proposals.iter() {
            if opp.id.starts_with(OPPORTUNITY_PREFIX)
                && opp.content.contains("\"state\":\"proposal\"")
            {
                facts.push(Fact {
                    key: ContextKey::Proposals,
                    id: format!("{}{}", PROPOSAL_PREFIX, opp.id),
                    content: serde_json::json!({
                        "type": "proposal",
                        "opportunity_id": opp.id,
                        "line_items": [],
                        "pricing": "from_rules",
                        "terms": "standard",
                        "validity_days": 30,
                        "state": "draft"
                    })
                    .to_string(),
                });
            }
        }

        AgentEffect::with_facts(facts)
    }
}

/// Closes deals from signed contracts.
#[derive(Debug, Clone, Default)]
pub struct DealCloserAgent;

impl Agent for DealCloserAgent {
    fn name(&self) -> &str {
        "deal_closer"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Signals, ContextKey::Proposals]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        ctx.get(ContextKey::Signals)
            .iter()
            .any(|c| c.content.contains("\"state\":\"signed\""))
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let signals = ctx.get(ContextKey::Signals);
        let mut facts = Vec::new();

        for contract in signals.iter() {
            if contract.content.contains("\"state\":\"signed\"") {
                facts.push(Fact {
                    key: ContextKey::Proposals,
                    id: format!("{}{}", DEAL_PREFIX, contract.id),
                    content: serde_json::json!({
                        "type": "deal",
                        "contract_id": contract.id,
                        "value": 50000,
                        "currency": "USD",
                        "close_date": "2026-01-12",
                        "state": "pending_handoff"
                    })
                    .to_string(),
                });
            }
        }

        AgentEffect::with_facts(facts)
    }
}

/// Schedules handoffs for closed deals.
#[derive(Debug, Clone, Default)]
pub struct HandoffSchedulerAgent;

impl Agent for HandoffSchedulerAgent {
    fn name(&self) -> &str {
        "handoff_scheduler"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Proposals]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        ctx.get(ContextKey::Proposals).iter().any(|d| {
            d.id.starts_with(DEAL_PREFIX) && d.content.contains("\"state\":\"pending_handoff\"")
        })
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let proposals = ctx.get(ContextKey::Proposals);
        let mut facts = Vec::new();

        for deal in proposals.iter() {
            if deal.id.starts_with(DEAL_PREFIX)
                && deal.content.contains("\"state\":\"pending_handoff\"")
            {
                facts.push(Fact {
                    key: ContextKey::Proposals,
                    id: format!("{}{}", HANDOFF_PREFIX, deal.id),
                    content: serde_json::json!({
                        "type": "handoff",
                        "deal_id": deal.id,
                        "success_owner": "cs_user_456",
                        "handoff_date": "2026-01-14",
                        "contract_summary": "extracted",
                        "key_stakeholders": []
                    })
                    .to_string(),
                });
            }
        }

        AgentEffect::with_facts(facts)
    }
}

/// Detects stale opportunities with no recent activity.
#[derive(Debug, Clone, Default)]
pub struct StaleOpportunityDetectorAgent;

impl Agent for StaleOpportunityDetectorAgent {
    fn name(&self) -> &str {
        "stale_opportunity_detector"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Proposals, ContextKey::Signals]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        ctx.get(ContextKey::Proposals)
            .iter()
            .any(|o| o.id.starts_with(OPPORTUNITY_PREFIX))
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let proposals = ctx.get(ContextKey::Proposals);
        let signals = ctx.get(ContextKey::Signals);
        let mut facts = Vec::new();

        for opp in proposals.iter() {
            if opp.id.starts_with(OPPORTUNITY_PREFIX) {
                let has_recent_activity = signals.iter().any(|a| a.content.contains(&opp.id));

                if !has_recent_activity {
                    facts.push(Fact {
                        key: ContextKey::Proposals,
                        id: format!("{}stale:{}", OPPORTUNITY_PREFIX, opp.id),
                        content: serde_json::json!({
                            "type": "stale_opportunity",
                            "opportunity_id": opp.id,
                            "new_state": "stalled",
                            "escalation": "owner_manager"
                        })
                        .to_string(),
                    });
                }
            }
        }

        AgentEffect::with_facts(facts)
    }
}

// ============================================================================
// Invariants
// ============================================================================

/// Ensures leads have source attribution.
#[derive(Debug, Clone, Default)]
pub struct LeadHasSourceInvariant;

impl Invariant for LeadHasSourceInvariant {
    fn name(&self) -> &str {
        "lead_has_source"
    }

    fn class(&self) -> InvariantClass {
        InvariantClass::Structural
    }

    fn check(&self, ctx: &Context) -> InvariantResult {
        for lead in ctx.get(ContextKey::Seeds).iter() {
            if lead.id.starts_with(LEAD_PREFIX) && !lead.content.contains("\"source\":") {
                return InvariantResult::Violated(Violation::with_facts(
                    format!("Lead {} missing source", lead.id),
                    vec![lead.id.clone()],
                ));
            }
        }
        InvariantResult::Ok
    }
}

/// Ensures closed-won deals trigger handoffs.
#[derive(Debug, Clone, Default)]
pub struct ClosedWonTriggersHandoffInvariant;

impl Invariant for ClosedWonTriggersHandoffInvariant {
    fn name(&self) -> &str {
        "closed_won_triggers_handoff"
    }

    fn class(&self) -> InvariantClass {
        InvariantClass::Acceptance
    }

    fn check(&self, ctx: &Context) -> InvariantResult {
        let proposals = ctx.get(ContextKey::Proposals);
        for deal in proposals.iter() {
            if deal.id.starts_with(DEAL_PREFIX) {
                let has_handoff = proposals
                    .iter()
                    .any(|h| h.id.starts_with(HANDOFF_PREFIX) && h.content.contains(&deal.id));
                if !has_handoff {
                    return InvariantResult::Violated(Violation::with_facts(
                        format!("Deal {} has no handoff scheduled", deal.id),
                        vec![deal.id.clone()],
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

    #[test]
    fn agents_have_correct_names() {
        assert_eq!(LeadEnrichmentAgent.name(), "lead_enrichment");
        assert_eq!(LeadScorerAgent.name(), "lead_scorer");
        assert_eq!(LeadRouterAgent.name(), "lead_router");
        assert_eq!(SequenceSelectorAgent.name(), "sequence_selector");
        assert_eq!(ProposalGeneratorAgent.name(), "proposal_generator");
        assert_eq!(DealCloserAgent.name(), "deal_closer");
        assert_eq!(HandoffSchedulerAgent.name(), "handoff_scheduler");
        assert_eq!(
            StaleOpportunityDetectorAgent.name(),
            "stale_opportunity_detector"
        );
    }
}
