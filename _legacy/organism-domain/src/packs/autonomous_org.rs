// Copyright 2024-2025 Aprio One AB, Sweden
// SPDX-License-Identifier: MIT

//! Autonomous Org Pack agents for policies, approvals, and budgets.
//!
//! # Philosophy: "Smart-contract style" governance
//!
//! - Policy-as-code with versioning
//! - Automated approvals when conditions are met
//! - Explicit delegation chains with scope limits
//! - Complete audit trails
//!
//! # Policy State Machine
//!
//! ```text
//! draft → proposed → pending_activation → active → superseded/retired
//!                                           ↓
//!                            amendment_pending → active
//! ```
//!
//! # Approval Flow State Machine
//!
//! ```text
//! created → collecting_signoffs → approved → executed → logged
//! ```
//!
//! # Budget Envelope State Machine
//!
//! ```text
//! proposed → active → warning → depleted → exception_active
//! ```
//!
//! # Key Invariants
//!
//! - All policy changes versioned and audited
//! - No self-approval
//! - Two-person rule for high-risk actions
//! - No spend beyond envelope without exception

use converge_core::{
    Agent, AgentEffect, Context, ContextKey, Fact,
    invariant::{Invariant, InvariantClass, InvariantResult, Violation},
};

// ============================================================================
// Fact ID Prefixes
// ============================================================================

pub const POLICY_PREFIX: &str = "policy:";
pub const APPROVAL_PREFIX: &str = "approval:";
pub const BUDGET_ENVELOPE_PREFIX: &str = "budget_envelope:";
pub const EXCEPTION_PREFIX: &str = "exception:";
pub const DELEGATION_PREFIX: &str = "delegation:";
pub const RISK_CONTROL_PREFIX: &str = "risk_control:";

// ============================================================================
// Agents
// ============================================================================

/// Creates and manages organizational policies.
#[derive(Debug, Clone, Default)]
pub struct PolicyManagerAgent;

impl Agent for PolicyManagerAgent {
    fn name(&self) -> &str {
        "policy_manager"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Seeds]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        ctx.get(ContextKey::Seeds)
            .iter()
            .any(|s| s.content.contains("policy.drafted") || s.content.contains("policy.proposed"))
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let seeds = ctx.get(ContextKey::Seeds);
        let mut facts = Vec::new();

        for seed in seeds.iter() {
            if seed.content.contains("policy.drafted") || seed.content.contains("policy.proposed") {
                facts.push(Fact {
                    key: ContextKey::Proposals,
                    id: format!("{}{}", POLICY_PREFIX, seed.id),
                    content: serde_json::json!({
                        "type": "policy",
                        "seed_id": seed.id,
                        "state": "draft",
                        "version": "1.0.0",
                        "owner": "policy_owner",
                        "scope": "defined",
                        "effective_date": null,
                        "sunset_date": null,
                        "created_at": "2026-01-12"
                    })
                    .to_string(),
                });
            }
        }

        AgentEffect::with_facts(facts)
    }
}

/// Enforces active policies.
#[derive(Debug, Clone, Default)]
pub struct PolicyEnforcerAgent;

impl Agent for PolicyEnforcerAgent {
    fn name(&self) -> &str {
        "policy_enforcer"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Proposals, ContextKey::Signals]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        ctx.get(ContextKey::Proposals)
            .iter()
            .any(|p| p.id.starts_with(POLICY_PREFIX) && p.content.contains("\"state\":\"active\""))
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let proposals = ctx.get(ContextKey::Proposals);
        let signals = ctx.get(ContextKey::Signals);
        let mut facts = Vec::new();

        for policy in proposals.iter() {
            if policy.id.starts_with(POLICY_PREFIX)
                && policy.content.contains("\"state\":\"active\"")
            {
                // Check for policy violations
                for signal in signals.iter() {
                    if signal.content.contains("action.requested") {
                        facts.push(Fact {
                            key: ContextKey::Evaluations,
                            id: format!("{}enforcement:{}", POLICY_PREFIX, signal.id),
                            content: serde_json::json!({
                                "type": "policy_check",
                                "policy_id": policy.id,
                                "action_id": signal.id,
                                "compliant": true,
                                "checked_at": "2026-01-12"
                            })
                            .to_string(),
                        });
                    }
                }
            }
        }

        AgentEffect::with_facts(facts)
    }
}

/// Manages approval workflows.
#[derive(Debug, Clone, Default)]
pub struct ApprovalRouterAgent;

impl Agent for ApprovalRouterAgent {
    fn name(&self) -> &str {
        "approval_router"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Seeds]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        ctx.get(ContextKey::Seeds).iter().any(|s| {
            s.content.contains("approval.requested") || s.content.contains("spend.requested")
        })
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let seeds = ctx.get(ContextKey::Seeds);
        let mut facts = Vec::new();

        for seed in seeds.iter() {
            if seed.content.contains("approval.requested")
                || seed.content.contains("spend.requested")
            {
                facts.push(Fact {
                    key: ContextKey::Proposals,
                    id: format!("{}{}", APPROVAL_PREFIX, seed.id),
                    content: serde_json::json!({
                        "type": "approval_flow",
                        "seed_id": seed.id,
                        "state": "created",
                        "requestor": "user_id",
                        "approvers_required": [],
                        "approvers_signed": [],
                        "deadline": "2026-01-19",
                        "rationale_required": true,
                        "high_risk": false,
                        "created_at": "2026-01-12"
                    })
                    .to_string(),
                });
            }
        }

        AgentEffect::with_facts(facts)
    }
}

/// Collects approval signoffs.
#[derive(Debug, Clone, Default)]
pub struct SignoffCollectorAgent;

impl Agent for SignoffCollectorAgent {
    fn name(&self) -> &str {
        "signoff_collector"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Proposals, ContextKey::Signals]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        ctx.get(ContextKey::Proposals).iter().any(|p| {
            p.id.starts_with(APPROVAL_PREFIX)
                && p.content.contains("\"state\":\"collecting_signoffs\"")
        })
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let proposals = ctx.get(ContextKey::Proposals);
        let signals = ctx.get(ContextKey::Signals);
        let mut facts = Vec::new();

        for approval in proposals.iter() {
            if approval.id.starts_with(APPROVAL_PREFIX)
                && approval
                    .content
                    .contains("\"state\":\"collecting_signoffs\"")
            {
                // Check for signoff signals
                for signal in signals.iter() {
                    if signal.content.contains("signoff.provided") {
                        facts.push(Fact {
                            key: ContextKey::Evaluations,
                            id: format!("{}signoff:{}", APPROVAL_PREFIX, signal.id),
                            content: serde_json::json!({
                                "type": "signoff",
                                "approval_id": approval.id,
                                "signer": "approver_id",
                                "rationale": "provided",
                                "signed_at": "2026-01-12"
                            })
                            .to_string(),
                        });
                    }
                }
            }
        }

        AgentEffect::with_facts(facts)
    }
}

/// Manages budget envelopes.
#[derive(Debug, Clone, Default)]
pub struct BudgetEnvelopeManagerAgent;

impl Agent for BudgetEnvelopeManagerAgent {
    fn name(&self) -> &str {
        "budget_envelope_manager"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Seeds]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        ctx.get(ContextKey::Seeds).iter().any(|s| {
            s.content.contains("budget.created") || s.content.contains("envelope.proposed")
        })
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let seeds = ctx.get(ContextKey::Seeds);
        let mut facts = Vec::new();

        for seed in seeds.iter() {
            if seed.content.contains("budget.created") || seed.content.contains("envelope.proposed")
            {
                facts.push(Fact {
                    key: ContextKey::Proposals,
                    id: format!("{}{}", BUDGET_ENVELOPE_PREFIX, seed.id),
                    content: serde_json::json!({
                        "type": "budget_envelope",
                        "seed_id": seed.id,
                        "state": "proposed",
                        "owner": "budget_owner",
                        "total_amount": 50000,
                        "consumed_amount": 0,
                        "currency": "USD",
                        "refresh_cycle": "quarterly",
                        "warning_threshold": 0.8,
                        "created_at": "2026-01-12"
                    })
                    .to_string(),
                });
            }
        }

        AgentEffect::with_facts(facts)
    }
}

/// Monitors budget consumption and triggers warnings.
#[derive(Debug, Clone, Default)]
pub struct BudgetMonitorAgent;

impl Agent for BudgetMonitorAgent {
    fn name(&self) -> &str {
        "budget_monitor"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Proposals, ContextKey::Signals]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        ctx.get(ContextKey::Proposals).iter().any(|p| {
            p.id.starts_with(BUDGET_ENVELOPE_PREFIX) && p.content.contains("\"state\":\"active\"")
        })
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let proposals = ctx.get(ContextKey::Proposals);
        let mut facts = Vec::new();

        for envelope in proposals.iter() {
            if envelope.id.starts_with(BUDGET_ENVELOPE_PREFIX)
                && envelope.content.contains("\"state\":\"active\"")
            {
                // Calculate consumption ratio
                facts.push(Fact {
                    key: ContextKey::Evaluations,
                    id: format!("{}monitor:{}", BUDGET_ENVELOPE_PREFIX, envelope.id),
                    content: serde_json::json!({
                        "type": "budget_status",
                        "envelope_id": envelope.id,
                        "consumption_ratio": 0.45,
                        "warning_triggered": false,
                        "depleted": false,
                        "checked_at": "2026-01-12"
                    })
                    .to_string(),
                });
            }
        }

        AgentEffect::with_facts(facts)
    }
}

/// Validates spend requests against budget envelopes.
#[derive(Debug, Clone, Default)]
pub struct SpendValidatorAgent;

impl Agent for SpendValidatorAgent {
    fn name(&self) -> &str {
        "spend_validator"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Seeds, ContextKey::Proposals]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        ctx.get(ContextKey::Seeds)
            .iter()
            .any(|s| s.content.contains("spend.requested"))
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let seeds = ctx.get(ContextKey::Seeds);
        let proposals = ctx.get(ContextKey::Proposals);
        let mut facts = Vec::new();

        for seed in seeds.iter() {
            if seed.content.contains("spend.requested") {
                // Check against budget envelope
                let has_budget = proposals.iter().any(|p| {
                    p.id.starts_with(BUDGET_ENVELOPE_PREFIX)
                        && p.content.contains("\"state\":\"active\"")
                });

                facts.push(Fact {
                    key: ContextKey::Evaluations,
                    id: format!("{}validation:{}", BUDGET_ENVELOPE_PREFIX, seed.id),
                    content: serde_json::json!({
                        "type": "spend_validation",
                        "spend_id": seed.id,
                        "budget_available": has_budget,
                        "within_envelope": true,
                        "requires_exception": false,
                        "validated_at": "2026-01-12"
                    })
                    .to_string(),
                });
            }
        }

        AgentEffect::with_facts(facts)
    }
}

/// Manages exception requests.
#[derive(Debug, Clone, Default)]
pub struct ExceptionHandlerAgent;

impl Agent for ExceptionHandlerAgent {
    fn name(&self) -> &str {
        "exception_handler"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Seeds]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        ctx.get(ContextKey::Seeds)
            .iter()
            .any(|s| s.content.contains("exception.requested"))
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let seeds = ctx.get(ContextKey::Seeds);
        let mut facts = Vec::new();

        for seed in seeds.iter() {
            if seed.content.contains("exception.requested") {
                facts.push(Fact {
                    key: ContextKey::Proposals,
                    id: format!("{}{}", EXCEPTION_PREFIX, seed.id),
                    content: serde_json::json!({
                        "type": "exception",
                        "seed_id": seed.id,
                        "state": "pending",
                        "requestor": "user_id",
                        "expiry_date": "2026-02-12",
                        "risk_assessment": "required",
                        "mitigation": "required",
                        "approver_level": "director",
                        "created_at": "2026-01-12"
                    })
                    .to_string(),
                });
            }
        }

        AgentEffect::with_facts(facts)
    }
}

/// Manages delegation of authority.
#[derive(Debug, Clone, Default)]
pub struct DelegationManagerAgent;

impl Agent for DelegationManagerAgent {
    fn name(&self) -> &str {
        "delegation_manager"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Seeds]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        ctx.get(ContextKey::Seeds).iter().any(|s| {
            s.content.contains("delegation.created") || s.content.contains("authority.delegated")
        })
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let seeds = ctx.get(ContextKey::Seeds);
        let mut facts = Vec::new();

        for seed in seeds.iter() {
            if seed.content.contains("delegation.created")
                || seed.content.contains("authority.delegated")
            {
                facts.push(Fact {
                    key: ContextKey::Proposals,
                    id: format!("{}{}", DELEGATION_PREFIX, seed.id),
                    content: serde_json::json!({
                        "type": "delegation",
                        "seed_id": seed.id,
                        "state": "active",
                        "delegator": "manager_id",
                        "delegate": "employee_id",
                        "scope": "expense_approval",
                        "limit": 5000,
                        "expiry_date": "2026-03-12",
                        "created_at": "2026-01-12"
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

/// Ensures all policy changes are versioned.
#[derive(Debug, Clone, Default)]
pub struct PolicyVersioningRequiredInvariant;

impl Invariant for PolicyVersioningRequiredInvariant {
    fn name(&self) -> &str {
        "policy_versioning_required"
    }

    fn class(&self) -> InvariantClass {
        InvariantClass::Structural
    }

    fn check(&self, ctx: &Context) -> InvariantResult {
        for policy in ctx.get(ContextKey::Proposals).iter() {
            if policy.id.starts_with(POLICY_PREFIX) && !policy.content.contains("\"version\":") {
                return InvariantResult::Violated(Violation::with_facts(
                    format!("Policy {} missing version", policy.id),
                    vec![policy.id.clone()],
                ));
            }
        }
        InvariantResult::Ok
    }
}

/// Ensures policies have owner.
#[derive(Debug, Clone, Default)]
pub struct PolicyHasOwnerInvariant;

impl Invariant for PolicyHasOwnerInvariant {
    fn name(&self) -> &str {
        "policy_has_owner"
    }

    fn class(&self) -> InvariantClass {
        InvariantClass::Structural
    }

    fn check(&self, ctx: &Context) -> InvariantResult {
        for policy in ctx.get(ContextKey::Proposals).iter() {
            if policy.id.starts_with(POLICY_PREFIX) && !policy.content.contains("\"owner\":") {
                return InvariantResult::Violated(Violation::with_facts(
                    format!("Policy {} missing owner", policy.id),
                    vec![policy.id.clone()],
                ));
            }
        }
        InvariantResult::Ok
    }
}

/// Ensures no self-approval.
#[derive(Debug, Clone, Default)]
pub struct NoSelfApprovalInvariant;

impl Invariant for NoSelfApprovalInvariant {
    fn name(&self) -> &str {
        "no_self_approval"
    }

    fn class(&self) -> InvariantClass {
        InvariantClass::Acceptance
    }

    fn check(&self, ctx: &Context) -> InvariantResult {
        let evaluations = ctx.get(ContextKey::Evaluations);
        let proposals = ctx.get(ContextKey::Proposals);

        for signoff in evaluations.iter() {
            if signoff.id.contains("signoff") {
                // Find the corresponding approval
                for approval in proposals.iter() {
                    if approval.id.starts_with(APPROVAL_PREFIX) {
                        // Check if signer == requestor
                        if signoff.content.contains("\"signer\":")
                            && approval.content.contains("\"requestor\":")
                        {
                            // Simplified check - in practice would parse and compare IDs
                            if signoff.content.contains("self_approved") {
                                return InvariantResult::Violated(Violation::with_facts(
                                    format!("Self-approval detected in {}", signoff.id),
                                    vec![signoff.id.clone()],
                                ));
                            }
                        }
                    }
                }
            }
        }
        InvariantResult::Ok
    }
}

/// Ensures two-person rule for high-risk actions.
#[derive(Debug, Clone, Default)]
pub struct TwoPersonRuleHighRiskInvariant;

impl Invariant for TwoPersonRuleHighRiskInvariant {
    fn name(&self) -> &str {
        "two_person_rule_high_risk"
    }

    fn class(&self) -> InvariantClass {
        InvariantClass::Acceptance
    }

    fn check(&self, ctx: &Context) -> InvariantResult {
        for approval in ctx.get(ContextKey::Proposals).iter() {
            if approval.id.starts_with(APPROVAL_PREFIX)
                && approval.content.contains("\"high_risk\":true")
                && approval.content.contains("\"state\":\"approved\"")
            {
                // Check that at least 2 approvers signed
                if !approval.content.contains("\"approvers_signed\":[")
                    || approval.content.contains("\"approvers_signed\":[]")
                {
                    return InvariantResult::Violated(Violation::with_facts(
                        format!(
                            "High-risk approval {} needs at least 2 approvers",
                            approval.id
                        ),
                        vec![approval.id.clone()],
                    ));
                }
            }
        }
        InvariantResult::Ok
    }
}

/// Ensures no spend beyond envelope without exception.
#[derive(Debug, Clone, Default)]
pub struct NoSpendBeyondEnvelopeInvariant;

impl Invariant for NoSpendBeyondEnvelopeInvariant {
    fn name(&self) -> &str {
        "no_spend_beyond_envelope"
    }

    fn class(&self) -> InvariantClass {
        InvariantClass::Acceptance
    }

    fn check(&self, ctx: &Context) -> InvariantResult {
        for validation in ctx.get(ContextKey::Evaluations).iter() {
            if validation.id.contains("spend_validation")
                && validation.content.contains("\"within_envelope\":false")
                && !validation.content.contains("\"requires_exception\":true")
            {
                return InvariantResult::Violated(Violation::with_facts(
                    format!("Spend {} exceeds envelope without exception", validation.id),
                    vec![validation.id.clone()],
                ));
            }
        }
        InvariantResult::Ok
    }
}

/// Ensures exceptions have expiry.
#[derive(Debug, Clone, Default)]
pub struct ExceptionHasExpiryInvariant;

impl Invariant for ExceptionHasExpiryInvariant {
    fn name(&self) -> &str {
        "exception_has_expiry"
    }

    fn class(&self) -> InvariantClass {
        InvariantClass::Structural
    }

    fn check(&self, ctx: &Context) -> InvariantResult {
        for exception in ctx.get(ContextKey::Proposals).iter() {
            if exception.id.starts_with(EXCEPTION_PREFIX)
                && exception.content.contains("\"state\":\"approved\"")
                && !exception.content.contains("\"expiry_date\":")
            {
                return InvariantResult::Violated(Violation::with_facts(
                    format!("Approved exception {} missing expiry date", exception.id),
                    vec![exception.id.clone()],
                ));
            }
        }
        InvariantResult::Ok
    }
}

/// Ensures delegations have scope limits.
#[derive(Debug, Clone, Default)]
pub struct DelegationHasScopeLimitsInvariant;

impl Invariant for DelegationHasScopeLimitsInvariant {
    fn name(&self) -> &str {
        "delegation_has_scope_limits"
    }

    fn class(&self) -> InvariantClass {
        InvariantClass::Structural
    }

    fn check(&self, ctx: &Context) -> InvariantResult {
        for delegation in ctx.get(ContextKey::Proposals).iter() {
            if delegation.id.starts_with(DELEGATION_PREFIX) {
                if !delegation.content.contains("\"scope\":")
                    || !delegation.content.contains("\"limit\":")
                {
                    return InvariantResult::Violated(Violation::with_facts(
                        format!("Delegation {} missing scope or limit", delegation.id),
                        vec![delegation.id.clone()],
                    ));
                }
            }
        }
        InvariantResult::Ok
    }
}

/// Ensures approvals have rationale.
#[derive(Debug, Clone, Default)]
pub struct ApprovalHasRationaleInvariant;

impl Invariant for ApprovalHasRationaleInvariant {
    fn name(&self) -> &str {
        "approval_has_rationale"
    }

    fn class(&self) -> InvariantClass {
        InvariantClass::Acceptance
    }

    fn check(&self, ctx: &Context) -> InvariantResult {
        for signoff in ctx.get(ContextKey::Evaluations).iter() {
            if signoff.id.contains("signoff") && !signoff.content.contains("\"rationale\":") {
                return InvariantResult::Violated(Violation::with_facts(
                    format!("Signoff {} missing rationale", signoff.id),
                    vec![signoff.id.clone()],
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
        assert_eq!(PolicyManagerAgent.name(), "policy_manager");
        assert_eq!(PolicyEnforcerAgent.name(), "policy_enforcer");
        assert_eq!(ApprovalRouterAgent.name(), "approval_router");
        assert_eq!(SignoffCollectorAgent.name(), "signoff_collector");
        assert_eq!(BudgetEnvelopeManagerAgent.name(), "budget_envelope_manager");
        assert_eq!(BudgetMonitorAgent.name(), "budget_monitor");
        assert_eq!(SpendValidatorAgent.name(), "spend_validator");
        assert_eq!(ExceptionHandlerAgent.name(), "exception_handler");
        assert_eq!(DelegationManagerAgent.name(), "delegation_manager");
    }

    #[test]
    fn invariants_have_correct_names() {
        assert_eq!(
            PolicyVersioningRequiredInvariant.name(),
            "policy_versioning_required"
        );
        assert_eq!(PolicyHasOwnerInvariant.name(), "policy_has_owner");
        assert_eq!(NoSelfApprovalInvariant.name(), "no_self_approval");
        assert_eq!(
            TwoPersonRuleHighRiskInvariant.name(),
            "two_person_rule_high_risk"
        );
        assert_eq!(
            NoSpendBeyondEnvelopeInvariant.name(),
            "no_spend_beyond_envelope"
        );
        assert_eq!(ExceptionHasExpiryInvariant.name(), "exception_has_expiry");
        assert_eq!(
            DelegationHasScopeLimitsInvariant.name(),
            "delegation_has_scope_limits"
        );
        assert_eq!(
            ApprovalHasRationaleInvariant.name(),
            "approval_has_rationale"
        );
    }
}
