//! Customers pack — Revenue operations.
//!
//! Lifecycle: Lead → Enrich → Score → Route → Propose → Close → Handoff
//!
//! Fact prefixes: `lead:`, `opportunity:`, `proposal:`, `deal:`,
//! `handoff:`, `sequence:`

use crate::pack::{AgentMeta, ContextKey, InvariantClass, InvariantMeta};

pub const AGENTS: &[AgentMeta] = &[
    AgentMeta {
        name: "lead_enrichment",
        dependencies: &[ContextKey::Seeds],
        fact_prefix: "lead:",
        target_key: ContextKey::Signals,
        description: "Enriches leads with company/contact data",
    },
    AgentMeta {
        name: "lead_scorer",
        dependencies: &[ContextKey::Signals],
        fact_prefix: "lead:",
        target_key: ContextKey::Evaluations,
        description: "ICP fit scoring",
    },
    AgentMeta {
        name: "lead_router",
        dependencies: &[ContextKey::Evaluations],
        fact_prefix: "lead:",
        target_key: ContextKey::Proposals,
        description: "Routes leads to sales owners",
    },
    AgentMeta {
        name: "sequence_selector",
        dependencies: &[ContextKey::Evaluations],
        fact_prefix: "sequence:",
        target_key: ContextKey::Proposals,
        description: "Selects outreach sequences",
    },
    AgentMeta {
        name: "proposal_generator",
        dependencies: &[ContextKey::Proposals],
        fact_prefix: "proposal:",
        target_key: ContextKey::Proposals,
        description: "Creates proposals from opportunities",
    },
    AgentMeta {
        name: "deal_closer",
        dependencies: &[ContextKey::Proposals],
        fact_prefix: "deal:",
        target_key: ContextKey::Proposals,
        description: "Closes deals from signed contracts",
    },
    AgentMeta {
        name: "handoff_scheduler",
        dependencies: &[ContextKey::Proposals],
        fact_prefix: "handoff:",
        target_key: ContextKey::Proposals,
        description: "Schedules CSM handoffs for closed deals",
    },
    AgentMeta {
        name: "stale_opportunity_detector",
        dependencies: &[ContextKey::Proposals],
        fact_prefix: "opportunity:",
        target_key: ContextKey::Evaluations,
        description: "Flags inactive opportunities",
    },
];

pub const INVARIANTS: &[InvariantMeta] = &[
    InvariantMeta {
        name: "lead_has_source",
        class: InvariantClass::Structural,
        description: "Every lead must have a source",
    },
    InvariantMeta {
        name: "closed_won_triggers_handoff",
        class: InvariantClass::Acceptance,
        description: "Closed-won deals must trigger CSM handoff",
    },
];

pub const PROFILE: crate::pack::PackProfile = crate::pack::PackProfile {
    entities: &["lead", "opportunity", "deal", "proposal", "handoff"],
    required_capabilities: &["web", "social"],
    uses_llm: false,
    requires_hitl: false,
    handles_irreversible: false,
    keywords: &[
        "revenue",
        "sales",
        "pipeline",
        "icp",
        "scoring",
        "qualification",
        "outreach",
    ],
};
