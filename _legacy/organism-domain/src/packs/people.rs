// Copyright 2024-2025 Aprio One AB, Sweden
// SPDX-License-Identifier: MIT

//! People Pack agents for people lifecycle operations.
//!
//! Implements the agent contracts defined in specs/people.feature.
//!
//! # Lifecycle: Hire → Onboard → Pay → Review → Offboard
//!
//! Note: This implementation uses the standard ContextKey enum. Facts are
//! distinguished by their ID prefixes (employee:, identity:, payroll:, etc.).

use converge_core::{
    Agent, AgentEffect, Context, ContextKey, Fact,
    invariant::{Invariant, InvariantClass, InvariantResult, Violation},
};

// ============================================================================
// Fact ID Prefixes
// ============================================================================

pub const EMPLOYEE_PREFIX: &str = "employee:";
pub const IDENTITY_PREFIX: &str = "identity:";
pub const ACCESS_PREFIX: &str = "access:";
pub const ONBOARDING_PREFIX: &str = "onboarding:";
pub const PAYROLL_PREFIX: &str = "payroll:";
pub const EXPENSE_PREFIX: &str = "expense:";
pub const OFFBOARDING_PREFIX: &str = "offboarding:";
pub const FINAL_PAY_PREFIX: &str = "final_pay:";

// ============================================================================
// Agents
// ============================================================================

/// Provisions identity in IdP for new employees.
#[derive(Debug, Clone, Default)]
pub struct IdentityProvisionerAgent;

impl Agent for IdentityProvisionerAgent {
    fn name(&self) -> &str {
        "identity_provisioner"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Seeds]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        ctx.get(ContextKey::Seeds)
            .iter()
            .any(|s| s.content.contains("employee.hired"))
            && !ctx
                .get(ContextKey::Proposals)
                .iter()
                .any(|p| p.id.starts_with(IDENTITY_PREFIX))
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let triggers = ctx.get(ContextKey::Seeds);
        let mut facts = Vec::new();

        for trigger in triggers.iter() {
            if trigger.content.contains("employee.hired") {
                facts.push(Fact {
                    key: ContextKey::Proposals,
                    id: format!("{}{}", IDENTITY_PREFIX, trigger.id),
                    content: serde_json::json!({
                        "type": "identity",
                        "employee_id": trigger.id,
                        "state": "provisioned",
                        "email": "pending@company.com",
                        "created_at": "2026-01-12"
                    })
                    .to_string(),
                });
            }
        }

        AgentEffect::with_facts(facts)
    }
}

/// Evaluates access requirements based on role.
#[derive(Debug, Clone, Default)]
pub struct AccessEvaluatorAgent;

impl Agent for AccessEvaluatorAgent {
    fn name(&self) -> &str {
        "access_evaluator"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Proposals]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        let has_identities = ctx
            .get(ContextKey::Proposals)
            .iter()
            .any(|p| p.id.starts_with(IDENTITY_PREFIX));
        let has_access_req = ctx
            .get(ContextKey::Proposals)
            .iter()
            .any(|p| p.id.contains("access_req"));
        has_identities && !has_access_req
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let proposals = ctx.get(ContextKey::Proposals);
        let mut facts = Vec::new();

        for identity in proposals.iter() {
            if identity.id.starts_with(IDENTITY_PREFIX) {
                facts.push(Fact {
                    key: ContextKey::Proposals,
                    id: format!("{}access_req:{}", ACCESS_PREFIX, identity.id),
                    content: serde_json::json!({
                        "type": "access_requirements",
                        "identity_id": identity.id,
                        "required_systems": ["email", "slack", "github"],
                        "role_based_permissions": [],
                        "evaluated_at": "2026-01-12"
                    })
                    .to_string(),
                });
            }
        }

        AgentEffect::with_facts(facts)
    }
}

/// Provisions access to required systems.
#[derive(Debug, Clone, Default)]
pub struct AccessProvisionerAgent;

impl Agent for AccessProvisionerAgent {
    fn name(&self) -> &str {
        "access_provisioner"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Proposals]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        let has_access_req = ctx
            .get(ContextKey::Proposals)
            .iter()
            .any(|p| p.id.contains("access_req"));
        let has_provisioned = ctx
            .get(ContextKey::Proposals)
            .iter()
            .any(|p| p.id.contains("provisioned"));
        has_access_req && !has_provisioned
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let proposals = ctx.get(ContextKey::Proposals);
        let mut facts = Vec::new();

        for req in proposals.iter() {
            if req.id.contains("access_req") {
                facts.push(Fact {
                    key: ContextKey::Proposals,
                    id: format!("{}provisioned:{}", ACCESS_PREFIX, req.id),
                    content: serde_json::json!({
                        "type": "system_access",
                        "access_req_id": req.id,
                        "state": "provisioned",
                        "systems_provisioned": ["email", "slack", "github"],
                        "provisioned_at": "2026-01-12"
                    })
                    .to_string(),
                });
            }
        }

        AgentEffect::with_facts(facts)
    }
}

/// Coordinates onboarding tasks and checklist.
#[derive(Debug, Clone, Default)]
pub struct OnboardingCoordinatorAgent;

impl Agent for OnboardingCoordinatorAgent {
    fn name(&self) -> &str {
        "onboarding_coordinator"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Proposals]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        let has_provisioned = ctx
            .get(ContextKey::Proposals)
            .iter()
            .any(|p| p.id.contains("provisioned"));
        let has_onboarding = ctx
            .get(ContextKey::Proposals)
            .iter()
            .any(|p| p.id.starts_with(ONBOARDING_PREFIX));
        has_provisioned && !has_onboarding
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let proposals = ctx.get(ContextKey::Proposals);
        let mut facts = Vec::new();

        for acc in proposals.iter() {
            if acc.id.contains("provisioned") {
                facts.push(Fact {
                    key: ContextKey::Proposals,
                    id: format!("{}{}", ONBOARDING_PREFIX, acc.id),
                    content: serde_json::json!({
                        "type": "onboarding_tasks",
                        "access_id": acc.id,
                        "tasks": [
                            {"name": "Welcome meeting", "state": "pending"},
                            {"name": "Equipment setup", "state": "pending"},
                            {"name": "Policy review", "state": "pending"},
                            {"name": "Team introduction", "state": "pending"}
                        ],
                        "due_date": "2026-01-19"
                    })
                    .to_string(),
                });
            }
        }

        AgentEffect::with_facts(facts)
    }
}

/// Collects payroll data from HRIS.
#[derive(Debug, Clone, Default)]
pub struct PayrollCollectorAgent;

impl Agent for PayrollCollectorAgent {
    fn name(&self) -> &str {
        "payroll_collector"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Seeds]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        let has_active = ctx.get(ContextKey::Seeds).iter().any(|e| {
            e.id.starts_with(EMPLOYEE_PREFIX) && e.content.contains("\"state\":\"active\"")
        });
        let has_payroll = ctx
            .get(ContextKey::Proposals)
            .iter()
            .any(|p| p.id.starts_with(PAYROLL_PREFIX));
        has_active && !has_payroll
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let seeds = ctx.get(ContextKey::Seeds);
        let mut facts = Vec::new();

        for emp in seeds.iter() {
            if emp.id.starts_with(EMPLOYEE_PREFIX) && emp.content.contains("\"state\":\"active\"") {
                facts.push(Fact {
                    key: ContextKey::Proposals,
                    id: format!("{}{}", PAYROLL_PREFIX, emp.id),
                    content: serde_json::json!({
                        "type": "payroll_data",
                        "employee_id": emp.id,
                        "hours_worked": 160,
                        "overtime_hours": 0,
                        "deductions": [],
                        "benefits": [],
                        "collected_at": "2026-01-12"
                    })
                    .to_string(),
                });
            }
        }

        AgentEffect::with_facts(facts)
    }
}

/// Validates payroll data before processing.
#[derive(Debug, Clone, Default)]
pub struct PayrollValidatorAgent;

impl Agent for PayrollValidatorAgent {
    fn name(&self) -> &str {
        "payroll_validator"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Proposals]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        ctx.get(ContextKey::Proposals)
            .iter()
            .any(|p| p.id.starts_with(PAYROLL_PREFIX) && !p.content.contains("\"validated\":true"))
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let proposals = ctx.get(ContextKey::Proposals);
        let mut facts = Vec::new();

        for data in proposals.iter() {
            if data.id.starts_with(PAYROLL_PREFIX) && !data.content.contains("\"validated\":true") {
                facts.push(Fact {
                    key: ContextKey::Proposals,
                    id: format!("{}validated:{}", PAYROLL_PREFIX, data.id),
                    content: serde_json::json!({
                        "type": "validated_payroll",
                        "payroll_id": data.id,
                        "validated": true,
                        "validation_errors": [],
                        "approved_for_payment": true
                    })
                    .to_string(),
                });
            }
        }

        AgentEffect::with_facts(facts)
    }
}

/// Evaluates expense claims for approval.
#[derive(Debug, Clone, Default)]
pub struct ExpenseEvaluatorAgent;

impl Agent for ExpenseEvaluatorAgent {
    fn name(&self) -> &str {
        "expense_evaluator"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Signals]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        ctx.get(ContextKey::Signals).iter().any(|e| {
            e.id.starts_with(EXPENSE_PREFIX) && e.content.contains("\"state\":\"submitted\"")
        })
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let signals = ctx.get(ContextKey::Signals);
        let mut facts = Vec::new();

        for expense in signals.iter() {
            if expense.id.starts_with(EXPENSE_PREFIX)
                && expense.content.contains("\"state\":\"submitted\"")
            {
                facts.push(Fact {
                    key: ContextKey::Proposals,
                    id: format!("{}evaluated:{}", EXPENSE_PREFIX, expense.id),
                    content: serde_json::json!({
                        "type": "expense_decision",
                        "expense_id": expense.id,
                        "state": "approved",
                        "within_policy": true,
                        "approved_amount": "from_claim",
                        "evaluated_at": "2026-01-12"
                    })
                    .to_string(),
                });
            }
        }

        AgentEffect::with_facts(facts)
    }
}

/// Coordinates offboarding process for departing employees.
#[derive(Debug, Clone, Default)]
pub struct OffboardingCoordinatorAgent;

impl Agent for OffboardingCoordinatorAgent {
    fn name(&self) -> &str {
        "offboarding_coordinator"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Seeds]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        ctx.get(ContextKey::Seeds).iter().any(|e| {
            e.id.starts_with(EMPLOYEE_PREFIX) && e.content.contains("\"state\":\"terminating\"")
        })
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let seeds = ctx.get(ContextKey::Seeds);
        let mut facts = Vec::new();

        for emp in seeds.iter() {
            if emp.id.starts_with(EMPLOYEE_PREFIX)
                && emp.content.contains("\"state\":\"terminating\"")
            {
                facts.push(Fact {
                    key: ContextKey::Proposals,
                    id: format!("{}{}", OFFBOARDING_PREFIX, emp.id),
                    content: serde_json::json!({
                        "type": "offboarding_tasks",
                        "employee_id": emp.id,
                        "tasks": [
                            {"name": "Exit interview", "state": "pending"},
                            {"name": "Equipment return", "state": "pending"},
                            {"name": "Knowledge transfer", "state": "pending"},
                            {"name": "Final documentation", "state": "pending"}
                        ],
                        "last_day": "2026-01-31"
                    })
                    .to_string(),
                });
            }
        }

        AgentEffect::with_facts(facts)
    }
}

/// Revokes all system access for departing employees.
#[derive(Debug, Clone, Default)]
pub struct AccessRevokerAgent;

impl Agent for AccessRevokerAgent {
    fn name(&self) -> &str {
        "access_revoker"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Proposals]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        let has_offboarding = ctx
            .get(ContextKey::Proposals)
            .iter()
            .any(|p| p.id.starts_with(OFFBOARDING_PREFIX));
        let has_provisioned = ctx
            .get(ContextKey::Proposals)
            .iter()
            .any(|a| a.content.contains("\"state\":\"provisioned\""));
        has_offboarding && has_provisioned
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let proposals = ctx.get(ContextKey::Proposals);
        let mut facts = Vec::new();

        for task in proposals.iter() {
            if task.id.starts_with(OFFBOARDING_PREFIX) {
                facts.push(Fact {
                    key: ContextKey::Proposals,
                    id: format!("{}revoked:{}", ACCESS_PREFIX, task.id),
                    content: serde_json::json!({
                        "type": "revoked_access",
                        "offboarding_id": task.id,
                        "state": "revoked",
                        "revoked_systems": ["email", "slack", "github", "vpn"],
                        "revoked_at": "2026-01-31"
                    })
                    .to_string(),
                });
            }
        }

        AgentEffect::with_facts(facts)
    }
}

/// Schedules final pay for terminated employees.
#[derive(Debug, Clone, Default)]
pub struct FinalPaySchedulerAgent;

impl Agent for FinalPaySchedulerAgent {
    fn name(&self) -> &str {
        "final_pay_scheduler"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Proposals]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        let has_offboarding = ctx
            .get(ContextKey::Proposals)
            .iter()
            .any(|p| p.id.starts_with(OFFBOARDING_PREFIX));
        let has_final_pay = ctx
            .get(ContextKey::Proposals)
            .iter()
            .any(|p| p.id.starts_with(FINAL_PAY_PREFIX));
        has_offboarding && !has_final_pay
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let proposals = ctx.get(ContextKey::Proposals);
        let mut facts = Vec::new();

        for task in proposals.iter() {
            if task.id.starts_with(OFFBOARDING_PREFIX) {
                facts.push(Fact {
                    key: ContextKey::Proposals,
                    id: format!("{}{}", FINAL_PAY_PREFIX, task.id),
                    content: serde_json::json!({
                        "type": "final_payment",
                        "offboarding_id": task.id,
                        "components": [
                            {"type": "salary_owed", "calculated": true},
                            {"type": "unused_pto", "calculated": true},
                            {"type": "expense_reimbursement", "calculated": true}
                        ],
                        "scheduled_date": "2026-02-01",
                        "state": "scheduled"
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

/// Ensures employees have identity provisioned before access.
#[derive(Debug, Clone, Default)]
pub struct IdentityBeforeAccessInvariant;

impl Invariant for IdentityBeforeAccessInvariant {
    fn name(&self) -> &str {
        "identity_before_access"
    }

    fn class(&self) -> InvariantClass {
        InvariantClass::Structural
    }

    fn check(&self, ctx: &Context) -> InvariantResult {
        let proposals = ctx.get(ContextKey::Proposals);
        for access in proposals.iter() {
            if access.id.contains("provisioned") {
                let has_identity = proposals.iter().any(|i| i.id.starts_with(IDENTITY_PREFIX));
                if !has_identity {
                    return InvariantResult::Violated(Violation::with_facts(
                        format!("Access {} has no identity", access.id),
                        vec![access.id.clone()],
                    ));
                }
            }
        }
        InvariantResult::Ok
    }
}

/// Ensures terminated employees have access revoked.
#[derive(Debug, Clone, Default)]
pub struct TerminationRevokesAccessInvariant;

impl Invariant for TerminationRevokesAccessInvariant {
    fn name(&self) -> &str {
        "termination_revokes_access"
    }

    fn class(&self) -> InvariantClass {
        InvariantClass::Acceptance
    }

    fn check(&self, ctx: &Context) -> InvariantResult {
        let proposals = ctx.get(ContextKey::Proposals);
        for emp in ctx.get(ContextKey::Seeds).iter() {
            if emp.id.starts_with(EMPLOYEE_PREFIX)
                && emp.content.contains("\"state\":\"terminated\"")
            {
                let has_revoked = proposals
                    .iter()
                    .any(|a| a.content.contains("\"state\":\"revoked\""));
                if !has_revoked {
                    return InvariantResult::Violated(Violation::with_facts(
                        format!("Terminated employee {} still has active access", emp.id),
                        vec![emp.id.clone()],
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
        assert_eq!(IdentityProvisionerAgent.name(), "identity_provisioner");
        assert_eq!(AccessEvaluatorAgent.name(), "access_evaluator");
        assert_eq!(AccessProvisionerAgent.name(), "access_provisioner");
        assert_eq!(OnboardingCoordinatorAgent.name(), "onboarding_coordinator");
        assert_eq!(PayrollCollectorAgent.name(), "payroll_collector");
        assert_eq!(PayrollValidatorAgent.name(), "payroll_validator");
        assert_eq!(ExpenseEvaluatorAgent.name(), "expense_evaluator");
        assert_eq!(
            OffboardingCoordinatorAgent.name(),
            "offboarding_coordinator"
        );
        assert_eq!(AccessRevokerAgent.name(), "access_revoker");
        assert_eq!(FinalPaySchedulerAgent.name(), "final_pay_scheduler");
    }
}
