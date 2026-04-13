//! LinkedIn research pack — Signal extraction, dossier building, outreach.
//!
//! Fact prefixes: `linkedin_signal:`, `linkedin_evidence:`, `linkedin_path:`,
//! `linkedin_dossier:`, `linkedin_outreach:`, `linkedin_approval:`, `linkedin_target:`

use crate::pack::{AgentMeta, ContextKey, InvariantClass, InvariantMeta};

pub const AGENTS: &[AgentMeta] = &[
    AgentMeta {
        name: "signal_ingest",
        dependencies: &[ContextKey::Seeds],
        fact_prefix: "linkedin_signal:",
        target_key: ContextKey::Proposals,
        description: "Captures initial signals",
    },
    AgentMeta {
        name: "evidence_validator",
        dependencies: &[ContextKey::Proposals],
        fact_prefix: "linkedin_evidence:",
        target_key: ContextKey::Evaluations,
        description: "Promotes to evidence",
    },
    AgentMeta {
        name: "dossier_builder",
        dependencies: &[ContextKey::Evaluations],
        fact_prefix: "linkedin_dossier:",
        target_key: ContextKey::Strategies,
        description: "Builds research dossiers",
    },
    AgentMeta {
        name: "path_verifier",
        dependencies: &[ContextKey::Evaluations],
        fact_prefix: "linkedin_path:",
        target_key: ContextKey::Strategies,
        description: "Verifies network paths",
    },
    AgentMeta {
        name: "target_discovery",
        dependencies: &[ContextKey::Seeds],
        fact_prefix: "linkedin_target:",
        target_key: ContextKey::Proposals,
        description: "Discovers targets via LinkedIn",
    },
    AgentMeta {
        name: "approval_recorder",
        dependencies: &[ContextKey::Strategies],
        fact_prefix: "linkedin_approval:",
        target_key: ContextKey::Constraints,
        description: "Records explicit approvals",
    },
];

pub const INVARIANTS: &[InvariantMeta] = &[
    InvariantMeta {
        name: "evidence_requires_provenance",
        class: InvariantClass::Structural,
        description: "Evidence must have provenance",
    },
    InvariantMeta {
        name: "network_path_requires_verification",
        class: InvariantClass::Semantic,
        description: "Network paths require verification",
    },
    InvariantMeta {
        name: "approval_required_for_external_action",
        class: InvariantClass::Acceptance,
        description: "External actions require approval",
    },
];

pub const PROFILE: crate::pack::PackProfile = crate::pack::PackProfile {
    entities: &[
        "linkedin_signal",
        "linkedin_evidence",
        "linkedin_dossier",
        "linkedin_target",
    ],
    required_capabilities: &["linkedin", "web", "social"],
    uses_llm: false,
    requires_hitl: true,
    handles_irreversible: false,
    keywords: &[
        "linkedin",
        "network",
        "dossier",
        "outreach",
        "professional",
        "research",
    ],
};
