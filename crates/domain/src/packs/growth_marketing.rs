//! Growth marketing pack — Campaigns, channels, attribution.
//!
//! Fact prefixes: `campaign:`, `channel:`, `content:`, `experiment:`,
//! `audience:`, `attribution:`, `budget:`, `performance:`

use crate::pack::{AgentMeta, ContextKey, InvariantClass, InvariantMeta};

pub const AGENTS: &[AgentMeta] = &[
    AgentMeta {
        name: "campaign_planner",
        dependencies: &[ContextKey::Seeds],
        fact_prefix: "campaign:",
        target_key: ContextKey::Proposals,
        description: "Creates campaigns",
    },
    AgentMeta {
        name: "channel_connector",
        dependencies: &[ContextKey::Seeds],
        fact_prefix: "channel:",
        target_key: ContextKey::Signals,
        description: "Platform integrations",
    },
    AgentMeta {
        name: "budget_allocator",
        dependencies: &[ContextKey::Proposals],
        fact_prefix: "budget:",
        target_key: ContextKey::Proposals,
        description: "Allocates budget",
    },
    AgentMeta {
        name: "content_scheduler",
        dependencies: &[ContextKey::Proposals],
        fact_prefix: "content:",
        target_key: ContextKey::Proposals,
        description: "Publishes content",
    },
    AgentMeta {
        name: "experiment_runner",
        dependencies: &[ContextKey::Proposals],
        fact_prefix: "experiment:",
        target_key: ContextKey::Proposals,
        description: "Runs A/B experiments",
    },
    AgentMeta {
        name: "performance_tracker",
        dependencies: &[ContextKey::Proposals],
        fact_prefix: "performance:",
        target_key: ContextKey::Evaluations,
        description: "Collects metrics",
    },
    AgentMeta {
        name: "attribution_analyzer",
        dependencies: &[ContextKey::Evaluations],
        fact_prefix: "attribution:",
        target_key: ContextKey::Evaluations,
        description: "Multi-touch attribution",
    },
    AgentMeta {
        name: "spend_guardian",
        dependencies: &[ContextKey::Proposals],
        fact_prefix: "budget:",
        target_key: ContextKey::Evaluations,
        description: "Budget guardrails enforcement",
    },
    AgentMeta {
        name: "audience_segmenter",
        dependencies: &[ContextKey::Signals],
        fact_prefix: "audience:",
        target_key: ContextKey::Proposals,
        description: "Creates audience segments",
    },
    AgentMeta {
        name: "campaign_optimizer",
        dependencies: &[ContextKey::Evaluations],
        fact_prefix: "campaign:",
        target_key: ContextKey::Proposals,
        description: "Optimization recommendations",
    },
];

pub const INVARIANTS: &[InvariantMeta] = &[
    InvariantMeta {
        name: "campaign_has_hypothesis",
        class: InvariantClass::Structural,
        description: "Campaigns must have a hypothesis",
    },
    InvariantMeta {
        name: "no_spend_without_goal",
        class: InvariantClass::Structural,
        description: "No spend without a goal",
    },
    InvariantMeta {
        name: "experiment_has_metrics",
        class: InvariantClass::Structural,
        description: "Experiments must have success metrics",
    },
    InvariantMeta {
        name: "budget_guardrails_enforced",
        class: InvariantClass::Semantic,
        description: "Budget guardrails enforced",
    },
];

pub const PROFILE: crate::pack::PackProfile = crate::pack::PackProfile {
    entities: &["campaign", "channel", "audience", "attribution", "content"],
    required_capabilities: &["web", "social"],
    uses_llm: false,
    requires_hitl: false,
    handles_irreversible: false,
    keywords: &[
        "campaign",
        "marketing",
        "attribution",
        "audience",
        "channel",
        "content",
        "experiment",
    ],
};
