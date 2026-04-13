//! Legal pack — Contracts, equity, IP governance.
//!
//! Fact prefixes: `contract:`, `equity:`, `ip_assignment:`, `board_approval:`,
//! `signature:`, `paid_action:`, `patent_submission:`, `approval:`

use crate::pack::{AgentMeta, ContextKey, InvariantClass, InvariantMeta};

pub const AGENTS: &[AgentMeta] = &[
    AgentMeta {
        name: "contract_generator",
        dependencies: &[ContextKey::Seeds],
        fact_prefix: "contract:",
        target_key: ContextKey::Proposals,
        description: "Generates MSA/DPA/SOW from deal triggers",
    },
    AgentMeta {
        name: "contract_reviewer",
        dependencies: &[ContextKey::Proposals],
        fact_prefix: "contract:",
        target_key: ContextKey::Evaluations,
        description: "Legal review",
    },
    AgentMeta {
        name: "signature_requestor",
        dependencies: &[ContextKey::Evaluations],
        fact_prefix: "signature:",
        target_key: ContextKey::Proposals,
        description: "Requests signatures (DocuSign etc.)",
    },
    AgentMeta {
        name: "contract_executor",
        dependencies: &[ContextKey::Proposals],
        fact_prefix: "contract:",
        target_key: ContextKey::Proposals,
        description: "Executes signed contracts",
    },
    AgentMeta {
        name: "expiration_monitor",
        dependencies: &[ContextKey::Proposals],
        fact_prefix: "contract:",
        target_key: ContextKey::Evaluations,
        description: "Renewal trigger",
    },
    AgentMeta {
        name: "equity_grant_processor",
        dependencies: &[ContextKey::Seeds],
        fact_prefix: "equity:",
        target_key: ContextKey::Proposals,
        description: "Processes equity grants",
    },
    AgentMeta {
        name: "board_approval",
        dependencies: &[ContextKey::Seeds],
        fact_prefix: "board_approval:",
        target_key: ContextKey::Proposals,
        description: "Board approval signal",
    },
    AgentMeta {
        name: "ip_assignment",
        dependencies: &[ContextKey::Seeds],
        fact_prefix: "ip_assignment:",
        target_key: ContextKey::Proposals,
        description: "IP assignments for contractors/employees",
    },
    AgentMeta {
        name: "ip_gate_validator",
        dependencies: &[ContextKey::Proposals],
        fact_prefix: "ip_assignment:",
        target_key: ContextKey::Evaluations,
        description: "Validates IP before payment",
    },
];

pub const INVARIANTS: &[InvariantMeta] = &[
    InvariantMeta {
        name: "signature_required",
        class: InvariantClass::Structural,
        description: "Contracts must be signed",
    },
    InvariantMeta {
        name: "ip_assignment_before_payment",
        class: InvariantClass::Acceptance,
        description: "IP must be assigned before payment",
    },
    InvariantMeta {
        name: "board_approval_required",
        class: InvariantClass::Acceptance,
        description: "Board approval required for major decisions",
    },
    InvariantMeta {
        name: "paid_action_requires_approval",
        class: InvariantClass::Acceptance,
        description: "Payment actions require approval",
    },
    InvariantMeta {
        name: "submission_requires_approval",
        class: InvariantClass::Acceptance,
        description: "Patent submissions require approval",
    },
    InvariantMeta {
        name: "submission_requires_evidence",
        class: InvariantClass::Acceptance,
        description: "Patent submissions require evidence",
    },
];

pub const PROFILE: crate::pack::PackProfile = crate::pack::PackProfile {
    entities: &["contract", "equity", "ip_assignment", "signature", "patent"],
    required_capabilities: &[],
    uses_llm: false,
    requires_hitl: true,
    handles_irreversible: true,
    keywords: &[
        "contract",
        "legal",
        "compliance",
        "signature",
        "equity",
        "ip",
        "board",
    ],
};
