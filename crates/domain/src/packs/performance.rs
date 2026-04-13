//! Performance pack — Reviews, goals, improvement plans.
//!
//! Fact prefixes: `review_cycle:`, `goal:`, `improvement_plan:`,
//! `feedback:`, `comp_change:`

use crate::pack::{AgentMeta, ContextKey, InvariantClass, InvariantMeta};

pub const AGENTS: &[AgentMeta] = &[
    AgentMeta {
        name: "review_cycle_planner",
        dependencies: &[ContextKey::Seeds],
        fact_prefix: "review_cycle:",
        target_key: ContextKey::Proposals,
        description: "Plans review cycles",
    },
    AgentMeta {
        name: "feedback_collector",
        dependencies: &[ContextKey::Seeds],
        fact_prefix: "feedback:",
        target_key: ContextKey::Evaluations,
        description: "Aggregates peer feedback",
    },
    AgentMeta {
        name: "calibration_facilitator",
        dependencies: &[ContextKey::Evaluations],
        fact_prefix: "review_cycle:",
        target_key: ContextKey::Evaluations,
        description: "Calibration sessions",
    },
    AgentMeta {
        name: "goal_tracker",
        dependencies: &[ContextKey::Seeds],
        fact_prefix: "goal:",
        target_key: ContextKey::Proposals,
        description: "Creates goals",
    },
    AgentMeta {
        name: "goal_progress_monitor",
        dependencies: &[ContextKey::Proposals],
        fact_prefix: "goal:",
        target_key: ContextKey::Evaluations,
        description: "Tracks progress",
    },
    AgentMeta {
        name: "improvement_plan_creator",
        dependencies: &[ContextKey::Evaluations],
        fact_prefix: "improvement_plan:",
        target_key: ContextKey::Proposals,
        description: "Creates PIPs",
    },
    AgentMeta {
        name: "pip_milestone_tracker",
        dependencies: &[ContextKey::Proposals],
        fact_prefix: "improvement_plan:",
        target_key: ContextKey::Evaluations,
        description: "Tracks PIP milestones",
    },
    AgentMeta {
        name: "compensation_change",
        dependencies: &[ContextKey::Evaluations],
        fact_prefix: "comp_change:",
        target_key: ContextKey::Proposals,
        description: "Salary/promotion changes",
    },
];

pub const INVARIANTS: &[InvariantMeta] = &[
    InvariantMeta {
        name: "review_has_owner_timeframe_criteria",
        class: InvariantClass::Structural,
        description: "Reviews must have owner, timeframe, and criteria",
    },
    InvariantMeta {
        name: "goals_have_measurable_outcomes",
        class: InvariantClass::Acceptance,
        description: "Goals must have measurable outcomes",
    },
    InvariantMeta {
        name: "no_comp_change_without_evidence",
        class: InvariantClass::Acceptance,
        description: "Compensation changes require evidence",
    },
    InvariantMeta {
        name: "pip_has_clear_milestones",
        class: InvariantClass::Structural,
        description: "PIPs must have clear milestones",
    },
    InvariantMeta {
        name: "pip_has_support_resources",
        class: InvariantClass::Acceptance,
        description: "PIPs must include support resources",
    },
    InvariantMeta {
        name: "feedback_has_author",
        class: InvariantClass::Structural,
        description: "Feedback must have an author",
    },
    InvariantMeta {
        name: "calibration_before_promotion",
        class: InvariantClass::Acceptance,
        description: "Calibration required before promotion",
    },
];

pub const PROFILE: crate::pack::PackProfile = crate::pack::PackProfile {
    entities: &[
        "review_cycle",
        "goal",
        "improvement_plan",
        "feedback",
        "comp_change",
    ],
    required_capabilities: &[],
    uses_llm: false,
    requires_hitl: true,
    handles_irreversible: true,
    keywords: &[
        "review",
        "performance",
        "goal",
        "feedback",
        "compensation",
        "promotion",
        "pip",
    ],
};
