//! Procurement pack — Purchase requests, assets, subscriptions.
//!
//! Fact prefixes: `request:`, `approval:`, `order:`, `asset:`,
//! `subscription:`, `vendor:`, `renewal:`, `budget:`

use crate::pack::{AgentMeta, ContextKey, InvariantClass, InvariantMeta};

pub const AGENTS: &[AgentMeta] = &[
    AgentMeta {
        name: "request_intake",
        dependencies: &[ContextKey::Seeds],
        fact_prefix: "request:",
        target_key: ContextKey::Proposals,
        description: "Intakes purchase requests",
    },
    AgentMeta {
        name: "approval_router",
        dependencies: &[ContextKey::Proposals],
        fact_prefix: "approval:",
        target_key: ContextKey::Proposals,
        description: "Routes to approvers",
    },
    AgentMeta {
        name: "purchase_executor",
        dependencies: &[ContextKey::Proposals],
        fact_prefix: "order:",
        target_key: ContextKey::Proposals,
        description: "Creates purchase orders",
    },
    AgentMeta {
        name: "asset_tracker",
        dependencies: &[ContextKey::Proposals],
        fact_prefix: "asset:",
        target_key: ContextKey::Proposals,
        description: "Registers received assets",
    },
    AgentMeta {
        name: "subscription_manager",
        dependencies: &[ContextKey::Proposals],
        fact_prefix: "subscription:",
        target_key: ContextKey::Evaluations,
        description: "Manages SaaS subscriptions",
    },
    AgentMeta {
        name: "renewal_tracker",
        dependencies: &[ContextKey::Proposals],
        fact_prefix: "renewal:",
        target_key: ContextKey::Signals,
        description: "Tracks renewal dates",
    },
    AgentMeta {
        name: "vendor_manager",
        dependencies: &[ContextKey::Seeds],
        fact_prefix: "vendor:",
        target_key: ContextKey::Proposals,
        description: "Vendor onboarding",
    },
    AgentMeta {
        name: "budget_monitor",
        dependencies: &[ContextKey::Proposals],
        fact_prefix: "budget:",
        target_key: ContextKey::Evaluations,
        description: "Checks budget",
    },
    AgentMeta {
        name: "asset_auditor",
        dependencies: &[ContextKey::Proposals],
        fact_prefix: "asset:",
        target_key: ContextKey::Evaluations,
        description: "Audits assets",
    },
    AgentMeta {
        name: "license_optimizer",
        dependencies: &[ContextKey::Evaluations],
        fact_prefix: "subscription:",
        target_key: ContextKey::Proposals,
        description: "Optimization recommendations",
    },
];

pub const INVARIANTS: &[InvariantMeta] = &[
    InvariantMeta {
        name: "spend_needs_owner",
        class: InvariantClass::Structural,
        description: "Spend must have an owner",
    },
    InvariantMeta {
        name: "spend_needs_budget",
        class: InvariantClass::Structural,
        description: "Spend must have budget",
    },
    InvariantMeta {
        name: "renewals_not_missed",
        class: InvariantClass::Semantic,
        description: "Renewals must not be missed",
    },
    InvariantMeta {
        name: "asset_has_assignment",
        class: InvariantClass::Structural,
        description: "Assets must have assignment",
    },
];

pub const PROFILE: crate::pack::PackProfile = crate::pack::PackProfile {
    entities: &[
        "request",
        "order",
        "asset",
        "subscription",
        "vendor",
        "renewal",
    ],
    required_capabilities: &[],
    uses_llm: false,
    requires_hitl: false,
    handles_irreversible: false,
    keywords: &[
        "purchase",
        "procurement",
        "asset",
        "subscription",
        "license",
        "renewal",
        "expense",
    ],
};
