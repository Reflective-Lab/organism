//! Virtual teams pack — Team formation, personas, content publishing.
//!
//! Fact prefixes: `team:`, `persona:`, `content_draft:`, `channel:`, `publish:`

use crate::pack::{AgentMeta, ContextKey, InvariantClass, InvariantMeta};

pub const AGENTS: &[AgentMeta] = &[
    AgentMeta {
        name: "team_formation",
        dependencies: &[ContextKey::Seeds],
        fact_prefix: "team:",
        target_key: ContextKey::Proposals,
        description: "Creates teams",
    },
    AgentMeta {
        name: "team_lifecycle",
        dependencies: &[ContextKey::Proposals],
        fact_prefix: "team:",
        target_key: ContextKey::Evaluations,
        description: "Manages team states",
    },
    AgentMeta {
        name: "persona_creator",
        dependencies: &[ContextKey::Seeds],
        fact_prefix: "persona:",
        target_key: ContextKey::Proposals,
        description: "Drafts personas with guardrails",
    },
    AgentMeta {
        name: "persona_reviewer",
        dependencies: &[ContextKey::Proposals],
        fact_prefix: "persona:",
        target_key: ContextKey::Evaluations,
        description: "Brand/compliance review",
    },
    AgentMeta {
        name: "content_draft_creator",
        dependencies: &[ContextKey::Proposals],
        fact_prefix: "content_draft:",
        target_key: ContextKey::Proposals,
        description: "AI content generation",
    },
    AgentMeta {
        name: "content_reviewer",
        dependencies: &[ContextKey::Proposals],
        fact_prefix: "content_draft:",
        target_key: ContextKey::Evaluations,
        description: "Human review of drafts",
    },
    AgentMeta {
        name: "publish_approval",
        dependencies: &[ContextKey::Evaluations],
        fact_prefix: "publish:",
        target_key: ContextKey::Proposals,
        description: "Approval chain",
    },
    AgentMeta {
        name: "channel_manager",
        dependencies: &[ContextKey::Seeds],
        fact_prefix: "channel:",
        target_key: ContextKey::Proposals,
        description: "Channel permissions",
    },
    AgentMeta {
        name: "agent_audit",
        dependencies: &[ContextKey::Proposals],
        fact_prefix: "team:",
        target_key: ContextKey::Evaluations,
        description: "Audits agent actions",
    },
];

pub const INVARIANTS: &[InvariantMeta] = &[
    InvariantMeta {
        name: "agents_unsafe_by_default",
        class: InvariantClass::Acceptance,
        description: "Agents are unsafe by default",
    },
    InvariantMeta {
        name: "agent_actions_auditable",
        class: InvariantClass::Structural,
        description: "Agent actions must be auditable",
    },
    InvariantMeta {
        name: "persona_has_guardrails",
        class: InvariantClass::Structural,
        description: "Personas must have guardrails",
    },
    InvariantMeta {
        name: "persona_has_owner",
        class: InvariantClass::Structural,
        description: "Personas must have owner",
    },
    InvariantMeta {
        name: "external_post_provenance",
        class: InvariantClass::Structural,
        description: "External posts need provenance",
    },
    InvariantMeta {
        name: "team_has_charter",
        class: InvariantClass::Acceptance,
        description: "Teams must have charter",
    },
    InvariantMeta {
        name: "team_has_owner",
        class: InvariantClass::Structural,
        description: "Teams must have owner",
    },
    InvariantMeta {
        name: "external_channel_requires_approval",
        class: InvariantClass::Acceptance,
        description: "External channels require approval",
    },
];

pub const PROFILE: crate::pack::PackProfile = crate::pack::PackProfile {
    entities: &["team", "persona", "content_draft", "channel", "publish"],
    required_capabilities: &[],
    uses_llm: true,
    requires_hitl: true,
    handles_irreversible: false,
    keywords: &[
        "team", "persona", "content", "publish", "channel", "brand", "agent",
    ],
};
