//! Simulation swarm.
//!
//! Parallel stress-testing of candidate plans before commit. Multiple
//! simulators run concurrently across five dimensions: outcome, cost,
//! policy, causal, operational. Each returns probability distributions,
//! not point estimates.
//!
//! Mirrors validation patterns from aircraft design, trading systems,
//! and chip design.

pub mod causal;
pub mod cost;
pub mod operational;
pub mod outcome;
pub mod policy;
pub mod types;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub use causal::{CausalSimulationAgent, CausalSimulator, CausalSimulatorConfig};
pub use cost::{CostSimulationAgent, CostSimulator, CostSimulatorConfig};
pub use operational::{
    OperationalSimulationAgent, OperationalSimulator, OperationalSimulatorConfig,
};
pub use outcome::{OutcomeSimulationAgent, OutcomeSimulator, OutcomeSimulatorConfig};
pub use policy::{PolicySimulationAgent, PolicySimulator, PolicySimulatorConfig};
pub use types::{RiskLikelihood, SimulationVerdict};

// ── Simulation Result ──────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulationResult {
    pub plan_id: Uuid,
    pub runs: u32,
    pub dimensions: Vec<DimensionResult>,
    pub overall_confidence: f64,
    pub recommendation: SimulationRecommendation,
}

impl SimulationResult {
    /// Build a result with `overall_confidence` set to the mean of dimension
    /// confidences. The recommendation stays caller-supplied because its
    /// derivation rules are consumer-specific.
    #[must_use]
    pub fn from_dimensions(
        plan_id: Uuid,
        runs: u32,
        dimensions: Vec<DimensionResult>,
        recommendation: SimulationRecommendation,
    ) -> Self {
        let overall_confidence = DimensionResult::mean_confidence(&dimensions);
        Self {
            plan_id,
            runs,
            dimensions,
            overall_confidence,
            recommendation,
        }
    }

    /// Canonical JSON sub-payload that consumers embed under `"simulation"`
    /// inside their decision proposals.
    #[must_use]
    pub fn summary(&self) -> serde_json::Value {
        serde_json::json!({
            "overall_confidence": self.overall_confidence,
            "recommendation": format!("{:?}", self.recommendation),
            "dimensions": self
                .dimensions
                .iter()
                .map(|d| serde_json::json!({
                    "dimension": format!("{:?}", d.dimension),
                    "passed": d.passed,
                    "confidence": d.confidence,
                    "findings": d.findings,
                }))
                .collect::<Vec<_>>(),
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DimensionResult {
    pub dimension: SimulationDimension,
    pub passed: bool,
    pub confidence: f64,
    pub findings: Vec<String>,
    pub samples: Vec<Sample>,
}

impl DimensionResult {
    /// New dimension result with empty findings and samples.
    #[must_use]
    pub fn new(dimension: SimulationDimension, passed: bool, confidence: f64) -> Self {
        Self {
            dimension,
            passed,
            confidence,
            findings: vec![],
            samples: vec![],
        }
    }

    /// Append a finding line.
    #[must_use]
    pub fn with_finding(mut self, finding: impl Into<String>) -> Self {
        self.findings.push(finding.into());
        self
    }

    /// Replace findings.
    #[must_use]
    pub fn with_findings(mut self, findings: Vec<String>) -> Self {
        self.findings = findings;
        self
    }

    /// Append a sample.
    #[must_use]
    pub fn with_sample(mut self, sample: Sample) -> Self {
        self.samples.push(sample);
        self
    }

    /// Replace samples.
    #[must_use]
    pub fn with_samples(mut self, samples: Vec<Sample>) -> Self {
        self.samples = samples;
        self
    }

    /// Mean confidence across `dimensions`, or `0.0` for an empty slice.
    #[must_use]
    pub fn mean_confidence(dimensions: &[Self]) -> f64 {
        if dimensions.is_empty() {
            0.0
        } else {
            let sum: f64 = dimensions.iter().map(|d| d.confidence).sum();
            sum / f64::from(u32::try_from(dimensions.len()).unwrap_or(1))
        }
    }

    /// True when every dimension's `passed` is true (vacuously true for an
    /// empty slice).
    #[must_use]
    pub fn all_passed(dimensions: &[Self]) -> bool {
        dimensions.iter().all(|d| d.passed)
    }
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

// Simulation agents are Suggestors — see OutcomeSimulationAgent.
// No separate trait needed; the convergence loop IS the execution model.

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

    #[test]
    fn mean_confidence_empty_is_zero() {
        assert!((DimensionResult::mean_confidence(&[]) - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn mean_confidence_averages_dimensions() {
        let dims = vec![
            DimensionResult {
                dimension: SimulationDimension::Outcome,
                passed: true,
                confidence: 0.6,
                findings: vec![],
                samples: vec![],
            },
            DimensionResult {
                dimension: SimulationDimension::Cost,
                passed: true,
                confidence: 0.8,
                findings: vec![],
                samples: vec![],
            },
            DimensionResult {
                dimension: SimulationDimension::Policy,
                passed: false,
                confidence: 0.4,
                findings: vec![],
                samples: vec![],
            },
        ];
        let mean = DimensionResult::mean_confidence(&dims);
        assert!((mean - 0.6).abs() < f64::EPSILON);
    }

    #[test]
    fn all_passed_true_when_every_dimension_passed() {
        let dims = vec![
            sample_dimension_result(SimulationDimension::Outcome, true),
            sample_dimension_result(SimulationDimension::Cost, true),
        ];
        assert!(DimensionResult::all_passed(&dims));
    }

    #[test]
    fn all_passed_false_when_any_failed() {
        let dims = vec![
            sample_dimension_result(SimulationDimension::Outcome, true),
            sample_dimension_result(SimulationDimension::Cost, false),
        ];
        assert!(!DimensionResult::all_passed(&dims));
    }

    #[test]
    fn all_passed_vacuously_true_for_empty() {
        assert!(DimensionResult::all_passed(&[]));
    }

    #[test]
    fn dimension_result_new_has_empty_findings_and_samples() {
        let dr = DimensionResult::new(SimulationDimension::Cost, true, 0.85);
        assert_eq!(dr.dimension, SimulationDimension::Cost);
        assert!(dr.passed);
        assert!((dr.confidence - 0.85).abs() < f64::EPSILON);
        assert!(dr.findings.is_empty());
        assert!(dr.samples.is_empty());
    }

    #[test]
    fn dimension_result_with_finding_appends() {
        let dr = DimensionResult::new(SimulationDimension::Policy, true, 0.9)
            .with_finding("rule a passed")
            .with_finding("rule b passed");
        assert_eq!(dr.findings.len(), 2);
        assert_eq!(dr.findings[1], "rule b passed");
    }

    #[test]
    fn dimension_result_with_samples_replaces() {
        let dr = DimensionResult::new(SimulationDimension::Outcome, true, 0.7)
            .with_sample(Sample {
                value: 1.0,
                probability: 0.5,
            })
            .with_samples(vec![Sample {
                value: 2.0,
                probability: 0.9,
            }]);
        assert_eq!(dr.samples.len(), 1);
        assert!((dr.samples[0].value - 2.0).abs() < f64::EPSILON);
    }

    #[test]
    fn from_dimensions_sets_mean_confidence() {
        let dims = vec![
            sample_dimension_result(SimulationDimension::Outcome, true),
            sample_dimension_result(SimulationDimension::Cost, true),
        ];
        let result = SimulationResult::from_dimensions(
            plan_id(),
            10,
            dims,
            SimulationRecommendation::Proceed,
        );
        assert!((result.overall_confidence - 0.85).abs() < f64::EPSILON);
        assert_eq!(result.runs, 10);
        assert_eq!(result.recommendation, SimulationRecommendation::Proceed);
    }

    #[test]
    fn summary_has_canonical_shape() {
        let dims = vec![
            DimensionResult {
                dimension: SimulationDimension::Cost,
                passed: true,
                confidence: 0.9,
                findings: vec!["within budget".into()],
                samples: vec![],
            },
            DimensionResult {
                dimension: SimulationDimension::Policy,
                passed: false,
                confidence: 0.4,
                findings: vec![],
                samples: vec![],
            },
        ];
        let result = SimulationResult::from_dimensions(
            plan_id(),
            1,
            dims,
            SimulationRecommendation::ProceedWithCaution,
        );
        let summary = result.summary();
        assert!((summary["overall_confidence"].as_f64().unwrap() - 0.65).abs() < f64::EPSILON);
        assert_eq!(summary["recommendation"], "ProceedWithCaution");
        let dimensions = summary["dimensions"].as_array().expect("dimensions");
        assert_eq!(dimensions.len(), 2);
        assert_eq!(dimensions[0]["dimension"], "Cost");
        assert_eq!(dimensions[0]["passed"], true);
        assert!((dimensions[0]["confidence"].as_f64().unwrap() - 0.9).abs() < f64::EPSILON);
        assert_eq!(dimensions[0]["findings"][0], "within budget");
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
}
