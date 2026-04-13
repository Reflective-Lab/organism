//! Reskilling pack — Skills assessment, learning plans, credentials.
//!
//! Fact prefixes: `skill:`, `learning_plan:`, `credential:`,
//! `role_requirement:`, `competence_matrix:`

use crate::pack::{AgentMeta, ContextKey, InvariantClass, InvariantMeta};

pub const AGENTS: &[AgentMeta] = &[
    AgentMeta {
        name: "skill_assessor",
        dependencies: &[ContextKey::Seeds],
        fact_prefix: "skill:",
        target_key: ContextKey::Proposals,
        description: "Skill claims",
    },
    AgentMeta {
        name: "skill_validator",
        dependencies: &[ContextKey::Proposals],
        fact_prefix: "skill:",
        target_key: ContextKey::Evaluations,
        description: "Validates with evidence",
    },
    AgentMeta {
        name: "learning_plan_creator",
        dependencies: &[ContextKey::Evaluations],
        fact_prefix: "learning_plan:",
        target_key: ContextKey::Proposals,
        description: "Plans learning",
    },
    AgentMeta {
        name: "learning_progress_tracker",
        dependencies: &[ContextKey::Proposals],
        fact_prefix: "learning_plan:",
        target_key: ContextKey::Evaluations,
        description: "Tracks progress",
    },
    AgentMeta {
        name: "credential_manager",
        dependencies: &[ContextKey::Proposals],
        fact_prefix: "credential:",
        target_key: ContextKey::Proposals,
        description: "Credentials and renewal",
    },
    AgentMeta {
        name: "competence_matrix",
        dependencies: &[ContextKey::Evaluations],
        fact_prefix: "competence_matrix:",
        target_key: ContextKey::Strategies,
        description: "Team skills matrix",
    },
    AgentMeta {
        name: "role_competence_checker",
        dependencies: &[ContextKey::Evaluations],
        fact_prefix: "role_requirement:",
        target_key: ContextKey::Evaluations,
        description: "Pre-role-change validation",
    },
    AgentMeta {
        name: "critical_role_redundancy",
        dependencies: &[ContextKey::Strategies],
        fact_prefix: "competence_matrix:",
        target_key: ContextKey::Evaluations,
        description: "Detects single points of failure",
    },
];

pub const INVARIANTS: &[InvariantMeta] = &[
    InvariantMeta {
        name: "skill_claim_requires_evidence",
        class: InvariantClass::Acceptance,
        description: "Skill claims require evidence",
    },
    InvariantMeta {
        name: "skill_assessment_has_assessor",
        class: InvariantClass::Structural,
        description: "Skill assessments need an assessor",
    },
    InvariantMeta {
        name: "plan_links_to_business_need",
        class: InvariantClass::Acceptance,
        description: "Plans must link to business need",
    },
    InvariantMeta {
        name: "plan_has_milestones",
        class: InvariantClass::Structural,
        description: "Plans must have milestones",
    },
    InvariantMeta {
        name: "no_role_change_without_competence_delta",
        class: InvariantClass::Acceptance,
        description: "Role changes require competence assessment",
    },
    InvariantMeta {
        name: "credential_has_expiry",
        class: InvariantClass::Structural,
        description: "Credentials must have expiry",
    },
    InvariantMeta {
        name: "critical_role_redundancy",
        class: InvariantClass::Acceptance,
        description: "Critical roles need redundancy",
    },
];

pub const PROFILE: crate::pack::PackProfile = crate::pack::PackProfile {
    entities: &["skill", "learning_plan", "credential", "competence_matrix"],
    required_capabilities: &[],
    uses_llm: false,
    requires_hitl: false,
    handles_irreversible: false,
    keywords: &[
        "skill",
        "learning",
        "credential",
        "competence",
        "training",
        "reskilling",
        "certification",
    ],
};
