//! Knowledge lifecycle pack.
//!
//! Moved from converge-domain — this is organizational learning,
//! not kernel infrastructure.
//!
//! Lifecycle: Signal → Hypothesis → Experiment → Decision → Canonical
//!
//! Fact prefixes: `signal:`, `hypothesis:`, `experiment:`, `decision:`,
//! `canonical:`, `claim:`, `prior_art:`, `claim_chart:`, `patent_report:`

use crate::pack::{AgentMeta, ContextKey, InvariantClass, InvariantMeta};

pub const AGENTS: &[AgentMeta] = &[
    AgentMeta {
        name: "signal_capture",
        dependencies: &[ContextKey::Seeds],
        fact_prefix: "signal:",
        target_key: ContextKey::Signals,
        description: "Captures signals from Slack, meetings, customer feedback",
    },
    AgentMeta {
        name: "hypothesis_generator",
        dependencies: &[ContextKey::Signals],
        fact_prefix: "hypothesis:",
        target_key: ContextKey::Hypotheses,
        description: "Generates hypotheses from captured signals",
    },
    AgentMeta {
        name: "hypothesis_reviewer",
        dependencies: &[ContextKey::Hypotheses],
        fact_prefix: "hypothesis:",
        target_key: ContextKey::Evaluations,
        description: "Reviews and approves hypotheses for experimentation",
    },
    AgentMeta {
        name: "experiment_scheduler",
        dependencies: &[ContextKey::Hypotheses, ContextKey::Evaluations],
        fact_prefix: "experiment:",
        target_key: ContextKey::Proposals,
        description: "Schedules experiments for approved hypotheses",
    },
    AgentMeta {
        name: "experiment_runner",
        dependencies: &[ContextKey::Proposals],
        fact_prefix: "experiment:",
        target_key: ContextKey::Evaluations,
        description: "Monitors running experiments and collects results",
    },
    AgentMeta {
        name: "decision_memo",
        dependencies: &[ContextKey::Evaluations],
        fact_prefix: "decision:",
        target_key: ContextKey::Strategies,
        description: "Creates decision memos from completed experiments",
    },
    AgentMeta {
        name: "canonical_knowledge",
        dependencies: &[ContextKey::Strategies],
        fact_prefix: "canonical:",
        target_key: ContextKey::Strategies,
        description: "Records decisions as canonical organizational knowledge",
    },
    AgentMeta {
        name: "claim_validator",
        dependencies: &[ContextKey::Seeds, ContextKey::Signals],
        fact_prefix: "claim:",
        target_key: ContextKey::Evaluations,
        description: "Validates and enriches claims with provenance",
    },
];

pub const INVARIANTS: &[InvariantMeta] = &[
    InvariantMeta {
        name: "claim_has_provenance",
        class: InvariantClass::Structural,
        description: "Every claim must have provenance",
    },
    InvariantMeta {
        name: "no_orphan_experiments",
        class: InvariantClass::Semantic,
        description: "Experiments must link to hypotheses",
    },
    InvariantMeta {
        name: "experiment_has_metrics",
        class: InvariantClass::Structural,
        description: "Experiments must have success metrics defined",
    },
    InvariantMeta {
        name: "decision_has_owner",
        class: InvariantClass::Acceptance,
        description: "Decisions must have explicit owners",
    },
    InvariantMeta {
        name: "patent_evidence_has_provenance",
        class: InvariantClass::Structural,
        description: "Prior art evidence must include provenance",
    },
];

pub const PROFILE: crate::pack::PackProfile = crate::pack::PackProfile {
    entities: &[
        "signal",
        "hypothesis",
        "experiment",
        "decision",
        "canonical",
        "claim",
    ],
    required_capabilities: &[],
    uses_llm: false,
    requires_hitl: false,
    handles_irreversible: false,
    keywords: &[
        "signal",
        "hypothesis",
        "experiment",
        "decision",
        "knowledge",
        "learning",
        "evidence",
    ],
};
