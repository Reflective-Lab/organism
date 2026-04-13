//! Product engineering pack — Roadmaps, features, releases, incidents.
//!
//! Fact prefixes: `initiative:`, `feature:`, `task:`, `release:`,
//! `incident:`, `experiment:`, `tech_debt:`, `postmortem:`

use crate::pack::{AgentMeta, ContextKey, InvariantClass, InvariantMeta};

pub const AGENTS: &[AgentMeta] = &[
    AgentMeta {
        name: "roadmap_planner",
        dependencies: &[ContextKey::Seeds],
        fact_prefix: "initiative:",
        target_key: ContextKey::Proposals,
        description: "Plans roadmap from strategy",
    },
    AgentMeta {
        name: "feature_specifier",
        dependencies: &[ContextKey::Proposals],
        fact_prefix: "feature:",
        target_key: ContextKey::Evaluations,
        description: "Writes specifications",
    },
    AgentMeta {
        name: "task_decomposer",
        dependencies: &[ContextKey::Evaluations],
        fact_prefix: "task:",
        target_key: ContextKey::Proposals,
        description: "Breaks into tasks",
    },
    AgentMeta {
        name: "release_coordinator",
        dependencies: &[ContextKey::Proposals],
        fact_prefix: "release:",
        target_key: ContextKey::Proposals,
        description: "Orchestrates releases",
    },
    AgentMeta {
        name: "canary_analyzer",
        dependencies: &[ContextKey::Proposals],
        fact_prefix: "release:",
        target_key: ContextKey::Evaluations,
        description: "Monitors canary deployments",
    },
    AgentMeta {
        name: "incident_responder",
        dependencies: &[ContextKey::Seeds],
        fact_prefix: "incident:",
        target_key: ContextKey::Proposals,
        description: "Incident response",
    },
    AgentMeta {
        name: "postmortem_facilitator",
        dependencies: &[ContextKey::Proposals],
        fact_prefix: "postmortem:",
        target_key: ContextKey::Proposals,
        description: "Blameless postmortems",
    },
    AgentMeta {
        name: "experiment_designer",
        dependencies: &[ContextKey::Seeds],
        fact_prefix: "experiment:",
        target_key: ContextKey::Proposals,
        description: "Product experiment design",
    },
    AgentMeta {
        name: "metrics_observer",
        dependencies: &[ContextKey::Proposals],
        fact_prefix: "feature:",
        target_key: ContextKey::Evaluations,
        description: "Monitors product metrics",
    },
    AgentMeta {
        name: "tech_debt_tracker",
        dependencies: &[ContextKey::Evaluations],
        fact_prefix: "tech_debt:",
        target_key: ContextKey::Proposals,
        description: "Prioritizes tech debt",
    },
];

pub const INVARIANTS: &[InvariantMeta] = &[
    InvariantMeta {
        name: "feature_has_owner",
        class: InvariantClass::Structural,
        description: "Features must have an owner",
    },
    InvariantMeta {
        name: "release_has_rollback_plan",
        class: InvariantClass::Structural,
        description: "Releases must have rollback plan",
    },
    InvariantMeta {
        name: "incident_has_severity",
        class: InvariantClass::Structural,
        description: "Incidents must have severity",
    },
    InvariantMeta {
        name: "shipped_feature_has_metrics",
        class: InvariantClass::Semantic,
        description: "Shipped features must have metrics",
    },
];

pub const PROFILE: crate::pack::PackProfile = crate::pack::PackProfile {
    entities: &[
        "feature",
        "release",
        "incident",
        "task",
        "initiative",
        "tech_debt",
    ],
    required_capabilities: &[],
    uses_llm: false,
    requires_hitl: false,
    handles_irreversible: false,
    keywords: &[
        "roadmap",
        "feature",
        "release",
        "incident",
        "deploy",
        "sprint",
        "engineering",
    ],
};
