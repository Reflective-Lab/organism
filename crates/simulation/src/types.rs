//! Typed vocabulary for simulation agents.
//!
//! Replaces string-based likelihood parsing and untyped JSON payloads
//! with compile-time checked types.

use converge_pack::FactId;
use serde::{Deserialize, Serialize};

use crate::SimulationDimension;

// ── Risk Likelihood ───────────────────────────────────────────────

/// Five-level risk likelihood scale with associated probabilities.
/// Replaces string matching like `"very_likely" | "VeryLikely" => 0.9`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RiskLikelihood {
    VeryLikely,
    Likely,
    Possible,
    Unlikely,
    Rare,
}

impl RiskLikelihood {
    /// Convert to probability for simulation.
    #[must_use]
    pub fn probability(&self) -> f64 {
        match self {
            Self::VeryLikely => 0.9,
            Self::Likely => 0.7,
            Self::Possible => 0.4,
            Self::Unlikely => 0.15,
            Self::Rare => 0.05,
        }
    }

    /// Parse from string, returning None for unknown values.
    #[must_use]
    pub fn from_str_lossy(s: &str) -> Option<Self> {
        match s {
            "very_likely" | "VeryLikely" => Some(Self::VeryLikely),
            "likely" | "Likely" => Some(Self::Likely),
            "possible" | "Possible" => Some(Self::Possible),
            "unlikely" | "Unlikely" => Some(Self::Unlikely),
            "rare" | "Rare" => Some(Self::Rare),
            _ => None,
        }
    }
}

// ── Simulation Verdict ────────────────────────────────────────────

/// Typed payload for simulation agent evaluations/constraints.
/// Replaces `serde_json::json!({...})` with compile-time structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulationVerdict {
    pub strategy_id: FactId,
    pub dimension: SimulationDimension,
    pub passed: bool,
    pub confidence: f64,
    pub findings: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recommendation: Option<SimulationRecommendation>,
}

/// Recommendation from a simulation verdict.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SimulationRecommendation {
    Proceed,
    ProceedWithCaution,
    DoNotProceed,
}

impl SimulationVerdict {
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).expect("SimulationVerdict is always serializable")
    }

    /// Create a fact ID from the verdict.
    pub fn fact_id(&self) -> String {
        let dim = match self.dimension {
            SimulationDimension::Outcome => "sim",
            SimulationDimension::Cost => "cost",
            SimulationDimension::Policy => "policy",
            SimulationDimension::Causal => "causal",
            SimulationDimension::Operational => "ops",
        };
        let result = if self.passed { "pass" } else { "fail" };
        format!("{dim}-{result}-{}", self.strategy_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn risk_likelihood_serde() {
        let json = serde_json::to_string(&RiskLikelihood::VeryLikely).unwrap();
        assert_eq!(json, "\"very_likely\"");

        let back: RiskLikelihood = serde_json::from_str("\"unlikely\"").unwrap();
        assert_eq!(back, RiskLikelihood::Unlikely);
    }

    #[test]
    fn risk_likelihood_probabilities() {
        assert!((RiskLikelihood::VeryLikely.probability() - 0.9).abs() < f64::EPSILON);
        assert!((RiskLikelihood::Rare.probability() - 0.05).abs() < f64::EPSILON);
    }

    #[test]
    fn risk_likelihood_from_str_lossy() {
        assert_eq!(
            RiskLikelihood::from_str_lossy("very_likely"),
            Some(RiskLikelihood::VeryLikely)
        );
        assert_eq!(
            RiskLikelihood::from_str_lossy("Likely"),
            Some(RiskLikelihood::Likely)
        );
        assert_eq!(RiskLikelihood::from_str_lossy("unknown"), None);
    }

    #[test]
    fn simulation_verdict_serde_roundtrip() {
        let verdict = SimulationVerdict {
            strategy_id: "strat-1".into(),
            dimension: SimulationDimension::Cost,
            passed: true,
            confidence: 0.85,
            findings: vec!["within budget".into()],
            recommendation: None,
        };

        let json = verdict.to_json();
        let back: SimulationVerdict = serde_json::from_str(&json).unwrap();
        assert_eq!(back.dimension, SimulationDimension::Cost);
        assert!(back.passed);
        assert!((back.confidence - 0.85).abs() < f64::EPSILON);
    }

    #[test]
    fn simulation_verdict_with_recommendation() {
        let verdict = SimulationVerdict {
            strategy_id: "s1".into(),
            dimension: SimulationDimension::Outcome,
            passed: false,
            confidence: 0.3,
            findings: vec![],
            recommendation: Some(SimulationRecommendation::DoNotProceed),
        };

        let json = verdict.to_json();
        assert!(json.contains("do_not_proceed"));
    }

    #[test]
    fn simulation_verdict_fact_id() {
        let verdict = SimulationVerdict {
            strategy_id: "abc".into(),
            dimension: SimulationDimension::Cost,
            passed: true,
            confidence: 0.9,
            findings: vec![],
            recommendation: None,
        };
        assert_eq!(verdict.fact_id(), "cost-pass-abc");

        let fail_verdict = SimulationVerdict {
            strategy_id: "xyz".into(),
            dimension: SimulationDimension::Operational,
            passed: false,
            confidence: 0.2,
            findings: vec![],
            recommendation: Some(SimulationRecommendation::DoNotProceed),
        };
        assert_eq!(fail_verdict.fact_id(), "ops-fail-xyz");
    }
}
