//! Formation outcome records for learning and audit.

use converge_kernel::formation::{FormationKind, SuggestorCapability, SuggestorRole};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::compiler::{CompiledFormationPlan, GovernanceClass, ReplayMode};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FormationRunScope {
    pub plan_id: Uuid,
    pub correlation_id: Uuid,
    pub tenant_id: Option<String>,
    pub tournament_id: Option<Uuid>,
    pub candidate_id: Option<Uuid>,
}

impl FormationRunScope {
    pub fn from_compiled_plan(plan: &CompiledFormationPlan) -> Self {
        Self {
            plan_id: plan.plan_id,
            correlation_id: plan.correlation_id,
            tenant_id: plan.tenant_id.clone(),
            tournament_id: None,
            candidate_id: None,
        }
    }

    pub fn with_tournament_id(mut self, tournament_id: Uuid) -> Self {
        self.tournament_id = Some(tournament_id);
        self
    }

    pub fn with_candidate_id(mut self, candidate_id: Uuid) -> Self {
        self.candidate_id = Some(candidate_id);
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FormationOutcomeStatus {
    Planned,
    Converged,
    NeedsReview,
    CriteriaBlocked,
    BudgetExhausted,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct QualityScoreBps(u16);

impl QualityScoreBps {
    pub const MAX: u16 = 10_000;

    pub fn new(value: u16) -> Result<Self, QualityScoreError> {
        if value <= Self::MAX {
            Ok(Self(value))
        } else {
            Err(QualityScoreError::OutOfRange { value })
        }
    }

    pub fn as_u16(self) -> u16 {
        self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum QualityScoreError {
    #[error("quality score must be between 0 and 10000 bps, got {value}")]
    OutOfRange { value: u16 },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BusinessQualitySignal {
    pub metric: String,
    pub score: QualityScoreBps,
    pub evidence: String,
}

impl BusinessQualitySignal {
    pub fn new(
        metric: impl Into<String>,
        score: QualityScoreBps,
        evidence: impl Into<String>,
    ) -> Self {
        Self {
            metric: metric.into(),
            score,
            evidence: evidence.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OutcomeRosterMember {
    pub suggestor_id: String,
    pub role: SuggestorRole,
    pub capabilities: Vec<SuggestorCapability>,
    pub replay_mode: ReplayMode,
    pub governance_class: GovernanceClass,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OutcomeProviderAssignment {
    pub suggestor_id: String,
    pub role: SuggestorRole,
    pub provider_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FormationOutcomeRecord {
    pub scope: FormationRunScope,
    pub template_id: String,
    pub template_kind: FormationKind,
    pub roster: Vec<OutcomeRosterMember>,
    pub provider_assignments: Vec<OutcomeProviderAssignment>,
    pub status: FormationOutcomeStatus,
    pub stop_reason: Option<String>,
    pub gate_triggers: Vec<String>,
    pub quality_signal: Option<BusinessQualitySignal>,
    pub writeback_target: Option<String>,
}

impl FormationOutcomeRecord {
    pub fn from_compiled_plan(
        plan: &CompiledFormationPlan,
        status: FormationOutcomeStatus,
    ) -> Self {
        Self {
            scope: FormationRunScope::from_compiled_plan(plan),
            template_id: plan.template_id.clone(),
            template_kind: plan.template_kind,
            roster: plan
                .roster
                .iter()
                .map(|member| OutcomeRosterMember {
                    suggestor_id: member.suggestor_id.clone(),
                    role: member.role,
                    capabilities: member.capabilities.clone(),
                    replay_mode: member.replay_mode,
                    governance_class: member.governance_class,
                })
                .collect(),
            provider_assignments: plan
                .provider_assignments
                .iter()
                .map(|assignment| OutcomeProviderAssignment {
                    suggestor_id: assignment.suggestor_id.clone(),
                    role: assignment.role,
                    provider_id: assignment.provider_id.clone(),
                })
                .collect(),
            status,
            stop_reason: None,
            gate_triggers: Vec::new(),
            quality_signal: None,
            writeback_target: None,
        }
    }

    pub fn with_stop_reason(mut self, stop_reason: impl Into<String>) -> Self {
        self.stop_reason = Some(stop_reason.into());
        self
    }

    pub fn with_gate_trigger(mut self, gate_trigger: impl Into<String>) -> Self {
        self.gate_triggers.push(gate_trigger.into());
        self
    }

    pub fn with_quality_signal(mut self, signal: BusinessQualitySignal) -> Self {
        self.quality_signal = Some(signal);
        self
    }

    pub fn with_writeback_target(mut self, target: impl Into<String>) -> Self {
        self.writeback_target = Some(target.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiler::{CompiledSuggestorRole, RoleProviderAssignment};

    fn id(n: u128) -> Uuid {
        Uuid::from_u128(n)
    }

    #[test]
    fn quality_score_rejects_values_above_100_percent() {
        assert_eq!(QualityScoreBps::new(10_000).unwrap().as_u16(), 10_000);
        assert!(matches!(
            QualityScoreBps::new(10_001),
            Err(QualityScoreError::OutOfRange { value: 10_001 })
        ));
    }

    #[test]
    fn outcome_record_copies_compiled_plan_context() {
        let plan = CompiledFormationPlan {
            plan_id: id(1),
            correlation_id: id(2),
            tenant_id: Some("tenant-a".to_string()),
            template_id: "vendor-selection-decide".to_string(),
            template_kind: FormationKind::Static,
            roster: vec![CompiledSuggestorRole {
                suggestor_id: "decision-synthesis".to_string(),
                role: SuggestorRole::Synthesis,
                capabilities: vec![SuggestorCapability::LlmReasoning],
                reads: Vec::new(),
                writes: Vec::new(),
                input_contracts: Vec::new(),
                output_contracts: Vec::new(),
                replay_mode: ReplayMode::Preferred,
                governance_class: GovernanceClass::RegulatedDecision,
            }],
            provider_assignments: vec![RoleProviderAssignment {
                suggestor_id: "decision-synthesis".to_string(),
                role: SuggestorRole::Synthesis,
                provider_id: "reasoning-llm".to_string(),
                requirements: converge_provider_api::BackendRequirements::reasoning_llm(),
            }],
            trace: Vec::new(),
        };

        let record =
            FormationOutcomeRecord::from_compiled_plan(&plan, FormationOutcomeStatus::NeedsReview)
                .with_stop_reason("DPO approval required")
                .with_gate_trigger("dpo-gap-acceptance")
                .with_quality_signal(BusinessQualitySignal::new(
                    "audit_completeness",
                    QualityScoreBps::new(9_200).unwrap(),
                    "all score cells linked to source evidence",
                ))
                .with_writeback_target("decision://vendor-selection/demo");

        assert_eq!(record.scope.correlation_id, id(2));
        assert_eq!(record.scope.tenant_id.as_deref(), Some("tenant-a"));
        assert_eq!(record.roster[0].suggestor_id, "decision-synthesis");
        assert_eq!(record.provider_assignments[0].provider_id, "reasoning-llm");
        assert_eq!(record.gate_triggers, vec!["dpo-gap-acceptance"]);
        assert_eq!(
            record
                .quality_signal
                .as_ref()
                .map(|signal| signal.score.as_u16()),
            Some(9_200)
        );
    }
}
