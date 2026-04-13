//! Ops support pack — Ticket intake, triage, escalation, SLA.
//!
//! Fact prefixes: `ticket:`, `conversation:`, `escalation:`, `sla:`,
//! `root_cause:`, `prevention:`, `internal_request:`, `kb_article:`

use crate::pack::{AgentMeta, ContextKey, InvariantClass, InvariantMeta};

pub const AGENTS: &[AgentMeta] = &[
    AgentMeta {
        name: "ticket_intake",
        dependencies: &[ContextKey::Seeds],
        fact_prefix: "ticket:",
        target_key: ContextKey::Signals,
        description: "Normalizes from all channels",
    },
    AgentMeta {
        name: "ticket_triager",
        dependencies: &[ContextKey::Signals],
        fact_prefix: "ticket:",
        target_key: ContextKey::Proposals,
        description: "Categorizes/prioritizes",
    },
    AgentMeta {
        name: "auto_responder",
        dependencies: &[ContextKey::Proposals],
        fact_prefix: "ticket:",
        target_key: ContextKey::Proposals,
        description: "Auto-response for known issues",
    },
    AgentMeta {
        name: "ticket_router",
        dependencies: &[ContextKey::Proposals],
        fact_prefix: "ticket:",
        target_key: ContextKey::Proposals,
        description: "Routes to handlers",
    },
    AgentMeta {
        name: "sla_monitor",
        dependencies: &[ContextKey::Proposals],
        fact_prefix: "sla:",
        target_key: ContextKey::Evaluations,
        description: "Tracks SLA compliance",
    },
    AgentMeta {
        name: "escalation_handler",
        dependencies: &[ContextKey::Evaluations],
        fact_prefix: "escalation:",
        target_key: ContextKey::Proposals,
        description: "Escalation routing",
    },
    AgentMeta {
        name: "resolution_tracker",
        dependencies: &[ContextKey::Proposals],
        fact_prefix: "ticket:",
        target_key: ContextKey::Evaluations,
        description: "Validates closure",
    },
    AgentMeta {
        name: "pattern_detector",
        dependencies: &[ContextKey::Evaluations],
        fact_prefix: "root_cause:",
        target_key: ContextKey::Proposals,
        description: "Detects recurring issues",
    },
    AgentMeta {
        name: "internal_request_router",
        dependencies: &[ContextKey::Seeds],
        fact_prefix: "internal_request:",
        target_key: ContextKey::Proposals,
        description: "Routes internal requests",
    },
    AgentMeta {
        name: "kb_updater",
        dependencies: &[ContextKey::Evaluations],
        fact_prefix: "kb_article:",
        target_key: ContextKey::Proposals,
        description: "Creates KB from solutions",
    },
];

pub const INVARIANTS: &[InvariantMeta] = &[
    InvariantMeta {
        name: "no_orphan_tickets",
        class: InvariantClass::Structural,
        description: "Tickets must have assignment",
    },
    InvariantMeta {
        name: "sla_breach_escalates",
        class: InvariantClass::Semantic,
        description: "SLA breaches must escalate",
    },
    InvariantMeta {
        name: "closure_requires_resolution",
        class: InvariantClass::Acceptance,
        description: "Closure requires resolution",
    },
    InvariantMeta {
        name: "escalation_has_reason",
        class: InvariantClass::Structural,
        description: "Escalations must have reason",
    },
];

pub const PROFILE: crate::pack::PackProfile = crate::pack::PackProfile {
    entities: &["ticket", "escalation", "sla", "root_cause", "kb_article"],
    required_capabilities: &[],
    uses_llm: false,
    requires_hitl: false,
    handles_irreversible: false,
    keywords: &[
        "ticket",
        "support",
        "escalation",
        "sla",
        "triage",
        "helpdesk",
        "incident",
    ],
};
