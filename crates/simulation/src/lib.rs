//! Simulation swarm.
//!
//! Parallel stress-testing of candidate plans before commit. Multiple
//! simulators run concurrently across five dimensions: outcome, cost,
//! policy, causal, operational. Each returns probability distributions,
//! not point estimates.
//!
//! Mirrors validation patterns from aircraft design, trading systems,
//! and chip design.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ── Simulation Result ──────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulationResult {
    pub plan_id: Uuid,
    pub runs: u32,
    pub dimensions: Vec<DimensionResult>,
    pub overall_confidence: f64,
    pub recommendation: SimulationRecommendation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DimensionResult {
    pub dimension: SimulationDimension,
    pub passed: bool,
    pub confidence: f64,
    pub findings: Vec<String>,
    pub samples: Vec<Sample>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SimulationDimension {
    Outcome,
    Cost,
    Policy,
    Causal,
    Operational,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SimulationRecommendation {
    Proceed,
    ProceedWithCaution,
    DoNotProceed,
}

// ── Sample ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Sample {
    pub value: f64,
    pub probability: f64,
}

// ── Simulation Report (legacy compat) ──────────────────────────────

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SimulationReport {
    pub results: Vec<SimulationResult>,
}

// ── Simulator Trait ────────────────────────────────────────────────

pub trait SimulationRunner: Send + Sync {
    fn dimension(&self) -> SimulationDimension;
    fn simulate(&self, plan: &serde_json::Value) -> DimensionResult;
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn plan_id() -> Uuid {
        Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap()
    }

    fn sample_dimension_result(dim: SimulationDimension, passed: bool) -> DimensionResult {
        DimensionResult {
            dimension: dim,
            passed,
            confidence: 0.85,
            findings: vec!["ok".into()],
            samples: vec![Sample {
                value: 1.0,
                probability: 0.9,
            }],
        }
    }

    #[test]
    fn simulation_dimension_all_variants_distinct() {
        let variants = [
            SimulationDimension::Outcome,
            SimulationDimension::Cost,
            SimulationDimension::Policy,
            SimulationDimension::Causal,
            SimulationDimension::Operational,
        ];
        for (i, a) in variants.iter().enumerate() {
            for (j, b) in variants.iter().enumerate() {
                assert_eq!(i == j, a == b);
            }
        }
    }

    #[test]
    fn recommendation_all_variants_distinct() {
        let variants = [
            SimulationRecommendation::Proceed,
            SimulationRecommendation::ProceedWithCaution,
            SimulationRecommendation::DoNotProceed,
        ];
        for (i, a) in variants.iter().enumerate() {
            for (j, b) in variants.iter().enumerate() {
                assert_eq!(i == j, a == b);
            }
        }
    }

    #[test]
    fn simulation_dimension_serde_snake_case() {
        let json = serde_json::to_string(&SimulationDimension::Outcome).unwrap();
        assert_eq!(json, "\"outcome\"");
        let json = serde_json::to_string(&SimulationDimension::Cost).unwrap();
        assert_eq!(json, "\"cost\"");
        let json = serde_json::to_string(&SimulationDimension::Operational).unwrap();
        assert_eq!(json, "\"operational\"");
    }

    #[test]
    fn recommendation_serde_snake_case() {
        let json = serde_json::to_string(&SimulationRecommendation::ProceedWithCaution).unwrap();
        assert_eq!(json, "\"proceed_with_caution\"");
        let json = serde_json::to_string(&SimulationRecommendation::DoNotProceed).unwrap();
        assert_eq!(json, "\"do_not_proceed\"");
    }

    #[test]
    fn sample_serde_roundtrip() {
        let s = Sample {
            value: 42.5,
            probability: 0.73,
        };
        let json = serde_json::to_string(&s).unwrap();
        let back: Sample = serde_json::from_str(&json).unwrap();
        assert!((back.value - 42.5).abs() < f64::EPSILON);
        assert!((back.probability - 0.73).abs() < f64::EPSILON);
    }

    #[test]
    fn dimension_result_serde_roundtrip() {
        let dr = sample_dimension_result(SimulationDimension::Cost, true);
        let json = serde_json::to_string(&dr).unwrap();
        let back: DimensionResult = serde_json::from_str(&json).unwrap();
        assert_eq!(back.dimension, SimulationDimension::Cost);
        assert!(back.passed);
        assert_eq!(back.findings.len(), 1);
        assert_eq!(back.samples.len(), 1);
    }

    #[test]
    fn dimension_result_empty_findings_and_samples() {
        let dr = DimensionResult {
            dimension: SimulationDimension::Policy,
            passed: false,
            confidence: 0.0,
            findings: vec![],
            samples: vec![],
        };
        let json = serde_json::to_string(&dr).unwrap();
        let back: DimensionResult = serde_json::from_str(&json).unwrap();
        assert!(back.findings.is_empty());
        assert!(back.samples.is_empty());
    }

    #[test]
    fn simulation_result_serde_roundtrip() {
        let sr = SimulationResult {
            plan_id: plan_id(),
            runs: 1000,
            dimensions: vec![
                sample_dimension_result(SimulationDimension::Outcome, true),
                sample_dimension_result(SimulationDimension::Cost, false),
            ],
            overall_confidence: 0.72,
            recommendation: SimulationRecommendation::ProceedWithCaution,
        };
        let json = serde_json::to_string(&sr).unwrap();
        let back: SimulationResult = serde_json::from_str(&json).unwrap();
        assert_eq!(back.plan_id, plan_id());
        assert_eq!(back.runs, 1000);
        assert_eq!(back.dimensions.len(), 2);
        assert_eq!(
            back.recommendation,
            SimulationRecommendation::ProceedWithCaution
        );
    }

    #[test]
    fn simulation_result_zero_runs() {
        let sr = SimulationResult {
            plan_id: plan_id(),
            runs: 0,
            dimensions: vec![],
            overall_confidence: 0.0,
            recommendation: SimulationRecommendation::DoNotProceed,
        };
        let json = serde_json::to_string(&sr).unwrap();
        let back: SimulationResult = serde_json::from_str(&json).unwrap();
        assert_eq!(back.runs, 0);
        assert!(back.dimensions.is_empty());
    }

    #[test]
    fn simulation_report_default_is_empty() {
        let report = SimulationReport::default();
        assert!(report.results.is_empty());
    }

    #[test]
    fn simulation_report_serde_roundtrip() {
        let report = SimulationReport {
            results: vec![SimulationResult {
                plan_id: plan_id(),
                runs: 50,
                dimensions: vec![],
                overall_confidence: 0.5,
                recommendation: SimulationRecommendation::Proceed,
            }],
        };
        let json = serde_json::to_string(&report).unwrap();
        let back: SimulationReport = serde_json::from_str(&json).unwrap();
        assert_eq!(back.results.len(), 1);
        assert_eq!(back.results[0].runs, 50);
    }

    proptest! {
        #[test]
        fn sample_roundtrips_reasonable_values(
            value in -1e15f64..1e15f64,
            probability in 0.0f64..=1.0,
        ) {
            let s = Sample { value, probability };
            let json = serde_json::to_string(&s).unwrap();
            let back: Sample = serde_json::from_str(&json).unwrap();
            prop_assert!((back.value - value).abs() < 1e-6 * value.abs().max(1.0));
            prop_assert!((back.probability - probability).abs() < f64::EPSILON);
        }

        #[test]
        fn confidence_survives_roundtrip(conf in 0.0f64..=1.0) {
            let sr = SimulationResult {
                plan_id: plan_id(),
                runs: 1,
                dimensions: vec![],
                overall_confidence: conf,
                recommendation: SimulationRecommendation::Proceed,
            };
            let json = serde_json::to_string(&sr).unwrap();
            let back: SimulationResult = serde_json::from_str(&json).unwrap();
            prop_assert!((back.overall_confidence - conf).abs() < f64::EPSILON);
        }
    }

    // NOTE: The `SimulationRunner` trait cannot be unit-tested here — it
    // requires concrete implementations. Tests for trait impls belong in
    // the crate that provides the implementation.
}
