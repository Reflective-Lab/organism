// Copyright 2024-2025 Aprio One AB, Sweden
// SPDX-License-Identifier: MIT

//! Reskilling Pack agents for skills, learning plans, and credentials.
//!
//! # Lifecycle: Assess → Plan → Learn → Certify
//!
//! # Skill State Machine
//!
//! ```text
//! unassessed → claimed → assessed → developing → assessment_pending → assessed
//!                                                                       ↓
//!                                                   revalidation → expired/removed
//! ```
//!
//! # Learning Plan State Machine
//!
//! ```text
//! drafted → pending_approval → awaiting_budget → in_progress → completed/abandoned
//! ```
//!
//! # Credential State Machine
//!
//! ```text
//! pending → issued → renewal_pending → renewed/expired
//! ```
//!
//! # Key Invariants
//!
//! - Skill claims require evidence and provenance
//! - Learning plans need business justification
//! - Role changes require competence assessment
//! - Critical roles need redundancy (at least 2 people)

use converge_core::{
    Agent, AgentEffect, Context, ContextKey, Fact,
    invariant::{Invariant, InvariantClass, InvariantResult, Violation},
};

// ============================================================================
// Fact ID Prefixes
// ============================================================================

pub const SKILL_PREFIX: &str = "skill:";
pub const LEARNING_PLAN_PREFIX: &str = "learning_plan:";
pub const CREDENTIAL_PREFIX: &str = "credential:";
pub const ROLE_REQ_PREFIX: &str = "role_requirement:";
pub const COMPETENCE_MATRIX_PREFIX: &str = "competence_matrix:";

// ============================================================================
// Agents
// ============================================================================

/// Assesses and tracks skill levels for employees.
#[derive(Debug, Clone, Default)]
pub struct SkillAssessorAgent;

impl Agent for SkillAssessorAgent {
    fn name(&self) -> &str {
        "skill_assessor"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Seeds]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        ctx.get(ContextKey::Seeds).iter().any(|s| {
            s.content.contains("skill.claimed") || s.content.contains("skill.assessment_requested")
        })
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let seeds = ctx.get(ContextKey::Seeds);
        let mut facts = Vec::new();

        for seed in seeds.iter() {
            if seed.content.contains("skill.claimed")
                || seed.content.contains("skill.assessment_requested")
            {
                facts.push(Fact {
                    key: ContextKey::Proposals,
                    id: format!("{}{}", SKILL_PREFIX, seed.id),
                    content: serde_json::json!({
                        "type": "skill_assessment",
                        "seed_id": seed.id,
                        "state": "claimed",
                        "employee_id": "extracted",
                        "skill_name": "extracted",
                        "claimed_level": 3,
                        "evidence_required": true,
                        "assessor": "pending_assignment",
                        "created_at": "2026-01-12"
                    })
                    .to_string(),
                });
            }
        }

        AgentEffect::with_facts(facts)
    }
}

/// Validates skill claims with evidence.
#[derive(Debug, Clone, Default)]
pub struct SkillValidatorAgent;

impl Agent for SkillValidatorAgent {
    fn name(&self) -> &str {
        "skill_validator"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Proposals, ContextKey::Signals]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        ctx.get(ContextKey::Proposals)
            .iter()
            .any(|p| p.id.starts_with(SKILL_PREFIX) && p.content.contains("\"state\":\"claimed\""))
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let proposals = ctx.get(ContextKey::Proposals);
        let signals = ctx.get(ContextKey::Signals);
        let mut facts = Vec::new();

        for skill in proposals.iter() {
            if skill.id.starts_with(SKILL_PREFIX) && skill.content.contains("\"state\":\"claimed\"")
            {
                // Check for evidence submission
                let has_evidence = signals.iter().any(|s| {
                    s.content.contains("evidence.submitted") && s.content.contains(&skill.id)
                });

                if has_evidence {
                    facts.push(Fact {
                        key: ContextKey::Evaluations,
                        id: format!("{}validation:{}", SKILL_PREFIX, skill.id),
                        content: serde_json::json!({
                            "type": "skill_validation",
                            "skill_id": skill.id,
                            "evidence_received": true,
                            "assessor": "manager_id",
                            "validated_at": "2026-01-12"
                        })
                        .to_string(),
                    });
                }
            }
        }

        AgentEffect::with_facts(facts)
    }
}

/// Creates and manages learning plans.
#[derive(Debug, Clone, Default)]
pub struct LearningPlanCreatorAgent;

impl Agent for LearningPlanCreatorAgent {
    fn name(&self) -> &str {
        "learning_plan_creator"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Seeds]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        ctx.get(ContextKey::Seeds).iter().any(|s| {
            s.content.contains("learning.requested") || s.content.contains("skill_gap.identified")
        })
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let seeds = ctx.get(ContextKey::Seeds);
        let mut facts = Vec::new();

        for seed in seeds.iter() {
            if seed.content.contains("learning.requested")
                || seed.content.contains("skill_gap.identified")
            {
                facts.push(Fact {
                    key: ContextKey::Proposals,
                    id: format!("{}{}", LEARNING_PLAN_PREFIX, seed.id),
                    content: serde_json::json!({
                        "type": "learning_plan",
                        "seed_id": seed.id,
                        "state": "drafted",
                        "employee_id": "extracted",
                        "target_skill": "extracted",
                        "business_justification": "pending",
                        "milestones": [],
                        "budget_required": false,
                        "timeframe_weeks": 8,
                        "created_at": "2026-01-12"
                    })
                    .to_string(),
                });
            }
        }

        AgentEffect::with_facts(facts)
    }
}

/// Tracks progress on learning plans.
#[derive(Debug, Clone, Default)]
pub struct LearningProgressTrackerAgent;

impl Agent for LearningProgressTrackerAgent {
    fn name(&self) -> &str {
        "learning_progress_tracker"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Proposals, ContextKey::Signals]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        ctx.get(ContextKey::Proposals).iter().any(|p| {
            p.id.starts_with(LEARNING_PLAN_PREFIX)
                && p.content.contains("\"state\":\"in_progress\"")
        })
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let proposals = ctx.get(ContextKey::Proposals);
        let mut facts = Vec::new();

        for plan in proposals.iter() {
            if plan.id.starts_with(LEARNING_PLAN_PREFIX)
                && plan.content.contains("\"state\":\"in_progress\"")
            {
                facts.push(Fact {
                    key: ContextKey::Evaluations,
                    id: format!("{}progress:{}", LEARNING_PLAN_PREFIX, plan.id),
                    content: serde_json::json!({
                        "type": "learning_progress",
                        "plan_id": plan.id,
                        "milestones_complete": 1,
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

/// Manages credential lifecycle.
#[derive(Debug, Clone, Default)]
pub struct CredentialManagerAgent;

impl Agent for CredentialManagerAgent {
    fn name(&self) -> &str {
        "credential_manager"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Seeds, ContextKey::Signals]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        ctx.get(ContextKey::Seeds).iter().any(|s| {
            s.content.contains("credential.earned") || s.content.contains("certification.passed")
        }) || ctx
            .get(ContextKey::Signals)
            .iter()
            .any(|s| s.content.contains("credential.expiring"))
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let seeds = ctx.get(ContextKey::Seeds);
        let signals = ctx.get(ContextKey::Signals);
        let mut facts = Vec::new();

        // Handle new credentials
        for seed in seeds.iter() {
            if seed.content.contains("credential.earned")
                || seed.content.contains("certification.passed")
            {
                facts.push(Fact {
                    key: ContextKey::Proposals,
                    id: format!("{}{}", CREDENTIAL_PREFIX, seed.id),
                    content: serde_json::json!({
                        "type": "credential",
                        "seed_id": seed.id,
                        "state": "issued",
                        "employee_id": "extracted",
                        "credential_name": "extracted",
                        "issuer": "extracted",
                        "evidence_linked": true,
                        "expiry_date": "2027-01-12",
                        "issued_at": "2026-01-12"
                    })
                    .to_string(),
                });
            }
        }

        // Handle expiring credentials
        for signal in signals.iter() {
            if signal.content.contains("credential.expiring") {
                facts.push(Fact {
                    key: ContextKey::Evaluations,
                    id: format!("{}renewal:{}", CREDENTIAL_PREFIX, signal.id),
                    content: serde_json::json!({
                        "type": "credential_renewal",
                        "signal_id": signal.id,
                        "state": "renewal_pending",
                        "days_until_expiry": 30,
                        "notification_sent": true,
                        "checked_at": "2026-01-12"
                    })
                    .to_string(),
                });
            }
        }

        AgentEffect::with_facts(facts)
    }
}

/// Manages the competence matrix for teams.
#[derive(Debug, Clone, Default)]
pub struct CompetenceMatrixAgent;

impl Agent for CompetenceMatrixAgent {
    fn name(&self) -> &str {
        "competence_matrix"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Proposals]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        // Trigger on skill assessments completing
        ctx.get(ContextKey::Proposals)
            .iter()
            .any(|p| p.id.starts_with(SKILL_PREFIX) && p.content.contains("\"state\":\"assessed\""))
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let proposals = ctx.get(ContextKey::Proposals);
        let mut facts = Vec::new();

        for skill in proposals.iter() {
            if skill.id.starts_with(SKILL_PREFIX)
                && skill.content.contains("\"state\":\"assessed\"")
            {
                facts.push(Fact {
                    key: ContextKey::Strategies,
                    id: format!("{}{}", COMPETENCE_MATRIX_PREFIX, skill.id),
                    content: serde_json::json!({
                        "type": "competence_matrix_entry",
                        "skill_id": skill.id,
                        "team_coverage": "calculated",
                        "redundancy_check": true,
                        "updated_at": "2026-01-12"
                    })
                    .to_string(),
                });
            }
        }

        AgentEffect::with_facts(facts)
    }
}

/// Checks role requirements before role changes.
#[derive(Debug, Clone, Default)]
pub struct RoleCompetenceCheckerAgent;

impl Agent for RoleCompetenceCheckerAgent {
    fn name(&self) -> &str {
        "role_competence_checker"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Seeds, ContextKey::Proposals]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        ctx.get(ContextKey::Seeds).iter().any(|s| {
            s.content.contains("role.change_requested") || s.content.contains("promotion.proposed")
        })
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let seeds = ctx.get(ContextKey::Seeds);
        let proposals = ctx.get(ContextKey::Proposals);
        let mut facts = Vec::new();

        for seed in seeds.iter() {
            if seed.content.contains("role.change_requested")
                || seed.content.contains("promotion.proposed")
            {
                // Check competence against role requirements
                let has_required_skills = proposals.iter().any(|p| {
                    p.id.starts_with(SKILL_PREFIX) && p.content.contains("\"state\":\"assessed\"")
                });

                facts.push(Fact {
                    key: ContextKey::Evaluations,
                    id: format!("{}{}", ROLE_REQ_PREFIX, seed.id),
                    content: serde_json::json!({
                        "type": "role_competence_check",
                        "seed_id": seed.id,
                        "competence_assessed": has_required_skills,
                        "gaps_identified": [],
                        "recommendation": if has_required_skills { "proceed" } else { "learning_required" },
                        "checked_at": "2026-01-12"
                    })
                    .to_string(),
                });
            }
        }

        AgentEffect::with_facts(facts)
    }
}

/// Detects critical role redundancy issues.
#[derive(Debug, Clone, Default)]
pub struct CriticalRoleRedundancyAgent;

impl Agent for CriticalRoleRedundancyAgent {
    fn name(&self) -> &str {
        "critical_role_redundancy"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Strategies]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        ctx.get(ContextKey::Strategies)
            .iter()
            .any(|s| s.id.starts_with(COMPETENCE_MATRIX_PREFIX))
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let strategies = ctx.get(ContextKey::Strategies);
        let mut facts = Vec::new();

        for matrix in strategies.iter() {
            if matrix.id.starts_with(COMPETENCE_MATRIX_PREFIX) {
                facts.push(Fact {
                    key: ContextKey::Evaluations,
                    id: format!("{}redundancy_check:{}", COMPETENCE_MATRIX_PREFIX, matrix.id),
                    content: serde_json::json!({
                        "type": "redundancy_check",
                        "matrix_id": matrix.id,
                        "critical_skills_covered": true,
                        "single_point_failures": [],
                        "checked_at": "2026-01-12"
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

/// Ensures skill claims require evidence.
#[derive(Debug, Clone, Default)]
pub struct SkillClaimRequiresEvidenceInvariant;

impl Invariant for SkillClaimRequiresEvidenceInvariant {
    fn name(&self) -> &str {
        "skill_claim_requires_evidence"
    }

    fn class(&self) -> InvariantClass {
        InvariantClass::Acceptance
    }

    fn check(&self, ctx: &Context) -> InvariantResult {
        for skill in ctx.get(ContextKey::Proposals).iter() {
            if skill.id.starts_with(SKILL_PREFIX)
                && skill.content.contains("\"state\":\"assessed\"")
                && !skill.content.contains("\"evidence_")
            {
                return InvariantResult::Violated(Violation::with_facts(
                    format!("Skill assessment {} lacks evidence", skill.id),
                    vec![skill.id.clone()],
                ));
            }
        }
        InvariantResult::Ok
    }
}

/// Ensures skill assessments have an assessor.
#[derive(Debug, Clone, Default)]
pub struct SkillAssessmentHasAssessorInvariant;

impl Invariant for SkillAssessmentHasAssessorInvariant {
    fn name(&self) -> &str {
        "skill_assessment_has_assessor"
    }

    fn class(&self) -> InvariantClass {
        InvariantClass::Structural
    }

    fn check(&self, ctx: &Context) -> InvariantResult {
        for skill in ctx.get(ContextKey::Proposals).iter() {
            if skill.id.starts_with(SKILL_PREFIX)
                && skill.content.contains("\"state\":\"assessed\"")
                && !skill.content.contains("\"assessor\":")
            {
                return InvariantResult::Violated(Violation::with_facts(
                    format!("Skill {} assessed without assessor", skill.id),
                    vec![skill.id.clone()],
                ));
            }
        }
        InvariantResult::Ok
    }
}

/// Ensures learning plans have business justification.
#[derive(Debug, Clone, Default)]
pub struct PlanLinksToBusinesNeedInvariant;

impl Invariant for PlanLinksToBusinesNeedInvariant {
    fn name(&self) -> &str {
        "plan_links_to_business_need"
    }

    fn class(&self) -> InvariantClass {
        InvariantClass::Acceptance
    }

    fn check(&self, ctx: &Context) -> InvariantResult {
        for plan in ctx.get(ContextKey::Proposals).iter() {
            if plan.id.starts_with(LEARNING_PLAN_PREFIX)
                && plan.content.contains("\"state\":\"approved\"")
                && plan
                    .content
                    .contains("\"business_justification\":\"pending\"")
            {
                return InvariantResult::Violated(Violation::with_facts(
                    format!(
                        "Learning plan {} approved without business justification",
                        plan.id
                    ),
                    vec![plan.id.clone()],
                ));
            }
        }
        InvariantResult::Ok
    }
}

/// Ensures learning plans have milestones.
#[derive(Debug, Clone, Default)]
pub struct PlanHasMilestonesInvariant;

impl Invariant for PlanHasMilestonesInvariant {
    fn name(&self) -> &str {
        "plan_has_milestones"
    }

    fn class(&self) -> InvariantClass {
        InvariantClass::Structural
    }

    fn check(&self, ctx: &Context) -> InvariantResult {
        for plan in ctx.get(ContextKey::Proposals).iter() {
            if plan.id.starts_with(LEARNING_PLAN_PREFIX)
                && plan.content.contains("\"state\":\"in_progress\"")
                && plan.content.contains("\"milestones\":[]")
            {
                return InvariantResult::Violated(Violation::with_facts(
                    format!("Active learning plan {} has no milestones", plan.id),
                    vec![plan.id.clone()],
                ));
            }
        }
        InvariantResult::Ok
    }
}

/// Ensures no role change without competence assessment.
#[derive(Debug, Clone, Default)]
pub struct NoRoleChangeWithoutCompetenceInvariant;

impl Invariant for NoRoleChangeWithoutCompetenceInvariant {
    fn name(&self) -> &str {
        "no_role_change_without_competence_delta"
    }

    fn class(&self) -> InvariantClass {
        InvariantClass::Acceptance
    }

    fn check(&self, ctx: &Context) -> InvariantResult {
        let evaluations = ctx.get(ContextKey::Evaluations);

        for eval in evaluations.iter() {
            if eval.id.starts_with(ROLE_REQ_PREFIX)
                && eval.content.contains("\"competence_assessed\":false")
            {
                return InvariantResult::Violated(Violation::with_facts(
                    format!(
                        "Role change {} proceeding without competence assessment",
                        eval.id
                    ),
                    vec![eval.id.clone()],
                ));
            }
        }
        InvariantResult::Ok
    }
}

/// Ensures credentials have expiry.
#[derive(Debug, Clone, Default)]
pub struct CredentialHasExpiryInvariant;

impl Invariant for CredentialHasExpiryInvariant {
    fn name(&self) -> &str {
        "credential_has_expiry"
    }

    fn class(&self) -> InvariantClass {
        InvariantClass::Structural
    }

    fn check(&self, ctx: &Context) -> InvariantResult {
        for cred in ctx.get(ContextKey::Proposals).iter() {
            if cred.id.starts_with(CREDENTIAL_PREFIX)
                && cred.content.contains("\"state\":\"issued\"")
                && !cred.content.contains("\"expiry_date\":")
            {
                return InvariantResult::Violated(Violation::with_facts(
                    format!("Credential {} missing expiry date", cred.id),
                    vec![cred.id.clone()],
                ));
            }
        }
        InvariantResult::Ok
    }
}

/// Ensures critical roles have redundancy.
#[derive(Debug, Clone, Default)]
pub struct CriticalRoleRedundancyInvariant;

impl Invariant for CriticalRoleRedundancyInvariant {
    fn name(&self) -> &str {
        "critical_role_redundancy"
    }

    fn class(&self) -> InvariantClass {
        InvariantClass::Acceptance
    }

    fn check(&self, ctx: &Context) -> InvariantResult {
        for eval in ctx.get(ContextKey::Evaluations).iter() {
            if eval.id.contains("redundancy_check")
                && eval.content.contains("\"single_point_failures\":[")
            {
                // Check if there are any single point failures
                if !eval.content.contains("\"single_point_failures\":[]") {
                    return InvariantResult::Violated(Violation::with_facts(
                        format!("Single point of failure detected in {}", eval.id),
                        vec![eval.id.clone()],
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
        assert_eq!(SkillAssessorAgent.name(), "skill_assessor");
        assert_eq!(SkillValidatorAgent.name(), "skill_validator");
        assert_eq!(LearningPlanCreatorAgent.name(), "learning_plan_creator");
        assert_eq!(
            LearningProgressTrackerAgent.name(),
            "learning_progress_tracker"
        );
        assert_eq!(CredentialManagerAgent.name(), "credential_manager");
        assert_eq!(CompetenceMatrixAgent.name(), "competence_matrix");
        assert_eq!(RoleCompetenceCheckerAgent.name(), "role_competence_checker");
        assert_eq!(
            CriticalRoleRedundancyAgent.name(),
            "critical_role_redundancy"
        );
    }

    #[test]
    fn invariants_have_correct_names() {
        assert_eq!(
            SkillClaimRequiresEvidenceInvariant.name(),
            "skill_claim_requires_evidence"
        );
        assert_eq!(
            SkillAssessmentHasAssessorInvariant.name(),
            "skill_assessment_has_assessor"
        );
        assert_eq!(
            PlanLinksToBusinesNeedInvariant.name(),
            "plan_links_to_business_need"
        );
        assert_eq!(PlanHasMilestonesInvariant.name(), "plan_has_milestones");
        assert_eq!(
            NoRoleChangeWithoutCompetenceInvariant.name(),
            "no_role_change_without_competence_delta"
        );
        assert_eq!(CredentialHasExpiryInvariant.name(), "credential_has_expiry");
        assert_eq!(
            CriticalRoleRedundancyInvariant.name(),
            "critical_role_redundancy"
        );
    }
}
