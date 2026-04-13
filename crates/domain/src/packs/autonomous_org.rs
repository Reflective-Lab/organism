//! Autonomous org pack — Governance, policies, budgets, delegations.
//!
//! Fact prefixes: `policy:`, `approval:`, `budget_envelope:`, `exception:`,
//! `delegation:`, `risk_control:`

use crate::pack::{AgentMeta, ContextKey, InvariantClass, InvariantMeta};

pub const AGENTS: &[AgentMeta] = &[
    AgentMeta {
        name: "policy_manager",
        dependencies: &[ContextKey::Seeds],
        fact_prefix: "policy:",
        target_key: ContextKey::Proposals,
        description: "Creates/manages policies",
    },
    AgentMeta {
        name: "policy_enforcer",
        dependencies: &[ContextKey::Proposals],
        fact_prefix: "policy:",
        target_key: ContextKey::Evaluations,
        description: "Enforces active policies",
    },
    AgentMeta {
        name: "approval_router",
        dependencies: &[ContextKey::Proposals],
        fact_prefix: "approval:",
        target_key: ContextKey::Proposals,
        description: "Routes approvals",
    },
    AgentMeta {
        name: "signoff_collector",
        dependencies: &[ContextKey::Proposals],
        fact_prefix: "approval:",
        target_key: ContextKey::Evaluations,
        description: "Collects signoffs",
    },
    AgentMeta {
        name: "budget_envelope_manager",
        dependencies: &[ContextKey::Seeds],
        fact_prefix: "budget_envelope:",
        target_key: ContextKey::Proposals,
        description: "Creates budget envelopes",
    },
    AgentMeta {
        name: "budget_monitor",
        dependencies: &[ContextKey::Proposals],
        fact_prefix: "budget_envelope:",
        target_key: ContextKey::Evaluations,
        description: "Tracks budget consumption",
    },
    AgentMeta {
        name: "spend_validator",
        dependencies: &[ContextKey::Proposals],
        fact_prefix: "budget_envelope:",
        target_key: ContextKey::Evaluations,
        description: "Validates against envelope",
    },
    AgentMeta {
        name: "exception_handler",
        dependencies: &[ContextKey::Evaluations],
        fact_prefix: "exception:",
        target_key: ContextKey::Proposals,
        description: "Manages policy exceptions",
    },
    AgentMeta {
        name: "delegation_manager",
        dependencies: &[ContextKey::Seeds],
        fact_prefix: "delegation:",
        target_key: ContextKey::Proposals,
        description: "Authority delegation",
    },
];

pub const INVARIANTS: &[InvariantMeta] = &[
    InvariantMeta {
        name: "policy_versioning_required",
        class: InvariantClass::Structural,
        description: "Policies must be versioned",
    },
    InvariantMeta {
        name: "policy_has_owner",
        class: InvariantClass::Structural,
        description: "Policies must have an owner",
    },
    InvariantMeta {
        name: "no_self_approval",
        class: InvariantClass::Acceptance,
        description: "No self-approval",
    },
    InvariantMeta {
        name: "two_person_rule_high_risk",
        class: InvariantClass::Acceptance,
        description: "High-risk actions require two-person rule",
    },
    InvariantMeta {
        name: "no_spend_beyond_envelope",
        class: InvariantClass::Acceptance,
        description: "No spending beyond budget envelope",
    },
    InvariantMeta {
        name: "exception_has_expiry",
        class: InvariantClass::Structural,
        description: "Exceptions must have expiry",
    },
    InvariantMeta {
        name: "delegation_has_scope_limits",
        class: InvariantClass::Structural,
        description: "Delegations must have scope limits",
    },
    InvariantMeta {
        name: "approval_has_rationale",
        class: InvariantClass::Acceptance,
        description: "Approvals must have rationale",
    },
];

pub const PROFILE: crate::pack::PackProfile = crate::pack::PackProfile {
    entities: &[
        "policy",
        "approval",
        "budget_envelope",
        "delegation",
        "exception",
    ],
    required_capabilities: &[],
    uses_llm: false,
    requires_hitl: true,
    handles_irreversible: true,
    keywords: &[
        "governance",
        "policy",
        "approval",
        "budget",
        "delegation",
        "authority",
        "spend",
    ],
};
