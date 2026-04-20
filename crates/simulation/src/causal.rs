//! Causal simulator.
//!
//! Evaluates candidate plans for causal reasoning quality: are the assumed
//! cause-effect relationships supported by evidence, or do they conflate
//! correlation with causation? Checks for confounders, missing links, and
//! circular reasoning.

use crate::{DimensionResult, Sample, SimulationDimension};

#[derive(Debug, Clone)]
pub struct CausalSimulatorConfig {
    pub min_evidence_links: u32,
    pub confounder_penalty: f64,
    pub confidence_threshold: f64,
}

impl Default for CausalSimulatorConfig {
    fn default() -> Self {
        Self {
            min_evidence_links: 1,
            confounder_penalty: 0.2,
            confidence_threshold: 0.5,
        }
    }
}

pub struct CausalSimulator {
    config: CausalSimulatorConfig,
}

impl CausalSimulator {
    #[must_use]
    pub fn new(config: CausalSimulatorConfig) -> Self {
        Self { config }
    }

    fn extract_causal_claims(plan: &serde_json::Value) -> Vec<CausalClaim> {
        plan.get("annotation")
            .and_then(|a| a.get("causal_claims"))
            .and_then(|c| c.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| {
                        Some(CausalClaim {
                            cause: v.get("cause").and_then(|c| c.as_str())?.to_string(),
                            effect: v.get("effect").and_then(|e| e.as_str())?.to_string(),
                            evidence_count: v
                                .get("evidence_count")
                                .and_then(serde_json::Value::as_u64)
                                .map_or(0, |n| u32::try_from(n).unwrap_or(0)),
                            confounders: v
                                .get("confounders")
                                .and_then(|c| c.as_array())
                                .map(|arr| {
                                    arr.iter()
                                        .filter_map(|s| s.as_str().map(String::from))
                                        .collect()
                                })
                                .unwrap_or_default(),
                        })
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    fn extract_assumptions(plan: &serde_json::Value) -> Vec<String> {
        plan.get("annotation")
            .and_then(|a| a.get("assumptions"))
            .and_then(|a| a.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default()
    }

    fn sample(confidence: f64) -> Vec<Sample> {
        let buckets = 5;
        let mut samples = Vec::with_capacity(buckets);

        for i in 0..buckets {
            let bucket_center = (f64::from(u32::try_from(i).unwrap_or(0)) + 0.5)
                / f64::from(u32::try_from(buckets).unwrap_or(5));
            let distance = (bucket_center - confidence).abs();
            let weight = (-distance * 4.0).exp();
            samples.push(Sample {
                value: bucket_center,
                probability: weight,
            });
        }

        let total: f64 = samples.iter().map(|s| s.probability).sum();
        if total > 0.0 {
            for s in &mut samples {
                s.probability /= total;
            }
        }

        samples
    }

    pub fn simulate(&self, plan: &serde_json::Value) -> DimensionResult {
        let claims = Self::extract_causal_claims(plan);
        let assumptions = Self::extract_assumptions(plan);

        let mut findings = Vec::new();
        let mut weak_claims = 0u32;
        let mut confounded_claims = 0u32;
        let total_claims = u32::try_from(claims.len()).unwrap_or(0);

        if claims.is_empty() && assumptions.is_empty() {
            findings.push("no causal claims or assumptions declared — cannot assess".into());
            return DimensionResult {
                dimension: SimulationDimension::Causal,
                passed: true, // no claims = nothing to challenge
                confidence: 0.5,
                findings,
                samples: Self::sample(0.5),
            };
        }

        for claim in &claims {
            if claim.evidence_count < self.config.min_evidence_links {
                findings.push(format!(
                    "weak: '{}' → '{}' has {} evidence link(s), need {}",
                    claim.cause, claim.effect, claim.evidence_count, self.config.min_evidence_links,
                ));
                weak_claims += 1;
            }

            if !claim.confounders.is_empty() {
                findings.push(format!(
                    "confounders on '{}' → '{}': {}",
                    claim.cause,
                    claim.effect,
                    claim.confounders.join(", "),
                ));
                confounded_claims += 1;
            }
        }

        if !assumptions.is_empty() {
            findings.push(format!("{} unstated assumptions noted", assumptions.len()));
        }

        // Check for circular reasoning (A→B and B→A)
        for (i, a) in claims.iter().enumerate() {
            for b in claims.iter().skip(i + 1) {
                if a.cause == b.effect && a.effect == b.cause {
                    findings.push(format!(
                        "circular: '{}' ↔ '{}' — mutual causation claimed",
                        a.cause, a.effect,
                    ));
                    weak_claims += 1;
                }
            }
        }

        let weakness_ratio = if total_claims == 0 {
            0.0
        } else {
            f64::from(weak_claims) / f64::from(total_claims)
        };
        let confounder_penalty = f64::from(confounded_claims) * self.config.confounder_penalty;

        let confidence = (1.0 - weakness_ratio - confounder_penalty).clamp(0.0, 1.0);
        let passed = confidence >= self.config.confidence_threshold;
        let samples = Self::sample(confidence);

        if !passed {
            findings.push(format!(
                "below threshold: {confidence:.2} < {:.2}",
                self.config.confidence_threshold,
            ));
        }

        DimensionResult {
            dimension: SimulationDimension::Causal,
            passed,
            confidence,
            findings,
            samples,
        }
    }
}

struct CausalClaim {
    cause: String,
    effect: String,
    evidence_count: u32,
    confounders: Vec<String>,
}

// ── Suggestor Implementation ──────────────────────────────────────

use crate::types::{SimulationRecommendation, SimulationVerdict};
use converge_pack::{AgentEffect, Context, ContextKey, ProposedFact, Suggestor};

pub struct CausalSimulationAgent {
    simulator: CausalSimulator,
}

impl CausalSimulationAgent {
    #[must_use]
    pub fn new(config: CausalSimulatorConfig) -> Self {
        Self {
            simulator: CausalSimulator::new(config),
        }
    }

    #[must_use]
    pub fn default_config() -> Self {
        Self {
            simulator: CausalSimulator::new(CausalSimulatorConfig::default()),
        }
    }
}

#[async_trait::async_trait]
#[allow(clippy::unnecessary_literal_bound)]
impl Suggestor for CausalSimulationAgent {
    fn name(&self) -> &'static str {
        "causal-simulation"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Strategies]
    }

    fn accepts(&self, ctx: &dyn Context) -> bool {
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
                dimension: crate::SimulationDimension::Causal,
                passed: result.passed,
                confidence: result.confidence,
                findings: result.findings,
                recommendation: if result.passed {
                    None
                } else {
                    Some(SimulationRecommendation::DoNotProceed)
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
                "causal-simulation",
            ));
        }

        AgentEffect::with_proposals(proposals)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn default_simulator() -> CausalSimulator {
        CausalSimulator::new(CausalSimulatorConfig::default())
    }

    #[test]
    fn strong_evidence_passes() {
        let sim = default_simulator();
        let plan = json!({
            "annotation": {
                "causal_claims": [{
                    "cause": "training",
                    "effect": "productivity",
                    "evidence_count": 5,
                    "confounders": []
                }]
            }
        });
        let result = sim.simulate(&plan);
        assert_eq!(result.dimension, SimulationDimension::Causal);
        assert!(result.passed);
        assert!(result.confidence > 0.8);
    }

    #[test]
    fn weak_evidence_penalized() {
        let sim = CausalSimulator::new(CausalSimulatorConfig {
            min_evidence_links: 3,
            ..CausalSimulatorConfig::default()
        });
        let plan = json!({
            "annotation": {
                "causal_claims": [{
                    "cause": "marketing",
                    "effect": "sales",
                    "evidence_count": 1,
                    "confounders": []
                }]
            }
        });
        let result = sim.simulate(&plan);
        assert!(!result.passed);
        assert!(result.findings.iter().any(|f| f.contains("weak")));
    }

    #[test]
    fn confounders_reduce_confidence() {
        let sim = default_simulator();
        let plan_clean = json!({
            "annotation": {
                "causal_claims": [{
                    "cause": "training",
                    "effect": "output",
                    "evidence_count": 5,
                    "confounders": []
                }]
            }
        });
        let plan_confounded = json!({
            "annotation": {
                "causal_claims": [{
                    "cause": "training",
                    "effect": "output",
                    "evidence_count": 5,
                    "confounders": ["seasonal_demand", "new_tools"]
                }]
            }
        });
        let clean = sim.simulate(&plan_clean);
        let confounded = sim.simulate(&plan_confounded);
        assert!(clean.confidence > confounded.confidence);
    }

    #[test]
    fn circular_reasoning_detected() {
        let sim = default_simulator();
        let plan = json!({
            "annotation": {
                "causal_claims": [
                    {"cause": "A", "effect": "B", "evidence_count": 2, "confounders": []},
                    {"cause": "B", "effect": "A", "evidence_count": 2, "confounders": []}
                ]
            }
        });
        let result = sim.simulate(&plan);
        assert!(result.findings.iter().any(|f| f.contains("circular")));
    }

    #[test]
    fn no_claims_passes_vacuously() {
        let sim = default_simulator();
        let plan = json!({});
        let result = sim.simulate(&plan);
        assert!(result.passed);
        assert!((result.confidence - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn assumptions_noted() {
        let sim = default_simulator();
        let plan = json!({
            "annotation": {
                "causal_claims": [{
                    "cause": "X",
                    "effect": "Y",
                    "evidence_count": 3,
                    "confounders": []
                }],
                "assumptions": ["stable market", "no regulation changes"]
            }
        });
        let result = sim.simulate(&plan);
        assert!(result.findings.iter().any(|f| f.contains("assumptions")));
    }
}
