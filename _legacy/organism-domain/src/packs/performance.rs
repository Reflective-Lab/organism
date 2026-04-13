// Copyright 2024-2025 Aprio One AB, Sweden
// SPDX-License-Identifier: MIT

//! Performance Pack agents for reviews, goals, and improvement plans.
//!
//! # Lifecycle: Plan → Feedback → Review → Calibrate → Communicate
//!
//! # Review Cycle State Machine
//!
//! ```text
//! planned → collecting_feedback → manager_review → calibration → finalized → communicated → archived
//! ```
//!
//! # Goal State Machine
//!
//! ```text
//! drafted → pending_approval → active → completed → verified
//! ```
//!
//! # Improvement Plan State Machine
//!
//! ```text
//! drafted → approved → active → milestone_review → completed/extended/failed
//! ```
//!
//! # Key Invariants
//!
//! - Reviews need owner, timeframe, and evaluation criteria
//! - No comp changes without evidence trail + approval
//! - PIPs need specific, time-bound milestones
//! - Goals must be measurable with deadlines

use converge_core::{
    Agent, AgentEffect, Context, ContextKey, Fact,
    invariant::{Invariant, InvariantClass, InvariantResult, Violation},
};

// ============================================================================
// Fact ID Prefixes
// ============================================================================

pub const REVIEW_CYCLE_PREFIX: &str = "review_cycle:";
pub const GOAL_PREFIX: &str = "goal:";
pub const IMPROVEMENT_PLAN_PREFIX: &str = "improvement_plan:";
pub const FEEDBACK_PREFIX: &str = "feedback:";
pub const COMP_CHANGE_PREFIX: &str = "comp_change:";

// ============================================================================
// Agents
// ============================================================================

/// Plans and initiates review cycles.
#[derive(Debug, Clone, Default)]
pub struct ReviewCyclePlannerAgent;

impl Agent for ReviewCyclePlannerAgent {
    fn name(&self) -> &str {
        "review_cycle_planner"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Seeds]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        ctx.get(ContextKey::Seeds).iter().any(|s| {
            s.content.contains("review_cycle.schedule") || s.content.contains("quarterly_review")
        }) && !ctx
            .get(ContextKey::Proposals)
            .iter()
            .any(|p| p.id.starts_with(REVIEW_CYCLE_PREFIX))
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let seeds = ctx.get(ContextKey::Seeds);
        let mut facts = Vec::new();

        for seed in seeds.iter() {
            if seed.content.contains("review_cycle.schedule")
                || seed.content.contains("quarterly_review")
            {
                facts.push(Fact {
                    key: ContextKey::Proposals,
                    id: format!("{}{}", REVIEW_CYCLE_PREFIX, seed.id),
                    content: serde_json::json!({
                        "type": "review_cycle",
                        "seed_id": seed.id,
                        "state": "planned",
                        "owner": "hr_lead",
                        "timeframe": {
                            "start": "2026-01-15",
                            "end": "2026-02-15"
                        },
                        "criteria": ["goal_achievement", "competencies", "values_alignment"],
                        "created_at": "2026-01-12"
                    })
                    .to_string(),
                });
            }
        }

        AgentEffect::with_facts(facts)
    }
}

/// Collects and aggregates feedback during review cycles.
#[derive(Debug, Clone, Default)]
pub struct FeedbackCollectorAgent;

impl Agent for FeedbackCollectorAgent {
    fn name(&self) -> &str {
        "feedback_collector"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Proposals, ContextKey::Signals]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        ctx.get(ContextKey::Proposals).iter().any(|p| {
            p.id.starts_with(REVIEW_CYCLE_PREFIX)
                && p.content.contains("\"state\":\"collecting_feedback\"")
        })
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let proposals = ctx.get(ContextKey::Proposals);
        let signals = ctx.get(ContextKey::Signals);
        let mut facts = Vec::new();

        for cycle in proposals.iter() {
            if cycle.id.starts_with(REVIEW_CYCLE_PREFIX)
                && cycle.content.contains("\"state\":\"collecting_feedback\"")
            {
                // Collect feedback from signals
                for signal in signals.iter() {
                    if signal.content.contains("feedback.submitted") {
                        facts.push(Fact {
                            key: ContextKey::Evaluations,
                            id: format!("{}{}", FEEDBACK_PREFIX, signal.id),
                            content: serde_json::json!({
                                "type": "feedback",
                                "cycle_id": cycle.id,
                                "signal_id": signal.id,
                                "author": "identified",
                                "feedback_type": "peer",
                                "collected_at": "2026-01-12"
                            })
                            .to_string(),
                        });
                    }
                }
            }
        }

        AgentEffect::with_facts(facts)
    }
}

/// Manages review calibration across teams.
#[derive(Debug, Clone, Default)]
pub struct CalibrationFacilitatorAgent;

impl Agent for CalibrationFacilitatorAgent {
    fn name(&self) -> &str {
        "calibration_facilitator"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Proposals]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        ctx.get(ContextKey::Proposals).iter().any(|p| {
            p.id.starts_with(REVIEW_CYCLE_PREFIX) && p.content.contains("\"state\":\"calibration\"")
        })
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let proposals = ctx.get(ContextKey::Proposals);
        let mut facts = Vec::new();

        for cycle in proposals.iter() {
            if cycle.id.starts_with(REVIEW_CYCLE_PREFIX)
                && cycle.content.contains("\"state\":\"calibration\"")
            {
                facts.push(Fact {
                    key: ContextKey::Evaluations,
                    id: format!("{}calibration:{}", REVIEW_CYCLE_PREFIX, cycle.id),
                    content: serde_json::json!({
                        "type": "calibration_session",
                        "cycle_id": cycle.id,
                        "state": "scheduled",
                        "peer_comparison": true,
                        "outliers_flagged": false,
                        "scheduled_at": "2026-01-12"
                    })
                    .to_string(),
                });
            }
        }

        AgentEffect::with_facts(facts)
    }
}

/// Creates and tracks goals for employees.
#[derive(Debug, Clone, Default)]
pub struct GoalTrackerAgent;

impl Agent for GoalTrackerAgent {
    fn name(&self) -> &str {
        "goal_tracker"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Seeds]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        ctx.get(ContextKey::Seeds)
            .iter()
            .any(|s| s.content.contains("goal.created") || s.content.contains("okr.set"))
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let seeds = ctx.get(ContextKey::Seeds);
        let mut facts = Vec::new();

        for seed in seeds.iter() {
            if seed.content.contains("goal.created") || seed.content.contains("okr.set") {
                facts.push(Fact {
                    key: ContextKey::Proposals,
                    id: format!("{}{}", GOAL_PREFIX, seed.id),
                    content: serde_json::json!({
                        "type": "goal",
                        "seed_id": seed.id,
                        "state": "drafted",
                        "owner": "employee_id",
                        "deadline": "2026-Q1",
                        "success_criteria": [],
                        "measurable": true,
                        "created_at": "2026-01-12"
                    })
                    .to_string(),
                });
            }
        }

        AgentEffect::with_facts(facts)
    }
}

/// Monitors goal progress and triggers reviews.
#[derive(Debug, Clone, Default)]
pub struct GoalProgressMonitorAgent;

impl Agent for GoalProgressMonitorAgent {
    fn name(&self) -> &str {
        "goal_progress_monitor"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Proposals, ContextKey::Signals]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        ctx.get(ContextKey::Proposals)
            .iter()
            .any(|p| p.id.starts_with(GOAL_PREFIX) && p.content.contains("\"state\":\"active\""))
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let proposals = ctx.get(ContextKey::Proposals);
        let signals = ctx.get(ContextKey::Signals);
        let mut facts = Vec::new();

        for goal in proposals.iter() {
            if goal.id.starts_with(GOAL_PREFIX) && goal.content.contains("\"state\":\"active\"") {
                // Check for progress updates
                let has_progress = signals
                    .iter()
                    .any(|s| s.content.contains("goal.progress") && s.content.contains(&goal.id));

                facts.push(Fact {
                    key: ContextKey::Evaluations,
                    id: format!("{}progress:{}", GOAL_PREFIX, goal.id),
                    content: serde_json::json!({
                        "type": "goal_progress",
                        "goal_id": goal.id,
                        "has_updates": has_progress,
                        "on_track": true,
                        "checked_at": "2026-01-12"
                    })
                    .to_string(),
                });
            }
        }

        AgentEffect::with_facts(facts)
    }
}

/// Creates and manages performance improvement plans.
#[derive(Debug, Clone, Default)]
pub struct ImprovementPlanCreatorAgent;

impl Agent for ImprovementPlanCreatorAgent {
    fn name(&self) -> &str {
        "improvement_plan_creator"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Seeds]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        ctx.get(ContextKey::Seeds).iter().any(|s| {
            s.content.contains("pip.initiated") || s.content.contains("performance.concern")
        })
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let seeds = ctx.get(ContextKey::Seeds);
        let mut facts = Vec::new();

        for seed in seeds.iter() {
            if seed.content.contains("pip.initiated")
                || seed.content.contains("performance.concern")
            {
                facts.push(Fact {
                    key: ContextKey::Proposals,
                    id: format!("{}{}", IMPROVEMENT_PLAN_PREFIX, seed.id),
                    content: serde_json::json!({
                        "type": "improvement_plan",
                        "seed_id": seed.id,
                        "state": "drafted",
                        "employee_id": "extracted",
                        "milestones": [],
                        "support_resources": [],
                        "duration_days": 60,
                        "hr_notified": true,
                        "created_at": "2026-01-12"
                    })
                    .to_string(),
                });
            }
        }

        AgentEffect::with_facts(facts)
    }
}

/// Tracks improvement plan milestones.
#[derive(Debug, Clone, Default)]
pub struct PipMilestoneTrackerAgent;

impl Agent for PipMilestoneTrackerAgent {
    fn name(&self) -> &str {
        "pip_milestone_tracker"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Proposals, ContextKey::Signals]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        ctx.get(ContextKey::Proposals).iter().any(|p| {
            p.id.starts_with(IMPROVEMENT_PLAN_PREFIX) && p.content.contains("\"state\":\"active\"")
        })
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let proposals = ctx.get(ContextKey::Proposals);
        let mut facts = Vec::new();

        for pip in proposals.iter() {
            if pip.id.starts_with(IMPROVEMENT_PLAN_PREFIX)
                && pip.content.contains("\"state\":\"active\"")
            {
                facts.push(Fact {
                    key: ContextKey::Evaluations,
                    id: format!("{}milestone_check:{}", IMPROVEMENT_PLAN_PREFIX, pip.id),
                    content: serde_json::json!({
                        "type": "pip_milestone_check",
                        "pip_id": pip.id,
                        "milestones_complete": 0,
                        "milestones_total": 3,
                        "on_track": true,
                        "checked_at": "2026-01-12"
                    })
                    .to_string(),
                });
            }
        }

        AgentEffect::with_facts(facts)
    }
}

/// Processes compensation change requests with evidence validation.
#[derive(Debug, Clone, Default)]
pub struct CompensationChangeAgent;

impl Agent for CompensationChangeAgent {
    fn name(&self) -> &str {
        "compensation_change"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Seeds, ContextKey::Proposals]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        ctx.get(ContextKey::Seeds).iter().any(|s| {
            s.content.contains("comp.change_requested") || s.content.contains("promotion.proposed")
        })
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let seeds = ctx.get(ContextKey::Seeds);
        let proposals = ctx.get(ContextKey::Proposals);
        let mut facts = Vec::new();

        for seed in seeds.iter() {
            if seed.content.contains("comp.change_requested")
                || seed.content.contains("promotion.proposed")
            {
                // Check for calibration evidence
                let has_calibration = proposals.iter().any(|p| {
                    p.content.contains("calibration")
                        && p.content.contains("\"state\":\"completed\"")
                });

                facts.push(Fact {
                    key: ContextKey::Proposals,
                    id: format!("{}{}", COMP_CHANGE_PREFIX, seed.id),
                    content: serde_json::json!({
                        "type": "comp_change_request",
                        "seed_id": seed.id,
                        "state": "pending_evidence",
                        "calibration_complete": has_calibration,
                        "evidence_trail": [],
                        "requires_approval": ["manager", "hr"],
                        "created_at": "2026-01-12"
                    })
                    .to_string(),
                });
            }
        }

        AgentEffect::with_facts(facts)
    }
}

// ============================================================================
// Invariants
// ============================================================================

/// Ensures review cycles have owner, timeframe, and criteria.
#[derive(Debug, Clone, Default)]
pub struct ReviewHasOwnerTimeframeCriteriaInvariant;

impl Invariant for ReviewHasOwnerTimeframeCriteriaInvariant {
    fn name(&self) -> &str {
        "review_has_owner_timeframe_criteria"
    }

    fn class(&self) -> InvariantClass {
        InvariantClass::Structural
    }

    fn check(&self, ctx: &Context) -> InvariantResult {
        for cycle in ctx.get(ContextKey::Proposals).iter() {
            if cycle.id.starts_with(REVIEW_CYCLE_PREFIX) {
                if !cycle.content.contains("\"owner\":") {
                    return InvariantResult::Violated(Violation::with_facts(
                        format!("Review cycle {} missing owner", cycle.id),
                        vec![cycle.id.clone()],
                    ));
                }
                if !cycle.content.contains("\"timeframe\":") {
                    return InvariantResult::Violated(Violation::with_facts(
                        format!("Review cycle {} missing timeframe", cycle.id),
                        vec![cycle.id.clone()],
                    ));
                }
                if !cycle.content.contains("\"criteria\":") {
                    return InvariantResult::Violated(Violation::with_facts(
                        format!("Review cycle {} missing criteria", cycle.id),
                        vec![cycle.id.clone()],
                    ));
                }
            }
        }
        InvariantResult::Ok
    }
}

/// Ensures goals have measurable outcomes.
#[derive(Debug, Clone, Default)]
pub struct GoalsHaveMeasurableOutcomesInvariant;

impl Invariant for GoalsHaveMeasurableOutcomesInvariant {
    fn name(&self) -> &str {
        "goals_have_measurable_outcomes"
    }

    fn class(&self) -> InvariantClass {
        InvariantClass::Acceptance
    }

    fn check(&self, ctx: &Context) -> InvariantResult {
        for goal in ctx.get(ContextKey::Proposals).iter() {
            if goal.id.starts_with(GOAL_PREFIX)
                && goal.content.contains("\"state\":\"active\"")
                && !goal.content.contains("\"measurable\":true")
            {
                return InvariantResult::Violated(Violation::with_facts(
                    format!("Active goal {} lacks measurable outcomes", goal.id),
                    vec![goal.id.clone()],
                ));
            }
        }
        InvariantResult::Ok
    }
}

/// Ensures comp changes have evidence trail.
#[derive(Debug, Clone, Default)]
pub struct NoCompChangeWithoutEvidenceInvariant;

impl Invariant for NoCompChangeWithoutEvidenceInvariant {
    fn name(&self) -> &str {
        "no_comp_change_without_evidence"
    }

    fn class(&self) -> InvariantClass {
        InvariantClass::Acceptance
    }

    fn check(&self, ctx: &Context) -> InvariantResult {
        for change in ctx.get(ContextKey::Proposals).iter() {
            if change.id.starts_with(COMP_CHANGE_PREFIX)
                && change.content.contains("\"state\":\"approved\"")
                && change.content.contains("\"evidence_trail\":[]")
            {
                return InvariantResult::Violated(Violation::with_facts(
                    format!("Comp change {} approved without evidence", change.id),
                    vec![change.id.clone()],
                ));
            }
        }
        InvariantResult::Ok
    }
}

/// Ensures PIPs have clear milestones.
#[derive(Debug, Clone, Default)]
pub struct PipHasClearMilestonesInvariant;

impl Invariant for PipHasClearMilestonesInvariant {
    fn name(&self) -> &str {
        "pip_has_clear_milestones"
    }

    fn class(&self) -> InvariantClass {
        InvariantClass::Structural
    }

    fn check(&self, ctx: &Context) -> InvariantResult {
        for pip in ctx.get(ContextKey::Proposals).iter() {
            if pip.id.starts_with(IMPROVEMENT_PLAN_PREFIX)
                && pip.content.contains("\"state\":\"active\"")
                && pip.content.contains("\"milestones\":[]")
            {
                return InvariantResult::Violated(Violation::with_facts(
                    format!("Active PIP {} has no milestones", pip.id),
                    vec![pip.id.clone()],
                ));
            }
        }
        InvariantResult::Ok
    }
}

/// Ensures PIPs have support resources specified.
#[derive(Debug, Clone, Default)]
pub struct PipHasSupportResourcesInvariant;

impl Invariant for PipHasSupportResourcesInvariant {
    fn name(&self) -> &str {
        "pip_has_support_resources"
    }

    fn class(&self) -> InvariantClass {
        InvariantClass::Acceptance
    }

    fn check(&self, ctx: &Context) -> InvariantResult {
        for pip in ctx.get(ContextKey::Proposals).iter() {
            if pip.id.starts_with(IMPROVEMENT_PLAN_PREFIX)
                && pip.content.contains("\"state\":\"active\"")
                && pip.content.contains("\"support_resources\":[]")
            {
                return InvariantResult::Violated(Violation::with_facts(
                    format!("Active PIP {} has no support resources", pip.id),
                    vec![pip.id.clone()],
                ));
            }
        }
        InvariantResult::Ok
    }
}

/// Ensures feedback has identified author.
#[derive(Debug, Clone, Default)]
pub struct FeedbackHasAuthorInvariant;

impl Invariant for FeedbackHasAuthorInvariant {
    fn name(&self) -> &str {
        "feedback_has_author"
    }

    fn class(&self) -> InvariantClass {
        InvariantClass::Structural
    }

    fn check(&self, ctx: &Context) -> InvariantResult {
        for feedback in ctx.get(ContextKey::Evaluations).iter() {
            if feedback.id.starts_with(FEEDBACK_PREFIX) && !feedback.content.contains("\"author\":")
            {
                return InvariantResult::Violated(Violation::with_facts(
                    format!("Feedback {} missing author", feedback.id),
                    vec![feedback.id.clone()],
                ));
            }
        }
        InvariantResult::Ok
    }
}

/// Ensures calibration before promotion.
#[derive(Debug, Clone, Default)]
pub struct CalibrationBeforePromotionInvariant;

impl Invariant for CalibrationBeforePromotionInvariant {
    fn name(&self) -> &str {
        "calibration_before_promotion"
    }

    fn class(&self) -> InvariantClass {
        InvariantClass::Acceptance
    }

    fn check(&self, ctx: &Context) -> InvariantResult {
        let proposals = ctx.get(ContextKey::Proposals);

        for change in proposals.iter() {
            if change.id.starts_with(COMP_CHANGE_PREFIX)
                && change.content.contains("promotion")
                && change.content.contains("\"state\":\"approved\"")
            {
                let has_calibration = change.content.contains("\"calibration_complete\":true");
                if !has_calibration {
                    return InvariantResult::Violated(Violation::with_facts(
                        format!("Promotion {} approved without calibration", change.id),
                        vec![change.id.clone()],
                    ));
                }
            }
        }
        InvariantResult::Ok
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn agents_have_correct_names() {
        assert_eq!(ReviewCyclePlannerAgent.name(), "review_cycle_planner");
        assert_eq!(FeedbackCollectorAgent.name(), "feedback_collector");
        assert_eq!(
            CalibrationFacilitatorAgent.name(),
            "calibration_facilitator"
        );
        assert_eq!(GoalTrackerAgent.name(), "goal_tracker");
        assert_eq!(GoalProgressMonitorAgent.name(), "goal_progress_monitor");
        assert_eq!(
            ImprovementPlanCreatorAgent.name(),
            "improvement_plan_creator"
        );
        assert_eq!(PipMilestoneTrackerAgent.name(), "pip_milestone_tracker");
        assert_eq!(CompensationChangeAgent.name(), "compensation_change");
    }

    #[test]
    fn invariants_have_correct_names() {
        assert_eq!(
            ReviewHasOwnerTimeframeCriteriaInvariant.name(),
            "review_has_owner_timeframe_criteria"
        );
        assert_eq!(
            GoalsHaveMeasurableOutcomesInvariant.name(),
            "goals_have_measurable_outcomes"
        );
        assert_eq!(
            NoCompChangeWithoutEvidenceInvariant.name(),
            "no_comp_change_without_evidence"
        );
        assert_eq!(
            PipHasClearMilestonesInvariant.name(),
            "pip_has_clear_milestones"
        );
        assert_eq!(
            PipHasSupportResourcesInvariant.name(),
            "pip_has_support_resources"
        );
        assert_eq!(FeedbackHasAuthorInvariant.name(), "feedback_has_author");
        assert_eq!(
            CalibrationBeforePromotionInvariant.name(),
            "calibration_before_promotion"
        );
    }
}
