//! Typed vocabulary for adversarial agents.
//!
//! Replaces string-based agent identification, complexity levels,
//! and untyped JSON payloads with compile-time checked types.

use serde::{Deserialize, Serialize};

use crate::Severity;

// ── Agent Identity ────────────────────────────────────────────────

/// All organism agents that participate in convergence.
/// Used as a discriminator in evaluation/constraint payloads instead of
/// raw strings like `"assumption-breaker"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AgentId {
    AssumptionBreaker,
    ConstraintChecker,
    EconomicSkeptic,
    OperationalSkeptic,
    OutcomeSimulation,
    CostSimulation,
    PolicySimulation,
    CausalSimulation,
    OperationalSimulation,
    PlanningPrior,
}

impl AgentId {
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::AssumptionBreaker => "assumption-breaker",
            Self::ConstraintChecker => "constraint-checker",
            Self::EconomicSkeptic => "economic-skeptic",
            Self::OperationalSkeptic => "operational-skeptic",
            Self::OutcomeSimulation => "outcome-simulation",
            Self::CostSimulation => "cost-simulation",
            Self::PolicySimulation => "policy-simulation",
            Self::CausalSimulation => "causal-simulation",
            Self::OperationalSimulation => "operational-simulation",
            Self::PlanningPrior => "planning-prior",
        }
    }
}

impl std::fmt::Display for AgentId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

// ── Complexity ────────────────────────────────────────────────────

/// Complexity level for operational feasibility assessment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Complexity {
    Low,
    Medium,
    High,
    Critical,
}

impl Complexity {
    /// Minimum timeline (days) for this complexity level.
    #[must_use]
    pub fn min_timeline_days(&self) -> u32 {
        match self {
            Self::Low => 0,
            Self::Medium => 14,
            Self::High => 30,
            Self::Critical => 90,
        }
    }

    /// Minimum team size for this complexity level.
    #[must_use]
    pub fn min_team_size(&self) -> u32 {
        match self {
            Self::Low | Self::Medium => 1,
            Self::High => 3,
            Self::Critical => 5,
        }
    }
}

// ── Adversarial Verdict ───────────────────────────────────────────

/// Typed payload for adversarial agent evaluations/constraints.
/// Replaces `serde_json::json!({...})` with compile-time structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdversarialVerdict {
    pub strategy_id: String,
    pub agent: AgentId,
    pub kind: crate::SkepticismKind,
    pub passed: bool,
    pub severity: Severity,
    pub findings: Vec<String>,
}

impl AdversarialVerdict {
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).expect("AdversarialVerdict is always serializable")
    }

    /// Create a fact ID from the verdict.
    pub fn fact_id(&self, strategy_id: &str) -> String {
        let prefix = if self.passed { "pass" } else { "block" };
        format!("{}-{prefix}-{strategy_id}", self.agent)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SkepticismKind;

    #[test]
    fn agent_id_serde_kebab() {
        let json = serde_json::to_string(&AgentId::AssumptionBreaker).unwrap();
        assert_eq!(json, "\"assumption-breaker\"");

        let json = serde_json::to_string(&AgentId::OutcomeSimulation).unwrap();
        assert_eq!(json, "\"outcome-simulation\"");

        let back: AgentId = serde_json::from_str("\"economic-skeptic\"").unwrap();
        assert_eq!(back, AgentId::EconomicSkeptic);
    }

    #[test]
    fn agent_id_as_str_matches_serde() {
        for agent in [
            AgentId::AssumptionBreaker,
            AgentId::ConstraintChecker,
            AgentId::EconomicSkeptic,
            AgentId::OperationalSkeptic,
            AgentId::OutcomeSimulation,
            AgentId::CostSimulation,
            AgentId::PolicySimulation,
            AgentId::CausalSimulation,
            AgentId::OperationalSimulation,
            AgentId::PlanningPrior,
        ] {
            let serde_str = serde_json::to_string(&agent).unwrap();
            let expected = format!("\"{}\"", agent.as_str());
            assert_eq!(serde_str, expected);
        }
    }

    #[test]
    fn complexity_serde_snake_case() {
        let json = serde_json::to_string(&Complexity::Critical).unwrap();
        assert_eq!(json, "\"critical\"");

        let back: Complexity = serde_json::from_str("\"high\"").unwrap();
        assert_eq!(back, Complexity::High);
    }

    #[test]
    fn complexity_thresholds() {
        assert_eq!(Complexity::Critical.min_timeline_days(), 90);
        assert_eq!(Complexity::High.min_team_size(), 3);
        assert_eq!(Complexity::Critical.min_team_size(), 5);
    }

    #[test]
    fn adversarial_verdict_serde_roundtrip() {
        let verdict = AdversarialVerdict {
            strategy_id: "strat-1".into(),
            agent: AgentId::EconomicSkeptic,
            kind: SkepticismKind::EconomicSkepticism,
            passed: false,
            severity: Severity::Blocker,
            findings: vec!["budget exceeded".into()],
        };

        let json = verdict.to_json();
        let back: AdversarialVerdict = serde_json::from_str(&json).unwrap();
        assert_eq!(back.agent, AgentId::EconomicSkeptic);
        assert!(!back.passed);
        assert_eq!(back.findings.len(), 1);
    }

    #[test]
    fn adversarial_verdict_fact_id() {
        let verdict = AdversarialVerdict {
            strategy_id: "s1".into(),
            agent: AgentId::AssumptionBreaker,
            kind: SkepticismKind::AssumptionBreaking,
            passed: true,
            severity: Severity::Advisory,
            findings: vec![],
        };
        assert_eq!(verdict.fact_id("s1"), "assumption-breaker-pass-s1");
    }
}
