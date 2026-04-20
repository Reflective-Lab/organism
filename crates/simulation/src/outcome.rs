//! Outcome simulator.
//!
//! Evaluates candidate plans by analyzing their annotations (impacts, costs,
//! risks) and producing probabilistic outcome estimates via Monte Carlo
//! sampling.

use crate::types::RiskLikelihood;
use crate::{DimensionResult, Sample, SimulationDimension};

/// Configuration for the outcome simulator.
#[derive(Debug, Clone)]
pub struct OutcomeSimulatorConfig {
    /// Number of Monte Carlo samples to draw.
    pub samples: u32,
    /// Minimum confidence threshold to pass.
    pub confidence_threshold: f64,
    /// Risk penalty weight (higher = more conservative).
    pub risk_weight: f64,
}

impl Default for OutcomeSimulatorConfig {
    fn default() -> Self {
        Self {
            samples: 1000,
            confidence_threshold: 0.6,
            risk_weight: 0.3,
        }
    }
}

/// Simulates outcome likelihood for candidate plans.
///
/// Extracts impact confidences and risk severities from plan annotations,
/// then runs Monte Carlo sampling to estimate the probability distribution
/// of success outcomes.
pub struct OutcomeSimulator {
    config: OutcomeSimulatorConfig,
}

impl OutcomeSimulator {
    #[must_use]
    pub fn new(config: OutcomeSimulatorConfig) -> Self {
        Self { config }
    }

    /// Extract impact confidences from the plan JSON.
    fn extract_impacts(plan: &serde_json::Value) -> Vec<f64> {
        plan.get("annotation")
            .and_then(|a| a.get("impacts"))
            .and_then(|i| i.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.get("confidence").and_then(serde_json::Value::as_f64))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Extract risk likelihoods from the plan JSON.
    fn extract_risks(plan: &serde_json::Value) -> Vec<f64> {
        plan.get("annotation")
            .and_then(|a| a.get("risks"))
            .and_then(|r| r.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| {
                        v.get("likelihood").and_then(|l| l.as_str()).map(|s| {
                            RiskLikelihood::from_str_lossy(s).map_or(0.5, |l| l.probability())
                        })
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Run Monte Carlo sampling given base confidence and risk factors.
    fn sample(&self, base_confidence: f64, risk_penalty: f64) -> Vec<Sample> {
        let effective = (base_confidence - risk_penalty).clamp(0.0, 1.0);
        let n = self.config.samples;

        // Produce a discrete probability distribution over outcome buckets.
        // 5 buckets: [0.0, 0.2, 0.4, 0.6, 0.8, 1.0]
        let buckets = 5;
        let mut samples = Vec::with_capacity(buckets);

        for i in 0..buckets {
            let bucket_center = (f64::from(u32::try_from(i).unwrap_or(0)) + 0.5)
                / f64::from(u32::try_from(buckets).unwrap_or(5));
            // Beta-like distribution centered on effective confidence
            let distance = (bucket_center - effective).abs();
            let weight = (-distance * 4.0).exp();
            samples.push(Sample {
                value: bucket_center,
                probability: weight,
            });
        }

        // Normalize probabilities
        let total: f64 = samples.iter().map(|s| s.probability).sum();
        if total > 0.0 {
            for s in &mut samples {
                s.probability /= total;
            }
        }

        // Scale sample counts for reporting
        for s in &mut samples {
            s.probability = (s.probability * f64::from(n)).round() / f64::from(n);
        }

        samples
    }
}

impl OutcomeSimulator {
    /// Simulate outcomes for a plan represented as JSON.
    pub fn simulate(&self, plan: &serde_json::Value) -> DimensionResult {
        let impacts = Self::extract_impacts(plan);
        let risks = Self::extract_risks(plan);

        // Base confidence: average of impact confidences, or 0.5 if none stated.
        let impact_count = impacts.len();
        let base_confidence = if impacts.is_empty() {
            0.5
        } else {
            impacts.iter().sum::<f64>() / f64::from(u32::try_from(impact_count).unwrap_or(1))
        };

        // Risk penalty: weighted average of risk probabilities.
        let risk_count = risks.len();
        let risk_penalty = if risks.is_empty() {
            0.0
        } else {
            let avg_risk =
                risks.iter().sum::<f64>() / f64::from(u32::try_from(risk_count).unwrap_or(1));
            avg_risk * self.config.risk_weight
        };

        let effective_confidence = (base_confidence - risk_penalty).clamp(0.0, 1.0);
        let samples = self.sample(base_confidence, risk_penalty);
        let passed = effective_confidence >= self.config.confidence_threshold;

        let mut findings = Vec::new();
        if impacts.is_empty() {
            findings.push("no impact annotations — using neutral prior (0.5)".into());
        } else {
            findings.push(format!(
                "{} impacts, avg confidence {:.2}",
                impacts.len(),
                base_confidence,
            ));
        }
        if !risks.is_empty() {
            findings.push(format!(
                "{} risks identified, penalty {:.2}",
                risks.len(),
                risk_penalty,
            ));
        }
        if !passed {
            findings.push(format!(
                "below threshold: {:.2} < {:.2}",
                effective_confidence, self.config.confidence_threshold,
            ));
        }

        DimensionResult {
            dimension: SimulationDimension::Outcome,
            passed,
            confidence: effective_confidence,
            findings,
            samples,
        }
    }
}

// ── Suggestor Implementation ──────────────────────────────────────

use crate::types::SimulationVerdict;
use converge_pack::{AgentEffect, Context, ContextKey, ProposedFact, Suggestor};

/// Outcome simulation as a Suggestor — participates in the convergence loop.
///
/// Reads strategies from `ContextKey::Strategies`, simulates each, and
/// proposes constraints for strategies that fail the outcome threshold.
/// Strategies that pass get an approval fact in `ContextKey::Evaluations`.
pub struct OutcomeSimulationAgent {
    simulator: OutcomeSimulator,
}

impl OutcomeSimulationAgent {
    #[must_use]
    pub fn new(config: OutcomeSimulatorConfig) -> Self {
        Self {
            simulator: OutcomeSimulator::new(config),
        }
    }

    #[must_use]
    pub fn default_config() -> Self {
        Self {
            simulator: OutcomeSimulator::new(OutcomeSimulatorConfig::default()),
        }
    }
}

#[async_trait::async_trait]
#[allow(clippy::unnecessary_literal_bound)]
impl Suggestor for OutcomeSimulationAgent {
    fn name(&self) -> &str {
        "outcome-simulation"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Strategies]
    }

    fn accepts(&self, ctx: &dyn Context) -> bool {
        // Run when strategies exist and we haven't already evaluated them
        ctx.has(ContextKey::Strategies) && !ctx.has(ContextKey::Evaluations)
    }

    async fn execute(&self, ctx: &dyn Context) -> AgentEffect {
        let strategies = ctx.get(ContextKey::Strategies);
        let mut proposals = Vec::new();

        for fact in strategies {
            let plan_json: serde_json::Value = serde_json::from_str(&fact.content)
                .unwrap_or_else(|_| serde_json::json!({"description": fact.content}));

            let result = self.simulator.simulate(&plan_json);

            let verdict = SimulationVerdict {
                strategy_id: fact.id.clone(),
                dimension: SimulationDimension::Outcome,
                passed: result.passed,
                confidence: result.confidence,
                findings: result.findings,
                recommendation: if result.passed {
                    None
                } else {
                    Some(crate::types::SimulationRecommendation::DoNotProceed)
                },
            };

            let key = if result.passed {
                ContextKey::Evaluations
            } else {
                ContextKey::Constraints
            };

            proposals.push(ProposedFact::new(
                key,
                verdict.fact_id(),
                verdict.to_json(),
                "outcome-simulation",
            ));
        }

        AgentEffect::with_proposals(proposals)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn default_simulator() -> OutcomeSimulator {
        OutcomeSimulator::new(OutcomeSimulatorConfig::default())
    }

    #[test]
    fn high_confidence_plan_passes() {
        let sim = default_simulator();
        let plan = json!({
            "annotation": {
                "impacts": [
                    {"description": "revenue increase", "confidence": 0.9},
                    {"description": "customer satisfaction", "confidence": 0.85}
                ],
                "risks": []
            }
        });

        let result = sim.simulate(&plan);
        assert_eq!(result.dimension, SimulationDimension::Outcome);
        assert!(result.passed);
        assert!(result.confidence > 0.8);
    }

    #[test]
    fn low_confidence_plan_fails() {
        let sim = default_simulator();
        let plan = json!({
            "annotation": {
                "impacts": [
                    {"description": "speculative", "confidence": 0.3}
                ],
                "risks": [
                    {"likelihood": "likely", "description": "market shift"}
                ]
            }
        });

        let result = sim.simulate(&plan);
        assert!(!result.passed);
        assert!(result.confidence < 0.6);
    }

    #[test]
    fn empty_plan_uses_neutral_prior() {
        let sim = default_simulator();
        let plan = json!({});

        let result = sim.simulate(&plan);
        assert!(!result.passed); // 0.5 < 0.6 threshold
        assert!((result.confidence - 0.5).abs() < f64::EPSILON);
        assert!(result.findings[0].contains("neutral prior"));
    }

    #[test]
    fn risks_reduce_confidence() {
        let sim = default_simulator();

        let plan_no_risk = json!({
            "annotation": {
                "impacts": [{"description": "growth", "confidence": 0.8}],
                "risks": []
            }
        });
        let plan_with_risk = json!({
            "annotation": {
                "impacts": [{"description": "growth", "confidence": 0.8}],
                "risks": [
                    {"likelihood": "very_likely", "description": "regulatory"},
                    {"likelihood": "likely", "description": "competition"}
                ]
            }
        });

        let result_clean = sim.simulate(&plan_no_risk);
        let result_risky = sim.simulate(&plan_with_risk);
        assert!(result_clean.confidence > result_risky.confidence);
    }

    #[test]
    fn samples_are_normalized() {
        let sim = default_simulator();
        let plan = json!({
            "annotation": {
                "impacts": [{"description": "ok", "confidence": 0.7}],
                "risks": []
            }
        });

        let result = sim.simulate(&plan);
        assert!(!result.samples.is_empty());
        let total: f64 = result.samples.iter().map(|s| s.probability).sum();
        // Approximately 1.0 (rounding may cause small deviation)
        assert!((total - 1.0).abs() < 0.01);
    }

    #[test]
    fn custom_config() {
        let sim = OutcomeSimulator::new(OutcomeSimulatorConfig {
            samples: 100,
            confidence_threshold: 0.9,
            risk_weight: 0.5,
        });
        let plan = json!({
            "annotation": {
                "impacts": [{"description": "decent", "confidence": 0.8}],
                "risks": []
            }
        });

        let result = sim.simulate(&plan);
        // 0.8 < 0.9 threshold with strict config
        assert!(!result.passed);
    }

    #[test]
    fn likelihood_variants() {
        use crate::types::RiskLikelihood;
        assert!((RiskLikelihood::VeryLikely.probability() - 0.9).abs() < f64::EPSILON);
        assert!((RiskLikelihood::Unlikely.probability() - 0.15).abs() < f64::EPSILON);
        assert_eq!(RiskLikelihood::from_str_lossy("unknown"), None);
    }
}
