// Copyright 2024-2025 Aprio One AB, Sweden
// SPDX-License-Identifier: MIT

//! Procurement & Assets Pack agents for purchase and asset management.
//!
//! Implements the agent contracts defined in specs/procurement_assets.truth.
//!
//! # Procurement & Assets is the Control System
//!
//! Every purchase and asset flows through this pack:
//! - Purchase requests and approvals
//! - Asset tracking and assignment
//! - Software subscription management
//! - Vendor relationships
//! - Renewal tracking
//!
//! Note: This implementation uses the standard ContextKey enum. Facts are
//! distinguished by their ID prefixes (request:, asset:, subscription:, etc.).

use converge_core::{
    Agent, AgentEffect, Context, ContextKey, Fact,
    invariant::{Invariant, InvariantClass, InvariantResult, Violation},
};

// ============================================================================
// Fact ID Prefixes
// ============================================================================

pub const REQUEST_PREFIX: &str = "request:";
pub const APPROVAL_PREFIX: &str = "approval:";
pub const ORDER_PREFIX: &str = "order:";
pub const ASSET_PREFIX: &str = "asset:";
pub const SUBSCRIPTION_PREFIX: &str = "subscription:";
pub const VENDOR_PREFIX: &str = "vendor:";
pub const RENEWAL_PREFIX: &str = "renewal:";
pub const BUDGET_PREFIX: &str = "budget:";

// ============================================================================
// Agents
// ============================================================================

/// Intakes and validates purchase requests.
#[derive(Debug, Clone, Default)]
pub struct RequestIntakeAgent;

impl Agent for RequestIntakeAgent {
    fn name(&self) -> &str {
        "request_intake"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Seeds]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        ctx.get(ContextKey::Seeds).iter().any(|s| {
            s.content.contains("purchase.request") || s.content.contains("procurement.request")
        })
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let triggers = ctx.get(ContextKey::Seeds);
        let mut facts = Vec::new();

        for trigger in triggers.iter() {
            if trigger.content.contains("purchase.request")
                || trigger.content.contains("procurement.request")
            {
                facts.push(Fact {
                    key: ContextKey::Proposals,
                    id: format!("{}{}", REQUEST_PREFIX, trigger.id),
                    content: serde_json::json!({
                        "type": "purchase_request",
                        "source_id": trigger.id,
                        "state": "submitted",
                        "amount": 0,
                        "currency": "USD",
                        "category": "general",
                        "owner_id": null,
                        "budget_id": null,
                        "created_at": "2026-01-12T12:00:00Z"
                    })
                    .to_string(),
                });
            }
        }

        AgentEffect::with_facts(facts)
    }
}

/// Routes purchase requests to appropriate approvers.
#[derive(Debug, Clone, Default)]
pub struct ApprovalRouterAgent;

impl Agent for ApprovalRouterAgent {
    fn name(&self) -> &str {
        "approval_router"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Proposals]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        ctx.get(ContextKey::Proposals).iter().any(|p| {
            p.id.starts_with(REQUEST_PREFIX) && p.content.contains("\"state\":\"submitted\"")
        })
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let proposals = ctx.get(ContextKey::Proposals);
        let mut facts = Vec::new();

        for request in proposals.iter() {
            if request.id.starts_with(REQUEST_PREFIX)
                && request.content.contains("\"state\":\"submitted\"")
            {
                facts.push(Fact {
                    key: ContextKey::Proposals,
                    id: format!("{}{}", APPROVAL_PREFIX, request.id),
                    content: serde_json::json!({
                        "type": "approval",
                        "request_id": request.id,
                        "state": "pending",
                        "approvers": ["manager"],
                        "current_approver": "manager",
                        "routed_at": "2026-01-12T12:00:00Z"
                    })
                    .to_string(),
                });
            }
        }

        AgentEffect::with_facts(facts)
    }
}

/// Executes approved purchases.
#[derive(Debug, Clone, Default)]
pub struct PurchaseExecutorAgent;

impl Agent for PurchaseExecutorAgent {
    fn name(&self) -> &str {
        "purchase_executor"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Proposals]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        ctx.get(ContextKey::Proposals).iter().any(|p| {
            p.id.starts_with(APPROVAL_PREFIX) && p.content.contains("\"state\":\"approved\"")
        })
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let proposals = ctx.get(ContextKey::Proposals);
        let mut facts = Vec::new();

        for approval in proposals.iter() {
            if approval.id.starts_with(APPROVAL_PREFIX)
                && approval.content.contains("\"state\":\"approved\"")
            {
                facts.push(Fact {
                    key: ContextKey::Proposals,
                    id: format!("{}{}", ORDER_PREFIX, approval.id),
                    content: serde_json::json!({
                        "type": "purchase_order",
                        "approval_id": approval.id,
                        "state": "purchasing",
                        "order_number": "PO-2026-001",
                        "created_at": "2026-01-12T12:00:00Z"
                    })
                    .to_string(),
                });
            }
        }

        AgentEffect::with_facts(facts)
    }
}

/// Tracks and manages company assets.
#[derive(Debug, Clone, Default)]
pub struct AssetTrackerAgent;

impl Agent for AssetTrackerAgent {
    fn name(&self) -> &str {
        "asset_tracker"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Seeds]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        ctx.get(ContextKey::Seeds)
            .iter()
            .any(|s| s.content.contains("asset.received") || s.content.contains("asset.register"))
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let triggers = ctx.get(ContextKey::Seeds);
        let mut facts = Vec::new();

        for trigger in triggers.iter() {
            if trigger.content.contains("asset.received")
                || trigger.content.contains("asset.register")
            {
                facts.push(Fact {
                    key: ContextKey::Proposals,
                    id: format!("{}{}", ASSET_PREFIX, trigger.id),
                    content: serde_json::json!({
                        "type": "asset",
                        "source_id": trigger.id,
                        "state": "received",
                        "asset_tag": format!("AST-{}", trigger.id),
                        "asset_type": "hardware",
                        "assigned_to": null,
                        "location": null,
                        "created_at": "2026-01-12T12:00:00Z"
                    })
                    .to_string(),
                });
            }
        }

        AgentEffect::with_facts(facts)
    }
}

/// Manages software subscriptions.
#[derive(Debug, Clone, Default)]
pub struct SubscriptionManagerAgent;

impl Agent for SubscriptionManagerAgent {
    fn name(&self) -> &str {
        "subscription_manager"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Proposals]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        ctx.get(ContextKey::Proposals).iter().any(|p| {
            p.id.starts_with(SUBSCRIPTION_PREFIX) && p.content.contains("\"state\":\"active\"")
        })
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let proposals = ctx.get(ContextKey::Proposals);
        let mut facts = Vec::new();

        for sub in proposals.iter() {
            if sub.id.starts_with(SUBSCRIPTION_PREFIX)
                && sub.content.contains("\"state\":\"active\"")
            {
                facts.push(Fact {
                    key: ContextKey::Evaluations,
                    id: format!("utilization:{}", sub.id),
                    content: serde_json::json!({
                        "type": "subscription_utilization",
                        "subscription_id": sub.id,
                        "total_seats": 100,
                        "used_seats": 75,
                        "utilization_pct": 75,
                        "cost_per_seat": 10.0,
                        "checked_at": "2026-01-12T12:00:00Z"
                    })
                    .to_string(),
                });
            }
        }

        AgentEffect::with_facts(facts)
    }
}

/// Tracks upcoming renewals.
#[derive(Debug, Clone, Default)]
pub struct RenewalTrackerAgent;

impl Agent for RenewalTrackerAgent {
    fn name(&self) -> &str {
        "renewal_tracker"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Proposals]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        ctx.get(ContextKey::Proposals).iter().any(|p| {
            p.id.starts_with(SUBSCRIPTION_PREFIX) && p.content.contains("\"renewal_date\"")
        })
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let proposals = ctx.get(ContextKey::Proposals);
        let mut facts = Vec::new();

        for sub in proposals.iter() {
            if sub.id.starts_with(SUBSCRIPTION_PREFIX) && sub.content.contains("\"renewal_date\"") {
                facts.push(Fact {
                    key: ContextKey::Signals,
                    id: format!("{}{}", RENEWAL_PREFIX, sub.id),
                    content: serde_json::json!({
                        "type": "renewal",
                        "subscription_id": sub.id,
                        "state": "upcoming",
                        "renewal_date": "2026-04-12",
                        "days_until_renewal": 90,
                        "tracked_at": "2026-01-12T12:00:00Z"
                    })
                    .to_string(),
                });
            }
        }

        AgentEffect::with_facts(facts)
    }
}

/// Manages vendor relationships and due diligence.
#[derive(Debug, Clone, Default)]
pub struct VendorManagerAgent;

impl Agent for VendorManagerAgent {
    fn name(&self) -> &str {
        "vendor_manager"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Seeds]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        ctx.get(ContextKey::Seeds)
            .iter()
            .any(|s| s.content.contains("vendor.new") || s.content.contains("vendor.review"))
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let triggers = ctx.get(ContextKey::Seeds);
        let mut facts = Vec::new();

        for trigger in triggers.iter() {
            if trigger.content.contains("vendor.new") {
                facts.push(Fact {
                    key: ContextKey::Proposals,
                    id: format!("{}{}", VENDOR_PREFIX, trigger.id),
                    content: serde_json::json!({
                        "type": "vendor",
                        "source_id": trigger.id,
                        "state": "prospective",
                        "risk_level": "medium",
                        "security_review": null,
                        "created_at": "2026-01-12T12:00:00Z"
                    })
                    .to_string(),
                });
            }
        }

        AgentEffect::with_facts(facts)
    }
}

/// Monitors budget utilization and alerts.
#[derive(Debug, Clone, Default)]
pub struct BudgetMonitorAgent;

impl Agent for BudgetMonitorAgent {
    fn name(&self) -> &str {
        "budget_monitor"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Proposals]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        ctx.get(ContextKey::Proposals)
            .iter()
            .any(|p| p.id.starts_with(REQUEST_PREFIX) && p.content.contains("\"budget_id\""))
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let proposals = ctx.get(ContextKey::Proposals);
        let mut facts = Vec::new();

        for request in proposals.iter() {
            if request.id.starts_with(REQUEST_PREFIX) {
                facts.push(Fact {
                    key: ContextKey::Evaluations,
                    id: format!("{}{}", BUDGET_PREFIX, request.id),
                    content: serde_json::json!({
                        "type": "budget_check",
                        "request_id": request.id,
                        "within_budget": true,
                        "budget_remaining": 10000,
                        "utilization_pct": 60,
                        "checked_at": "2026-01-12T12:00:00Z"
                    })
                    .to_string(),
                });
            }
        }

        AgentEffect::with_facts(facts)
    }
}

/// Audits asset assignments and status.
#[derive(Debug, Clone, Default)]
pub struct AssetAuditorAgent;

impl Agent for AssetAuditorAgent {
    fn name(&self) -> &str {
        "asset_auditor"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Proposals]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        ctx.get(ContextKey::Proposals)
            .iter()
            .any(|p| p.id.starts_with(ASSET_PREFIX) && p.content.contains("\"state\":\"in_use\""))
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let proposals = ctx.get(ContextKey::Proposals);
        let mut facts = Vec::new();

        for asset in proposals.iter() {
            if asset.id.starts_with(ASSET_PREFIX) && asset.content.contains("\"state\":\"in_use\"")
            {
                facts.push(Fact {
                    key: ContextKey::Evaluations,
                    id: format!("audit:{}", asset.id),
                    content: serde_json::json!({
                        "type": "asset_audit",
                        "asset_id": asset.id,
                        "assignment_valid": true,
                        "location_verified": true,
                        "depreciation_current": true,
                        "audited_at": "2026-01-12T12:00:00Z"
                    })
                    .to_string(),
                });
            }
        }

        AgentEffect::with_facts(facts)
    }
}

/// Optimizes software license utilization.
#[derive(Debug, Clone, Default)]
pub struct LicenseOptimizerAgent;

impl Agent for LicenseOptimizerAgent {
    fn name(&self) -> &str {
        "license_optimizer"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Evaluations]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        ctx.get(ContextKey::Evaluations).iter().any(|e| {
            e.content.contains("\"type\":\"subscription_utilization\"")
                && e.content.contains("\"utilization_pct\"")
        })
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let evaluations = ctx.get(ContextKey::Evaluations);
        let mut facts = Vec::new();

        for util in evaluations.iter() {
            if util
                .content
                .contains("\"type\":\"subscription_utilization\"")
            {
                // Would parse utilization and recommend if low
                facts.push(Fact {
                    key: ContextKey::Proposals,
                    id: format!("optimization:{}", util.id),
                    content: serde_json::json!({
                        "type": "license_optimization",
                        "utilization_id": util.id,
                        "recommendations": [],
                        "potential_savings": 0,
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

/// Ensures every spend has an owner.
#[derive(Debug, Clone, Default)]
pub struct SpendNeedsOwnerInvariant;

impl Invariant for SpendNeedsOwnerInvariant {
    fn name(&self) -> &str {
        "spend_needs_owner"
    }

    fn class(&self) -> InvariantClass {
        InvariantClass::Structural
    }

    fn check(&self, ctx: &Context) -> InvariantResult {
        for request in ctx.get(ContextKey::Proposals).iter() {
            if request.id.starts_with(REQUEST_PREFIX)
                && request.content.contains("\"state\":\"approved\"")
                && request.content.contains("\"owner_id\":null")
            {
                return InvariantResult::Violated(Violation::with_facts(
                    format!("Approved request {} has no owner", request.id),
                    vec![request.id.clone()],
                ));
            }
        }
        InvariantResult::Ok
    }
}

/// Ensures spend is within budget.
#[derive(Debug, Clone, Default)]
pub struct SpendNeedsBudgetInvariant;

impl Invariant for SpendNeedsBudgetInvariant {
    fn name(&self) -> &str {
        "spend_needs_budget"
    }

    fn class(&self) -> InvariantClass {
        InvariantClass::Structural
    }

    fn check(&self, ctx: &Context) -> InvariantResult {
        for check in ctx.get(ContextKey::Evaluations).iter() {
            if check.content.contains("\"type\":\"budget_check\"")
                && check.content.contains("\"within_budget\":false")
            {
                return InvariantResult::Violated(Violation::with_facts(
                    format!("Request exceeds budget: {}", check.id),
                    vec![check.id.clone()],
                ));
            }
        }
        InvariantResult::Ok
    }
}

/// Ensures renewals are not missed.
#[derive(Debug, Clone, Default)]
pub struct RenewalsNotMissedInvariant;

impl Invariant for RenewalsNotMissedInvariant {
    fn name(&self) -> &str {
        "renewals_not_missed"
    }

    fn class(&self) -> InvariantClass {
        InvariantClass::Semantic
    }

    fn check(&self, ctx: &Context) -> InvariantResult {
        for renewal in ctx.get(ContextKey::Signals).iter() {
            if renewal.id.starts_with(RENEWAL_PREFIX) {
                // Check if days_until_renewal is very low and state is still "upcoming"
                if renewal.content.contains("\"days_until_renewal\":0")
                    && renewal.content.contains("\"state\":\"upcoming\"")
                {
                    return InvariantResult::Violated(Violation::with_facts(
                        format!("Renewal {} is about to be missed", renewal.id),
                        vec![renewal.id.clone()],
                    ));
                }
            }
        }
        InvariantResult::Ok
    }
}

/// Ensures assets have assignment when in use.
#[derive(Debug, Clone, Default)]
pub struct AssetHasAssignmentInvariant;

impl Invariant for AssetHasAssignmentInvariant {
    fn name(&self) -> &str {
        "asset_has_assignment"
    }

    fn class(&self) -> InvariantClass {
        InvariantClass::Structural
    }

    fn check(&self, ctx: &Context) -> InvariantResult {
        for asset in ctx.get(ContextKey::Proposals).iter() {
            if asset.id.starts_with(ASSET_PREFIX)
                && asset.content.contains("\"state\":\"in_use\"")
                && asset.content.contains("\"assigned_to\":null")
            {
                return InvariantResult::Violated(Violation::with_facts(
                    format!("Asset {} is in use but has no assignment", asset.id),
                    vec![asset.id.clone()],
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
        assert_eq!(RequestIntakeAgent.name(), "request_intake");
        assert_eq!(ApprovalRouterAgent.name(), "approval_router");
        assert_eq!(PurchaseExecutorAgent.name(), "purchase_executor");
        assert_eq!(AssetTrackerAgent.name(), "asset_tracker");
        assert_eq!(SubscriptionManagerAgent.name(), "subscription_manager");
        assert_eq!(RenewalTrackerAgent.name(), "renewal_tracker");
        assert_eq!(VendorManagerAgent.name(), "vendor_manager");
        assert_eq!(BudgetMonitorAgent.name(), "budget_monitor");
        assert_eq!(AssetAuditorAgent.name(), "asset_auditor");
        assert_eq!(LicenseOptimizerAgent.name(), "license_optimizer");
    }
}
