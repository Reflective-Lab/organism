// Copyright 2024-2025 Aprio One AB, Sweden
// SPDX-License-Identifier: MIT

//! Partnerships & Vendors Pack agents for external relationship management.
//!
//! Implements the agent contracts defined in specs/partnerships_vendors.truth.
//!
//! # Partnerships & Vendors is the Ecosystem System
//!
//! Every external relationship flows through this pack:
//! - Strategic partner identification and management
//! - Vendor assessment and approval
//! - Contract negotiation and tracking
//! - Integration coordination
//! - Performance reviews and renewals
//!
//! Note: This implementation uses the standard ContextKey enum. Facts are
//! distinguished by their ID prefixes (partner:, supplier:, assessment:, etc.).

use converge_core::{
    Agent, AgentEffect, Context, ContextKey, Fact,
    invariant::{Invariant, InvariantClass, InvariantResult, Violation},
};

// ============================================================================
// Fact ID Prefixes
// ============================================================================

pub const PARTNER_PREFIX: &str = "partner:";
pub const SUPPLIER_PREFIX: &str = "supplier:";
pub const AGREEMENT_PREFIX: &str = "p_agreement:";
pub const ASSESSMENT_PREFIX: &str = "vendor_assessment:";
pub const INTEGRATION_PREFIX: &str = "integration:";
pub const DILIGENCE_PREFIX: &str = "diligence:";
pub const RELATIONSHIP_PREFIX: &str = "relationship:";
pub const RENEWAL_PREFIX: &str = "contract_renewal:";

// ============================================================================
// Agents
// ============================================================================

/// Identifies and qualifies potential partners.
#[derive(Debug, Clone, Default)]
pub struct PartnerSourcerAgent;

impl Agent for PartnerSourcerAgent {
    fn name(&self) -> &str {
        "partner_sourcer"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Seeds]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        ctx.get(ContextKey::Seeds).iter().any(|s| {
            s.content.contains("partner.prospect") || s.content.contains("partnership.opportunity")
        })
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let triggers = ctx.get(ContextKey::Seeds);
        let mut facts = Vec::new();

        for trigger in triggers.iter() {
            if trigger.content.contains("partner.prospect")
                || trigger.content.contains("partnership.opportunity")
            {
                facts.push(Fact {
                    key: ContextKey::Proposals,
                    id: format!("{}{}", PARTNER_PREFIX, trigger.id),
                    content: serde_json::json!({
                        "type": "partner",
                        "source_id": trigger.id,
                        "state": "identified",
                        "name": null,
                        "category": "strategic",
                        "fit_score": null,
                        "owner_id": null,
                        "created_at": "2026-01-12T12:00:00Z"
                    })
                    .to_string(),
                });
            }
        }

        AgentEffect::with_facts(facts)
    }
}

/// Manages vendor security and compliance assessments.
#[derive(Debug, Clone, Default)]
pub struct VendorAssessorAgent;

impl Agent for VendorAssessorAgent {
    fn name(&self) -> &str {
        "vendor_assessor"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Proposals]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        ctx.get(ContextKey::Proposals).iter().any(|p| {
            p.id.starts_with(SUPPLIER_PREFIX) && p.content.contains("\"state\":\"assessing\"")
        })
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let proposals = ctx.get(ContextKey::Proposals);
        let mut facts = Vec::new();

        for vendor in proposals.iter() {
            if vendor.id.starts_with(SUPPLIER_PREFIX)
                && vendor.content.contains("\"state\":\"assessing\"")
            {
                facts.push(Fact {
                    key: ContextKey::Proposals,
                    id: format!("{}{}", ASSESSMENT_PREFIX, vendor.id),
                    content: serde_json::json!({
                        "type": "vendor_assessment",
                        "vendor_id": vendor.id,
                        "state": "pending",
                        "questionnaire_type": "standard",
                        "risk_score": null,
                        "findings": [],
                        "assessor_id": null,
                        "deadline": null,
                        "created_at": "2026-01-12T12:00:00Z"
                    })
                    .to_string(),
                });
            }
        }

        AgentEffect::with_facts(facts)
    }
}

/// Assists in contract negotiation and term tracking.
#[derive(Debug, Clone, Default)]
pub struct ContractNegotiatorAgent;

impl Agent for ContractNegotiatorAgent {
    fn name(&self) -> &str {
        "contract_negotiator"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Proposals]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        ctx.get(ContextKey::Proposals).iter().any(|p| {
            p.id.starts_with(AGREEMENT_PREFIX) && p.content.contains("\"state\":\"negotiating\"")
        })
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let proposals = ctx.get(ContextKey::Proposals);
        let mut facts = Vec::new();

        for agreement in proposals.iter() {
            if agreement.id.starts_with(AGREEMENT_PREFIX)
                && agreement.content.contains("\"state\":\"negotiating\"")
            {
                facts.push(Fact {
                    key: ContextKey::Evaluations,
                    id: format!("negotiation:{}", agreement.id),
                    content: serde_json::json!({
                        "type": "negotiation_analysis",
                        "agreement_id": agreement.id,
                        "standard_term_deviations": [],
                        "risk_assessment": "medium",
                        "suggested_counters": [],
                        "approval_required": false,
                        "analyzed_at": "2026-01-12T12:00:00Z"
                    })
                    .to_string(),
                });
            }
        }

        AgentEffect::with_facts(facts)
    }
}

/// Maintains partner and vendor relationships.
#[derive(Debug, Clone, Default)]
pub struct RelationshipManagerAgent;

impl Agent for RelationshipManagerAgent {
    fn name(&self) -> &str {
        "relationship_manager"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Proposals]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        ctx.get(ContextKey::Proposals).iter().any(|p| {
            (p.id.starts_with(PARTNER_PREFIX) || p.id.starts_with(SUPPLIER_PREFIX))
                && p.content.contains("\"state\":\"active\"")
        })
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let proposals = ctx.get(ContextKey::Proposals);
        let mut facts = Vec::new();

        for entity in proposals.iter() {
            if (entity.id.starts_with(PARTNER_PREFIX) || entity.id.starts_with(SUPPLIER_PREFIX))
                && entity.content.contains("\"state\":\"active\"")
            {
                facts.push(Fact {
                    key: ContextKey::Evaluations,
                    id: format!("{}{}", RELATIONSHIP_PREFIX, entity.id),
                    content: serde_json::json!({
                        "type": "relationship_health",
                        "entity_id": entity.id,
                        "health_score": 0.8,
                        "engagement_level": "healthy",
                        "last_interaction": "2026-01-10T12:00:00Z",
                        "at_risk": false,
                        "recommendations": [],
                        "assessed_at": "2026-01-12T12:00:00Z"
                    })
                    .to_string(),
                });
            }
        }

        AgentEffect::with_facts(facts)
    }
}

/// Evaluates partner and vendor performance.
#[derive(Debug, Clone, Default)]
pub struct PerformanceReviewerAgent;

impl Agent for PerformanceReviewerAgent {
    fn name(&self) -> &str {
        "performance_reviewer"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Proposals]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        ctx.get(ContextKey::Proposals).iter().any(|p| {
            (p.id.starts_with(PARTNER_PREFIX) || p.id.starts_with(SUPPLIER_PREFIX))
                && p.content.contains("\"state\":\"under_review\"")
        })
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let proposals = ctx.get(ContextKey::Proposals);
        let mut facts = Vec::new();

        for entity in proposals.iter() {
            if (entity.id.starts_with(PARTNER_PREFIX) || entity.id.starts_with(SUPPLIER_PREFIX))
                && entity.content.contains("\"state\":\"under_review\"")
            {
                facts.push(Fact {
                    key: ContextKey::Evaluations,
                    id: format!("performance:{}", entity.id),
                    content: serde_json::json!({
                        "type": "performance_review",
                        "entity_id": entity.id,
                        "performance_score": 0.85,
                        "sla_compliance": 0.95,
                        "quality_score": 0.90,
                        "value_delivered": "meets_expectations",
                        "renewal_recommendation": "renew",
                        "reviewed_at": "2026-01-12T12:00:00Z"
                    })
                    .to_string(),
                });
            }
        }

        AgentEffect::with_facts(facts)
    }
}

/// Manages technical integrations with partners/vendors.
#[derive(Debug, Clone, Default)]
pub struct IntegrationCoordinatorAgent;

impl Agent for IntegrationCoordinatorAgent {
    fn name(&self) -> &str {
        "integration_coordinator"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Seeds]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        ctx.get(ContextKey::Seeds).iter().any(|s| {
            s.content.contains("integration.required") || s.content.contains("integration.plan")
        })
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let triggers = ctx.get(ContextKey::Seeds);
        let mut facts = Vec::new();

        for trigger in triggers.iter() {
            if trigger.content.contains("integration.required")
                || trigger.content.contains("integration.plan")
            {
                facts.push(Fact {
                    key: ContextKey::Proposals,
                    id: format!("{}{}", INTEGRATION_PREFIX, trigger.id),
                    content: serde_json::json!({
                        "type": "integration",
                        "source_id": trigger.id,
                        "state": "planning",
                        "partner_id": null,
                        "integration_type": "api",
                        "owner_id": null,
                        "dependencies": [],
                        "created_at": "2026-01-12T12:00:00Z"
                    })
                    .to_string(),
                });
            }
        }

        AgentEffect::with_facts(facts)
    }
}

/// Coordinates the due diligence process.
#[derive(Debug, Clone, Default)]
pub struct DueDiligenceCoordinatorAgent;

impl Agent for DueDiligenceCoordinatorAgent {
    fn name(&self) -> &str {
        "due_diligence_coordinator"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Proposals]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        ctx.get(ContextKey::Proposals).iter().any(|p| {
            p.id.starts_with(PARTNER_PREFIX) && p.content.contains("\"state\":\"evaluating\"")
        })
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let proposals = ctx.get(ContextKey::Proposals);
        let mut facts = Vec::new();

        for partner in proposals.iter() {
            if partner.id.starts_with(PARTNER_PREFIX)
                && partner.content.contains("\"state\":\"evaluating\"")
            {
                facts.push(Fact {
                    key: ContextKey::Proposals,
                    id: format!("{}{}", DILIGENCE_PREFIX, partner.id),
                    content: serde_json::json!({
                        "type": "due_diligence",
                        "partner_id": partner.id,
                        "state": "in_progress",
                        "checklist": [
                            {"item": "financial_review", "status": "pending"},
                            {"item": "legal_review", "status": "pending"},
                            {"item": "security_review", "status": "pending"},
                            {"item": "reference_check", "status": "pending"}
                        ],
                        "created_at": "2026-01-12T12:00:00Z"
                    })
                    .to_string(),
                });
            }
        }

        AgentEffect::with_facts(facts)
    }
}

/// Tracks and manages contract renewals.
#[derive(Debug, Clone, Default)]
pub struct PartnershipRenewalTrackerAgent;

impl Agent for PartnershipRenewalTrackerAgent {
    fn name(&self) -> &str {
        "partnership_renewal_tracker"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Proposals]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        ctx.get(ContextKey::Proposals).iter().any(|p| {
            p.id.starts_with(AGREEMENT_PREFIX) && p.content.contains("\"expiration_date\"")
        })
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let proposals = ctx.get(ContextKey::Proposals);
        let mut facts = Vec::new();

        for agreement in proposals.iter() {
            if agreement.id.starts_with(AGREEMENT_PREFIX)
                && agreement.content.contains("\"expiration_date\"")
            {
                facts.push(Fact {
                    key: ContextKey::Signals,
                    id: format!("{}{}", RENEWAL_PREFIX, agreement.id),
                    content: serde_json::json!({
                        "type": "contract_renewal",
                        "agreement_id": agreement.id,
                        "state": "upcoming",
                        "days_until_expiration": 90,
                        "auto_renew": false,
                        "review_required": true,
                        "tracked_at": "2026-01-12T12:00:00Z"
                    })
                    .to_string(),
                });
            }
        }

        AgentEffect::with_facts(facts)
    }
}

/// Monitors ongoing risk from partners and vendors.
#[derive(Debug, Clone, Default)]
pub struct RiskMonitorAgent;

impl Agent for RiskMonitorAgent {
    fn name(&self) -> &str {
        "risk_monitor"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Signals]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        ctx.get(ContextKey::Signals).iter().any(|s| {
            s.content.contains("vendor.risk")
                || s.content.contains("partner.risk")
                || s.content.contains("news.negative")
        })
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let signals = ctx.get(ContextKey::Signals);
        let mut facts = Vec::new();

        for signal in signals.iter() {
            if signal.content.contains("vendor.risk")
                || signal.content.contains("partner.risk")
                || signal.content.contains("news.negative")
            {
                facts.push(Fact {
                    key: ContextKey::Evaluations,
                    id: format!("risk_alert:{}", signal.id),
                    content: serde_json::json!({
                        "type": "risk_alert",
                        "signal_id": signal.id,
                        "risk_type": "external",
                        "severity": "medium",
                        "entity_id": null,
                        "description": "Risk signal detected",
                        "recommended_action": "review",
                        "detected_at": "2026-01-12T12:00:00Z"
                    })
                    .to_string(),
                });
            }
        }

        AgentEffect::with_facts(facts)
    }
}

/// Manages partner/vendor offboarding.
#[derive(Debug, Clone, Default)]
pub struct OffboardingCoordinatorAgent;

impl Agent for OffboardingCoordinatorAgent {
    fn name(&self) -> &str {
        "offboarding_coordinator"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Proposals]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        ctx.get(ContextKey::Proposals).iter().any(|p| {
            (p.id.starts_with(PARTNER_PREFIX) && p.content.contains("\"state\":\"churned\""))
                || (p.id.starts_with(SUPPLIER_PREFIX)
                    && p.content.contains("\"state\":\"terminated\""))
        })
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let proposals = ctx.get(ContextKey::Proposals);
        let mut facts = Vec::new();

        for entity in proposals.iter() {
            if (entity.id.starts_with(PARTNER_PREFIX)
                && entity.content.contains("\"state\":\"churned\""))
                || (entity.id.starts_with(SUPPLIER_PREFIX)
                    && entity.content.contains("\"state\":\"terminated\""))
            {
                facts.push(Fact {
                    key: ContextKey::Proposals,
                    id: format!("offboarding:{}", entity.id),
                    content: serde_json::json!({
                        "type": "offboarding_plan",
                        "entity_id": entity.id,
                        "state": "initiated",
                        "checklist": [
                            {"item": "revoke_access", "status": "pending"},
                            {"item": "data_migration", "status": "pending"},
                            {"item": "final_payment", "status": "pending"},
                            {"item": "communication", "status": "pending"}
                        ],
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

/// Ensures active vendors have valid assessments.
#[derive(Debug, Clone, Default)]
pub struct VendorHasAssessmentInvariant;

impl Invariant for VendorHasAssessmentInvariant {
    fn name(&self) -> &str {
        "vendor_has_assessment"
    }

    fn class(&self) -> InvariantClass {
        InvariantClass::Structural
    }

    fn check(&self, ctx: &Context) -> InvariantResult {
        let proposals = ctx.get(ContextKey::Proposals);

        for vendor in proposals.iter() {
            if vendor.id.starts_with(SUPPLIER_PREFIX)
                && vendor.content.contains("\"state\":\"active\"")
            {
                // Check if there's a valid assessment for this vendor
                let has_assessment = proposals.iter().any(|p| {
                    p.id.starts_with(ASSESSMENT_PREFIX)
                        && p.content.contains(&vendor.id)
                        && p.content.contains("\"state\":\"completed\"")
                });

                if !has_assessment {
                    return InvariantResult::Violated(Violation::with_facts(
                        format!("Active vendor {} has no valid assessment", vendor.id),
                        vec![vendor.id.clone()],
                    ));
                }
            }
        }
        InvariantResult::Ok
    }
}

/// Ensures active partners have executed agreements.
#[derive(Debug, Clone, Default)]
pub struct PartnerHasAgreementInvariant;

impl Invariant for PartnerHasAgreementInvariant {
    fn name(&self) -> &str {
        "partner_has_agreement"
    }

    fn class(&self) -> InvariantClass {
        InvariantClass::Structural
    }

    fn check(&self, ctx: &Context) -> InvariantResult {
        let proposals = ctx.get(ContextKey::Proposals);

        for partner in proposals.iter() {
            if partner.id.starts_with(PARTNER_PREFIX)
                && partner.content.contains("\"state\":\"active\"")
            {
                // Check if there's an executed agreement for this partner
                let has_agreement = proposals.iter().any(|p| {
                    p.id.starts_with(AGREEMENT_PREFIX)
                        && p.content.contains(&partner.id)
                        && p.content.contains("\"state\":\"executed\"")
                });

                if !has_agreement {
                    return InvariantResult::Violated(Violation::with_facts(
                        format!("Active partner {} has no executed agreement", partner.id),
                        vec![partner.id.clone()],
                    ));
                }
            }
        }
        InvariantResult::Ok
    }
}

/// Ensures live integrations have owners.
#[derive(Debug, Clone, Default)]
pub struct IntegrationHasOwnerInvariant;

impl Invariant for IntegrationHasOwnerInvariant {
    fn name(&self) -> &str {
        "integration_has_owner"
    }

    fn class(&self) -> InvariantClass {
        InvariantClass::Structural
    }

    fn check(&self, ctx: &Context) -> InvariantResult {
        for integration in ctx.get(ContextKey::Proposals).iter() {
            if integration.id.starts_with(INTEGRATION_PREFIX)
                && integration.content.contains("\"state\":\"live\"")
                && integration.content.contains("\"owner_id\":null")
            {
                return InvariantResult::Violated(Violation::with_facts(
                    format!("Live integration {} has no owner", integration.id),
                    vec![integration.id.clone()],
                ));
            }
        }
        InvariantResult::Ok
    }
}

/// Ensures high-risk vendors have executive approval.
#[derive(Debug, Clone, Default)]
pub struct HighRiskVendorRequiresApprovalInvariant;

impl Invariant for HighRiskVendorRequiresApprovalInvariant {
    fn name(&self) -> &str {
        "high_risk_vendor_requires_approval"
    }

    fn class(&self) -> InvariantClass {
        InvariantClass::Semantic
    }

    fn check(&self, ctx: &Context) -> InvariantResult {
        for vendor in ctx.get(ContextKey::Proposals).iter() {
            if vendor.id.starts_with(SUPPLIER_PREFIX)
                && vendor.content.contains("\"risk_level\":\"high\"")
                && vendor.content.contains("\"state\":\"approved\"")
                && !vendor.content.contains("\"executive_approval\":true")
            {
                return InvariantResult::Violated(Violation::with_facts(
                    format!(
                        "High-risk vendor {} approved without executive approval",
                        vendor.id
                    ),
                    vec![vendor.id.clone()],
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
        assert_eq!(PartnerSourcerAgent.name(), "partner_sourcer");
        assert_eq!(VendorAssessorAgent.name(), "vendor_assessor");
        assert_eq!(ContractNegotiatorAgent.name(), "contract_negotiator");
        assert_eq!(RelationshipManagerAgent.name(), "relationship_manager");
        assert_eq!(PerformanceReviewerAgent.name(), "performance_reviewer");
        assert_eq!(
            IntegrationCoordinatorAgent.name(),
            "integration_coordinator"
        );
        assert_eq!(
            DueDiligenceCoordinatorAgent.name(),
            "due_diligence_coordinator"
        );
        assert_eq!(
            PartnershipRenewalTrackerAgent.name(),
            "partnership_renewal_tracker"
        );
        assert_eq!(RiskMonitorAgent.name(), "risk_monitor");
        assert_eq!(
            OffboardingCoordinatorAgent.name(),
            "offboarding_coordinator"
        );
    }
}
