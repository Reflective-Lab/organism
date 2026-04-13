//! Partnerships pack — Vendor sourcing, evaluation, contracting.
//!
//! Fact prefixes: `partner:`, `supplier:`, `p_agreement:`, `vendor_assessment:`,
//! `integration:`, `diligence:`, `relationship:`, `contract_renewal:`

use crate::pack::{AgentMeta, ContextKey, InvariantClass, InvariantMeta};

pub const AGENTS: &[AgentMeta] = &[
    AgentMeta {
        name: "partner_sourcer",
        dependencies: &[ContextKey::Seeds],
        fact_prefix: "partner:",
        target_key: ContextKey::Proposals,
        description: "Identifies partner prospects",
    },
    AgentMeta {
        name: "vendor_assessor",
        dependencies: &[ContextKey::Seeds],
        fact_prefix: "vendor_assessment:",
        target_key: ContextKey::Proposals,
        description: "Security/compliance assessments",
    },
    AgentMeta {
        name: "contract_negotiator",
        dependencies: &[ContextKey::Proposals],
        fact_prefix: "p_agreement:",
        target_key: ContextKey::Evaluations,
        description: "Negotiation support",
    },
    AgentMeta {
        name: "relationship_manager",
        dependencies: &[ContextKey::Proposals],
        fact_prefix: "relationship:",
        target_key: ContextKey::Evaluations,
        description: "Health monitoring",
    },
    AgentMeta {
        name: "performance_reviewer",
        dependencies: &[ContextKey::Evaluations],
        fact_prefix: "relationship:",
        target_key: ContextKey::Evaluations,
        description: "Annual reviews",
    },
    AgentMeta {
        name: "integration_coordinator",
        dependencies: &[ContextKey::Proposals],
        fact_prefix: "integration:",
        target_key: ContextKey::Proposals,
        description: "Technical coordination",
    },
    AgentMeta {
        name: "due_diligence_coordinator",
        dependencies: &[ContextKey::Seeds],
        fact_prefix: "diligence:",
        target_key: ContextKey::Proposals,
        description: "Due diligence checklist",
    },
    AgentMeta {
        name: "partnership_renewal_tracker",
        dependencies: &[ContextKey::Proposals],
        fact_prefix: "contract_renewal:",
        target_key: ContextKey::Signals,
        description: "Renewal tracking",
    },
    AgentMeta {
        name: "risk_monitor",
        dependencies: &[ContextKey::Signals],
        fact_prefix: "relationship:",
        target_key: ContextKey::Evaluations,
        description: "External risk detection",
    },
    AgentMeta {
        name: "offboarding_coordinator",
        dependencies: &[ContextKey::Proposals],
        fact_prefix: "partner:",
        target_key: ContextKey::Proposals,
        description: "Exit planning",
    },
];

pub const INVARIANTS: &[InvariantMeta] = &[
    InvariantMeta {
        name: "vendor_has_assessment",
        class: InvariantClass::Structural,
        description: "Vendors must have assessment",
    },
    InvariantMeta {
        name: "partner_has_agreement",
        class: InvariantClass::Structural,
        description: "Partners must have agreement",
    },
    InvariantMeta {
        name: "integration_has_owner",
        class: InvariantClass::Structural,
        description: "Integrations must have owner",
    },
    InvariantMeta {
        name: "high_risk_vendor_requires_approval",
        class: InvariantClass::Semantic,
        description: "High-risk vendors require approval",
    },
];

pub const PROFILE: crate::pack::PackProfile = crate::pack::PackProfile {
    entities: &["partner", "supplier", "vendor", "integration", "assessment"],
    required_capabilities: &["web"],
    uses_llm: false,
    requires_hitl: true,
    handles_irreversible: false,
    keywords: &[
        "vendor",
        "partner",
        "supplier",
        "sourcing",
        "procurement",
        "assessment",
        "diligence",
    ],
};
