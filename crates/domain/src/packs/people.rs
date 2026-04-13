//! People pack — Employee lifecycle.
//!
//! Lifecycle: Hire → Identity → Access → Onboard → Pay → Expense → Offboard
//!
//! Fact prefixes: `employee:`, `identity:`, `access:`, `onboarding:`,
//! `payroll:`, `expense:`, `offboarding:`, `final_pay:`

use crate::pack::{AgentMeta, ContextKey, InvariantClass, InvariantMeta};

pub const AGENTS: &[AgentMeta] = &[
    AgentMeta {
        name: "identity_provisioner",
        dependencies: &[ContextKey::Seeds],
        fact_prefix: "identity:",
        target_key: ContextKey::Proposals,
        description: "IdP provisioning",
    },
    AgentMeta {
        name: "access_evaluator",
        dependencies: &[ContextKey::Proposals],
        fact_prefix: "access:",
        target_key: ContextKey::Proposals,
        description: "Access requirements by role",
    },
    AgentMeta {
        name: "access_provisioner",
        dependencies: &[ContextKey::Proposals],
        fact_prefix: "access:",
        target_key: ContextKey::Proposals,
        description: "System access provisioning",
    },
    AgentMeta {
        name: "onboarding_coordinator",
        dependencies: &[ContextKey::Proposals],
        fact_prefix: "onboarding:",
        target_key: ContextKey::Proposals,
        description: "Onboarding checklist",
    },
    AgentMeta {
        name: "payroll_collector",
        dependencies: &[ContextKey::Seeds],
        fact_prefix: "payroll:",
        target_key: ContextKey::Proposals,
        description: "Payroll data collection",
    },
    AgentMeta {
        name: "payroll_validator",
        dependencies: &[ContextKey::Proposals],
        fact_prefix: "payroll:",
        target_key: ContextKey::Evaluations,
        description: "Validates payroll",
    },
    AgentMeta {
        name: "expense_evaluator",
        dependencies: &[ContextKey::Seeds],
        fact_prefix: "expense:",
        target_key: ContextKey::Evaluations,
        description: "Evaluates expense claims",
    },
    AgentMeta {
        name: "offboarding_coordinator",
        dependencies: &[ContextKey::Seeds],
        fact_prefix: "offboarding:",
        target_key: ContextKey::Proposals,
        description: "Exit tasks",
    },
    AgentMeta {
        name: "access_revoker",
        dependencies: &[ContextKey::Proposals],
        fact_prefix: "access:",
        target_key: ContextKey::Proposals,
        description: "Revokes access on termination",
    },
    AgentMeta {
        name: "final_pay_scheduler",
        dependencies: &[ContextKey::Proposals],
        fact_prefix: "final_pay:",
        target_key: ContextKey::Proposals,
        description: "Final payment processing",
    },
];

pub const INVARIANTS: &[InvariantMeta] = &[
    InvariantMeta {
        name: "identity_before_access",
        class: InvariantClass::Structural,
        description: "Identity must be provisioned before access",
    },
    InvariantMeta {
        name: "termination_revokes_access",
        class: InvariantClass::Acceptance,
        description: "Termination immediately revokes access",
    },
];

pub const PROFILE: crate::pack::PackProfile = crate::pack::PackProfile {
    entities: &["employee", "identity", "access", "payroll", "expense"],
    required_capabilities: &[],
    uses_llm: false,
    requires_hitl: true,
    handles_irreversible: true,
    keywords: &[
        "hire", "onboard", "offboard", "payroll", "employee", "access", "identity", "hr",
    ],
};
